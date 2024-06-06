mod gnome;
use crate::gnome::Gnome;
pub use crate::gnome::GnomeId;
mod message;
mod multicast;
mod swarm;
pub use crate::gnome::Nat;
pub use crate::gnome::NetworkSettings;
pub use crate::gnome::PortAllocationRule;
pub use crate::swarm::SwarmID;
pub use crate::swarm::SwarmTime;
pub use message::{Header, Message, Payload};
use std::net::IpAddr;
mod manager;
pub use manager::Manager;
pub use message::BlockID;
pub use message::Configuration;
pub use neighbor::NeighborRequest;
pub use proposal::Data;
mod neighbor;
pub use crate::neighbor::Neighbor;
pub use crate::neighbor::NeighborResponse;
pub use crate::neighbor::Neighborhood;
mod next_state;
mod proposal;
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
    AddData(Data),
    AddNeighbor(Neighbor),
    DropNeighbor(GnomeId),
    AskData(GnomeId, NeighborRequest),
    SendData(GnomeId, NeighborRequest, NeighborResponse),
    Disconnect,
    Status,
    StartUnicast(GnomeId),
    StartMulticast(Vec<GnomeId>),
    StartBroadcast,
    Custom(u8, Data),
    NetworkSettingsUpdate(bool, IpAddr, u16, Nat),
    // SetAddress(IpAddr),
    // SetPort(u16),
    // SetNat(Nat),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CastID(pub u8);

pub enum Response {
    Block(BlockID, Data),
    DataInquiry(GnomeId, NeighborRequest),
    Listing(u8, [BlockID; 128]),
    Unicast(SwarmID, CastID, Receiver<Data>),
    MulticastSource(SwarmID, CastID, Sender<Data>),
    Multicast(SwarmID, CastID, Receiver<Data>),
    BroadcastSource(SwarmID, CastID, Sender<Data>),
    Broadcast(SwarmID, CastID, Receiver<Data>),
    ToGnome(NeighborResponse),
    Custom(u8, Data),
}

impl fmt::Debug for Response {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Response::Block(prop_id, data) => {
                write!(f, "{:?} {}", prop_id, data)
            }
            Response::DataInquiry(gnome_id, data_id) => {
                write!(f, "DataInquiry for {:?}: PropID-{:?}", gnome_id, data_id)
            }
            Response::Listing(count, _data) => {
                write!(f, "Listing with {:?} entries", count)
            }
            Response::Unicast(_sid, _cid, _rdata) => {
                write!(f, "Unicast {:?}", _cid)
            }
            Response::Multicast(_sid, _cid, _rdata) => {
                write!(f, "Multicast {:?}", _cid)
            }
            Response::MulticastSource(_sid, _cid, _sdata) => {
                write!(f, "Multicast source {:?}", _cid)
            }
            Response::Broadcast(_sid, _cid, _rdata) => {
                write!(f, "Broadcast {:?}", _cid)
            }
            Response::BroadcastSource(_sid, _cid, _sdata) => {
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
    pub token_sender: Sender<u32>,
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

pub fn start(
    gnome_id: GnomeId,
    network_settings: Option<NetworkSettings>,
    sender: Sender<NotificationBundle>,
) -> Manager {
    Manager::new(gnome_id, network_settings, sender)
}
