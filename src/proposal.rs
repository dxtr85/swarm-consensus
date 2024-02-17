use crate::GnomeId;
use crate::SwarmTime;
use std::fmt;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ProposalID(pub SwarmTime, pub GnomeId);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ProposalData(pub u8);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Proposal {
    pub proposal_id: ProposalID,
    pub data: ProposalData,
}

impl fmt::Display for ProposalID{
         fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
             write!(f, "PID-{}-{}", self.0.0, self.1.0)
         }
}
impl fmt::Display for ProposalData{
         fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
             write!(f, "[{}]", self.0)
         }
}
