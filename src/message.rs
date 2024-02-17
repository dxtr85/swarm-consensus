use crate::neighbor::NeighborRequest;
use crate::proposal::ProposalID;
use crate::Awareness;
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
    pub fn include_response(
        &self,
        request: NeighborRequest,
        response: NeighborResponse,
    ) -> Message {
        let data = Data::Response(request, response);
        Message { data, ..*self }
    }
    pub fn include_request(&self, request: NeighborRequest) -> Message {
        let data = Data::Request(request);
        Message { data, ..*self }
    }
}

impl Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}, {}\t {}", self.swarm_time, self.awareness, self.data)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Data {
    KeepAlive,
    Request(NeighborRequest),
    Response(NeighborRequest, NeighborResponse),
    ProposalId(ProposalID),
    Proposal(ProposalID, ProposalData),
}
impl Display for Data {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::KeepAlive => write!(f, "KeepAlive",),
            Self::Request(_) => write!(f, "Request"),
            Self::Response(_, _) => write!(f, "Response",),
            Self::ProposalId(prop_id) => {
                write!(f, "{}", prop_id)
            }
            Self::Proposal(prop_id, data) => {
                write!(f, "{} {}", prop_id, data)
            }
        }
    }
}
