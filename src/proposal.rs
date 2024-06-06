use std::fmt;

use crate::BlockID;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Data(pub u32);
impl Data {
    pub fn get_block_id(&self) -> BlockID {
        BlockID(self.0)
    }
}

impl fmt::Display for Data {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}]", self.0)
    }
}
