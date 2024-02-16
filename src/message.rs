use crate::neighbor::NeighborRequest;
use crate::Awareness;
use crate::GnomeId;
use crate::NeighborResponse;
use crate::ProposalData;
use crate::SwarmTime;
use std::fmt::Display;

// #[derive(Clone, Copy, Debug)]
// pub enum Message {
//     KeepAlive(Awareness),
//     Proposal(Awareness, Proposal),
// }

#[derive(Clone, Copy, Debug)]
pub struct Message {
    pub swarm_time: SwarmTime,
    pub awareness: Awareness,
    pub data: Data,
}

impl Message {
    pub fn include(&self, request: NeighborRequest, response: NeighborResponse) -> Message {
        let data = Data::Response(request, response);
        Message { data, ..*self }
    }
}

impl Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:12}, {:?}\t {}",
            self.swarm_time.0, self.awareness, self.data
        )
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Data {
    KeepAlive,
    Request,
    Response(NeighborRequest, NeighborResponse),
    ProposalId(SwarmTime, GnomeId),
    Proposal(SwarmTime, GnomeId, ProposalData),
}
impl Display for Data {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::KeepAlive => write!(f, "KeepAlive",),
            Self::Request => write!(f, "Request"),
            Self::Response(_, _) => write!(f, "Response",),
            Self::ProposalId(swarm_time, gnome_id) => {
                write!(f, "PropID-{}-{}", swarm_time.0, gnome_id.0)
            }
            Self::Proposal(swarm_time, gnome_id, data) => {
                write!(f, "PropID-{}-{} Data: {}", swarm_time.0, gnome_id.0, data.0)
            }
        }
    }
}
