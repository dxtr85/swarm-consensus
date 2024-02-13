use crate::Proposal;
use crate::SwarmTime;

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Awareness {
    Unaware(SwarmTime),
    Aware(SwarmTime, u8, Proposal),
    Confused(SwarmTime, u8),
}
impl Awareness {
    pub fn neighborhood(&self) -> Option<u8> {
        if let Awareness::Aware(_st, hood, _p) = self {
            return Some(*hood);
        }
        None
    }

    pub fn is_unaware(&self) -> bool {
        matches!(self, Awareness::Unaware(_st))
    }
}
