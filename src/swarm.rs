use crate::gnome::NetworkSettings;
use crate::gnome_to_manager::GnomeToManager;
use crate::key_registry::KeyRegistry;
use crate::manager_to_gnome::ManagerToGnome;
use crate::multicast::Multicast;
use crate::policy::Policy;
use crate::requirement::Requirement;
use crate::CapabiliTree;
use crate::Capabilities;
use crate::Gnome;
use crate::GnomeId;
use crate::GnomeToApp;
use crate::Neighbor;
use crate::ToGnome;
use crate::{CastID, WrappedMessage};
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt;
use std::ops::Add;
use std::ops::Sub;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread::spawn;

#[derive(PartialEq, PartialOrd, Eq, Clone, Copy, Debug)]
pub struct SwarmTime(pub u32);

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum SwarmType {
    Catalog,
    Forum,
    UserDefined(u8),
}

#[derive(PartialEq, Eq, Hash, Debug, Copy, Clone)]
pub struct SwarmID(pub u8);

pub struct Swarm {
    pub name: String,
    pub id: SwarmID,
    pub swarm_type: SwarmType,
    pub founder: GnomeId,
    pub sender: Sender<ToGnome>,
    active_unicasts: HashSet<CastID>,
    active_broadcasts: HashMap<CastID, Multicast>,
    active_multicasts: HashMap<CastID, Multicast>,
    pub key_reg: KeyRegistry,
    pub capability_reg: HashMap<Capabilities, CapabiliTree>,
    pub policy_reg: HashMap<Policy, Requirement>,
    pub verify: fn(GnomeId, &Vec<u8>, SwarmTime, &mut Vec<u8>, &[u8]) -> bool,
    last_accepted_pubkey_chunk: (u8, u8),
    // TODO: This struct (or SwarmManifesto and/or attrs) should be provided by the user,
    // or some other mean like another Swarm functioning as a swarm catalogue,
    // and we only need to define Traits that particular attributes should be bounded to.
    // TODO: here we need to handle multiple things
    // - SwarmManifesto defines SwarmType, DataStructures (both static and dynamic)
    //   known gnomes that are easy to access for everyone (anchor gnomes)
    //   maybe synchronization broadcasts and definitions of capabilities,
    //   user defined *casts and groups.
    //   Synchronization *casts should distribute DataStructures to new gnomes
    //   that have just joined a swarm.
    //   Since *casts have only one source we should have a mechanism implemented
    //   that will allow other gnome to take over a *cast once gnome serving as
    //   a source of a given *cast becomes unresponsive. This should apply to
    //   synchronization *casts and other *casts that are essential to Swarm's
    //   health. Other *casts should be ended after set inactivity period.
    //   This could be all done with Reconfigure headers:
    //   - Once inactivity period is passed some gnome can claim ownership
    //   - This claim is being admitted to pending reconfigurations
    //   - Current gnome acting as a source for given *cast can Deny that
    //   claim with another Reconfigure message within some time period
    //   - If Deny message is accepted, pending Takeover is dismissed
    //   - In other case given *cast needs to be reconfigured for new source-gnome.

    //   Some possible DataStructures:
    //   - Capabilities - attributes that are assigned to groups to give it's members
    //   certain priviledges, there can be for example a capability that is required
    //   in order to start a Broadcast.
    //   - Groups - sets of gnomes that have certain capabilities
    //   - Multicasts
    //   - Broadcasts
    //   - SwarmType: i.e a catalog swarm, or a forum swarm or anything.
    //     We need a way to be able to define unlimited number of SwarmTypes.
    //     This can be done by having a handle to a Swarm that has those
    //     definitions,
    //     Predefined SwarmType definitions:
    //     - catalog swarm type
    //     - forum/discussion swarm type
    //
    //   - TypeDefinitions: descriptions of datatypes used by this swarm
    //     Universal type definitions used by every swarm:
    //     - a type containing SwarmType definitions
    //     - a type containing TypeDefinitions definitions
    //   - DataIdentifiers: id -> type mapping of data used by this swarm
}

impl Swarm {
    pub fn join(
        name: String,
        app_sync_hash: u64,
        id: SwarmID,
        gnome_id: GnomeId,
        pub_key_der: Vec<u8>,
        priv_key_pem: String,
        neighbors: Option<Vec<Neighbor>>,
        mgr_sender: Sender<GnomeToManager>,
        mgr_receiver: Receiver<ManagerToGnome>,
        band_receiver: Receiver<u64>,
        net_settings_send: Sender<NetworkSettings>,
        network_settings: NetworkSettings,
        verify: fn(GnomeId, &Vec<u8>, SwarmTime, &mut Vec<u8>, &[u8]) -> bool,
        sign: fn(&str, SwarmTime, &mut Vec<u8>) -> Result<Vec<u8>, ()>,
    ) -> (Sender<ToGnome>, Receiver<GnomeToApp>) {
        let (sender, request_receiver) = channel::<ToGnome>();
        let (response_sender, receiver) = channel::<GnomeToApp>();
        let mut policy_reg = HashMap::new();
        policy_reg.insert(Policy::Default, Requirement::None);
        policy_reg.insert(
            Policy::DataWithFirstByte(0),
            Requirement::Has(Capabilities::Founder),
        );
        // policy_reg.insert(Policy::Data, Requirement::Has(Capabilities::Admin));
        policy_reg.insert(Policy::Data, Requirement::None);

        let swarm = Swarm {
            name,
            id,
            swarm_type: SwarmType::Catalog,
            founder: GnomeId(0),
            sender: sender.clone(),
            active_unicasts: HashSet::new(),
            active_broadcasts: HashMap::new(),
            active_multicasts: HashMap::new(),
            verify,
            key_reg: KeyRegistry::new8(),
            capability_reg: HashMap::new(),
            policy_reg,
            last_accepted_pubkey_chunk: (0, 0),
        };
        let gnome = if let Some(neighbors) = neighbors {
            // println!("PubKey {} {}", pub_key_pem, pub_key_pem.len());
            Gnome::new_with_neighbors(
                gnome_id,
                pub_key_der,
                priv_key_pem,
                swarm,
                response_sender,
                request_receiver,
                mgr_sender,
                mgr_receiver,
                band_receiver,
                neighbors,
                network_settings,
                net_settings_send,
                sign,
            )
        } else {
            Gnome::new(
                gnome_id,
                pub_key_der,
                priv_key_pem,
                swarm,
                response_sender,
                request_receiver,
                mgr_sender,
                mgr_receiver,
                band_receiver,
                network_settings,
                net_settings_send,
                sign,
            )
        };
        let _join_handle = spawn(move || {
            gnome.do_your_job(app_sync_hash);
        });

        (sender, receiver)
    }

    pub fn is_unicast_id_available(&self, cast_id: CastID) -> bool {
        !self.active_unicasts.contains(&cast_id)
    }
    pub fn is_multicast_id_available(&self, cast_id: CastID) -> bool {
        !self.active_multicasts.contains_key(&cast_id)
    }
    pub fn is_broadcast_id_available(&self, cast_id: CastID) -> bool {
        !self.active_broadcasts.contains_key(&cast_id)
    }
    pub fn next_broadcast_id(&self) -> Option<CastID> {
        for id in 0..=255 {
            let cid = CastID(id);
            if self.is_broadcast_id_available(cid) {
                return Some(cid);
            }
        }
        None
    }
    pub fn insert_unicast(&mut self, cast_id: CastID) {
        self.active_unicasts.insert(cast_id);
    }
    pub fn insert_multicast(&mut self, cast_id: CastID, multicast: Multicast) {
        self.active_multicasts.insert(cast_id, multicast);
    }
    pub fn insert_broadcast(&mut self, cast_id: CastID, broadcast: Multicast) {
        self.active_broadcasts.insert(cast_id, broadcast);
    }

    pub fn insert_capability(&mut self, cap: Capabilities, mut id_list: Vec<GnomeId>) {
        id_list.reverse();
        if let Some(tree) = self.capability_reg.get_mut(&cap) {
            while let Some(gnome_id) = id_list.pop() {
                tree.insert(gnome_id);
            }
        } else {
            let mut tree = CapabiliTree::create();
            while let Some(gnome_id) = id_list.pop() {
                tree.insert(gnome_id);
            }
            self.capability_reg.insert(cap, tree);
        }
    }
    pub fn mark_pubkeys_unsynced(&mut self) {
        self.last_accepted_pubkey_chunk = (0, 1);
    }
    pub fn insert_pubkeys(
        &mut self,
        chunk_no: u8,
        total_chunks: u8,
        mut pairs: Vec<(GnomeId, Vec<u8>)>,
    ) -> bool {
        // TODO: add a mechanism to insert pubkeys in order
        // that will require storing last added chunk id
        // and a hashmap of pending keys
        // in case some chunk went missing
        // then we will need to send a request for specific chunk,
        // or, for now, simply send another SyncRequest with sync_pubkey
        // flag being the only one set to true
        if chunk_no == self.last_accepted_pubkey_chunk.0 + 1
            && self.last_accepted_pubkey_chunk.1 == total_chunks
        {
            self.last_accepted_pubkey_chunk = if chunk_no == total_chunks {
                (0, 0)
            } else {
                (chunk_no, total_chunks)
            };
            while let Some((gnome_id, pubkey)) = pairs.pop() {
                self.key_reg.insert(gnome_id, pubkey);
            }
            true
        } else if self.last_accepted_pubkey_chunk == (0, 1) {
            if chunk_no == 1 {
                self.last_accepted_pubkey_chunk = (chunk_no, total_chunks);
                while let Some((gnome_id, pubkey)) = pairs.pop() {
                    self.key_reg.insert(gnome_id, pubkey);
                }
                true
            } else {
                self.last_accepted_pubkey_chunk = (0, 0);
                println!("Pubkey registry chunk not in order!");
                false
            }
        } else {
            self.last_accepted_pubkey_chunk = (0, 0);
            println!("Pubkey registry chunk not in order!");
            false
        }
    }

    pub fn serve_casts(&mut self) -> bool {
        let mut any_data_processed = false;
        any_data_processed |= self.serve_broadcasts();
        // self.serve_unicasts();
        any_data_processed
    }

    pub fn broadcasts_count(&self) -> u8 {
        self.active_broadcasts.len() as u8
    }
    pub fn multicasts_count(&self) -> u8 {
        self.active_multicasts.len() as u8
    }
    pub fn broadcast_ids(&self) -> Vec<(CastID, GnomeId)> {
        let mut ids = vec![];
        for (id, b_cast) in &self.active_broadcasts {
            ids.push((*id, b_cast.origin()));
        }
        ids
    }

    pub fn multicast_ids(&self) -> Vec<(CastID, GnomeId)> {
        let mut ids = vec![];
        for (id, m_cast) in &self.active_multicasts {
            ids.push((*id, m_cast.origin()));
        }
        ids
    }

    pub fn add_subscriber(
        &mut self,
        is_bcast: bool,
        cast_id: &CastID,
        sub_id: GnomeId,
        send: Sender<WrappedMessage>,
    ) -> Option<GnomeId> {
        if is_bcast {
            if let Some(bcast) = self.active_broadcasts.get_mut(cast_id) {
                bcast.add_subscriber((sub_id, send));
                Some(bcast.origin())
            } else {
                None
            }
        } else if let Some(mcast) = self.active_multicasts.get_mut(cast_id) {
            mcast.add_subscriber((sub_id, send));
            Some(mcast.origin())
        } else {
            None
        }
    }

    pub fn check_config_policy(&self, gnome_id: &GnomeId, c_id: u8, swarm: &Swarm) -> bool {
        let policy = self.config_to_policy(c_id);
        if let Some(req) = swarm.policy_reg.get(&policy) {
            req.is_fullfilled(gnome_id, &swarm.capability_reg)
        } else {
            swarm
                .policy_reg
                .get(&Policy::Default)
                .unwrap()
                .is_fullfilled(gnome_id, &swarm.capability_reg)
        }
    }
    pub fn set_founder(&mut self, gnome_id: GnomeId) {
        self.founder = gnome_id;
        let mut tree = CapabiliTree::create();
        tree.insert(gnome_id);
        self.capability_reg.insert(Capabilities::Founder, tree);
    }
    pub fn check_data_policy(&self, gnome_id: &GnomeId, swarm: &Swarm, first_byte: u8) -> bool {
        if let Some(req) = swarm.policy_reg.get(&Policy::DataWithFirstByte(first_byte)) {
            req.is_fullfilled(gnome_id, &swarm.capability_reg)
        } else if let Some(req) = swarm.policy_reg.get(&Policy::Data) {
            req.is_fullfilled(gnome_id, &swarm.capability_reg)
        } else {
            swarm
                .policy_reg
                .get(&Policy::Default)
                .unwrap()
                .is_fullfilled(gnome_id, &swarm.capability_reg)
        }
    }

    pub fn capabilities_chunks(&self) -> Vec<Vec<(Capabilities, Vec<GnomeId>)>> {
        let mut total = vec![];
        let mut pairs = Vec::with_capacity(100);
        let mut avail_bytes = 1200;

        for c_id in self.capability_reg.keys() {
            let mut gnome_ids = self.capability_reg.get(c_id).unwrap().id_vec();
            gnome_ids.reverse();
            let mut gnomes_to_add = vec![];
            while let Some(g_id) = gnome_ids.pop() {
                if avail_bytes >= 9 {
                    gnomes_to_add.push(g_id);
                    avail_bytes -= 8;
                } else {
                    pairs.push((*c_id, gnomes_to_add));
                    total.push(pairs);
                    pairs = Vec::with_capacity(100);
                    gnomes_to_add = vec![];
                    avail_bytes = 1200;
                }
            }
            pairs.push((*c_id, gnomes_to_add));
        }
        if !pairs.is_empty() {
            total.push(pairs);
        }
        total
    }

    pub fn bytes_to_capabilities(bytes: &mut Vec<u8>) -> HashMap<Capabilities, Vec<GnomeId>> {
        let mut cap_drain = bytes.drain(0..1);
        let cap_len = cap_drain.next().unwrap() as usize;
        drop(cap_drain);

        let mut capa_map = HashMap::with_capacity(cap_len);
        for _i in 0..cap_len {
            cap_drain = bytes.drain(0..2);
            let cap_id = cap_drain.next().unwrap();
            let gnome_count = cap_drain.next().unwrap();
            let mut gnomes = Vec::with_capacity(gnome_count as usize);
            drop(cap_drain);
            for _i in 0..gnome_count {
                cap_drain = bytes.drain(0..8);
                let mut gnome_id: u64 = 0;
                for id_part in cap_drain.by_ref() {
                    gnome_id <<= 8;
                    gnome_id += id_part as u64;
                }
                gnomes.push(GnomeId(gnome_id));
                drop(cap_drain);
            }
            capa_map.insert(Capabilities::from(cap_id), gnomes);
        }
        capa_map
    }

    pub fn policy_chunks(&self) -> Vec<Vec<(Policy, Requirement)>> {
        let mut total = vec![];
        let mut pairs = Vec::with_capacity(100);
        let mut avail_bytes: u16 = 1200;

        for (p_id, req) in self.policy_reg.clone() {
            let req_len = req.len();
            if req_len as u16 + 1 <= avail_bytes {
                pairs.push((p_id, req));
                avail_bytes -= 1 + req_len as u16;
            } else {
                total.push(pairs);
                pairs = Vec::with_capacity(100);
                avail_bytes = 1200;
            }
        }
        if !pairs.is_empty() {
            total.push(pairs);
        }
        total
    }

    pub fn bytes_to_policy(bytes: &mut Vec<u8>) -> HashMap<Policy, Requirement> {
        let policy_count = bytes.drain(0..1).next().unwrap();
        let mut policies = HashMap::with_capacity(policy_count as usize);
        // for i in 0..policy_count {
        while !bytes.is_empty() {
            // let policy_byte = bytes.drain(0..1).next().unwrap();
            let policy = Policy::from(bytes);
            // let req_size = bytes.drain(0..1).next().unwrap();
            let requirement = Requirement::from(bytes);
            policies.insert(policy, requirement);
        }
        policies
    }

    fn config_to_policy(&self, c_id: u8) -> Policy {
        match c_id {
            254 => Policy::StartBroadcast,
            253 => Policy::ChangeBroadcastOrigin,
            252 => Policy::EndBroadcast,
            251 => Policy::StartMulticast,
            250 => Policy::ChangeMulticastOrigin,
            249 => Policy::EndMulticast,
            248 => Policy::CreateGroup,
            247 => Policy::DeleteGroup,
            246 => Policy::ModifyGroup,
            245 => Policy::InsertPubkey,
            other => Policy::UserDefined(other),
        }
    }

    fn serve_broadcasts(&mut self) -> bool {
        let mut any_data_processed = false;
        for bcast in self.active_broadcasts.values_mut() {
            any_data_processed |= bcast.serve();
        }
        any_data_processed
    }
    // fn serve_unicasts(&mut self) {
    //     for (c_id, c_recv) in self.active_unicasts.items() {
    //         bcast.serve();
    //     }
    // }

    fn all_possible_cast_ids(&self, for_unicast: bool) -> HashSet<CastID> {
        let mut ids = HashSet::new();
        // Unicast ids 255 & 254 reserved for NeighborRequest NeighborResponse
        // message exchange
        let max: u8 = if for_unicast { 253 } else { 255 };
        for i in 0..=max {
            let cast_id = CastID(i);
            ids.insert(cast_id);
        }
        ids
    }

    pub fn avail_unicast_ids(&self) -> HashSet<CastID> {
        let mut available = self.all_possible_cast_ids(true);
        for occupied in self.active_unicasts.iter() {
            available.remove(occupied);
        }
        available
    }
    pub fn avail_multicast_ids(&self) -> HashSet<CastID> {
        let mut available = self.all_possible_cast_ids(false);
        for occupied in self.active_multicasts.keys() {
            available.remove(occupied);
        }
        available
    }
    pub fn avail_broadcast_ids(&self) -> HashSet<CastID> {
        let mut available = self.all_possible_cast_ids(false);
        for occupied in self.active_broadcasts.keys() {
            available.remove(occupied);
        }
        available
    }
    // pub fn get_subscribers(&self, cast_id: &CastID, is_broadcast: bool) -> Vec<GnomeId> {
    //     let mut subscribers = vec![];
    //     if is_broadcast {
    //         if let Some(bcast) = self.active_broadcasts.get(cast_id) {
    //             subscribers = bcast.subscribers()
    //         }
    //     } else if let Some(mcast) = self.active_multicasts.get(cast_id) {
    //         subscribers = mcast.subscribers()
    //     }
    //     subscribers
    // }
    pub fn get_source(&self, cast_id: &CastID, is_broadcast: bool) -> Option<GnomeId> {
        let mut source = None;
        if is_broadcast {
            if let Some(bcast) = self.active_broadcasts.get(cast_id) {
                source = Some(bcast.source())
            }
        } else if let Some(mcast) = self.active_multicasts.get(cast_id) {
            source = Some(mcast.source())
        }
        source
    }
    pub fn set_source(
        &mut self,
        cast_id: &CastID,
        is_broadcast: bool,
        source: (GnomeId, Receiver<WrappedMessage>),
    ) {
        if is_broadcast {
            if let Some(bcast) = self.active_broadcasts.get_mut(cast_id) {
                bcast.set_source(source);
            }
        } else if let Some(mcast) = self.active_multicasts.get_mut(cast_id) {
            mcast.set_source(source);
        }
    }
}

impl fmt::Display for SwarmTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ST{:010}", self.0)
    }
}
impl SwarmTime {
    pub fn inc(&self) -> Self {
        SwarmTime(self.0.wrapping_add(1))
    }
}

impl Sub for SwarmTime {
    type Output = Self;
    fn sub(self, rhs: SwarmTime) -> Self::Output {
        SwarmTime(self.0.wrapping_sub(rhs.0))
    }
}

impl Add for SwarmTime {
    type Output = SwarmTime;
    fn add(self, rhs: SwarmTime) -> Self::Output {
        SwarmTime(self.0 + rhs.0)
    }
}

impl SwarmType {
    pub fn from(byte: u8) -> Self {
        match byte {
            255 => SwarmType::Catalog,
            254 => SwarmType::Forum,
            other => SwarmType::UserDefined(other),
        }
    }

    pub fn as_byte(&self) -> u8 {
        match self {
            SwarmType::Catalog => 255,
            SwarmType::Forum => 254,
            SwarmType::UserDefined(num) => *num,
        }
    }
}
