mod capabilities;
mod gnome;
mod gnome_to_manager;
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
pub use crate::policy::Policy;
pub use crate::requirement::Requirement;
pub use crate::swarm::Swarm;
pub use crate::swarm::SwarmID;
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
// const MAX_PAYLOAD_SIZE: u32 = 1024;

#[derive(Debug)]
pub enum Request {
    UpdateAppRootHash(u64),
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
    Custom(u8, SyncData),
    NetworkSettingsUpdate(bool, IpAddr, u16, Nat),
    SwarmNeighbors(String),
    // SetAddress(IpAddr),
    // SetPort(u16),
    // SetNat(Nat),
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub struct CastID(pub u8);

pub enum Response {
    AppDataSynced(bool),
    AppSync(u8, u16, u16, u16, SyncData),
    AppSyncInquiry(GnomeId, u8, SyncData),
    Block(BlockID, SyncData),
    DataInquiry(GnomeId, NeighborRequest),
    Listing(Vec<BlockID>),
    UnicastOrigin(SwarmID, CastID, Sender<CastData>),
    Unicast(SwarmID, CastID, Receiver<CastData>),
    MulticastOrigin(SwarmID, CastID, Sender<SyncData>),
    Multicast(SwarmID, CastID, Receiver<CastData>),
    BroadcastOrigin(SwarmID, CastID, Sender<CastData>),
    Broadcast(SwarmID, CastID, Receiver<CastData>),
    Neighbors(String, Vec<GnomeId>),
    NewNeighbor(String, Neighbor),
    ToGnome(NeighborResponse),
    BCastData(CastID, CastData),
    Custom(u8, SyncData),
}

impl fmt::Debug for Response {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Response::AppDataSynced(is_synced) => {
                write!(f, "AppDataSynced: {}", is_synced)
            }
            Response::AppSync(_type, c_id, part_no, total, _data) => {
                write!(f, "AppSync[{}, {}/{}]", c_id, part_no, total)
            }
            Response::AppSyncInquiry(_g_id, s_type, _data) => {
                write!(f, "AppSync[{}]", s_type)
            }
            Response::Block(prop_id, data) => {
                write!(f, "{:?} {}", prop_id, data)
            }
            Response::BCastData(c_id, c_data) => {
                write!(f, "BCastData {} (len: {})", c_id.0, c_data.len())
            }
            Response::DataInquiry(gnome_id, data_id) => {
                write!(f, "DataInquiry for {:?}: PropID-{:?}", gnome_id, data_id)
            }
            Response::Listing(data) => {
                write!(f, "Listing with {:?} entries", data.len())
            }
            Response::Unicast(_sid, _cid, _rdata) => {
                write!(f, "Unicast {:?}", _cid)
            }
            Response::UnicastOrigin(_sid, _cid, _sdata) => {
                write!(f, "Unicast source {:?}", _cid)
            }
            Response::Multicast(_sid, _cid, _rdata) => {
                write!(f, "Multicast {:?}", _cid)
            }
            Response::MulticastOrigin(_sid, _cid, _sdata) => {
                write!(f, "Multicast source {:?}", _cid)
            }
            Response::Neighbors(_sid, _nid) => {
                write!(f, "Neighbors: {:?}", _nid)
            }
            Response::NewNeighbor(sname, n) => {
                write!(f, "{} has a new neighbor: {:?}", sname, n.id)
            }
            Response::Broadcast(_sid, _cid, _rdata) => {
                write!(f, "Broadcast {:?}", _cid)
            }
            Response::BroadcastOrigin(_sid, _cid, _sdata) => {
                write!(f, "Broadcast source {:?}", _cid)
            }
            Response::ToGnome(neighbor_response) => {
                write!(f, "ToGnome: {:?}", neighbor_response)
            }
            Response::Custom(id, _sdata) => {
                write!(f, "Custom response {}", id)
            }
        }
    }
}

pub struct NotificationBundle {
    pub swarm_name: String,
    pub request_sender: Sender<Request>,
    pub token_sender: Sender<u64>,
    pub network_settings_receiver: Receiver<NetworkSettings>,
}

// static mut GNOME_ID: GnomeId = GnomeId(0);

// fn gnome_id_dispenser() -> GnomeId {
//     unsafe {
//         let next_id = GNOME_ID;
//         GNOME_ID = GnomeId(GNOME_ID.0 + 1);
//         next_id
//     }
// }

// pub fn start(
//     gnome_id: GnomeId,
//     network_settings: Option<NetworkSettings>,
//     sender: Sender<NotificationBundle>,
// ) -> Manager {
//     Manager::new(gnome_id, network_settings, sender)
// }
