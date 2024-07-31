#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum Capabilities {
    Founder,
    Owner,
    Admin,
    Moderator,
    Superuser,
    Capability(u8),
}

impl Capabilities {
    pub fn from(byte: u8) -> Self {
        match byte {
            255 => Capabilities::Founder,
            254 => Capabilities::Owner,
            253 => Capabilities::Admin,
            252 => Capabilities::Moderator,
            251 => Capabilities::Superuser,
            other => Capabilities::Capability(other),
        }
    }
    pub fn byte(&self) -> u8 {
        match self {
            Capabilities::Founder => 255,
            Capabilities::Owner => 254,
            Capabilities::Admin => 253,
            Capabilities::Moderator => 252,
            Capabilities::Superuser => 251,
            Capabilities::Capability(other) => *other,
        }
    }
}
