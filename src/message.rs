use crate::neighbor::NeighborRequest;
use crate::neighbor::Neighborhood;
use crate::proposal::Data;
use crate::CastID;
use crate::CastMessage;
use crate::GnomeId;
use crate::NeighborResponse;
use crate::SwarmTime;
use std::fmt::Display;

#[derive(Clone, Debug)]
pub enum WrappedMessage {
    Cast(CastMessage),
    Regular(Message),
}
#[derive(Clone, Debug)]
pub struct Message {
    pub swarm_time: SwarmTime,
    pub neighborhood: Neighborhood,
    pub header: Header,
    pub payload: Payload,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Header {
    Sync,
    Reconfigure(ConfigType, GnomeId),
    Block(BlockID),
}
impl Header {
    pub fn non_zero_block(&self) -> bool {
        match *self {
            Self::Block(b_id) => b_id.0 > 0,
            _ => false,
        }
    }
    pub fn is_reconfigure(&self) -> bool {
        matches!(self, Header::Reconfigure(_t, _g))
    }
}
pub type ConfigType = u8;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Payload {
    KeepAlive(u64),
    Bye,
    Reconfigure(Configuration),
    Block(BlockID, Data),
    Request(NeighborRequest),
    Response(NeighborResponse),
    Unicast(CastID, Data),
    Multicast(CastID, Data),
    Broadcast(CastID, Data),
}

impl Payload {
    pub fn data(&self) -> Option<Data> {
        match *self {
            Payload::Block(_id, data) => Some(data),
            Payload::Unicast(_id, data) => Some(data),
            Payload::Broadcast(_id, data) => Some(data),
            Payload::Multicast(_id, data) => Some(data),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Configuration {
    StartBroadcast(GnomeId, CastID),
    ChangeBroadcastOrigin(GnomeId, CastID),
    EndBroadcast(CastID),
    StartMulticast(GnomeId, CastID),
    ChangeMulticastOrigin(GnomeId, CastID),
    EndMulticast(CastID),
    CreateGroup,
    DeleteGroup,
    ModifyGroup,
    UserDefined(u8),
}
impl Configuration {
    pub fn as_u8(&self) -> u8 {
        match *self {
            Self::StartBroadcast(_gid, _cid) => 254,
            Self::ChangeBroadcastOrigin(_gid, _cid) => 253,
            Self::EndBroadcast(_cid) => 252,
            Self::StartMulticast(_gid, _cid) => 251,
            Self::ChangeMulticastOrigin(_gid, _cid) => 250,
            Self::EndMulticast(_cid) => 249,
            Self::CreateGroup => 248,
            Self::DeleteGroup => 247,
            Self::ModifyGroup => 246,
            Self::UserDefined(other) => other,
        }
    }
    pub fn as_ct(&self) -> ConfigType {
        self.as_u8() as ConfigType
    }
    pub fn as_gid(&self) -> GnomeId {
        match *self {
            Self::StartBroadcast(gid, _cid) => gid,
            Self::ChangeBroadcastOrigin(gid, _cid) => gid,
            Self::EndBroadcast(_cid) => GnomeId(0),
            Self::StartMulticast(gid, _cid) => gid,
            Self::ChangeMulticastOrigin(gid, _cid) => gid,
            Self::EndMulticast(_cid) => GnomeId(0),
            Self::CreateGroup => GnomeId(0),
            Self::DeleteGroup => GnomeId(0),
            Self::ModifyGroup => GnomeId(0),
            Self::UserDefined(_other) => GnomeId(0),
        }
    }

    pub fn as_u32(&self) -> u32 {
        (self.as_u8() as u32) << 24
    }
    pub fn from_u32(value: u32) -> Configuration {
        match (value >> 24) as u8 {
            254 => Self::StartBroadcast(GnomeId(0), CastID(0)), //TODO: need source for this
            253 => Self::ChangeBroadcastOrigin(GnomeId(0), CastID(0)),
            252 => Self::EndBroadcast(CastID(0)),
            251 => Self::StartMulticast(GnomeId(0), CastID(0)),
            250 => Self::ChangeMulticastOrigin(GnomeId(0), CastID(0)),
            249 => Self::EndMulticast(CastID(0)),
            248 => Self::CreateGroup,
            247 => Self::DeleteGroup,
            246 => Self::ModifyGroup,
            other => Self::UserDefined(other),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BlockID(pub u32);

impl Message {
    pub fn new(
        swarm_time: SwarmTime,
        header: Header,
        payload: Payload,
        neighborhood: Neighborhood,
    ) -> Message {
        Message {
            swarm_time,
            neighborhood,
            header,
            payload,
        }
    }
    pub fn set_payload(&self, payload: Payload) -> Message {
        Message { payload, ..*self }
    }

    pub fn include_request(&self, request: NeighborRequest) -> Message {
        let payload = Payload::Request(request);
        Message { payload, ..*self }
    }

    pub fn include_response(&self, response: NeighborResponse) -> Message {
        let payload = Payload::Response(response);
        Message { payload, ..*self }
    }

    pub fn bye() -> Message {
        Message {
            swarm_time: SwarmTime(0),
            neighborhood: Neighborhood(0),
            header: Header::Sync,
            payload: Payload::Bye,
        }
    }
    pub fn block() -> Message {
        Message {
            swarm_time: SwarmTime(0),
            neighborhood: Neighborhood(0),
            header: Header::Block(BlockID(0)),
            payload: Payload::Block(BlockID(0), Data(0)),
        }
    }
    pub fn reconfigure() -> Message {
        Message {
            swarm_time: SwarmTime(0),
            neighborhood: Neighborhood(0),
            header: Header::Reconfigure(255 as ConfigType, GnomeId(0)),
            payload: Payload::Reconfigure(Configuration::StartBroadcast(GnomeId(0), CastID(0))),
        }
    }

    pub fn is_bye(&self) -> bool {
        self.payload == Payload::Bye
    }
    pub fn is_request(&self) -> bool {
        matches!(self.payload, Payload::Request(_))
    }

    pub fn is_response(&self) -> bool {
        matches!(self.payload, Payload::Response(_))
    }

    pub fn is_cast(&self) -> bool {
        matches!(
            self.payload,
            Payload::Unicast(_, _) | Payload::Multicast(_, _) | Payload::Broadcast(_, _)
        )
    }
    pub fn is_unicast(&self) -> bool {
        matches!(self.payload, Payload::Unicast(_, _))
    }
    pub fn is_multicast(&self) -> bool {
        matches!(self.payload, Payload::Multicast(_, _))
    }
    pub fn is_broadcast(&self) -> bool {
        matches!(self.payload, Payload::Broadcast(_, _))
    }
}

impl Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} {} {}",
            self.swarm_time, self.neighborhood, self.header, self.payload
        )
    }
}

impl Display for Header {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Header::Sync => write!(f, "Sync"),
            Header::Block(b_id) => write!(f, "{}", b_id),
            Header::Reconfigure(ct, gid) => write!(f, "Reconf-{}-{}", ct, gid),
        }
    }
}
impl Display for BlockID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BID-{}", self.0)
    }
}
impl Display for Payload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::KeepAlive(_bandwith) => write!(f, "",),
            Self::Bye => write!(f, "Bye",),
            Self::Reconfigure(_) => write!(f, "Reconfigure",),
            Self::Request(_) => write!(f, "Request"),
            Self::Response(_nresp) => write!(f, "Response"),
            Self::Block(block_id, data) => {
                write!(f, "{} {}", block_id, data)
            }
            Self::Unicast(_uid, _data) => write!(f, "Unicast",),
            Self::Multicast(_mid, _data) => write!(f, "Multicast",),
            Self::Broadcast(_bid, _data) => write!(f, "Broadcast",),
        }
    }
}
