
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
    UserDefined(u8),
}

impl Policy {
    pub fn from(byte: u8) -> Self {
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
            other => Policy::UserDefined(other),
        }
    }

    pub fn byte(&self) -> u8 {
        match self {
            Policy::Default => 255,
            Policy::Data => 254,
            Policy::StartBroadcast => 253,
            Policy::ChangeBroadcastOrigin => 252,
            Policy::EndBroadcast => 251,
            Policy::StartMulticast => 250,
            Policy::ChangeMulticastOrigin => 249,
            Policy::EndMulticast => 248,
            Policy::CreateGroup => 247,
            Policy::DeleteGroup => 246,
            Policy::ModifyGroup => 245,
            Policy::InsertPubkey => 244,
            Policy::UserDefined(other) => *other,
        }
    }
}
