use std::ops::DerefMut;

use crate::GnomeId;
pub enum KeyRegistry {
    Reg8(Box<[(GnomeId, Vec<u8>); 8]>),
    Reg32(Box<[(GnomeId, Vec<u8>); 32]>),
    Reg64(Box<[(GnomeId, Vec<u8>); 64]>),
    Reg128(Box<[(GnomeId, Vec<u8>); 128]>),
    Reg256(Box<[(GnomeId, Vec<u8>); 256]>),
    Reg512(Box<[(GnomeId, Vec<u8>); 512]>),
    Reg1024(Box<[(GnomeId, Vec<u8>); 1024]>),
    Reg2048(Box<[(GnomeId, Vec<u8>); 2048]>),
    Reg4096(Box<[(GnomeId, Vec<u8>); 4096]>),
    Reg8192(Box<[(GnomeId, Vec<u8>); 8192]>),
}

fn insert8(gnome_id: GnomeId, key: Vec<u8>, array: &mut [(GnomeId, Vec<u8>); 8]) {
    let mut temp = std::mem::replace(&mut array[0], (gnome_id, key));
    for elem in &mut array[1..] {
        if temp.0 == gnome_id || temp.0 .0 == 0 {
            println!("Insert complete: {:?}", temp.0);
            break;
        }
        temp = std::mem::replace(elem, temp);
    }
}
fn insert32(gnome_id: GnomeId, key: Vec<u8>, array: &mut [(GnomeId, Vec<u8>); 32]) {
    let mut temp = std::mem::replace(&mut array[0], (gnome_id, key));
    for elem in &mut array[1..] {
        if temp.0 == gnome_id || temp.0 .0 == 0 {
            println!("Insert complete: {:?}", temp.0);
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
            println!("Found: {:?}", id);
            return Some(key.clone());
        }
    }
    None
}
fn get32(gnome_id: GnomeId, array: &[(GnomeId, Vec<u8>); 32]) -> Option<Vec<u8>> {
    for (id, key) in array {
        if id.0 == gnome_id.0 {
            println!("Found: {:?}", id);
            return Some(key.clone());
        }
    }
    None
}
fn get64(gnome_id: GnomeId, array: &[(GnomeId, Vec<u8>); 64]) -> Option<Vec<u8>> {
    for (id, key) in array {
        if id.0 == gnome_id.0 {
            println!("Found: {:?}", id);
            return Some(key.clone());
        }
    }
    None
}
fn get128(gnome_id: GnomeId, array: &[(GnomeId, Vec<u8>); 128]) -> Option<Vec<u8>> {
    for (id, key) in array {
        if id.0 == gnome_id.0 {
            println!("Found: {:?}", id);
            return Some(key.clone());
        }
    }
    None
}
fn get256(gnome_id: GnomeId, array: &[(GnomeId, Vec<u8>); 256]) -> Option<Vec<u8>> {
    for (id, key) in array {
        if id.0 == gnome_id.0 {
            println!("Found: {:?}", id);
            return Some(key.clone());
        }
    }
    None
}
fn get512(gnome_id: GnomeId, array: &[(GnomeId, Vec<u8>); 512]) -> Option<Vec<u8>> {
    for (id, key) in array {
        if id.0 == gnome_id.0 {
            println!("Found: {:?}", id);
            return Some(key.clone());
        }
    }
    None
}
fn get1024(gnome_id: GnomeId, array: &[(GnomeId, Vec<u8>); 1024]) -> Option<Vec<u8>> {
    for (id, key) in array {
        if id.0 == gnome_id.0 {
            println!("Found: {:?}", id);
            return Some(key.clone());
        }
    }
    None
}
fn get2048(gnome_id: GnomeId, array: &[(GnomeId, Vec<u8>); 2048]) -> Option<Vec<u8>> {
    for (id, key) in array {
        if id.0 == gnome_id.0 {
            println!("Found: {:?}", id);
            return Some(key.clone());
        }
    }
    None
}
fn get4096(gnome_id: GnomeId, array: &[(GnomeId, Vec<u8>); 4096]) -> Option<Vec<u8>> {
    for (id, key) in array {
        if id.0 == gnome_id.0 {
            println!("Found: {:?}", id);
            return Some(key.clone());
        }
    }
    None
}
fn get8192(gnome_id: GnomeId, array: &[(GnomeId, Vec<u8>); 8192]) -> Option<Vec<u8>> {
    for (id, key) in array {
        if id.0 == gnome_id.0 {
            println!("Found: {:?}", id);
            return Some(key.clone());
        }
    }
    None
}
fn has8(gnome_id: GnomeId, array: &[(GnomeId, Vec<u8>); 8]) -> bool {
    for (id, _key) in array {
        if id.0 == gnome_id.0 {
            println!("Found: {:?}", id);
            return true;
        }
    }
    false
}
fn has32(gnome_id: GnomeId, array: &[(GnomeId, Vec<u8>); 32]) -> bool {
    for (id, _key) in array {
        if id.0 == gnome_id.0 {
            println!("Found: {:?}", id);
            return true;
        }
    }
    false
}
fn has64(gnome_id: GnomeId, array: &[(GnomeId, Vec<u8>); 64]) -> bool {
    for (id, _key) in array {
        if id.0 == gnome_id.0 {
            println!("Found: {:?}", id);
            return true;
        }
    }
    false
}
fn has128(gnome_id: GnomeId, array: &[(GnomeId, Vec<u8>); 128]) -> bool {
    for (id, _key) in array {
        if id.0 == gnome_id.0 {
            println!("Found: {:?}", id);
            return true;
        }
    }
    false
}
fn has256(gnome_id: GnomeId, array: &[(GnomeId, Vec<u8>); 256]) -> bool {
    for (id, _key) in array {
        if id.0 == gnome_id.0 {
            println!("Found: {:?}", id);
            return true;
        }
    }
    false
}
fn has512(gnome_id: GnomeId, array: &[(GnomeId, Vec<u8>); 512]) -> bool {
    for (id, _key) in array {
        if id.0 == gnome_id.0 {
            println!("Found: {:?}", id);
            return true;
        }
    }
    false
}
fn has1024(gnome_id: GnomeId, array: &[(GnomeId, Vec<u8>); 1024]) -> bool {
    for (id, _key) in array {
        if id.0 == gnome_id.0 {
            println!("Found: {:?}", id);
            return true;
        }
    }
    false
}
fn has2048(gnome_id: GnomeId, array: &[(GnomeId, Vec<u8>); 2048]) -> bool {
    for (id, _key) in array {
        if id.0 == gnome_id.0 {
            println!("Found: {:?}", id);
            return true;
        }
    }
    false
}
fn has4096(gnome_id: GnomeId, array: &[(GnomeId, Vec<u8>); 4096]) -> bool {
    for (id, _key) in array {
        if id.0 == gnome_id.0 {
            println!("Found: {:?}", id);
            return true;
        }
    }
    false
}
fn has8192(gnome_id: GnomeId, array: &[(GnomeId, Vec<u8>); 8192]) -> bool {
    for (id, _key) in array {
        if id.0 == gnome_id.0 {
            println!("Found: {:?}", id);
            return true;
        }
    }
    false
}
impl KeyRegistry {
    // fn get_array(&mut self)
    pub fn insert(&mut self, gnome_id: GnomeId, key: Vec<u8>) {
        println!("Inserting key for {}", gnome_id);
        match self {
            Self::Reg8(arr) => insert8(gnome_id, key, arr.deref_mut()),
            Self::Reg32(arr) => insert32(gnome_id, key, arr.deref_mut()),
            Self::Reg64(arr) => insert64(gnome_id, key, arr.deref_mut()),
            Self::Reg128(arr) => insert128(gnome_id, key, arr.deref_mut()),
            Self::Reg256(arr) => insert256(gnome_id, key, arr.deref_mut()),
            Self::Reg512(arr) => insert512(gnome_id, key, arr.deref_mut()),
            Self::Reg1024(arr) => insert1024(gnome_id, key, arr.deref_mut()),
            Self::Reg2048(arr) => insert2048(gnome_id, key, arr.deref_mut()),
            Self::Reg4096(arr) => insert4096(gnome_id, key, arr.deref_mut()),
            Self::Reg8192(arr) => insert8192(gnome_id, key, arr.deref_mut()),
        };
    }
    pub fn get(&self, gnome_id: GnomeId) -> Option<Vec<u8>> {
        println!("Searching for {}", gnome_id);
        match self {
            Self::Reg8(arr) => get8(gnome_id, arr),
            Self::Reg32(arr) => get32(gnome_id, arr),
            Self::Reg64(arr) => get64(gnome_id, arr),
            Self::Reg128(arr) => get128(gnome_id, arr),
            Self::Reg256(arr) => get256(gnome_id, arr),
            Self::Reg512(arr) => get512(gnome_id, arr),
            Self::Reg1024(arr) => get1024(gnome_id, arr),
            Self::Reg2048(arr) => get2048(gnome_id, arr),
            Self::Reg4096(arr) => get4096(gnome_id, arr),
            Self::Reg8192(arr) => get8192(gnome_id, arr),
        }
    }
    pub fn has_key(&self, gnome_id: GnomeId) -> bool {
        println!("Searching for {}", gnome_id);
        match self {
            Self::Reg8(arr) => has8(gnome_id, arr),
            Self::Reg32(arr) => has32(gnome_id, arr),
            Self::Reg64(arr) => has64(gnome_id, arr),
            Self::Reg128(arr) => has128(gnome_id, arr),
            Self::Reg256(arr) => has256(gnome_id, arr),
            Self::Reg512(arr) => has512(gnome_id, arr),
            Self::Reg1024(arr) => has1024(gnome_id, arr),
            Self::Reg2048(arr) => has2048(gnome_id, arr),
            Self::Reg4096(arr) => has4096(gnome_id, arr),
            Self::Reg8192(arr) => has8192(gnome_id, arr),
        }
    }
}
