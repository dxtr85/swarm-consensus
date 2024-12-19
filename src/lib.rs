mod capabilities;
mod gnome;
mod gnome_to_manager;
mod internal;
mod key_registry;
mod manager_to_gnome;
mod policy;
mod requirement;
use crate::gnome::Gnome;
pub use crate::gnome::GnomeId;
mod message;
mod multicast;
mod swarm;
pub use crate::capabilities::CapabiliTree;
pub use crate::capabilities::Capabilities;
pub use crate::gnome::Nat;
pub use crate::gnome::NetworkSettings;
pub use crate::gnome::PortAllocationRule;
pub use crate::key_registry::KeyRegistry;
pub use crate::neighbor::SwarmSyncRequestParams;
pub use crate::neighbor::SwarmSyncResponse;
pub use crate::policy::Policy;
pub use crate::requirement::Requirement;
pub use crate::swarm::Swarm;
pub use crate::swarm::SwarmID;
pub use crate::swarm::SwarmName;
pub use crate::swarm::SwarmTime;
pub use crate::swarm::SwarmType;
pub use data::CastData;
pub use data::SyncData;
pub use gnome_to_manager::GnomeToManager;
pub use manager_to_gnome::ManagerToGnome;
pub use message::BlockID;
pub use message::Configuration;
pub use message::{Header, Message, Payload, Signature, WrappedMessage};
pub use neighbor::NeighborRequest;
use std::net::IpAddr;
mod neighbor;
pub use crate::neighbor::Neighbor;
pub use crate::neighbor::NeighborResponse;
pub use crate::neighbor::Neighborhood;
pub use multicast::CastContent;
pub use multicast::CastMessage;
pub use multicast::CastType;
mod data;
mod next_state;
use crate::next_state::NextState;
use std::fmt;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;

#[cfg(test)]
mod tests;

// TODO: Also update turn_number after committing proposal to
//             proposal_time + 2 * swarm_diameter

const DEFAULT_NEIGHBORS_PER_GNOME: usize = 3;
const DEFAULT_SWARM_DIAMETER: SwarmTime = SwarmTime(7);

#[derive(Debug)]
pub enum ToGnome {
    // UpdateAppRootHash(u64),
    AddData(SyncData),
    AddNeighbor(Neighbor),
    DropNeighbor(GnomeId),
    ListNeighbors,
    AskData(GnomeId, NeighborRequest),
    SendData(GnomeId, NeighborResponse),
    Disconnect,
    Status,
    StartUnicast(GnomeId),
    StartMulticast(Vec<GnomeId>),
    StartBroadcast,
    EndBroadcast(CastID),
    UnsubscribeBroadcast(CastID),
    NetworkSettingsUpdate(bool, IpAddr, u16, Nat),
    SwarmNeighbors(SwarmName),
    Reconfigure(u8, SyncData),
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub struct CastID(pub u8);

pub enum GnomeToApp {
    SwarmReady,
    Block(BlockID, SyncData),
    DataInquiry(GnomeId, NeighborRequest),
    Listing(Vec<BlockID>),
    UnicastOrigin(SwarmID, CastID, Sender<CastData>),
    Unicast(SwarmID, CastID, Receiver<CastData>),
    MulticastOrigin(SwarmID, CastID, Sender<SyncData>),
    Multicast(SwarmID, CastID, Receiver<CastData>),
    BroadcastOrigin(SwarmID, CastID, Sender<CastData>, Receiver<CastData>),
    Broadcast(SwarmID, CastID, Receiver<CastData>),
    Neighbors(SwarmID, Vec<GnomeId>),
    NewNeighbor(SwarmName, Neighbor),
    ToGnome(NeighborResponse),
    BCastData(CastID, CastData),
    Custom(bool, u8, GnomeId, CastData),
    Reconfig(u8, SyncData),
}

impl fmt::Debug for GnomeToApp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GnomeToApp::SwarmReady => {
                write!(f, "SwarmReady")
            }
            GnomeToApp::Block(prop_id, data) => {
                write!(f, "{:?} {}", prop_id, data)
            }
            GnomeToApp::BCastData(c_id, c_data) => {
                write!(f, "BCastData {} (len: {})", c_id.0, c_data.len())
            }
            GnomeToApp::DataInquiry(gnome_id, data_id) => {
                write!(f, "DataInquiry for {:?}: PropID-{:?}", gnome_id, data_id)
            }
            GnomeToApp::Listing(data) => {
                write!(f, "Listing with {:?} entries", data.len())
            }
            GnomeToApp::Unicast(_sid, _cid, _rdata) => {
                write!(f, "Unicast {:?}", _cid)
            }
            GnomeToApp::UnicastOrigin(_sid, _cid, _sdata) => {
                write!(f, "Unicast source {:?}", _cid)
            }
            GnomeToApp::Multicast(_sid, _cid, _rdata) => {
                write!(f, "Multicast {:?}", _cid)
            }
            GnomeToApp::MulticastOrigin(_sid, _cid, _sdata) => {
                write!(f, "Multicast source {:?}", _cid)
            }
            GnomeToApp::Neighbors(_sid, _nid) => {
                write!(f, "Neighbors: {:?}", _nid)
            }
            GnomeToApp::NewNeighbor(sname, n) => {
                write!(f, "{} has a new neighbor: {:?}", sname, n.id)
            }
            GnomeToApp::Broadcast(_sid, _cid, _rdata) => {
                write!(f, "Broadcast {:?}", _cid)
            }
            GnomeToApp::BroadcastOrigin(_sid, _cid, _sdata, _rdata) => {
                write!(f, "Broadcast source {:?}", _cid)
            }
            GnomeToApp::ToGnome(neighbor_response) => {
                write!(f, "ToGnome: {:?}", neighbor_response)
            }
            GnomeToApp::Custom(is_request, id, gnome_id, _sdata) => {
                if *is_request {
                    write!(f, "Custom request {} from {}", id, gnome_id)
                } else {
                    write!(f, "Custom response {} from {}", id, gnome_id)
                }
            }
            GnomeToApp::Reconfig(id, _s_data) => {
                write!(f, "Reconfig: {}", id)
            }
        }
    }
}

pub struct NotificationBundle {
    pub swarm_name: SwarmName,
    pub request_sender: Sender<ToGnome>,
    pub token_sender: Sender<u64>,
    pub network_settings_receiver: Receiver<NetworkSettings>,
}
