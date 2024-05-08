use super::neighbor::Neighborhood;
use super::*;
use std::thread;

use std::{
    sync::mpsc::{channel, Receiver, Sender},
    time::Duration,
};

struct TestManager {
    manager: Manager,
    swarm_id: SwarmID,
    swarm_name: String,
    neighbors: Vec<(GnomeId, Sender<Message>)>,
}

impl TestManager {
    pub fn join_a_swarm(
        swarm_name: &str,
        neighbors_count: u64,
    ) -> (TestManager, Sender<Request>, Receiver<Response>) {
        let (send, recv) = channel();
        let mut manager = Manager::new(GnomeId(0), None, send);
        let mut neighbors = Vec::with_capacity(neighbors_count as usize);
        let mut mgr_neighbors: Vec<Neighbor> = Vec::with_capacity(neighbors_count as usize);
        for i in 1..neighbors_count + 1 {
            let (neighbor, sender, _receiver) = build_a_neighbor(GnomeId(i));
            mgr_neighbors.push(neighbor);
            neighbors.push((GnomeId(i), sender));
        }

        let (id, (req_sender, resp_receiver)) = manager
            .join_a_swarm(swarm_name.to_string(), Some(mgr_neighbors))
            .unwrap();
        let manager = TestManager {
            manager,
            swarm_name: swarm_name.to_string(),
            neighbors,
        };
        (manager, req_sender, resp_receiver)
    }

    pub fn status(&self) {
        self.manager.print_status(&self.swarm_id);
    }

    pub fn turn(&mut self, mut messages: Vec<Option<Message>>) {
        for (_id, sender) in &mut self.neighbors {
            if let Some(Some(message)) = messages.pop() {
                let _ = sender.send(message);
            }
        }
    }

    pub fn finish(self) {
        self.manager.finish();
    }
}

fn build_a_neighbor(id: GnomeId) -> (Neighbor, Sender<Message>, Receiver<Message>) {
    let (mocked_remote_sender, receiver) = channel::<Message>();
    let (sender, mocked_remote_receiver) = channel::<Message>();
    (
        Neighbor::from_id_channel_time(id, receiver, sender, SwarmTime(0), DEFAULT_SWARM_DIAMETER),
        mocked_remote_sender,
        mocked_remote_receiver,
    )
}

#[test]
fn gnome_message_exchange() {
    let (mut manager, req_sender, resp_receiver) =
        TestManager::join_a_swarm("gnome message exchange", 1);

    println!("Neighbors sent Proposal!");
    manager.turn(vec![Some(Message {
        swarm_time: SwarmTime(0),
        neighborhood: Neighborhood(0),
        header: Header::Sync,
        payload: Payload::KeepAlive,
    })]);

    thread::sleep(Duration::from_millis(100));
    let msg_res = resp_receiver.try_recv();
    assert!(msg_res.is_err(), "User received unexpected message!");
    let _ = req_sender.send(Request::AddData(Data(1)));

    manager.turn(vec![Some(Message {
        swarm_time: SwarmTime(1),
        neighborhood: Neighborhood(1),
        header: Header::Sync,
        payload: Payload::KeepAlive,
    })]);

    for i in 2..7 {
        thread::sleep(Duration::from_millis(100));
        manager.turn(vec![Some(Message {
            swarm_time: SwarmTime(i),
            neighborhood: Neighborhood(i as u8),
            header: Header::Sync,
            payload: Payload::KeepAlive,
        })]);
    }

    thread::sleep(Duration::from_millis(100));
    manager.turn(vec![Some(Message {
        swarm_time: SwarmTime(7),
        neighborhood: Neighborhood(0),
        header: Header::Sync,
        payload: Payload::KeepAlive,
    })]);

    for i in 0..7 {
        thread::sleep(Duration::from_millis(100));
        manager.turn(vec![Some(Message {
            swarm_time: SwarmTime(8 + i),
            neighborhood: Neighborhood(i as u8),
            header: message::Header::Block(BlockID(1)),
            payload: Payload::KeepAlive,
        })]);
    }

    let rcvd = resp_receiver.recv();
    assert!(rcvd.is_ok(), "User received invalid response!");

    let unwrapped = rcvd.unwrap();
    assert_eq!(
        unwrapped,
        Response::Block(BlockID(1), Data(1)),
        "User received unexpected response!"
    );

    thread::sleep(Duration::from_millis(100));
    manager.turn(vec![Some(Message {
        swarm_time: SwarmTime(15),
        neighborhood: Neighborhood(0),
        header: Header::Sync,
        payload: Payload::Request(NeighborRequest::ListingRequest(SwarmTime(0))),
    })]);

    println!("Trying receive another message...");
    let rcvd = resp_receiver.recv();
    assert!(rcvd.is_ok(), "User did not receive ListingRequest!");

    let unwrapped = rcvd.unwrap();
    let g_id = GnomeId(1);
    let n_request = NeighborRequest::ListingRequest(SwarmTime(0));
    assert_eq!(
        unwrapped,
        Response::DataInquiry(g_id, n_request),
        "User received unexpected inquiry!"
    );
    let _ = req_sender.send(Request::SendData(
        g_id,
        n_request,
        NeighborResponse::Listing(1, [BlockID(1); 128]),
    ));

    thread::sleep(Duration::from_millis(100));
    manager.turn(vec![Some(Message {
        swarm_time: SwarmTime(16),
        neighborhood: Neighborhood(1),
        header: Header::Sync,
        payload: Payload::KeepAlive,
    })]);

    manager.status();
    println!("Trying receive another message...");
    thread::sleep(Duration::from_millis(100));
    manager.turn(vec![Some(Message {
        swarm_time: SwarmTime(17),
        neighborhood: Neighborhood(2),
        header: Header::Sync,
        payload: Payload::KeepAlive,
    })]);
    manager.finish();
}

// #[test]
// fn exit_on_request() {
//     let (manager, _s, _r) = TestManager::join_a_swarm("exit on request", 0);
//     manager.status();
//     // TODO: below is not required
//     manager.finish();
// }
