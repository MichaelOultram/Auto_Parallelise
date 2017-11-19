//#![feature(plugin)]
//#![plugin(auto_parallelize)]

extern crate crypto;
use self::crypto::digest::Digest;
use self::crypto::sha2::Sha256;

extern crate auto_parallelize;
use auto_parallelize::taskpool::ThreadPool;
use std::sync::mpsc::Receiver;

use std::time::Instant;
use std::io::{BufRead, BufReader};
use std::fs::File;

//#[auto_parallelize]
fn load_dictionary() -> Vec<String> {
    let mut dict = vec![];

    // Put each line of dictionary.txt into the vector
    let f = File::open("passwords.txt").unwrap();
    let file = BufReader::new(&f);
    for line in file.lines(){
        let l = line.unwrap();
        dict.push(l);
    }

    dict
}

//#[auto_parallelize]
fn crack_password_parallel(dictionary: &Vec<String>, hash_password: String) -> Option<String>{
    let threadpool = ThreadPool::new(16 - 1);
    let mut handles = vec![];
    for word in dictionary {
        let word = word.clone();
        let hash_password = hash_password.clone();
        handles.push(threadpool.task_block(move || {
            // Hash word using Sha256
            let mut hasher = Sha256::new();
            hasher.input_str(&word);
            let hash_word = hasher.result_str();

            // Check if hash matches
            if hash_password == hash_word {
                Some(Some(word.clone()))
            } else {
                None
            }
        }));
    }

    for handle in handles {
        if let Some(result) = threadpool.result(handle) {
            return result;
        }
    }

    None
}

//#[auto_parallelize]
fn crack_password_single(dictionary: &Vec<String>, hash_password: String) -> Option<String>{
    for word in dictionary {
        // Hash word using Sha256
        let mut hasher = Sha256::new();
        hasher.input_str(word);
        let hash_word = hasher.result_str();

        // Check if hash matches
        if hash_password == hash_word {
            return Some(word.clone());
        }
    }
    None
}

//#[auto_parallelize]
fn main() {
    let now = Instant::now();

    let dictionary = load_dictionary();
    let hash_password = format!("61bdb3487ed81633ee4d7875745028739a92feb91f27d704fdfcfa8be5f0b3ee");
    let password = crack_password_single(&dictionary, hash_password);
    match password {
        Some(word) => println!("Found password: {}", word),
        None => println!("Could not find password"),
    }

    let elapsed = now.elapsed();
    let sec = (elapsed.as_secs() as f64) + (elapsed.subsec_nanos() as f64 / 1000_000_000.0);
    println!("Seconds: {}", sec);
}
