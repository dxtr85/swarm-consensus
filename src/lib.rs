mod awareness;
use crate::awareness::Awareness;
mod gnome;
use crate::gnome::{Gnome, GnomeId};
mod message;
mod swarm;
use crate::message::Message;
mod manager;
use manager::Manager;
mod neighbor;
use crate::neighbor::Neighbor;
mod next_state;
use crate::next_state::NextState;
use std::fmt;

#[cfg(test)]
mod tests;

// TODO: Also update turn_number after committing proposal to
//             proposal_time + 2 * swarm_diameter

const DEFAULT_NEIGHBORS_PER_GNOME: usize = 3;
const DEFAULT_SWARM_DIAMETER: u8 = 7;
// const MAX_PAYLOAD_SIZE: u32 = 1024;

type Proposal = u8;

pub enum Request {
    MakeProposal(Box<[u8; 1024]>),
    AddNeighbor(Neighbor),
    Disconnect,
    Status,
}

#[derive(PartialEq)]
pub enum Response {
    Data(Box<[u8; 1024]>),
}

impl fmt::Debug for Response {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Response::Data(boxed_data) => {
                write!(f, "Data: {:?}", &boxed_data[..16])
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
