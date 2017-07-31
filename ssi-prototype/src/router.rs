use std::thread;
use std::thread::{JoinHandle};
use std::sync::{mpsc};

use machine::Packet as Packet;

pub type Router = Vec<(mpsc::Sender<Packet>, mpsc::Receiver<Packet>)>;

pub fn add_machine(router : &mut Router, id : usize)
    -> (mpsc::Sender<Packet>, mpsc::Receiver<Packet>) {
    // Create channels
    let (router_send, machine_receive) = mpsc::channel();
    let (machine_send, router_recieve) = mpsc::channel();

    // Allow a 2 way connection between router and machine
    router.insert(id, (router_send, router_recieve));
    (machine_send, machine_receive)
}

pub fn start_router(router : Router){
    thread::spawn(move || loop {
        for pair in router.iter() {
            let (_, ref receiver) = *pair;
            match receiver.try_recv() {
                Ok(packet) => {
                    // Find the sender channel and forward the packet
                    let (ref send_to, _) = *router.get(packet.to_id).unwrap();
                    println!("{} -> {}", packet.from_id, packet.to_id);
                    send_to.send(packet);
                },
                Err(_) => {} // Ignore if no packet received
            }
        }
    });
}
