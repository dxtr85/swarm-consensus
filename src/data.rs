use crate::BlockID;
use std::hash::{DefaultHasher, Hasher};
use std::{fmt, hash::Hash};

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CastData(Vec<u8>);
impl CastData {
    pub fn new(contents: Vec<u8>) -> Result<Self, Vec<u8>> {
        // println!("new data: {:?}", contents);
        // if contents.len() > 1024 {
        if contents.len() > 1364 {
            return Err(contents);
        }
        // // Prefix is for later storing SwarmTime value before sign/verify
        // let mut with_prefix = vec![0, 0, 0, 0];
        // with_prefix.append(&mut contents);
        // Ok(Self(with_prefix))
        Ok(Self(contents))
    }

    pub fn empty() -> Self {
        CastData(vec![])
    }

    pub fn bytes(self) -> Vec<u8> {
        self.0
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SyncData(Vec<u8>);
impl SyncData {
    pub fn new(contents: Vec<u8>) -> Result<Self, Vec<u8>> {
        // println!("new data: {:?}", contents);
        // if contents.len() > 1024 {
        if contents.len() > 1168 {
            return Err(contents);
        }
        // // Prefix is for later storing SwarmTime value before sign/verify
        // let mut with_prefix = vec![0, 0, 0, 0];
        // with_prefix.append(&mut contents);
        // Ok(Self(with_prefix))
        Ok(Self(contents))
    }

    pub fn empty() -> Self {
        SyncData(vec![])
    }

    pub fn bytes(self) -> Vec<u8> {
        self.0
    }

    pub fn ref_bytes(&self) -> &Vec<u8> {
        &self.0
    }

    pub fn first_byte(&self) -> u8 {
        self.0[0]
    }
    pub fn second_byte(&self) -> u8 {
        self.0[1]
    }
    pub fn third_byte(&self) -> u8 {
        self.0[2]
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

impl fmt::Display for SyncData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[S|len:{:?}]", self.len())
    }
}
impl fmt::Display for CastData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[C|len:{:?}]", self.len())
    }
}
