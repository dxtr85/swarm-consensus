use crate::Gnome;
use crate::Neighbor;
use crate::Request;
use crate::Response;
use std::fmt;
use std::ops::Add;
use std::ops::Sub;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread::{spawn, JoinHandle};

#[derive(PartialEq, PartialOrd, Eq, Clone, Copy, Debug)]
pub struct SwarmTime(pub u32);

impl SwarmTime {
    pub fn inc(&self) -> Self {
        SwarmTime(self.0.wrapping_add(1))
    }
}

impl Sub for SwarmTime {
    type Output = Self;
    fn sub(self, rhs: SwarmTime) -> Self::Output {
        SwarmTime(self.0.wrapping_sub(rhs.0))
    }
}

impl Add for SwarmTime {
    type Output = SwarmTime;
    fn add(self, rhs: SwarmTime) -> Self::Output {
        SwarmTime(self.0 + rhs.0)
    }
}

#[derive(PartialEq, Eq, Hash, Debug, Copy, Clone)]
pub struct SwarmID(pub u8);

pub struct Swarm {
    pub name: String,
    pub id: SwarmID,
    pub sender: Sender<Request>,
    pub receiver: Option<Receiver<Response>>,
    pub join_handle: Option<JoinHandle<()>>,
}

impl Swarm {
    pub fn join(
        name: String,
        id: SwarmID,
        neighbors: Option<Vec<Neighbor>>,
        band_receiver: Receiver<u32>,
    ) -> Swarm {
        let (sender, request_receiver) = channel::<Request>();
        let (response_sender, receiver) = channel::<Response>();

        let gnome = if let Some(neighbors) = neighbors {
            Gnome::new_with_neighbors(
                id,
                response_sender,
                request_receiver,
                band_receiver,
                neighbors,
            )
        } else {
            Gnome::new(id, response_sender, request_receiver, band_receiver)
        };
        let join_handle = spawn(move || {
            gnome.do_your_job();
        });

        Swarm {
            name,
            id,
            sender,
            receiver: Some(receiver),
            join_handle: Some(join_handle),
        }
    }
}
impl fmt::Display for SwarmTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ST{:010}", self.0)
    }
}
