use crate::Gnome;
use crate::Neighbor;
use crate::Request;
use crate::Response;
use std::ops::Sub;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread::{spawn, JoinHandle};

#[derive(PartialEq, PartialOrd, Eq, Clone, Copy, Debug)]
pub struct SwarmTime(pub u32);

impl SwarmTime {
    pub fn inc(&self) -> Self {
        SwarmTime(self.0 + 1)
    }
}

impl Sub for SwarmTime {
    type Output = u32;
    fn sub(self, rhs: SwarmTime) -> Self::Output {
        self.0 - rhs.0
    }
}

pub struct Swarm {
    pub name: String,
    pub sender: Sender<Request>,
    pub receiver: Option<Receiver<Response>>,
    pub join_handle: Option<JoinHandle<()>>,
}

impl Swarm {
    pub fn join(name: String, neighbors: Option<Vec<Neighbor>>) -> Swarm {
        let (sender, request_receiver) = channel::<Request>();
        let (response_sender, receiver) = channel::<Response>();

        let gnome = if let Some(neighbors) = neighbors {
            Gnome::new_with_neighbors(response_sender, request_receiver, neighbors)
        } else {
            Gnome::new(response_sender, request_receiver)
        };
        let join_handle = spawn(move || {
            gnome.do_your_job();
        });

        Swarm {
            name,
            sender,
            receiver: Some(receiver),
            join_handle: Some(join_handle),
        }
    }
}
