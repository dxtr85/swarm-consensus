use crate::Gnome;
use crate::Neighbor;
use crate::Request;
use crate::Response;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread::{spawn, JoinHandle};

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

        let mut gnome = Gnome::new(response_sender, request_receiver);
        if let Some(neighbors) = neighbors {
            for neighbor in neighbors {
                gnome.add_neighbor(neighbor);
            }
        }
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
