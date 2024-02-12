use crate::Awareness;
// use crate::GnomeId;
use crate::Proposal;

#[derive(Clone, Copy, Debug)]
pub enum Message {
    KeepAlive(Awareness),
    Proposal(Awareness, Proposal),
}
