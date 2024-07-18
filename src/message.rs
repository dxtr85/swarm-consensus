// use crate::neighbor::NeighborRequest;
use crate::neighbor::Neighborhood;
use crate::proposal::Data;
use crate::CastID;
use crate::CastMessage;
use crate::GnomeId;
// use crate::NeighborResponse;
use crate::SwarmTime;
use std::fmt::Display;

#[derive(Clone, Debug)]
pub enum WrappedMessage {
    Cast(CastMessage),
    Regular(Message),
    NoOp,
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
    Reconfigure(Signature, Configuration),
    Block(BlockID, Signature, Data),
    // Request(NeighborRequest),
    // Response(NeighborResponse),
    // Unicast(CastID, Data),
    // Multicast(CastID, Data),
    // Broadcast(CastID, Data),
}

impl Payload {
    pub fn has_signature(&self) -> bool {
        self.has_data() || self.has_config()
    }
    pub fn signature_and_bytes(&mut self) -> Option<(&Signature, Vec<u8>)> {
        match self {
            Payload::Reconfigure(ref sign, _config) => Some((sign, _config.bytes())),
            Self::Block(_bid, ref sign, ref _data) => Some((sign, _data.clone().bytes())),
            _ => None,
        }
    }
    pub fn bytes(&mut self) -> Option<Vec<u8>> {
        match self {
            Payload::Reconfigure(ref _sign, ref config) => Some(config.bytes()),
            // TODO: find a way to not copy data
            Self::Block(_bid, ref _sign, ref data) => Some(data.clone().bytes()),
            _ => None,
        }
    }

    pub fn has_data(&self) -> bool {
        matches!(self, Payload::Block(_b, _sign, _d))
    }
    pub fn id_and_data(self) -> Option<(BlockID, Data)> {
        match self {
            Payload::Block(id, _sign, data) => Some((id, data)),
            // Payload::Unicast(_id, data) => Some(data),
            // Payload::Broadcast(_id, data) => Some(data),
            // Payload::Multicast(_id, data) => Some(data),
            _ => None,
        }
    }

    pub fn has_config(&self) -> bool {
        matches!(self, Payload::Reconfigure(_sign, _c))
    }
    pub fn config(self) -> Option<Configuration> {
        match self {
            Payload::Reconfigure(_signature, config) => Some(config),
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
    pub fn header_byte(&self) -> u8 {
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
        self.header_byte() as ConfigType
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
        (self.header_byte() as u32) << 24
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
    pub fn bytes(&self) -> Vec<u8> {
        let mut result_vec = vec![];
        result_vec.push(self.header_byte());
        for byte in self.content_bytes() {
            result_vec.push(byte);
        }
        result_vec
    }

    fn content_bytes(&self) -> Vec<u8> {
        let mut content_bytes = vec![];
        match *self {
            Self::StartBroadcast(gid, cid) => {
                for b in gid.0.to_be_bytes() {
                    content_bytes.push(b);
                }
                content_bytes.push(cid.0);
            }
            Self::ChangeBroadcastOrigin(gid, cid) => {
                for b in gid.0.to_be_bytes() {
                    content_bytes.push(b);
                }
                content_bytes.push(cid.0);
            }
            Self::EndBroadcast(cid) => {
                content_bytes.push(cid.0);
            }
            Self::StartMulticast(gid, cid) => {
                for b in gid.0.to_be_bytes() {
                    content_bytes.push(b);
                }
                content_bytes.push(cid.0);
            }
            Self::ChangeMulticastOrigin(gid, cid) => {
                for b in gid.0.to_be_bytes() {
                    content_bytes.push(b);
                }
                content_bytes.push(cid.0);
            }
            Self::EndMulticast(cid) => {
                content_bytes.push(cid.0);
            }
            Self::CreateGroup => {
                //TODO!
            }
            Self::DeleteGroup => {
                //TODO!
            }
            Self::ModifyGroup => {
                //TODO!
            }
            Self::UserDefined(_other) => {
                //TODO!
            }
        }
        content_bytes
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BlockID(pub u64);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Signature {
    Regular(Vec<u8>),
    Extended(Vec<u8>, Vec<u8>),
}
impl Signature {
    pub fn header_byte(&self) -> u8 {
        if matches!(*self, Signature::Regular(_)) {
            0
        } else {
            255
        }
    }
    pub fn len(&self) -> usize {
        match self {
            Self::Regular(signature) => signature.len(),
            Self::Extended(pub_key, signature) => pub_key.len() + signature.len(),
        }
    }
    pub fn bytes(self) -> Vec<u8> {
        match self {
            Self::Regular(signature) => signature,
            Self::Extended(mut pub_key, signature) => {
                // println!(
                //     "EXT: {:?}\n{:?}\n{}{}",
                //     pub_key,
                //     signature,
                //     pub_key.len(),
                //     signature.len()
                // );
                pub_key.extend(signature);
                pub_key
            }
        }
    }
}

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

    // pub fn include_request(&self, request: NeighborRequest) -> Message {
    //     let payload = Payload::Request(request);
    //     Message { payload, ..*self }
    // }

    // pub fn include_response(&self, response: NeighborResponse) -> Message {
    //     let payload = Payload::Response(response);
    //     Message { payload, ..*self }
    // }

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
            payload: Payload::Block(BlockID(0), Signature::Regular(vec![]), Data::empty()),
        }
    }
    pub fn reconfigure() -> Message {
        Message {
            swarm_time: SwarmTime(0),
            neighborhood: Neighborhood(0),
            header: Header::Reconfigure(255 as ConfigType, GnomeId(0)),
            payload: Payload::Reconfigure(
                Signature::Regular(vec![]),
                Configuration::StartBroadcast(GnomeId(0), CastID(0)),
            ),
        }
    }

    pub fn is_bye(&self) -> bool {
        self.payload == Payload::Bye
    }
    // pub fn is_request(&self) -> bool {
    //     matches!(self.payload, Payload::Request(_))
    // }

    // pub fn is_response(&self) -> bool {
    //     matches!(self.payload, Payload::Response(_))
    // }

    // pub fn is_cast(&self) -> bool {
    //     matches!(
    //         self.payload,
    //         Payload::Unicast(_, _) | Payload::Multicast(_, _) | Payload::Broadcast(_, _)
    //     )
    // }
    // pub fn is_unicast(&self) -> bool {
    //     matches!(self.payload, Payload::Unicast(_, _))
    // }
    // pub fn is_multicast(&self) -> bool {
    //     matches!(self.payload, Payload::Multicast(_, _))
    // }
    // pub fn is_broadcast(&self) -> bool {
    //     matches!(self.payload, Payload::Broadcast(_, _))
    // }
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
            Self::Reconfigure(_s, _) => write!(f, "Reconfigure",),
            // Self::Request(_) => write!(f, "Request"),
            // Self::Response(_nresp) => write!(f, "Response"),
            Self::Block(block_id, _signature, data) => {
                write!(f, "{}", data)
            } // Self::Unicast(_uid, _data) => write!(f, "Unicast",),
              // Self::Multicast(_mid, _data) => write!(f, "Multicast",),
              // Self::Broadcast(_bid, _data) => write!(f, "Broadcast",),
        }
    }
}
