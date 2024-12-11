use std::{
    // io::Read,
    ops::{Deref, DerefMut},
};

use crate::GnomeId;
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeyRegistry {
    Reg8(Box<[(GnomeId, Vec<u8>); 8]>),
    Reg32(Box<[(GnomeId, Vec<u8>); 32]>),
    Reg64(Box<[(GnomeId, Vec<u8>); 64]>),
    Reg128(Box<[(GnomeId, Vec<u8>); 128]>),
    Reg256(Box<[(GnomeId, Vec<u8>); 256]>),
    Reg512(Box<[(GnomeId, Vec<u8>); 512]>),
    Reg1024(Box<[(GnomeId, Vec<u8>); 1024]>),
    // Reg2048(Box<[(GnomeId, Vec<u8>); 2048]>),
    // Reg4096(Box<[(GnomeId, Vec<u8>); 4096]>),
    // Reg8192(Box<[(GnomeId, Vec<u8>); 8192]>),
}

fn insert8(gnome_id: GnomeId, key: Vec<u8>, array: &mut [(GnomeId, Vec<u8>); 8]) {
    let mut temp = std::mem::replace(&mut array[0], (gnome_id, key));
    for elem in &mut array[1..] {
        if temp.0 == gnome_id || temp.0 .0 == 0 {
            // eprintln!("Inserted: {}", gnome_id);
            break;
        }
        temp = std::mem::replace(elem, temp);
    }
    // for (id, _key) in array {
    //     eprintln!("Key: {}", id);
    // }
}
fn insert32(gnome_id: GnomeId, key: Vec<u8>, array: &mut [(GnomeId, Vec<u8>); 32]) {
    let mut temp = std::mem::replace(&mut array[0], (gnome_id, key));
    for elem in &mut array[1..] {
        if temp.0 == gnome_id || temp.0 .0 == 0 {
            // println!("Insert complete: {:?}", temp.0);
            break;
        }
        temp = std::mem::replace(elem, temp);
    }
}
fn insert64(gnome_id: GnomeId, key: Vec<u8>, array: &mut [(GnomeId, Vec<u8>); 64]) {
    let mut temp = std::mem::replace(&mut array[0], (gnome_id, key));
    for elem in &mut array[1..] {
        if temp.0 == gnome_id || temp.0 .0 == 0 {
            break;
        }
        temp = std::mem::replace(elem, temp);
    }
}
fn insert128(gnome_id: GnomeId, key: Vec<u8>, array: &mut [(GnomeId, Vec<u8>); 128]) {
    let mut temp = std::mem::replace(&mut array[0], (gnome_id, key));
    for elem in &mut array[1..] {
        if temp.0 == gnome_id || temp.0 .0 == 0 {
            break;
        }
        temp = std::mem::replace(elem, temp);
    }
}
fn insert256(gnome_id: GnomeId, key: Vec<u8>, array: &mut [(GnomeId, Vec<u8>); 256]) {
    let mut temp = std::mem::replace(&mut array[0], (gnome_id, key));
    for elem in &mut array[1..] {
        if temp.0 == gnome_id || temp.0 .0 == 0 {
            break;
        }
        temp = std::mem::replace(elem, temp);
    }
}
fn insert512(gnome_id: GnomeId, key: Vec<u8>, array: &mut [(GnomeId, Vec<u8>); 512]) {
    let mut temp = std::mem::replace(&mut array[0], (gnome_id, key));
    for elem in &mut array[1..] {
        if temp.0 == gnome_id || temp.0 .0 == 0 {
            break;
        }
        temp = std::mem::replace(elem, temp);
    }
}
fn insert1024(gnome_id: GnomeId, key: Vec<u8>, array: &mut [(GnomeId, Vec<u8>); 1024]) {
    let mut temp = std::mem::replace(&mut array[0], (gnome_id, key));
    for elem in &mut array[1..] {
        if temp.0 == gnome_id || temp.0 .0 == 0 {
            break;
        }
        temp = std::mem::replace(elem, temp);
    }
}
fn insert2048(gnome_id: GnomeId, key: Vec<u8>, array: &mut [(GnomeId, Vec<u8>); 2048]) {
    let mut temp = std::mem::replace(&mut array[0], (gnome_id, key));
    for elem in &mut array[1..] {
        if temp.0 == gnome_id || temp.0 .0 == 0 {
            break;
        }
        temp = std::mem::replace(elem, temp);
    }
}
fn insert4096(gnome_id: GnomeId, key: Vec<u8>, array: &mut [(GnomeId, Vec<u8>); 4096]) {
    let mut temp = std::mem::replace(&mut array[0], (gnome_id, key));
    for elem in &mut array[1..] {
        if temp.0 == gnome_id || temp.0 .0 == 0 {
            break;
        }
        temp = std::mem::replace(elem, temp);
    }
}
fn insert8192(gnome_id: GnomeId, key: Vec<u8>, array: &mut [(GnomeId, Vec<u8>); 8192]) {
    let mut temp = std::mem::replace(&mut array[0], (gnome_id, key));
    for elem in &mut array[1..] {
        if temp.0 == gnome_id || temp.0 .0 == 0 {
            break;
        }
        temp = std::mem::replace(elem, temp);
    }
}
fn get8(gnome_id: GnomeId, array: &[(GnomeId, Vec<u8>); 8]) -> Option<Vec<u8>> {
    for (id, key) in array {
        if id.0 == gnome_id.0 {
            // println!("Found: {:?}", id);
            return Some(key.clone());
        }
    }
    None
}
fn get32(gnome_id: GnomeId, array: &[(GnomeId, Vec<u8>); 32]) -> Option<Vec<u8>> {
    for (id, key) in array {
        if id.0 == gnome_id.0 {
            // println!("Found: {:?}", id);
            return Some(key.clone());
        }
    }
    None
}
fn get64(gnome_id: GnomeId, array: &[(GnomeId, Vec<u8>); 64]) -> Option<Vec<u8>> {
    for (id, key) in array {
        if id.0 == gnome_id.0 {
            // println!("Found: {:?}", id);
            return Some(key.clone());
        }
    }
    None
}
fn get128(gnome_id: GnomeId, array: &[(GnomeId, Vec<u8>); 128]) -> Option<Vec<u8>> {
    for (id, key) in array {
        if id.0 == gnome_id.0 {
            // println!("Found: {:?}", id);
            return Some(key.clone());
        }
    }
    None
}
fn get256(gnome_id: GnomeId, array: &[(GnomeId, Vec<u8>); 256]) -> Option<Vec<u8>> {
    for (id, key) in array {
        if id.0 == gnome_id.0 {
            // println!("Found: {:?}", id);
            return Some(key.clone());
        }
    }
    None
}
fn get512(gnome_id: GnomeId, array: &[(GnomeId, Vec<u8>); 512]) -> Option<Vec<u8>> {
    for (id, key) in array {
        if id.0 == gnome_id.0 {
            // println!("Found: {:?}", id);
            return Some(key.clone());
        }
    }
    None
}
fn get1024(gnome_id: GnomeId, array: &[(GnomeId, Vec<u8>); 1024]) -> Option<Vec<u8>> {
    for (id, key) in array {
        if id.0 == gnome_id.0 {
            // println!("Found: {:?}", id);
            return Some(key.clone());
        }
    }
    None
}
fn get2048(gnome_id: GnomeId, array: &[(GnomeId, Vec<u8>); 2048]) -> Option<Vec<u8>> {
    for (id, key) in array {
        if id.0 == gnome_id.0 {
            // println!("Found: {:?}", id);
            return Some(key.clone());
        }
    }
    None
}
fn get4096(gnome_id: GnomeId, array: &[(GnomeId, Vec<u8>); 4096]) -> Option<Vec<u8>> {
    for (id, key) in array {
        if id.0 == gnome_id.0 {
            // println!("Found: {:?}", id);
            return Some(key.clone());
        }
    }
    None
}
fn get8192(gnome_id: GnomeId, array: &[(GnomeId, Vec<u8>); 8192]) -> Option<Vec<u8>> {
    for (id, key) in array {
        if id.0 == gnome_id.0 {
            // println!("Found: {:?}", id);
            return Some(key.clone());
        }
    }
    None
}
fn has8(gnome_id: GnomeId, array: &[(GnomeId, Vec<u8>); 8]) -> bool {
    for (id, _key) in array {
        if id.0 == gnome_id.0 {
            // println!("Found: {:?}", id);
            return true;
        }
    }
    false
}
fn has32(gnome_id: GnomeId, array: &[(GnomeId, Vec<u8>); 32]) -> bool {
    for (id, _key) in array {
        if id.0 == gnome_id.0 {
            // println!("Found: {:?}", id);
            return true;
        }
    }
    false
}
fn has64(gnome_id: GnomeId, array: &[(GnomeId, Vec<u8>); 64]) -> bool {
    for (id, _key) in array {
        if id.0 == gnome_id.0 {
            // println!("Found: {:?}", id);
            return true;
        }
    }
    false
}
fn has128(gnome_id: GnomeId, array: &[(GnomeId, Vec<u8>); 128]) -> bool {
    for (id, _key) in array {
        if id.0 == gnome_id.0 {
            // println!("Found: {:?}", id);
            return true;
        }
    }
    false
}
fn has256(gnome_id: GnomeId, array: &[(GnomeId, Vec<u8>); 256]) -> bool {
    for (id, _key) in array {
        if id.0 == gnome_id.0 {
            // println!("Found: {:?}", id);
            return true;
        }
    }
    false
}
fn has512(gnome_id: GnomeId, array: &[(GnomeId, Vec<u8>); 512]) -> bool {
    for (id, _key) in array {
        if id.0 == gnome_id.0 {
            // println!("Found: {:?}", id);
            return true;
        }
    }
    false
}
fn has1024(gnome_id: GnomeId, array: &[(GnomeId, Vec<u8>); 1024]) -> bool {
    for (id, _key) in array {
        if id.0 == gnome_id.0 {
            // println!("Found: {:?}", id);
            return true;
        }
    }
    false
}
fn has2048(gnome_id: GnomeId, array: &[(GnomeId, Vec<u8>); 2048]) -> bool {
    for (id, _key) in array {
        if id.0 == gnome_id.0 {
            // println!("Found: {:?}", id);
            return true;
        }
    }
    false
}
fn has4096(gnome_id: GnomeId, array: &[(GnomeId, Vec<u8>); 4096]) -> bool {
    for (id, _key) in array {
        if id.0 == gnome_id.0 {
            // println!("Found: {:?}", id);
            return true;
        }
    }
    false
}
fn has8192(gnome_id: GnomeId, array: &[(GnomeId, Vec<u8>); 8192]) -> bool {
    for (id, _key) in array {
        if id.0 == gnome_id.0 {
            // println!("Found: {:?}", id);
            return true;
        }
    }
    false
}

fn chunks_rev8(array: &[(GnomeId, Vec<u8>); 8]) -> Vec<Vec<(GnomeId, Vec<u8>)>> {
    let mut total = Vec::with_capacity(2);
    let mut bytes = Vec::with_capacity(4);
    let array: [(GnomeId, Vec<u8>); 8] = array.clone();
    let mut inserted: u8 = 0;
    for (g_id, pub_key) in array {
        if inserted == 4 {
            total.push(bytes);
            bytes = Vec::with_capacity(4);
            inserted = 0;
        }
        if g_id.0 == 0 {
            break;
        }
        inserted += 1;
        bytes.push((g_id, pub_key));
    }
    if !bytes.is_empty() {
        total.push(bytes);
    }
    total
}

fn chunks_rev32(array: &[(GnomeId, Vec<u8>); 32]) -> Vec<Vec<(GnomeId, Vec<u8>)>> {
    let mut total = Vec::with_capacity(8);
    let mut bytes = Vec::with_capacity(4);
    let array: [(GnomeId, Vec<u8>); 32] = array.clone();
    let mut inserted: u8 = 0;
    for (g_id, pub_key) in array {
        if inserted == 4 {
            total.push(bytes);
            bytes = Vec::with_capacity(4);
            inserted = 0;
        }
        if g_id.0 == 0 {
            break;
        }
        inserted += 1;
        bytes.push((g_id, pub_key));
    }
    if !bytes.is_empty() {
        total.push(bytes);
    }
    total
}

fn chunks_rev64(array: &[(GnomeId, Vec<u8>); 64]) -> Vec<Vec<(GnomeId, Vec<u8>)>> {
    let mut total = Vec::with_capacity(16);
    let mut bytes = Vec::with_capacity(4);
    let array: [(GnomeId, Vec<u8>); 64] = array.clone();
    let mut inserted: u8 = 0;
    for (g_id, pub_key) in array {
        if inserted == 4 {
            total.push(bytes);
            bytes = Vec::with_capacity(4);
            inserted = 0;
        }
        if g_id.0 == 0 {
            break;
        }
        inserted += 1;
        bytes.push((g_id, pub_key));
    }
    if !bytes.is_empty() {
        total.push(bytes);
    }
    total
}

fn chunks_rev128(array: &[(GnomeId, Vec<u8>); 128]) -> Vec<Vec<(GnomeId, Vec<u8>)>> {
    let mut total = Vec::with_capacity(32);
    let mut bytes = Vec::with_capacity(4);
    let array: [(GnomeId, Vec<u8>); 128] = array.clone();
    let mut inserted: u8 = 0;
    for (g_id, pub_key) in array {
        if inserted == 4 {
            total.push(bytes);
            bytes = Vec::with_capacity(4);
            inserted = 0;
        }
        if g_id.0 == 0 {
            break;
        }
        inserted += 1;
        bytes.push((g_id, pub_key));
    }
    if !bytes.is_empty() {
        total.push(bytes);
    }
    total
}

fn chunks_rev256(array: &[(GnomeId, Vec<u8>); 256]) -> Vec<Vec<(GnomeId, Vec<u8>)>> {
    let mut total = Vec::with_capacity(64);
    let mut bytes = Vec::with_capacity(4);
    let array: [(GnomeId, Vec<u8>); 256] = array.clone();
    let mut inserted: u8 = 0;
    for (g_id, pub_key) in array {
        if inserted == 4 {
            total.push(bytes);
            bytes = Vec::with_capacity(4);
            inserted = 0;
        }
        if g_id.0 == 0 {
            break;
        }
        inserted += 1;
        bytes.push((g_id, pub_key));
    }
    if !bytes.is_empty() {
        total.push(bytes);
    }
    total
}
fn chunks_rev512(array: &[(GnomeId, Vec<u8>); 512]) -> Vec<Vec<(GnomeId, Vec<u8>)>> {
    let mut total = Vec::with_capacity(128);
    let mut bytes = Vec::with_capacity(4);
    let array: [(GnomeId, Vec<u8>); 512] = array.clone();
    let mut inserted: u8 = 0;
    for (g_id, pub_key) in array {
        if inserted == 4 {
            total.push(bytes);
            bytes = Vec::with_capacity(4);
            inserted = 0;
        }
        if g_id.0 == 0 {
            break;
        }
        inserted += 1;
        bytes.push((g_id, pub_key));
    }
    if !bytes.is_empty() {
        total.push(bytes);
    }
    total
}
fn chunks_rev1024(array: &[(GnomeId, Vec<u8>); 1024]) -> Vec<Vec<(GnomeId, Vec<u8>)>> {
    let mut total = Vec::with_capacity(256);
    let mut bytes = Vec::with_capacity(4);
    let array: [(GnomeId, Vec<u8>); 1024] = array.clone();
    let mut inserted: u8 = 0;
    for (g_id, pub_key) in array {
        if inserted == 4 {
            total.push(bytes);
            bytes = Vec::with_capacity(4);
            inserted = 0;
        }
        if g_id.0 == 0 {
            break;
        }
        inserted += 1;
        bytes.push((g_id, pub_key));
    }
    if !bytes.is_empty() {
        total.push(bytes);
    }
    total
}

// fn bytes_rev2048(array: &[(GnomeId, Vec<u8>); 2048]) -> Vec<u8> {
//     let mut bytes = Vec::with_capacity(2 + 2048 * 82);
//     // This byte is used to inform how many entries have been inserted
//     bytes.push(0);
//     bytes.push(0);
//     let mut array: [(GnomeId, Vec<u8>); 2048] = array.clone();
//     array.reverse();
//     let mut inserted: u16 = 0;
//     for (g_id, mut pub_key) in array {
//         if g_id.0 == 0 {
//             continue;
//         }
//         inserted += 1;
//         for byte in g_id.0.to_be_bytes() {
//             bytes.push(byte);
//         }
//         bytes.append(&mut pub_key);
//     }

//     let [b0, b1] = inserted.to_be_bytes();
//     bytes[0] = b0;
//     bytes[1] = b1;
//     bytes
// }

// fn bytes_rev4096(array: &[(GnomeId, Vec<u8>); 4096]) -> Vec<u8> {
//     let mut bytes = Vec::with_capacity(2 + 4096 * 82);
//     // This byte is used to inform how many entries have been inserted
//     bytes.push(0);
//     bytes.push(0);
//     let mut array: [(GnomeId, Vec<u8>); 4096] = array.clone();
//     array.reverse();
//     let mut inserted: u16 = 0;
//     for (g_id, mut pub_key) in array {
//         if g_id.0 == 0 {
//             continue;
//         }
//         inserted += 1;
//         for byte in g_id.0.to_be_bytes() {
//             bytes.push(byte);
//         }
//         bytes.append(&mut pub_key);
//     }

//     let [b0, b1] = inserted.to_be_bytes();
//     bytes[0] = b0;
//     bytes[1] = b1;
//     bytes
// }
// fn bytes_rev8192(array: &[(GnomeId, Vec<u8>); 8192]) -> Vec<u8> {
//     let mut bytes = Vec::with_capacity(2 + 8192 * 82);
//     // This byte is used to inform how many entries have been inserted
//     bytes.push(0);
//     bytes.push(0);
//     let mut array: [(GnomeId, Vec<u8>); 8192] = array.clone();
//     array.reverse();
//     let mut inserted: u16 = 0;
//     for (g_id, mut pub_key) in array {
//         if g_id.0 == 0 {
//             continue;
//         }
//         inserted += 1;
//         for byte in g_id.0.to_be_bytes() {
//             bytes.push(byte);
//         }
//         bytes.append(&mut pub_key);
//     }

//     let [b0, b1] = inserted.to_be_bytes();
//     bytes[0] = b0;
//     bytes[1] = b1;
//     bytes
// }

impl KeyRegistry {
    pub fn new8() -> Self {
        KeyRegistry::Reg8(Box::new(core::array::from_fn(|_i| (GnomeId(0), vec![]))))
    }
    pub fn new32() -> Self {
        KeyRegistry::Reg32(Box::new(core::array::from_fn(|_i| (GnomeId(0), vec![]))))
    }
    pub fn new64() -> Self {
        KeyRegistry::Reg64(Box::new(core::array::from_fn(|_i| (GnomeId(0), vec![]))))
    }
    pub fn new128() -> Self {
        KeyRegistry::Reg128(Box::new(core::array::from_fn(|_i| (GnomeId(0), vec![]))))
    }
    pub fn new256() -> Self {
        KeyRegistry::Reg256(Box::new(core::array::from_fn(|_i| (GnomeId(0), vec![]))))
    }
    pub fn new512() -> Self {
        KeyRegistry::Reg512(Box::new(core::array::from_fn(|_i| (GnomeId(0), vec![]))))
    }
    pub fn new1024() -> Self {
        KeyRegistry::Reg1024(Box::new(core::array::from_fn(|_i| (GnomeId(0), vec![]))))
    }
    // pub fn new2048() -> Self {
    //     KeyRegistry::Reg2048(Box::new(core::array::from_fn(|_i| (GnomeId(0), vec![]))))
    // }
    // pub fn new4096() -> Self {
    //     KeyRegistry::Reg4096(Box::new(core::array::from_fn(|_i| (GnomeId(0), vec![]))))
    // }
    // pub fn new8192() -> Self {
    //     KeyRegistry::Reg8192(Box::new(core::array::from_fn(|_i| (GnomeId(0), vec![]))))
    // }
    pub fn byte(&self) -> u8 {
        match self {
            Self::Reg8(_arr) => 1,
            Self::Reg32(_arr) => 2,
            Self::Reg64(_arr) => 3,
            Self::Reg128(_arr) => 4,
            Self::Reg256(_arr) => 5,
            Self::Reg512(_arr) => 6,
            Self::Reg1024(_arr) => 7,
            // Self::Reg2048(_arr) => 8,
            // Self::Reg4096(_arr) => 9,
            // Self::Reg8192(_arr) => 10,
        }
    }
    pub fn from(bytes: &mut Vec<u8>) -> Self {
        // println!("KeyReg from {:?}", bytes);
        let mut drain = bytes.drain(0..3);
        let header = drain.next().unwrap();
        let mut count: u16 = drain.next().unwrap() as u16;
        count <<= 8;
        count += drain.next().unwrap() as u16;
        drop(drain);
        // println!("KeyReg after drain {:?}", bytes);
        let mut registry = match header {
            1 => KeyRegistry::new8(),
            2 => KeyRegistry::new32(),
            3 => KeyRegistry::new64(),
            4 => KeyRegistry::new128(),
            5 => KeyRegistry::new256(),
            6 => KeyRegistry::new512(),
            7 => KeyRegistry::new1024(),
            // 8 => KeyRegistry::new2048(),
            // 9 => KeyRegistry::new4096(),
            // 10 => KeyRegistry::new8192(),
            _ => KeyRegistry::new8(),
        };
        // println!("Sync KeyReg of {}", count);
        for _i in 0..count {
            let mut g_id_drain = bytes.drain(0..8);
            let mut g_id_arr: [u8; 8] = [0; 8];
            for i in 0..8 {
                g_id_arr[i] = g_id_drain.next().unwrap();
            }
            let g_id: u64 = u64::from_be_bytes(g_id_arr);
            drop(g_id_drain);
            let mut pub_key_drain = bytes.drain(0..74);
            let mut pub_key_arr: [u8; 74] = [0; 74];
            for i in 0..74 {
                pub_key_arr[i] = pub_key_drain.next().unwrap();
            }
            let pub_key: Vec<u8> = Vec::from(pub_key_arr);
            registry.insert(GnomeId(g_id), pub_key);
        }
        registry
    }
    pub fn size_byte(&self) -> u8 {
        match self {
            Self::Reg8(_arr) => 1,
            Self::Reg32(_arr) => 2,
            Self::Reg64(_arr) => 3,
            Self::Reg128(_arr) => 4,
            Self::Reg256(_arr) => 5,
            Self::Reg512(_arr) => 6,
            Self::Reg1024(_arr) => 7,
        }
    }
    pub fn chunks(&self) -> Vec<Vec<(GnomeId, Vec<u8>)>> {
        // println!("Returning KeyRegistry as bytes");
        match self {
            Self::Reg8(arr) => chunks_rev8(arr.deref()),
            Self::Reg32(arr) => chunks_rev32(arr.deref()),
            Self::Reg64(arr) => chunks_rev64(arr.deref()),
            Self::Reg128(arr) => chunks_rev128(arr.deref()),
            Self::Reg256(arr) => chunks_rev256(arr.deref()),
            Self::Reg512(arr) => chunks_rev512(arr.deref()),
            Self::Reg1024(arr) => chunks_rev1024(arr.deref()),
        }
    }

    pub fn insert(&mut self, gnome_id: GnomeId, key: Vec<u8>) {
        // eprintln!("Inserting key for {}", gnome_id);
        match self {
            Self::Reg8(arr) => insert8(gnome_id, key, arr.deref_mut()),
            Self::Reg32(arr) => insert32(gnome_id, key, arr.deref_mut()),
            Self::Reg64(arr) => insert64(gnome_id, key, arr.deref_mut()),
            Self::Reg128(arr) => insert128(gnome_id, key, arr.deref_mut()),
            Self::Reg256(arr) => insert256(gnome_id, key, arr.deref_mut()),
            Self::Reg512(arr) => insert512(gnome_id, key, arr.deref_mut()),
            Self::Reg1024(arr) => insert1024(gnome_id, key, arr.deref_mut()),
            // Self::Reg2048(arr) => insert2048(gnome_id, key, arr.deref_mut()),
            // Self::Reg4096(arr) => insert4096(gnome_id, key, arr.deref_mut()),
            // Self::Reg8192(arr) => insert8192(gnome_id, key, arr.deref_mut()),
        };
    }
    pub fn get(&self, gnome_id: GnomeId) -> Option<Vec<u8>> {
        // println!("Searching for {}", gnome_id);
        match self {
            Self::Reg8(arr) => get8(gnome_id, arr),
            Self::Reg32(arr) => get32(gnome_id, arr),
            Self::Reg64(arr) => get64(gnome_id, arr),
            Self::Reg128(arr) => get128(gnome_id, arr),
            Self::Reg256(arr) => get256(gnome_id, arr),
            Self::Reg512(arr) => get512(gnome_id, arr),
            Self::Reg1024(arr) => get1024(gnome_id, arr),
            // Self::Reg2048(arr) => get2048(gnome_id, arr),
            // Self::Reg4096(arr) => get4096(gnome_id, arr),
            // Self::Reg8192(arr) => get8192(gnome_id, arr),
        }
    }
    pub fn has_key(&self, gnome_id: GnomeId) -> bool {
        // println!("Searching for {}", gnome_id);
        match self {
            Self::Reg8(arr) => has8(gnome_id, arr),
            Self::Reg32(arr) => has32(gnome_id, arr),
            Self::Reg64(arr) => has64(gnome_id, arr),
            Self::Reg128(arr) => has128(gnome_id, arr),
            Self::Reg256(arr) => has256(gnome_id, arr),
            Self::Reg512(arr) => has512(gnome_id, arr),
            Self::Reg1024(arr) => has1024(gnome_id, arr),
            // Self::Reg2048(arr) => has2048(gnome_id, arr),
            // Self::Reg4096(arr) => has4096(gnome_id, arr),
            // Self::Reg8192(arr) => has8192(gnome_id, arr),
        }
    }
}
