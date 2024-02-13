use super::*;
use std::sync::mpsc::{channel, Receiver, Sender};

fn build_a_neighbor(id: GnomeId) -> (Neighbor, Sender<Message>, Receiver<Message>) {
    let (s_req, r_req) = channel::<Message>();
    let (s_res, r_res) = channel::<Message>();
    (
        Neighbor::from_id_channel_time(id, r_req, s_res, SwarmTime(0)),
        s_req,
        r_res,
    )
}

#[test]
fn gnome_message_exchange() {
    let mut manager = Manager::new();
    let left_id = gnome::GnomeId(1);
    let right_id = gnome::GnomeId(2);
    let (left, left_s, _) = build_a_neighbor(left_id);
    let (right, right_s, _) = build_a_neighbor(right_id);

    let neighbors: Vec<Neighbor> = vec![left, right];
    let (_, resp_receiver) = manager.join_a_swarm("message exchange".to_string(), Some(neighbors));
    manager.get_status("message exchange");
    let p1: Proposal = (SwarmTime(0), 1);
    let bp1 = Box::new([p1.1; 1024]);
    // let p2: Proposal = (SwarmTime(0), 2);
    // let bp2 = Box::new([p2.1; 1024]);
    let left_awareness = Awareness::Aware(SwarmTime(12), 5, p1);
    let right_awareness = Awareness::Aware(SwarmTime(12), 5, p1);
    println!("Neighbors sent Proposal!");

    let _ = left_s.send(Message::Proposal(left_awareness, p1));
    let _ = right_s.send(Message::Proposal(right_awareness, p1));
    manager.get_status("message exchange");

    let msg_res = resp_receiver.try_recv();
    assert!(msg_res.is_err(), "User received unexpected message!");
    let left_awareness = Awareness::Aware(SwarmTime(13), 6, p1);
    let right_awareness = Awareness::Aware(SwarmTime(13), 6, p1);

    let _ = left_s.send(Message::KeepAlive(left_awareness));
    let _ = right_s.send(Message::KeepAlive(right_awareness));
    manager.get_status("message exchange");

    let rcvd = resp_receiver.recv();

    assert!(rcvd.is_ok(), "User received invalid response!");

    let unwrapped = rcvd.unwrap();
    assert_eq!(
        unwrapped,
        Response::Data(bp1),
        "User received unexpected response!"
    );

    println!("Received {:?}", unwrapped);
    manager.finish();
}

#[test]
fn exit_on_request() {
    let mut manager = Manager::new();
    let _ = manager.join_a_swarm("exit on request".to_string(), None);
    manager.get_status("exit on request");
    // TODO: below is not required
    // manager.finish();
}
