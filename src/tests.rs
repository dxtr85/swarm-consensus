use super::*;
use std::thread;

use std::{
    sync::mpsc::{channel, Receiver, Sender},
    time::Duration,
};

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
    // let right_id = gnome::GnomeId(2);
    let (left, left_s, _) = build_a_neighbor(left_id);
    // let (right, right_s, _) = build_a_neighbor(right_id);

    let neighbors: Vec<Neighbor> = vec![left];
    let (_, resp_receiver) = manager.join_a_swarm("message exchange".to_string(), Some(neighbors));
    // manager.print_status("message exchange");
    println!("Neighbors sent Proposal!");
    let _ = left_s.send(Message {
        swarm_time: SwarmTime(0),
        neighborhood: neighbor::Neighborhood(0),
        header: message::Header::Block(BlockID(1)),
        payload: message::Payload::Block(BlockID(1), Data(1)),
    });
    // let _ = right_s.send(Message {
    //     swarm_time: SwarmTime(5),
    //     neighborhood: neighbor::Neighborhood(6),
    //     header: message::Header::Block(BlockID(1)),
    //     data: Data::Payload(BlockID(1), Payload(1)),
    // });
    // let _ = right_s.send(Message {
    //     swarm_time: SwarmTime(5),
    //     awareness: right_awareness,
    //     data: Data::Proposal(ProposalID(SwarmTime(0), left_id), ProposalData(1)),
    // });
    // thread::sleep(Duration::from_millis(100));
    // manager.print_status("message exchange");

    // println!("łan");
    thread::sleep(Duration::from_millis(100));
    let msg_res = resp_receiver.try_recv();
    assert!(msg_res.is_err(), "User received unexpected message!");

    // println!("tu");
    let _ = left_s.send(Message {
        swarm_time: SwarmTime(1),
        neighborhood: neighbor::Neighborhood(1),
        header: message::Header::Block(BlockID(1)),
        payload: Payload::KeepAlive,
    });
    // println!("fri");
    // let _ = right_s.send(Message {
    //     swarm_time: SwarmTime(6),
    //     awareness: right_awareness,
    //     data: Data::ProposalId(ProposalID(SwarmTime(0), left_id)),
    // });
    // manager.print_status("message exchange");
    // println!("for");

    thread::sleep(Duration::from_millis(100));
    let rcvd = resp_receiver.recv();

    // println!("fajf");
    assert!(rcvd.is_ok(), "User received invalid response!");

    let unwrapped = rcvd.unwrap();
    assert_eq!(
        unwrapped,
        Response::Block(BlockID(1), Data(1)),
        "User received unexpected response!"
    );

    println!("^^^^^^^^^^^^ NEW {:?}", unwrapped);
    manager.print_status("message exchange");
    manager.finish();
}

// #[test]
// fn exit_on_request() {
//     let mut manager = Manager::new();
//     let _ = manager.join_a_swarm("exit on request".to_string(), None);
//     manager.print_status("exit on request");
//     // TODO: below is not required
//     // manager.finish();
// }
