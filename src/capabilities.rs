use crate::GnomeId;

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum Capabilities {
    Founder,
    Owner,
    Admin,
    Moderator,
    Superuser,
    Capability(u8),
}

#[derive(Debug, Clone)]
pub enum CapabiliTree {
    Empty(u8, u64),
    Filled(Box<CapabiLeaf>),
}

#[derive(Debug, Clone)]
pub struct CapabiLeaf {
    level: u8,
    midpoint: u64,
    pub gnome_id: GnomeId,
    left: CapabiliTree,
    right: CapabiliTree,
}

impl CapabiliTree {
    pub fn create() -> Self {
        CapabiliTree::Empty(0, u64::MAX >> 1)
    }
    pub fn contains(&self, gnome_id: &GnomeId) -> bool {
        match self {
            Self::Empty(_l, _m) => false,
            Self::Filled(node) => node.contains(gnome_id),
        }
    }
    pub fn insert(&mut self, gnome_id: GnomeId) {
        match self {
            Self::Empty(level, midpoint) => {
                let new_level = *level + 1;
                let shift = u64::MAX >> (new_level + 1);
                let midpoint_left = *midpoint - shift;
                let midpoint_right = *midpoint + shift;
                *self = CapabiliTree::Filled(Box::new(CapabiLeaf {
                    level: *level,
                    midpoint: *midpoint,
                    gnome_id,
                    left: CapabiliTree::Empty(new_level, midpoint_left),
                    right: CapabiliTree::Empty(new_level, midpoint_right),
                }));
            }
            Self::Filled(ref mut node) => {
                node.insert(gnome_id);
            }
        }
    }

    pub fn remove(&mut self, gnome_id: GnomeId) -> bool {
        match self {
            Self::Filled(node) => {
                let collapse = node.remove(gnome_id);
                if collapse {
                    *self = Self::Empty(node.level, node.midpoint);
                }
            }
            Self::Empty(_l, _m) => {
                // println!("Remove called on empty CapabiliTree");
            }
        }
        false
    }

    pub fn empty(&self) -> bool {
        matches!(self, Self::Empty(_l, _m))
    }

    pub fn id_vec(&self) -> Vec<GnomeId> {
        match self {
            Self::Empty(_l, _m) => vec![],
            Self::Filled(node) => node.id_vec(),
        }
    }

    pub fn get_all_members(&self) -> Vec<GnomeId> {
        // TODO
        vec![]
    }

    fn gnome_id(&self) -> Option<GnomeId> {
        match self {
            Self::Empty(_l, _i) => None,
            Self::Filled(node) => Some(node.gnome_id),
        }
    }
}

impl CapabiLeaf {
    pub fn insert(&mut self, gnome_id: GnomeId) {
        let old_diff = self.gnome_id.0.abs_diff(self.midpoint);
        let new_diff = gnome_id.0.abs_diff(self.midpoint);
        if old_diff > new_diff {
            let old_id = std::mem::replace(&mut self.gnome_id, gnome_id);
            if old_id.0 > self.midpoint {
                self.right.insert(old_id);
            } else {
                self.left.insert(old_id);
            };
        } else if gnome_id.0 > self.midpoint {
            // Additional check in order not to insert duplicates
            if gnome_id != self.gnome_id {
                self.right.insert(gnome_id);
            }
        } else if gnome_id != self.gnome_id {
            self.left.insert(gnome_id);
        }
    }
    pub fn remove(&mut self, gnome_id: GnomeId) -> bool {
        if self.gnome_id == gnome_id {
            let left_empty = self.left.empty();
            let right_empty = self.right.empty();
            if left_empty && right_empty {
                true
            } else if right_empty {
                self.gnome_id = self.left.gnome_id().unwrap();
                self.left.remove(self.gnome_id);
                false
            } else if left_empty {
                self.gnome_id = self.right.gnome_id().unwrap();
                self.right.remove(self.gnome_id);
                false
            } else {
                let right_id = self.right.gnome_id().unwrap();
                let left_id = self.left.gnome_id().unwrap();
                let right_diff = right_id.0.abs_diff(self.midpoint);
                let left_diff = left_id.0.abs_diff(self.midpoint);
                if left_diff < right_diff {
                    self.gnome_id = left_id;
                    self.left.remove(self.gnome_id);
                } else {
                    self.gnome_id = right_id;
                    self.right.remove(self.gnome_id);
                }
                false
            }
        } else if gnome_id.0 > self.midpoint {
            self.right.remove(gnome_id)
        } else {
            self.left.remove(gnome_id)
        }
    }

    pub fn contains(&self, gnome_id: &GnomeId) -> bool {
        if self.gnome_id == *gnome_id {
            true
        } else if gnome_id.0 > self.midpoint {
            self.right.contains(gnome_id)
        } else {
            self.left.contains(gnome_id)
        }
    }

    pub fn id_vec(&self) -> Vec<GnomeId> {
        let mut vector = vec![self.gnome_id];
        let mut l_ver = self.left.id_vec();
        let mut r_ver = self.right.id_vec();
        vector.append(&mut l_ver);
        vector.append(&mut r_ver);
        vector
    }
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
