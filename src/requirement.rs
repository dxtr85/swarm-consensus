use std::collections::HashMap;

use crate::{Capabilities, GnomeId};

pub enum Requirement {
    And(Box<Requirement>, Box<Requirement>),
    Or(Box<Requirement>, Box<Requirement>),
    Has(Capabilities),
    None,
}

impl Requirement {
    pub fn is_fullfilled(
        &self,
        gnome_id: &GnomeId,
        caps: &HashMap<Capabilities, Vec<GnomeId>>,
    ) -> bool {
        match self {
            Requirement::None => true,
            Requirement::Has(capa) => {
                if let Some(list) = caps.get(capa) {
                    list.contains(gnome_id)
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
}
