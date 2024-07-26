#[derive(PartialEq, Eq, Hash)]
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
