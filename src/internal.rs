use crate::{CastData, CastID, GnomeId, NeighborRequest, NeighborResponse};

pub enum InternalMsg {
    SubscribeCast(bool, CastID, GnomeId),
    UnsubscribeCast(bool, CastID),
    RequestOut(GnomeId, NeighborRequest),
    ResponseOut(GnomeId, NeighborResponse),
    FindNewCastSource(bool, CastID, GnomeId),
    SendToCastSource(bool, CastID, CastData),
}
