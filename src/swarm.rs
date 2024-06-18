use crate::gnome::NetworkSettings;
use crate::multicast::Multicast;
use crate::CastID;
use crate::Gnome;
use crate::GnomeId;
use crate::Message;
use crate::Neighbor;
use crate::Request;
use crate::Response;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt;
use std::ops::Add;
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

#[derive(PartialEq, Eq, Hash, Debug, Copy, Clone)]
pub struct SwarmID(pub u8);

pub struct Swarm {
    pub name: String,
    pub id: SwarmID,
    pub sender: Sender<Request>,
    active_unicasts: HashMap<CastID, GnomeId>,
    active_broadcasts: HashMap<CastID, Multicast>,
    active_multicasts: HashMap<CastID, Multicast>,
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
        neighbors: Option<Vec<Neighbor>>,
        band_receiver: Receiver<u64>,
        net_settings_send: Sender<NetworkSettings>,
        network_settings: NetworkSettings,
    ) -> (Sender<Request>, Receiver<Response>) {
        let (sender, request_receiver) = channel::<Request>();
        let (response_sender, receiver) = channel::<Response>();

        let swarm = Swarm {
            name,
            id,
            sender: sender.clone(),
            active_unicasts: HashMap::new(),
            active_broadcasts: HashMap::new(),
            active_multicasts: HashMap::new(),
        };
        let gnome = if let Some(neighbors) = neighbors {
            Gnome::new_with_neighbors(
                gnome_id,
                swarm,
                response_sender,
                request_receiver,
                band_receiver,
                neighbors,
                network_settings,
                net_settings_send,
            )
        } else {
            Gnome::new(
                gnome_id,
                swarm,
                response_sender,
                request_receiver,
                band_receiver,
                network_settings,
                net_settings_send,
            )
        };
        let _join_handle = spawn(move || {
            gnome.do_your_job();
        });

        (sender, receiver)
    }

    pub fn is_unicast_id_available(&self, cast_id: CastID) -> bool {
        !self.active_unicasts.contains_key(&cast_id)
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
    pub fn insert_unicast(&mut self, cast_id: CastID, gnome_id: GnomeId) {
        self.active_unicasts.insert(cast_id, gnome_id);
    }
    pub fn insert_multicast(&mut self, cast_id: CastID, multicast: Multicast) {
        self.active_multicasts.insert(cast_id, multicast);
    }
    pub fn insert_broadcast(&mut self, cast_id: CastID, broadcast: Multicast) {
        self.active_broadcasts.insert(cast_id, broadcast);
    }

    pub fn serve_casts(&mut self) {
        self.serve_broadcasts();
    }

    pub fn broadcasts_count(&self) -> u8 {
        self.active_broadcasts.len() as u8
    }
    pub fn multicasts_count(&self) -> u8 {
        self.active_multicasts.len() as u8
    }
    pub fn broadcast_ids(&self) -> Vec<CastID> {
        let mut ids = vec![];
        for id in self.active_broadcasts.keys() {
            ids.push(*id);
        }
        ids
    }

    pub fn multicast_ids(&self) -> Vec<CastID> {
        let mut ids = vec![];
        for id in self.active_multicasts.keys() {
            ids.push(*id);
        }
        ids
    }

    pub fn add_subscriber(
        &mut self,
        is_bcast: bool,
        cast_id: &CastID,
        sub_id: GnomeId,
        send: Sender<Message>,
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

    fn all_possible_cast_ids(&self) -> HashSet<CastID> {
        let mut ids = HashSet::new();
        for i in 0..=255 {
            let cast_id = CastID(i);
            ids.insert(cast_id);
        }
        ids
    }

    pub fn avail_unicast_ids(&self) -> HashSet<CastID> {
        let mut available = self.all_possible_cast_ids();
        for occupied in self.active_unicasts.keys() {
            available.remove(occupied);
        }
        available
    }
    pub fn avail_multicast_ids(&self) -> HashSet<CastID> {
        let mut available = self.all_possible_cast_ids();
        for occupied in self.active_multicasts.keys() {
            available.remove(occupied);
        }
        available
    }
    pub fn avail_broadcast_ids(&self) -> HashSet<CastID> {
        let mut available = self.all_possible_cast_ids();
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
        source: (GnomeId, Receiver<Message>),
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
