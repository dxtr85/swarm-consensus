use crate::{CastID, GnomeId};

pub enum InternalMsg{
    SubscribeBroadcast(CastID,GnomeId)
}
