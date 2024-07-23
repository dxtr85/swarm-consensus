use crate::gnome::NetworkSettings;
use crate::gnome_to_manager::GnomeToManager;
use crate::manager_to_gnome::ManagerToGnome;
use crate::multicast::Multicast;
use crate::Gnome;
use crate::GnomeId;
use crate::Neighbor;
use crate::Request;
use crate::Response;
use crate::{CastID, WrappedMessage};
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt;
use std::ops::Add;
use std::ops::DerefMut;
use std::ops::Sub;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread::spawn;

#[derive(PartialEq, PartialOrd, Eq, Clone, Copy, Debug)]
pub struct SwarmTime(pub u32);

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

// #[derive(Clone)]
// pub enum PubKey {
//     Empty,
//     Der(Vec<u8>),
// }

pub enum KeyRegistry {
    Disabled,
    Reg32(Box<[(GnomeId, Vec<u8>); 32]>),
    Reg64(Box<[(GnomeId, Vec<u8>); 64]>),
    Reg128(Box<[(GnomeId, Vec<u8>); 128]>),
    Reg256(Box<[(GnomeId, Vec<u8>); 256]>),
    Reg512(Box<[(GnomeId, Vec<u8>); 512]>),
    Reg1024(Box<[(GnomeId, Vec<u8>); 1024]>),
    Reg2048(Box<[(GnomeId, Vec<u8>); 2048]>),
    Reg4096(Box<[(GnomeId, Vec<u8>); 4096]>),
    Reg8192(Box<[(GnomeId, Vec<u8>); 8192]>),
}

fn insert32(gnome_id: GnomeId, key: Vec<u8>, array: &mut [(GnomeId, Vec<u8>); 32]) {
    let mut temp = std::mem::replace(&mut array[0], (gnome_id, key));
    for elem in &mut array[1..] {
        if temp.0 == gnome_id || temp.0 .0 == 0 {
            println!("Insert complete: {:?}", temp.0);
            break;
        }
        temp = std::mem::replace(elem, temp);
    }
}
fn insert64(gnome_id: GnomeId, key: Vec<u8>, array: &mut [(GnomeId, Vec<u8>); 64]) {
    let mut temp = std::mem::replace(&mut array[0], (gnome_id, key));
    for elem in &mut array[1..] {
        if temp.0 == gnome_id || temp.0 .0 == 0 {
            break;
        }
        temp = std::mem::replace(elem, temp);
    }
}
fn insert128(gnome_id: GnomeId, key: Vec<u8>, array: &mut [(GnomeId, Vec<u8>); 128]) {
    let mut temp = std::mem::replace(&mut array[0], (gnome_id, key));
    for elem in &mut array[1..] {
        if temp.0 == gnome_id || temp.0 .0 == 0 {
            break;
        }
        temp = std::mem::replace(elem, temp);
    }
}
fn insert256(gnome_id: GnomeId, key: Vec<u8>, array: &mut [(GnomeId, Vec<u8>); 256]) {
    let mut temp = std::mem::replace(&mut array[0], (gnome_id, key));
    for elem in &mut array[1..] {
        if temp.0 == gnome_id || temp.0 .0 == 0 {
            break;
        }
        temp = std::mem::replace(elem, temp);
    }
}
fn insert512(gnome_id: GnomeId, key: Vec<u8>, array: &mut [(GnomeId, Vec<u8>); 512]) {
    let mut temp = std::mem::replace(&mut array[0], (gnome_id, key));
    for elem in &mut array[1..] {
        if temp.0 == gnome_id || temp.0 .0 == 0 {
            break;
        }
        temp = std::mem::replace(elem, temp);
    }
}
fn insert1024(gnome_id: GnomeId, key: Vec<u8>, array: &mut [(GnomeId, Vec<u8>); 1024]) {
    let mut temp = std::mem::replace(&mut array[0], (gnome_id, key));
    for elem in &mut array[1..] {
        if temp.0 == gnome_id || temp.0 .0 == 0 {
            break;
        }
        temp = std::mem::replace(elem, temp);
    }
}
fn insert2048(gnome_id: GnomeId, key: Vec<u8>, array: &mut [(GnomeId, Vec<u8>); 2048]) {
    let mut temp = std::mem::replace(&mut array[0], (gnome_id, key));
    for elem in &mut array[1..] {
        if temp.0 == gnome_id || temp.0 .0 == 0 {
            break;
        }
        temp = std::mem::replace(elem, temp);
    }
}
fn insert4096(gnome_id: GnomeId, key: Vec<u8>, array: &mut [(GnomeId, Vec<u8>); 4096]) {
    let mut temp = std::mem::replace(&mut array[0], (gnome_id, key));
    for elem in &mut array[1..] {
        if temp.0 == gnome_id || temp.0 .0 == 0 {
            break;
        }
        temp = std::mem::replace(elem, temp);
    }
}
fn insert8192(gnome_id: GnomeId, key: Vec<u8>, array: &mut [(GnomeId, Vec<u8>); 8192]) {
    let mut temp = std::mem::replace(&mut array[0], (gnome_id, key));
    for elem in &mut array[1..] {
        if temp.0 == gnome_id || temp.0 .0 == 0 {
            break;
        }
        temp = std::mem::replace(elem, temp);
    }
}
fn get32(gnome_id: GnomeId, array: &[(GnomeId, Vec<u8>); 32]) -> Option<Vec<u8>> {
    for (id, key) in array {
        if id.0 == gnome_id.0 {
            println!("Found: {:?}", id);
            return Some(key.clone());
        }
    }
    None
}
fn get64(gnome_id: GnomeId, array: &[(GnomeId, Vec<u8>); 64]) -> Option<Vec<u8>> {
    for (id, key) in array {
        if id.0 == gnome_id.0 {
            println!("Found: {:?}", id);
            return Some(key.clone());
        }
    }
    None
}
fn get128(gnome_id: GnomeId, array: &[(GnomeId, Vec<u8>); 128]) -> Option<Vec<u8>> {
    for (id, key) in array {
        if id.0 == gnome_id.0 {
            println!("Found: {:?}", id);
            return Some(key.clone());
        }
    }
    None
}
fn get256(gnome_id: GnomeId, array: &[(GnomeId, Vec<u8>); 256]) -> Option<Vec<u8>> {
    for (id, key) in array {
        if id.0 == gnome_id.0 {
            println!("Found: {:?}", id);
            return Some(key.clone());
        }
    }
    None
}
fn get512(gnome_id: GnomeId, array: &[(GnomeId, Vec<u8>); 512]) -> Option<Vec<u8>> {
    for (id, key) in array {
        if id.0 == gnome_id.0 {
            println!("Found: {:?}", id);
            return Some(key.clone());
        }
    }
    None
}
fn get1024(gnome_id: GnomeId, array: &[(GnomeId, Vec<u8>); 1024]) -> Option<Vec<u8>> {
    for (id, key) in array {
        if id.0 == gnome_id.0 {
            println!("Found: {:?}", id);
            return Some(key.clone());
        }
    }
    None
}
fn get2048(gnome_id: GnomeId, array: &[(GnomeId, Vec<u8>); 2048]) -> Option<Vec<u8>> {
    for (id, key) in array {
        if id.0 == gnome_id.0 {
            println!("Found: {:?}", id);
            return Some(key.clone());
        }
    }
    None
}
fn get4096(gnome_id: GnomeId, array: &[(GnomeId, Vec<u8>); 4096]) -> Option<Vec<u8>> {
    for (id, key) in array {
        if id.0 == gnome_id.0 {
            println!("Found: {:?}", id);
            return Some(key.clone());
        }
    }
    None
}
fn get8192(gnome_id: GnomeId, array: &[(GnomeId, Vec<u8>); 8192]) -> Option<Vec<u8>> {
    for (id, key) in array {
        if id.0 == gnome_id.0 {
            println!("Found: {:?}", id);
            return Some(key.clone());
        }
    }
    None
}
fn has32(gnome_id: GnomeId, array: &[(GnomeId, Vec<u8>); 32]) -> bool {
    for (id, _key) in array {
        if id.0 == gnome_id.0 {
            println!("Found: {:?}", id);
            return true;
        }
    }
    false
}
fn has64(gnome_id: GnomeId, array: &[(GnomeId, Vec<u8>); 64]) -> bool {
    for (id, _key) in array {
        if id.0 == gnome_id.0 {
            println!("Found: {:?}", id);
            return true;
        }
    }
    false
}
fn has128(gnome_id: GnomeId, array: &[(GnomeId, Vec<u8>); 128]) -> bool {
    for (id, _key) in array {
        if id.0 == gnome_id.0 {
            println!("Found: {:?}", id);
            return true;
        }
    }
    false
}
fn has256(gnome_id: GnomeId, array: &[(GnomeId, Vec<u8>); 256]) -> bool {
    for (id, _key) in array {
        if id.0 == gnome_id.0 {
            println!("Found: {:?}", id);
            return true;
        }
    }
    false
}
fn has512(gnome_id: GnomeId, array: &[(GnomeId, Vec<u8>); 512]) -> bool {
    for (id, _key) in array {
        if id.0 == gnome_id.0 {
            println!("Found: {:?}", id);
            return true;
        }
    }
    false
}
fn has1024(gnome_id: GnomeId, array: &[(GnomeId, Vec<u8>); 1024]) -> bool {
    for (id, _key) in array {
        if id.0 == gnome_id.0 {
            println!("Found: {:?}", id);
            return true;
        }
    }
    false
}
fn has2048(gnome_id: GnomeId, array: &[(GnomeId, Vec<u8>); 2048]) -> bool {
    for (id, _key) in array {
        if id.0 == gnome_id.0 {
            println!("Found: {:?}", id);
            return true;
        }
    }
    false
}
fn has4096(gnome_id: GnomeId, array: &[(GnomeId, Vec<u8>); 4096]) -> bool {
    for (id, _key) in array {
        if id.0 == gnome_id.0 {
            println!("Found: {:?}", id);
            return true;
        }
    }
    false
}
fn has8192(gnome_id: GnomeId, array: &[(GnomeId, Vec<u8>); 8192]) -> bool {
    for (id, _key) in array {
        if id.0 == gnome_id.0 {
            println!("Found: {:?}", id);
            return true;
        }
    }
    false
}
impl KeyRegistry {
    // fn get_array(&mut self)
    pub fn insert(&mut self, gnome_id: GnomeId, key: Vec<u8>) {
        println!("Inserting key for {}", gnome_id);
        match self {
            Self::Disabled => (),
            Self::Reg32(arr) => insert32(gnome_id, key, arr.deref_mut()),
            Self::Reg64(arr) => insert64(gnome_id, key, arr.deref_mut()),
            Self::Reg128(arr) => insert128(gnome_id, key, arr.deref_mut()),
            Self::Reg256(arr) => insert256(gnome_id, key, arr.deref_mut()),
            Self::Reg512(arr) => insert512(gnome_id, key, arr.deref_mut()),
            Self::Reg1024(arr) => insert1024(gnome_id, key, arr.deref_mut()),
            Self::Reg2048(arr) => insert2048(gnome_id, key, arr.deref_mut()),
            Self::Reg4096(arr) => insert4096(gnome_id, key, arr.deref_mut()),
            Self::Reg8192(arr) => insert8192(gnome_id, key, arr.deref_mut()),
        };
    }
    pub fn get(&self, gnome_id: GnomeId) -> Option<Vec<u8>> {
        println!("Searching for {}", gnome_id);
        match self {
            Self::Disabled => None,
            Self::Reg32(arr) => get32(gnome_id, arr),
            Self::Reg64(arr) => get64(gnome_id, arr),
            Self::Reg128(arr) => get128(gnome_id, arr),
            Self::Reg256(arr) => get256(gnome_id, arr),
            Self::Reg512(arr) => get512(gnome_id, arr),
            Self::Reg1024(arr) => get1024(gnome_id, arr),
            Self::Reg2048(arr) => get2048(gnome_id, arr),
            Self::Reg4096(arr) => get4096(gnome_id, arr),
            Self::Reg8192(arr) => get8192(gnome_id, arr),
        }
    }
    pub fn has_key(&self, gnome_id: GnomeId) -> bool {
        println!("Searching for {}", gnome_id);
        match self {
            Self::Disabled => false,
            Self::Reg32(arr) => has32(gnome_id, arr),
            Self::Reg64(arr) => has64(gnome_id, arr),
            Self::Reg128(arr) => has128(gnome_id, arr),
            Self::Reg256(arr) => has256(gnome_id, arr),
            Self::Reg512(arr) => has512(gnome_id, arr),
            Self::Reg1024(arr) => has1024(gnome_id, arr),
            Self::Reg2048(arr) => has2048(gnome_id, arr),
            Self::Reg4096(arr) => has4096(gnome_id, arr),
            Self::Reg8192(arr) => has8192(gnome_id, arr),
        }
    }
}
#[derive(PartialEq, Eq, Hash, Debug, Copy, Clone)]
pub struct SwarmID(pub u8);

pub struct Swarm {
    pub name: String,
    pub id: SwarmID,
    pub sender: Sender<Request>,
    active_unicasts: HashSet<CastID>,
    active_broadcasts: HashMap<CastID, Multicast>,
    active_multicasts: HashMap<CastID, Multicast>,
    pub key_reg: KeyRegistry,
    pub verify: fn(GnomeId, &Vec<u8>, SwarmTime, &mut Vec<u8>, &[u8]) -> bool,
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
    //   - This claim is being admitted to pending refonfigurations
    //   - Current gnome acting as a source for given *cast can Deny that
    //   claim with another Reconfigure message within some time period
    //   - If Deny message is accepted, pending Takeover is dismissed
    //   - In other case given *cast needs to be reconfigured for new source-gnome.

    //   Some possible DataStructures:
    //   - Capabilities - attributes that are assigned to groups to give it's members
    //   certain priviledges, there can be for example a capability that is required
    //   in order to start a Broadcast. If a gnome sends StartBroadcast message,
    //   after full round it is accepted and then when it comes to creating
    //   logic and structures responsible for serving that broadcast, first each
    //   gnome needs to authorize that message against necessary capabilities befor
    //   given action gets executed
    //   - Groups - sets of gnomes that have certain capabilities
    //   - Multicasts
    //   - Broadcasts
    //   - and other stuff
}

impl Swarm {
    pub fn join(
        name: String,
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
    ) -> (Sender<Request>, Receiver<Response>) {
        let (sender, request_receiver) = channel::<Request>();
        let (response_sender, receiver) = channel::<Response>();

        let swarm = Swarm {
            name,
            id,
            sender: sender.clone(),
            active_unicasts: HashSet::new(),
            active_broadcasts: HashMap::new(),
            active_multicasts: HashMap::new(),
            verify,
            key_reg: KeyRegistry::Reg32(Box::new(core::array::from_fn(|_i| (GnomeId(0), vec![])))),
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
            gnome.do_your_job();
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

    pub fn serve_casts(&mut self) {
        self.serve_broadcasts();
        // self.serve_unicasts();
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

    fn serve_broadcasts(&mut self) {
        for bcast in self.active_broadcasts.values_mut() {
            bcast.serve();
        }
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
