#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;
extern crate rand;
extern crate statrs;

use std::thread;
use std::sync::{Mutex, Arc};

use std::io::BufReader;
use std::io::BufRead;
use std::fs::File;

mod process;
use process::Process as Process;


mod machine;
use machine::Machine as Machine;


fn dictionary() -> Vec<String> {
    let mut dict = vec![];

    // Put each line of dictionary.txt into the vector
    let f = File::open("dictionary.txt").unwrap();
    let file = BufReader::new(&f);
    for line in file.lines(){
        let l = line.unwrap();
        dict.push(l);
    }

    dict
}

fn main() {
    // Random number generator
    let mut rng = rand::StdRng::new().unwrap();

    // Generate Process tree
    let mut init_process : Process = {
        // Load the dictionary from file
        let dict = dictionary();
        Process::generate_process_tree(&mut rng, 1000, &dict)
    };
    init_process.status = process::Status::Runnable;
    println!("{}", init_process.to_json());

    // Generate Machines
    let mut machine_handles = vec![];
    for machine_id in 0..4 {
        let machine = Machine::new(machine_id);
        let join_handle = thread::spawn(move || machine.switch());
        machine_handles.push(join_handle);
    }

    // Wait for all machines to have finished
    for m in machine_handles {
        match m.join() {
            Ok(_) => {},
            Err(e) => panic!(e),
        }
    }

}
