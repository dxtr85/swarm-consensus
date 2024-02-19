use crate::neighbor::NeighborRequest;
use crate::neighbor::Neighborhood;
use crate::proposal::Data;
use crate::SwarmTime;
use std::fmt::Display;

#[derive(Clone, Copy, Debug)]
pub struct Message {
    pub swarm_time: SwarmTime,
    pub neighborhood: Neighborhood,
    pub header: Header,
    pub payload: Payload,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Header {
    Block(BlockID),
    Sync,
}

#[derive(Clone, Copy, Debug)]
pub enum Payload {
    KeepAlive,
    Block(BlockID, Data),
    Request(NeighborRequest),
    Listing(u8, [BlockID; 128]),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BlockID(pub u8);

impl Message {
    pub fn set_payload(&self, payload: Payload) -> Message {
        Message { payload, ..*self }
    }

    pub fn include_request(&self, request: NeighborRequest) -> Message {
        let payload = Payload::Request(request);
        Message { payload, ..*self }
    }
}

impl Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} {}",
            self.swarm_time, self.neighborhood, self.payload
        )
    }
}

impl Display for BlockID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BID-{}", self.0)
    }
}
impl Display for Payload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::KeepAlive => write!(f, "KeepAlive",),
            Self::Request(_) => write!(f, "Request"),
            Self::Listing(count, _) => write!(f, "Listing with {} elements", count),
            Self::Block(block_id, data) => {
                write!(f, "{} {}", block_id, data)
            }
        }
    }
}
