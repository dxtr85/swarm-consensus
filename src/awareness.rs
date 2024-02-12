use crate::Proposal;
use crate::SwarmTime;

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Awareness {
    Unaware(SwarmTime),
    Aware(SwarmTime, u8, Proposal),
    Confused(SwarmTime, u8),
}
