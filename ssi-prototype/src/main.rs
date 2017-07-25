#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;
extern crate rand;

use std::io;
use std::io::BufReader;
use std::io::BufRead;
use std::fs::File;
use std::path::Path;

mod process;
use process::Process as Process;
use process::Status as Status;

fn dictionary() -> Vec<String> {
    let mut dict = Vec::new();

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
    // Load the dictionary from file
    let dict = dictionary();

    // Process queue
    let mut queue = Process::generate_amount(100, dict);

    // Update some processes
    for x in 0..3 {
        queue[x].status = Status::Runnable;
    }

    // Print the queue
    for p in queue {
        println!("{}", p.to_string());
    }

}
