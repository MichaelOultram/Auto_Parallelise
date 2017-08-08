extern crate rand;
extern crate imgui;
extern crate ssi_model;

use std::thread;
use std::thread::JoinHandle;
use std::io::BufReader;
use std::io::BufRead;
use std::fs::File;

use ssi_model::*;
use ssi_model::process::Process as Process;
use ssi_model::machine::Machine as Machine;

const NUM_PROCESSES : u64 = 500;
const NUM_MACHINES : usize = 10;

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
    println!("Generating Process Tree");
    let mut init_process : Process = {
        // Load the dictionary from file
        let dict = dictionary();
        Process::generate_process_tree(&mut rng, NUM_PROCESSES, &dict)
    };
    init_process.status = process::Status::Runnable;
    println!("Done!");
    // println!("Depth: {}", init_process.depth());
    // return;


    // Create Router
    println!("Creating Router");
    let mut router = (vec![], vec![]);
    println!("Done!");

    // Generate Machines
    println!("Generating Machines");
    let machine_handles : Vec<JoinHandle<()>>;
    {
        // Create the machines
        let mut machines : Vec<Machine> = (0..NUM_MACHINES).into_iter()
            .map(|machine_id| Machine::new(machine_id, &mut router)).collect();

        // Give the init process to the first machine
        machines[0].global_queue.push_back(init_process);

        // Start all machine threads
        machine_handles = machines.into_iter().map(|mut m| thread::spawn(move || m.switch())).collect();
    }
    println!("Done!");

    println!("Start the Router");
    router::start_router(router);
    println!("Done!");


    // Wait for all machines to have finished
    for m in machine_handles {
        match m.join() {
            Ok(_) => {},
            Err(e) => panic!(e),
        }
    }

}
