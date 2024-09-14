use crate::{CastData, CastID, GnomeId, NeighborRequest, NeighborResponse};

pub enum InternalMsg {
    SubscribeBroadcast(CastID, GnomeId),
    RequestOut(GnomeId, NeighborRequest),
    ResponseOut(GnomeId, NeighborResponse),
}
