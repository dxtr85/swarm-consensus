use crate::BlockID;
use std::hash::{DefaultHasher, Hasher};
use std::{fmt, hash::Hash};

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Data(Vec<u8>);
impl Data {
    pub fn new(contents: Vec<u8>) -> Result<Self, Vec<u8>> {
        // println!("new data: {:?}", contents);
        if contents.len() > 1024 {
            return Err(contents);
        }
        // // Prefix is for later storing SwarmTime value before sign/verify
        // let mut with_prefix = vec![0, 0, 0, 0];
        // with_prefix.append(&mut contents);
        // Ok(Self(with_prefix))
        Ok(Self(contents))
    }

    pub fn empty() -> Self {
        Data(vec![])
    }

    pub fn bytes(self) -> Vec<u8> {
        self.0
    }
    pub fn first_byte(&self) -> u8 {
        self.0[0]
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.0.hash(&mut hasher);
        hasher.finish()
    }

    pub fn get_block_id(&self) -> BlockID {
        BlockID(self.hash())
    }
}

impl fmt::Display for Data {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[len:{:?}]", self.len())
    }
}
