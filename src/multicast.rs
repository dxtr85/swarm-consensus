use crate::GnomeId;

pub struct Multicast {
    source: GnomeId,
    subscribers: Vec<GnomeId>,
}

impl Multicast {
    pub fn subscribers(&self) -> Vec<GnomeId> {
        self.subscribers.clone()
    }
    pub fn source(&self) -> GnomeId {
        self.source
    }
    pub fn set_source(&mut self, source: GnomeId) {
        self.source = source;
    }
}
