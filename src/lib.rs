mod gnome;
use crate::gnome::Gnome;
pub use crate::gnome::GnomeId;
mod message;
mod swarm;
// use crate::message::*;
pub use crate::swarm::SwarmTime;
pub use message::{Header, Message, Payload};
mod manager;
use manager::Manager;
pub use message::BlockID;
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

#[cfg(test)]
mod tests;

// TODO: Also update turn_number after committing proposal to
//             proposal_time + 2 * swarm_diameter

const DEFAULT_NEIGHBORS_PER_GNOME: usize = 3;
const DEFAULT_SWARM_DIAMETER: SwarmTime = SwarmTime(7);
// const MAX_PAYLOAD_SIZE: u32 = 1024;

pub enum Request {
    AddData(Data),
    AddNeighbor(Neighbor),
    DropNeighbor(GnomeId),
    AskData(GnomeId, NeighborRequest),
    SendData(GnomeId, NeighborRequest, NeighborResponse),
    Disconnect,
    Status,
}

#[derive(PartialEq)]
pub enum Response {
    Block(BlockID, Data),
    DataInquiry(GnomeId, NeighborRequest),
    Listing(u8, [BlockID; 128]),
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
        }
    }
}

static mut GNOME_ID: GnomeId = GnomeId(0);

fn gnome_id_dispenser() -> GnomeId {
    unsafe {
        let next_id = GNOME_ID;
        GNOME_ID = GnomeId(GNOME_ID.0 + 1);
        next_id
    }
}

pub fn start() -> Manager {
    Manager::new()
}
