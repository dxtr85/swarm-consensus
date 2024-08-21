#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum Policy {
    Default,
    Data,
    StartBroadcast,
    ChangeBroadcastOrigin,
    EndBroadcast,
    StartMulticast,
    ChangeMulticastOrigin,
    EndMulticast,
    CreateGroup,
    DeleteGroup,
    ModifyGroup,
    InsertPubkey,
    DataWithFirstByte(u8),
    UserDefined(u8),
}

impl Policy {
    pub fn from(bytes: &mut Vec<u8>) -> Self {
        let byte = bytes.drain(0..1).next().unwrap();
        match byte {
            255 => Policy::Default,
            254 => Policy::Data,
            253 => Policy::StartBroadcast,
            252 => Policy::ChangeBroadcastOrigin,
            251 => Policy::EndBroadcast,
            250 => Policy::StartMulticast,
            249 => Policy::ChangeMulticastOrigin,
            248 => Policy::EndMulticast,
            247 => Policy::CreateGroup,
            246 => Policy::DeleteGroup,
            245 => Policy::ModifyGroup,
            244 => Policy::InsertPubkey,
            243 => {
                let byte = bytes.drain(0..1).next().unwrap();
                Policy::DataWithFirstByte(byte)
            }
            other => Policy::UserDefined(other),
        }
    }

    pub fn append_bytes_to(&self, bytes: &mut Vec<u8>) {
        match self {
            Policy::Default => bytes.push(255),
            Policy::Data => bytes.push(254),
            Policy::StartBroadcast => bytes.push(253),
            Policy::ChangeBroadcastOrigin => bytes.push(252),
            Policy::EndBroadcast => bytes.push(251),
            Policy::StartMulticast => bytes.push(250),
            Policy::ChangeMulticastOrigin => bytes.push(249),
            Policy::EndMulticast => bytes.push(248),
            Policy::CreateGroup => bytes.push(247),
            Policy::DeleteGroup => bytes.push(246),
            Policy::ModifyGroup => bytes.push(245),
            Policy::InsertPubkey => bytes.push(244),
            Policy::DataWithFirstByte(byte) => {
                bytes.push(243);
                bytes.push(*byte);
            }
            Policy::UserDefined(other) => bytes.push(*other),
        }
    }
}
