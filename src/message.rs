use crate::Proposal;
use crate::GnomeId;
use crate::Awareness;

#[derive(Clone, Copy, Debug)]
pub enum Message {
    KeepAlive(GnomeId, Awareness),
    Proposal(GnomeId, Awareness, Proposal),
}
