use crate::Awareness;
use crate::GnomeId;
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

impl Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "<<{:12}, {:?}\t {}",
            self.swarm_time.0, self.awareness, self.data
        )
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Data {
    KeepAlive,
    Request,
    Response,
    ProposalId(SwarmTime, GnomeId),
    Proposal(SwarmTime, GnomeId, ProposalData),
}
impl Display for Data {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::KeepAlive => write!(f, "KeepAlive",),
            Self::Request => write!(f, "Request"),
            Self::Response => write!(f, "Response",),
            Self::ProposalId(swarm_time, gnome_id) => {
                write!(f, "PropID-{}-{}", swarm_time.0, gnome_id.0)
            }
            Self::Proposal(swarm_time, gnome_id, data) => {
                write!(f, "PropID-{}-{} Data: {}", swarm_time.0, gnome_id.0, data.0)
            }
        }
    }
}
