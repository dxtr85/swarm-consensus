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

    pub fn text(&self) -> String {
        match self {
            Policy::Default => "Default".to_string(),
            Policy::Data => "Data".to_string(),
            Policy::StartBroadcast => "StartBroadcast".to_string(),
            Policy::ChangeBroadcastOrigin => "ChangeBroadcastOrigin".to_string(),
            Policy::EndBroadcast => "EndBroadcast".to_string(),
            Policy::StartMulticast => "StartMulticast".to_string(),
            Policy::ChangeMulticastOrigin => "ChangeMulticastOrigin".to_string(),
            Policy::EndMulticast => "EndMulticast".to_string(),
            Policy::CreateGroup => "CreateGroup".to_string(),
            Policy::DeleteGroup => "DeleteGroup".to_string(),
            Policy::ModifyGroup => "ModifyGroup".to_string(),
            Policy::InsertPubkey => "InsertPubkey".to_string(),
            Policy::DataWithFirstByte(b) => format!("DataWithFirstByte({})", b),
            Policy::UserDefined(o) => format!("UserDefined({})", o),
        }
    }
}
