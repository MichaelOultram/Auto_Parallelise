use std::thread;
use std::sync::{Arc, Mutex, mpsc};

use machine::Packet as Packet;

pub type Router = Vec<(Arc<Mutex<mpsc::Sender<Packet>>>, Arc<Mutex<mpsc::Receiver<Packet>>>)>;

pub fn add_machine(router : &mut Router, id : usize) -> (mpsc::Sender<Packet>, mpsc::Receiver<Packet>) {
    // Create channels
    let (router_send, machine_receive) = mpsc::channel();
    let (machine_send, router_recieve) = mpsc::channel();

    // Allow a 2 way connection between router and machine
    router.insert(id, (Arc::new(Mutex::new(router_send)), Arc::new(Mutex::new(router_recieve))));
    (machine_send, machine_receive)
}

pub fn start_router(router : Router) {
    // Create a send table for use in other threads
    let send_table = Arc::new(Mutex::new(vec![]));

    let mut id = 0; // TODO: Replace with an enumerator
    // For every route
    for (sender, receiver) in router {
        // Add the route to the send table
        send_table.lock().unwrap().insert(id, sender);
        let send_table_c = send_table.clone();
        // Create a new thread for dealing with this receiver
        thread::spawn(move || loop {
            // Receive a packet
            match receiver.lock().unwrap().try_recv() {
                Ok(packet) => {
                    // Find the sender channel from the send_table
                    match send_table_c.lock() {
                        Ok(send_table) => {
                            let send_to_mutex = send_table.get(packet.to_id).unwrap();
                            match send_to_mutex.lock() {
                                Ok(send_to) => {send_to.send(packet);},
                                Err(e) => println!("{:?}", e),
                            }
                        },
                        Err(e) => println!("{:?}", e),
                    }
                },
                Err(e) => println!("{:?}", e), // TODO: Getting RecvErrors when nothing has been sent
            }
        });
        id += 1;
    }
}
