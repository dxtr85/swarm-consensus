use crate::neighbor::NeighborRequest;
use crate::neighbor::Neighborhood;
use crate::proposal::Data;
use crate::CastID;
use crate::NeighborResponse;
use crate::SwarmTime;
use std::fmt::Display;

#[derive(Clone, Copy, Debug)]
pub struct Message {
    pub swarm_time: SwarmTime,
    pub neighborhood: Neighborhood,
    pub header: Header,
    pub payload: Payload,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Header {
    Sync,
    Reconfigure,
    Block(BlockID),
}
impl Header {
    pub fn non_zero_block(&self) -> bool {
        match *self {
            Self::Block(b_id) => b_id.0 > 0,
            _ => false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Payload {
    KeepAlive,
    Bye,
    Reconfigure(Configuration),
    Block(BlockID, Data),
    Request(NeighborRequest),
    Response(NeighborResponse),
    Unicast(CastID, Data),
    Multicast(CastID, Data),
    Broadcast(CastID, Data),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Configuration {
    StartBroadcast,
    ChangeBroadcastSource,
    EndBroadcast,
    StartMulticast,
    ChangeMulticastSource,
    EndMulticast,
    CreateGroup,
    DeleteGroup,
    ModifyGroup,
    UserDefined(u8),
}
impl Configuration {
    pub fn as_u8(&self) -> u8 {
        match *self {
            Self::StartBroadcast => 254,
            Self::ChangeBroadcastSource => 253,
            Self::EndBroadcast => 252,
            Self::StartMulticast => 251,
            Self::ChangeMulticastSource => 250,
            Self::EndMulticast => 249,
            Self::CreateGroup => 248,
            Self::DeleteGroup => 247,
            Self::ModifyGroup => 246,
            Self::UserDefined(other) => other,
        }
    }
    pub fn as_u32(&self) -> u32 {
        (self.as_u8() as u32) << 24
    }
    pub fn from_u32(value: u32) -> Configuration {
        match (value >> 24) as u8 {
            254 => Self::StartBroadcast,
            253 => Self::ChangeBroadcastSource,
            252 => Self::EndBroadcast,
            251 => Self::StartMulticast,
            250 => Self::ChangeMulticastSource,
            249 => Self::EndMulticast,
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
    pub fn set_payload(&self, payload: Payload) -> Message {
        Message { payload, ..*self }
    }

    pub fn include_request(&self, request: NeighborRequest) -> Message {
        let payload = Payload::Request(request);
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
            header: Header::Reconfigure,
            payload: Payload::Reconfigure(Configuration::StartBroadcast),
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
            Header::Reconfigure => write!(f, "Reconf"),
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
            Self::KeepAlive => write!(f, "",),
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
