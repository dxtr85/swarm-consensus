// use crate::capabilities::CapabiLeaf;
use crate::DEFAULT_SWARM_DIAMETER;
// use crate::gnome::NetworkSettings;
use crate::gnome_to_manager::GnomeToManager;
use crate::key_registry::KeyRegistry;
use crate::manager_to_gnome::ManagerToGnome;
use crate::message::Header;
use crate::message::Payload;
use crate::multicast::Multicast;
use crate::policy::Policy;
use crate::requirement::Requirement;
use crate::CapabiliTree;
use crate::Capabilities;
use crate::Gnome;
use crate::GnomeId;
use crate::GnomeToApp;
use crate::Message;
use crate::Neighbor;
use crate::Signature;
use crate::ToGnome;
use crate::{CastID, WrappedMessage};
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt;
// use std::io::Read;
use std::ops::Add;
use std::ops::Sub;
use std::path::PathBuf;
use std::str::FromStr;
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
impl fmt::Display for SwarmID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "S={}", self.0)
    }
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct SwarmName {
    pub founder: GnomeId,
    pub name: String,
}

impl SwarmName {
    pub fn new(founder: GnomeId, name: String) -> Result<Self, ()> {
        // TODO: or maybe max len == 55 so that entire SwarmName does not exceed 64 bytes?
        if name.len() > 119 {
            Err(())
        } else {
            Ok(SwarmName { founder, name })
        }
    }
    pub fn from(v: &[u8]) -> Result<Self, ()> {
        // eprintln!("SwarmName from {} bytes: {:?}", v.len(), v);
        let name_len = v[0];
        if name_len < 128 {
            let founder = GnomeId::any();
            let name = String::from_utf8(v[1..1 + name_len as usize].try_into().unwrap()).unwrap();
            Ok(SwarmName { founder, name })
        } else {
            let founder: GnomeId = GnomeId(u64::from_be_bytes([
                v[1], v[2], v[3], v[4], v[5], v[6], v[7], v[8],
            ]));
            let name =
                String::from_utf8(v[9..name_len as usize - 127].try_into().unwrap()).unwrap();
            Ok(SwarmName { founder, name })
        }
    }
    pub fn as_bytes(&self) -> Vec<u8> {
        let name_len = self.name.len();
        let mut bytes = Vec::with_capacity(9 + name_len);
        if !self.founder.is_any() {
            bytes.push(name_len as u8 + 136);
            for byte in self.founder.0.to_be_bytes() {
                bytes.push(byte);
            }
        } else {
            bytes.push(name_len as u8);
        }
        for byte in self.name.clone().into_bytes() {
            bytes.push(byte);
        }
        bytes
    }
    pub fn to_path(&self) -> PathBuf {
        let mut s = self.name.clone();
        while s.starts_with('/') {
            s = String::from_str(&s[1..]).unwrap();
        }
        PathBuf::new().join(s).join(format!("{}", self.founder))
    }
}
impl fmt::Display for SwarmName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}", self.founder, self.name)
    }
}

#[derive(Debug, Clone)]
pub enum ByteSet {
    None,
    Bytes(HashSet<u8>),
    Pairs(HashSet<u16>),
}
impl ByteSet {
    pub fn empty() -> Self {
        ByteSet::None
    }
    pub fn new(set: HashSet<u8>) -> Self {
        ByteSet::Bytes(set)
    }
    pub fn new_pairs(set: HashSet<u16>) -> Self {
        ByteSet::Pairs(set)
    }
    pub fn contains(&self, b: &u8) -> bool {
        match self {
            ByteSet::Bytes(b_set) => b_set.contains(b),
            ByteSet::Pairs(_s) => {
                eprintln!("ByteSet byte check against pairs");
                false
            }
            ByteSet::None => {
                eprintln!("ByteSet byte check against empty");
                false
            }
        }
    }
    pub fn contains_pair(&self, b: &u16) -> bool {
        match self {
            ByteSet::Pairs(p_set) => p_set.contains(b),
            ByteSet::Bytes(_s) => {
                eprintln!("ByteSet pair check against bytes");
                false
            }
            ByteSet::None => {
                eprintln!("ByteSet pair check against empty");
                false
            }
        }
    }
    pub fn add(&mut self, b: u8) {
        let mut new_set = None;
        match self {
            ByteSet::None => {
                let mut a_set = HashSet::new();
                a_set.insert(b);
                new_set = Some(a_set);
            }
            ByteSet::Bytes(e_set) => {
                e_set.insert(b);
            }
            ByteSet::Pairs(_p) => {
                eprintln!("ByteSet add a byte to pair-set");
            }
        }
        if let Some(set) = new_set.take() {
            *self = ByteSet::Bytes(set);
        }
    }
    pub fn add_pair(&mut self, b: u16) {
        let mut new_set = None;
        match self {
            ByteSet::None => {
                let mut a_set = HashSet::new();
                a_set.insert(b);
                new_set = Some(a_set);
            }
            ByteSet::Bytes(_set) => {
                eprintln!("ByteSet add a pair to byte-set");
            }
            ByteSet::Pairs(e_set) => {
                e_set.insert(b);
            }
        }
        if let Some(set) = new_set.take() {
            *self = ByteSet::Pairs(set);
        }
    }
    pub fn is_none(&self) -> bool {
        matches!(self, ByteSet::None)
    }
    pub fn is_pair(&self) -> bool {
        matches!(self, ByteSet::Pairs(_p))
    }
    pub fn bytes(&self) -> Vec<u8> {
        match self {
            ByteSet::None => {
                vec![]
            }
            ByteSet::Bytes(_set) => {
                // let ordered = Vec::from(_set.clone()).sort();
                let mut ordered: Vec<u8> = _set.clone().into_iter().collect();
                ordered.sort();
                ordered
            }
            ByteSet::Pairs(e_set) => {
                let mut ordered: Vec<u16> = e_set.clone().into_iter().collect();
                ordered.sort();
                let mut bytes = Vec::with_capacity(2 * ordered.len());
                for val in ordered {
                    let vs = val.to_be_bytes();
                    bytes.push(vs[0]);
                    bytes.push(vs[1]);
                }
                bytes
            }
        }
    }
}
impl Default for ByteSet {
    fn default() -> Self {
        ByteSet::None
    }
}
pub struct Swarm {
    pub name: SwarmName,
    pub id: SwarmID,
    pub swarm_type: SwarmType,
    pub diameter: SwarmTime,
    pub sender: Sender<ToGnome>,
    active_unicasts: HashSet<CastID>,
    active_broadcasts: HashMap<CastID, Multicast>,
    active_multicasts: HashMap<CastID, Multicast>,
    pub key_reg: KeyRegistry,
    pub capability_reg: HashMap<Capabilities, CapabiliTree>,
    pub policy_reg: HashMap<Policy, Requirement>,
    pub byteset_reg: HashMap<u8, ByteSet>,
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
        name: SwarmName,
        // TODO: allow for swarm_diameter customization
        // Each swarm may have different diameter, accepted values 1-15,
        // where 1 means each Gnome has active communication channel to every other Gnome.
        // diameter: SwarmTime,
        id: SwarmID,
        gnome_id: GnomeId,
        pub_key_der: Vec<u8>,
        priv_key_pem: String,
        neighbors: Option<Vec<Neighbor>>,
        mgr_sender: Sender<GnomeToManager>,
        mgr_receiver: Receiver<ManagerToGnome>,
        // band_receiver: Receiver<u64>,
        // net_settings_send: Sender<NetworkSettings>,
        net_settings_send: Sender<Vec<u8>>,
        assigned_bandwidth: u64,
        verify: fn(GnomeId, &Vec<u8>, SwarmTime, &mut Vec<u8>, &[u8]) -> bool,
        sign: fn(&str, SwarmTime, &mut Vec<u8>) -> Result<Vec<u8>, ()>,
        sha_hash: fn(&[u8]) -> u64,
    ) -> (Sender<ToGnome>, Receiver<GnomeToApp>) {
        let (sender, request_receiver) = channel::<ToGnome>();
        let (response_sender, receiver) = channel::<GnomeToApp>();
        let mut policy_reg = HashMap::new();
        let diameter = DEFAULT_SWARM_DIAMETER;
        policy_reg.insert(Policy::Default, Requirement::Has(Capabilities::Founder));
        let mut capability_reg = HashMap::new();
        if !name.founder.is_any() {
            let mut ct = CapabiliTree::create();
            ct.insert(name.founder);
            capability_reg.insert(Capabilities::Founder, ct);
        }
        // policy_reg.insert(
        //     Policy::DataWithFirstByte(0),
        //     Requirement::Has(Capabilities::Founder),
        // );
        // policy_reg.insert(Policy::Data, Requirement::Has(Capabilities::Admin));
        // policy_reg.insert(Policy::Data, Requirement::None);

        let swarm = Swarm {
            name,
            id,
            swarm_type: SwarmType::Catalog,
            diameter,
            sender: sender.clone(),
            active_unicasts: HashSet::new(),
            active_broadcasts: HashMap::new(),
            active_multicasts: HashMap::new(),
            verify,
            key_reg: KeyRegistry::new8(),
            capability_reg,
            policy_reg,
            byteset_reg: HashMap::new(),
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
                // band_receiver,
                neighbors,
                // network_settings,
                net_settings_send,
                sign,
                sha_hash,
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
                // band_receiver,
                // network_settings,
                net_settings_send,
                sign,
                sha_hash,
            )
        };
        let _join_handle = spawn(move || {
            gnome.do_your_job(assigned_bandwidth);
            eprintln!("Gnome is done");
        });

        (sender, receiver)
    }
    pub fn verify_policy(&self, message: &Message) -> bool {
        match message.header {
            Header::Reconfigure(c_id, ref gnome_id) => {
                eprintln!("Verify Reconfigure policy...");
                self.check_config_policy(gnome_id, c_id)
            }
            Header::Block(_b_id) => {
                eprintln!("Block, payload: {}", message.payload);
                if let Payload::Block(_bid, ref _sign, ref data) = message.payload {
                    eprintln!("Verify Data policy...{}", _bid);
                    match _sign {
                        Signature::Regular(gnome_id, _s) => self.check_data_policy(
                            gnome_id,
                            data.first_byte(),
                            data.second_byte(),
                            data.third_byte(),
                        ),
                        Signature::Extended(gnome_id, _p, _s) => self.check_data_policy(
                            gnome_id,
                            data.first_byte(),
                            data.second_byte(),
                            data.third_byte(),
                        ),
                    }
                } else {
                    false
                }
            }
            _ => {
                eprintln!("Verify policy should not be called on {:?}", message);
                true
            }
        }
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
    pub fn next_multicast_id(&self) -> Option<CastID> {
        for id in 0..=255 {
            let cid = CastID(id);
            if self.is_multicast_id_available(cid) {
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

    //TODO: we may only wish to unsubscribe from our own cast,
    //      while still sending to others
    pub fn unsubscribe_cast(
        &mut self,
        own_gnome_id: GnomeId,
        is_bcast: bool,
        cast_id: &CastID,
    ) -> Option<(GnomeId, Vec<GnomeId>)> {
        if is_bcast {
            if let Some(mut cast) = self.active_broadcasts.remove(cast_id) {
                if own_gnome_id == cast.origin() {
                    cast.dont_send_to_app();
                    self.active_broadcasts.insert(*cast_id, cast);
                    None
                } else {
                    Some((cast.origin(), cast.subscribers()))
                }
            } else {
                None
            }
        } else if let Some(mut cast) = self.active_multicasts.remove(cast_id) {
            if own_gnome_id == cast.origin() {
                cast.dont_send_to_app();
                self.active_multicasts.insert(*cast_id, cast);
                None
            } else {
                Some((cast.origin(), cast.subscribers()))
            }
        } else {
            None
        }
    }
    pub fn remove_cast(
        &mut self,
        is_bcast: bool,
        cast_id: &CastID,
    ) -> Option<(GnomeId, Vec<GnomeId>)> {
        if is_bcast {
            if let Some(cast) = self.active_broadcasts.remove(cast_id) {
                Some((cast.source(), cast.subscribers()))
            } else {
                None
            }
        } else if let Some(cast) = self.active_multicasts.remove(cast_id) {
            Some((cast.source(), cast.subscribers()))
        } else {
            None
        }
    }

    pub fn insert_capability(&mut self, cap: Capabilities, mut id_list: Vec<GnomeId>) {
        eprintln!("Insert capability {:?}: {:?}", cap, id_list);
        if matches!(Capabilities::Founder, cap) {
            if let Some(gnome_id) = id_list.pop() {
                let mut tree = CapabiliTree::create();
                eprintln!("overwrite Founder");
                tree.insert(gnome_id);
                self.capability_reg.insert(cap, tree);
            }
            return;
        }
        id_list.reverse();
        if let Some(tree) = self.capability_reg.get_mut(&cap) {
            eprintln!("append");
            while let Some(gnome_id) = id_list.pop() {
                tree.insert(gnome_id);
            }
        } else {
            eprintln!("cnew");
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

    pub fn serve_casts(&mut self, available_tokens: u64) -> (bool, u64) {
        //Castings are not served in case we have no tokens available
        let mut any_data_processed = false;
        let mut total_tokens_used = 0;
        if available_tokens == 0 {
            return (any_data_processed, total_tokens_used);
        }
        let (was_busy, used_tokens) = self.serve_broadcasts(available_tokens);
        total_tokens_used += used_tokens;
        any_data_processed |= was_busy;
        let (was_busy, used_tokens) = self.serve_multicasts(available_tokens);
        total_tokens_used += used_tokens;
        any_data_processed |= was_busy;
        // let (was_busy, tokens_used) = self.serve_unicasts(tokens_remaining);
        (any_data_processed, total_tokens_used)
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

    pub fn remove_subscriber(&mut self, is_bcast: bool, cast_id: &CastID, sub_id: GnomeId) -> bool {
        if is_bcast {
            if let Some(bcast) = self.active_broadcasts.get_mut(cast_id) {
                bcast.remove_subscriber(&sub_id).is_some()
            } else {
                false
            }
        } else if let Some(mcast) = self.active_multicasts.get_mut(cast_id) {
            mcast.remove_subscriber(&sub_id).is_some()
        } else {
            false
        }
    }

    pub fn get_alt_sources(
        &mut self,
        is_bcast: bool,
        cast_id: &CastID,
        old_source: GnomeId,
    ) -> Vec<GnomeId> {
        if is_bcast {
            if let Some(bcast) = self.active_broadcasts.get_mut(cast_id) {
                bcast.get_alt_sources(old_source)
            } else {
                vec![]
            }
        } else if let Some(mcast) = self.active_multicasts.get_mut(cast_id) {
            mcast.get_alt_sources(old_source)
        } else {
            vec![]
        }
    }
    pub fn check_config_policy(&self, gnome_id: &GnomeId, c_id: u8) -> bool {
        let policy = self.config_to_policy(c_id);
        if let Some(req) = self.policy_reg.get(&policy) {
            req.is_fullfilled(
                gnome_id,
                &self.capability_reg,
                &self.byteset_reg,
                None,
                None,
            )
        } else {
            self.policy_reg
                .get(&Policy::Default)
                .unwrap()
                .is_fullfilled(
                    gnome_id,
                    &self.capability_reg,
                    &self.byteset_reg,
                    None,
                    None,
                )
        }
    }
    pub fn set_founder(&mut self, gnome_id: GnomeId) {
        eprintln!(
            "Swarm {} setting founder from: {} to: {}",
            self.name.name, self.name.founder, gnome_id
        );
        self.name.founder = gnome_id;
        let mut tree = CapabiliTree::create();
        tree.insert(gnome_id);
        self.capability_reg.insert(Capabilities::Founder, tree);
    }
    pub fn check_data_policy(
        &self,
        gnome_id: &GnomeId,
        first_byte: u8,
        second_byte: u8,
        third_byte: u8,
    ) -> bool {
        eprintln!("founder: {}", self.name.founder);
        eprintln!("g_id: {}", gnome_id);
        if let Some(req) = self.policy_reg.get(&Policy::DataWithFirstByte(first_byte)) {
            eprintln!("poli 1, {:?}", req);
            req.is_fullfilled(
                gnome_id,
                &self.capability_reg,
                &self.byteset_reg,
                Some(second_byte),
                Some(third_byte),
            )
        } else if let Some(req) = self.policy_reg.get(&Policy::Data) {
            eprintln!("poli 2, {:?}", req);
            req.is_fullfilled(
                gnome_id,
                &self.capability_reg,
                &self.byteset_reg,
                Some(second_byte),
                Some(third_byte),
            )
        } else {
            // eprintln!(
            //     "default: {:?}\ncapa: {:?}",
            //     self.policy_reg.get(&Policy::Default),
            //     self.capability_reg
            //         .get(&Capabilities::Founder)
            //         .unwrap()
            //         .contains(&self.name.founder)
            // );
            self.policy_reg
                .get(&Policy::Default)
                .unwrap()
                .is_fullfilled(
                    gnome_id,
                    &self.capability_reg,
                    &self.byteset_reg,
                    Some(second_byte),
                    Some(third_byte),
                )
        }
    }
    pub fn set_policy(&mut self, pol: Policy, req: Requirement) {
        eprintln!("Swarm is setting new policy: {:?}", pol);
        self.policy_reg.insert(pol, req);
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

    fn serve_broadcasts(&mut self, available_tokens: u64) -> (bool, u64) {
        let mut any_data_processed = false;
        let mut total_tokens_used = 0;
        let mut tokens_remaining = available_tokens;
        for bcast in self.active_broadcasts.values_mut() {
            let (was_busy, used_tokens) = bcast.serve(tokens_remaining);
            any_data_processed |= was_busy;
            total_tokens_used += used_tokens;
            // if tokens_remaining == 0 {
            if total_tokens_used >= available_tokens {
                break;
            }
            tokens_remaining -= used_tokens;
        }
        (any_data_processed, total_tokens_used)
    }
    fn serve_multicasts(&mut self, available_tokens: u64) -> (bool, u64) {
        let mut any_data_processed = false;
        let mut total_tokens_used = 0;
        let mut tokens_remaining = available_tokens;
        for mcast in self.active_multicasts.values_mut() {
            let (was_busy, used_tokens) = mcast.serve(tokens_remaining);
            any_data_processed |= was_busy;
            total_tokens_used += used_tokens;
            // if tokens_remaining == 0 {
            if total_tokens_used >= available_tokens {
                break;
            }
            tokens_remaining -= used_tokens;
        }
        (any_data_processed, total_tokens_used)
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
