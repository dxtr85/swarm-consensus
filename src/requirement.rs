use std::collections::HashMap;

use crate::{CapabiliTree, Capabilities, GnomeId};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Requirement {
    And(Box<Requirement>, Box<Requirement>),
    Or(Box<Requirement>, Box<Requirement>),
    Has(Capabilities),
    None,
}

impl Requirement {
    pub fn from(bytes: &mut Vec<u8>) -> Self {
        let byte = bytes.drain(0..1).next().unwrap();
        match byte {
            200 => {
                let left_req = Requirement::from(bytes);
                let right_req = Requirement::from(bytes);
                Requirement::And(Box::new(left_req), Box::new(right_req))
            }
            100 => {
                let left_req = Requirement::from(bytes);
                let right_req = Requirement::from(bytes);
                Requirement::Or(Box::new(left_req), Box::new(right_req))
            }
            50 => {
                let c_byte = bytes.drain(0..1).next().unwrap();
                Requirement::Has(Capabilities::from(c_byte))
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

    pub fn bytes(&self, bytes: &mut Vec<u8>) {
        match self {
            Self::And(r_l, r_r) => {
                bytes.push(200);
                r_l.bytes(bytes);
                r_r.bytes(bytes)
            }
            Self::Or(r_l, r_r) => {
                bytes.push(100);
                r_l.bytes(bytes);
                r_r.bytes(bytes)
            }
            Self::Has(_c) => {
                bytes.push(50);
                bytes.push(_c.byte());
            }
            Self::None => bytes.push(0),
        }
    }

    pub fn is_fullfilled(
        &self,
        gnome_id: &GnomeId,
        caps: &HashMap<Capabilities, CapabiliTree>,
    ) -> bool {
        match self {
            Requirement::None => true,
            Requirement::Has(capa) => {
                if let Some(tree) = caps.get(capa) {
                    tree.contains(gnome_id)
                } else {
                    false
                }
            }
            Requirement::And(req_one, req_two) => {
                req_one.is_fullfilled(gnome_id, caps) && req_two.is_fullfilled(gnome_id, caps)
            }
            Requirement::Or(req_one, req_two) => {
                req_one.is_fullfilled(gnome_id, caps) || req_two.is_fullfilled(gnome_id, caps)
            }
        }
    }
    pub fn len(&self) -> u8 {
        match self {
            Self::And(r_l, r_r) => 1 + r_l.len() + r_r.len(),
            Self::Or(r_l, r_r) => 1 + r_l.len() + r_r.len(),
            Self::Has(_c) => 2,
            Self::None => 1,
        }
    }
}
