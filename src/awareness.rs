use crate::Proposal;

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Awareness {
    Unaware,
    Aware(u8, Proposal),
    Confused(u8),
}
