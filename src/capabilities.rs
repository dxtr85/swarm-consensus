#[derive(PartialEq, Eq, Hash)]
pub enum Capabilities {
    Founder,
    Owner,
    Admin,
    Moderator,
    Superuser,
    Capability(u8),
}
