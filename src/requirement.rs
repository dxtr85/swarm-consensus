use std::collections::HashMap;

use crate::{swarm::ByteSet, CapabiliTree, Capabilities, GnomeId};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Requirement {
    And(Box<Requirement>, Box<Requirement>),
    Or(Box<Requirement>, Box<Requirement>),
    Has(Capabilities),
    DataByte2InSet(u8),
    DataByte2Is(u8),
    DataByte2IsNot(u8),
    DataByte3InSet(u8),
    DataByte3Is(u8),
    DataByte3IsNot(u8),
    DataBytes2And3InSet(u8),
    None,
}

impl Requirement {
    pub fn from(bytes: &mut Vec<u8>) -> Self {
        let byte = bytes.drain(0..1).next().unwrap();
        match byte {
            220 => {
                let left_req = Requirement::from(bytes);
                let right_req = Requirement::from(bytes);
                Requirement::And(Box::new(left_req), Box::new(right_req))
            }
            210 => {
                let left_req = Requirement::from(bytes);
                let right_req = Requirement::from(bytes);
                Requirement::Or(Box::new(left_req), Box::new(right_req))
            }
            100 => {
                let c_byte = bytes.drain(0..1).next().unwrap();
                Requirement::Has(Capabilities::from(c_byte))
            }
            90 => {
                let s_id = bytes.drain(0..1).next().unwrap();
                Requirement::DataByte2InSet(s_id)
            }
            80 => {
                let val = bytes.drain(0..1).next().unwrap();
                Requirement::DataByte2Is(val)
            }
            70 => {
                let val = bytes.drain(0..1).next().unwrap();
                Requirement::DataByte2IsNot(val)
            }
            60 => {
                let s_id = bytes.drain(0..1).next().unwrap();
                Requirement::DataByte3InSet(s_id)
            }
            50 => {
                let val = bytes.drain(0..1).next().unwrap();
                Requirement::DataByte3Is(val)
            }
            40 => {
                let val = bytes.drain(0..1).next().unwrap();
                Requirement::DataByte3IsNot(val)
            }
            30 => {
                let s_id = bytes.drain(0..1).next().unwrap();
                Requirement::DataBytes2And3InSet(s_id)
            }
            0 => Requirement::None,
            other => {
                panic!(
                    "Unexpected value while parsing bytes for Requirement: {}",
                    other
                )
            }
        }
    }
    pub fn text(&self) -> String {
        match self {
            Self::And(_l, _r) => {
                format!("And(_, _)")
            }
            Self::Or(_l, _r) => {
                format!("Or(_, _)")
            }
            Self::Has(_c) => {
                format!("Has({:?})", _c)
            }
            Self::DataByte2InSet(s_id) => {
                format!("DataByte2InSet({s_id})")
            }
            Self::DataByte2Is(val) => {
                format!("DataByte2Is({val})")
            }
            Self::DataByte2IsNot(s_id) => {
                format!("DataByte2IsNot({s_id})")
            }
            Self::DataByte3InSet(s_id) => {
                format!("DataByte3InSet({s_id})")
            }
            Self::DataByte3Is(val) => {
                format!("DataByte3Is({val})")
            }
            Self::DataByte3IsNot(s_id) => {
                format!("DataByte3IsNot({s_id})")
            }
            Self::DataBytes2And3InSet(s_id) => {
                format!("DataBytes2And3InSet({s_id})")
            }
            Self::None => format!("None"),
        }
    }

    pub fn append_bytes_to(&self, bytes: &mut Vec<u8>) {
        match self {
            Self::And(r_l, r_r) => {
                bytes.push(220);
                r_l.append_bytes_to(bytes);
                r_r.append_bytes_to(bytes)
            }
            Self::Or(r_l, r_r) => {
                bytes.push(210);
                r_l.append_bytes_to(bytes);
                r_r.append_bytes_to(bytes)
            }
            Self::Has(_c) => {
                bytes.push(100);
                bytes.push(_c.byte());
            }
            Self::DataByte2InSet(s_id) => {
                bytes.push(90);
                bytes.push(*s_id);
            }
            Self::DataByte2Is(val) => {
                bytes.push(80);
                bytes.push(*val);
            }
            Self::DataByte2IsNot(s_id) => {
                bytes.push(70);
                bytes.push(*s_id);
            }
            Self::DataByte3InSet(s_id) => {
                bytes.push(60);
                bytes.push(*s_id);
            }
            Self::DataByte3Is(val) => {
                bytes.push(50);
                bytes.push(*val);
            }
            Self::DataByte3IsNot(s_id) => {
                bytes.push(40);
                bytes.push(*s_id);
            }
            Self::DataBytes2And3InSet(s_id) => {
                bytes.push(30);
                bytes.push(*s_id);
            }
            Self::None => bytes.push(0),
        }
    }

    pub fn is_fullfilled(
        &self,
        gnome_id: &GnomeId,
        caps: &HashMap<Capabilities, CapabiliTree>,
        b_sets: &HashMap<u8, ByteSet>,
        byte_2: Option<u8>,
        byte_3: Option<u8>,
    ) -> bool {
        match self {
            Requirement::DataByte2InSet(s_id) => {
                if let Some(b_2) = byte_2 {
                    if let Some(set) = b_sets.get(s_id) {
                        set.contains(&b_2)
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            Requirement::DataByte2Is(val) => {
                if let Some(b_2) = byte_2 {
                    b_2 == *val
                } else {
                    false
                }
            }
            Requirement::DataByte2IsNot(val) => {
                if let Some(b_2) = byte_2 {
                    b_2 != *val
                } else {
                    false
                }
            }
            Requirement::DataByte3InSet(s_id) => {
                if let Some(b_3) = byte_3 {
                    if let Some(set) = b_sets.get(s_id) {
                        set.contains(&b_3)
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            Requirement::DataByte3Is(val) => {
                if let Some(b_3) = byte_3 {
                    b_3 == *val
                } else {
                    false
                }
            }
            Requirement::DataByte3IsNot(val) => {
                if let Some(b_3) = byte_3 {
                    b_3 != *val
                } else {
                    false
                }
            }
            Requirement::DataBytes2And3InSet(s_id) => {
                if let Some(b_2) = byte_2 {
                    if let Some(b_3) = byte_3 {
                        if let Some(set) = b_sets.get(s_id) {
                            set.contains_pair(&u16::from_be_bytes([b_2, b_3]))
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            Requirement::None => {
                // eprintln!("Req: None ");
                true
            }
            Requirement::Has(capa) => {
                eprintln!("Req: Has({:?})", capa);
                if let Some(tree) = caps.get(capa) {
                    tree.contains(gnome_id)
                } else {
                    eprintln!("no tree for req");
                    false
                }
            }
            Requirement::And(req_one, req_two) => {
                // eprintln!("Req: And");
                req_one.is_fullfilled(gnome_id, caps, b_sets, byte_2, byte_3)
                    && req_two.is_fullfilled(gnome_id, caps, b_sets, byte_2, byte_3)
            }
            Requirement::Or(req_one, req_two) => {
                // eprintln!("Req: Or");
                req_one.is_fullfilled(gnome_id, caps, b_sets, byte_2, byte_3)
                    || req_two.is_fullfilled(gnome_id, caps, b_sets, byte_2, byte_3)
            }
        }
    }
    pub fn len(&self) -> u8 {
        match self {
            Self::And(r_l, r_r) => 1 + r_l.len() + r_r.len(),
            Self::Or(r_l, r_r) => 1 + r_l.len() + r_r.len(),
            Self::Has(_c) => 2,
            Self::None => 1,
            _other => 2,
        }
    }
    pub fn mapping(
        inct_logic: bool,
        user_caps: Vec<u8>,
        byte_sets: Vec<u8>,
        two_byte_sets: Vec<u8>,
    ) -> (Vec<Requirement>, Vec<String>) {
        let mut reqs = Vec::with_capacity(512);
        let mut strs = Vec::with_capacity(512);
        // let p = Requirement::;
        let mut r_iter = Requirement::iter(inct_logic, user_caps, byte_sets, two_byte_sets);
        while let Some(r) = r_iter.next() {
            strs.push(r.text());
            reqs.push(r);
        }
        (reqs, strs)
    }
    fn iter(
        inct_logic: bool,
        user_caps: Vec<u8>,
        byte_sets: Vec<u8>,
        two_byte_sets: Vec<u8>,
    ) -> ReqIter {
        ReqIter::new(inct_logic, user_caps, byte_sets, two_byte_sets)
    }
}
struct ReqIter {
    items: Vec<Requirement>,
}
impl ReqIter {
    pub fn new(
        inct_logic: bool,
        user_caps: Vec<u8>,
        byte_sets: Vec<u8>,
        two_byte_sets: Vec<u8>,
    ) -> Self {
        let mut items = Vec::with_capacity(512);
        if inct_logic {
            let empty = Box::new(Requirement::None);
            let r = Requirement::And(empty.clone(), empty.clone());
            items.push(r);
            let r = Requirement::Or(empty.clone(), empty);
            items.push(r);
        }
        items.push(Requirement::Has(Capabilities::Founder));
        items.push(Requirement::Has(Capabilities::Owner));
        items.push(Requirement::Has(Capabilities::Admin));
        items.push(Requirement::Has(Capabilities::Moderator));
        items.push(Requirement::Has(Capabilities::Superuser));
        items.push(Requirement::None);
        for i in user_caps {
            items.push(Requirement::Has(Capabilities::Capability(i)));
        }

        for i in &byte_sets {
            items.push(Requirement::DataByte2InSet(*i));
        }

        for i in 0..=255 {
            items.push(Requirement::DataByte2Is(i));
        }

        for i in 0..=255 {
            items.push(Requirement::DataByte2IsNot(i));
        }

        for i in byte_sets {
            items.push(Requirement::DataByte3InSet(i));
        }

        for i in 0..=255 {
            items.push(Requirement::DataByte3Is(i));
        }

        for i in 0..=255 {
            items.push(Requirement::DataByte3IsNot(i));
        }

        for i in two_byte_sets {
            items.push(Requirement::DataBytes2And3InSet(i));
        }
        // Do not add None
        // None,

        ReqIter { items }
    }
    pub fn next(&mut self) -> Option<Requirement> {
        if self.items.is_empty() {
            None
        } else {
            Some(self.items.remove(0))
        }
    }
}
