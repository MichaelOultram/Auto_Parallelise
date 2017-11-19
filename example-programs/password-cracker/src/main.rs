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
            let mut hash_word: String = word.clone();
            for _ in 0..40 {
                let mut hasher = Sha256::new();
                hasher.input_str(&hash_word);
                hash_word = hasher.result_str();
            }

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
fn crack_password_sequential(dictionary: &Vec<String>, hash_password: String) -> Option<String>{
    for word in dictionary {
        // Hash word using Sha256
        let mut hash_word: String = word.clone();
        for _ in 0..40 {
            let mut hasher = Sha256::new();
            hasher.input_str(&hash_word);
            hash_word = hasher.result_str();
        }

        // Check if hash matches
        if hash_password == hash_word {
            return Some(word.clone());
        }
    }
    None
}

fn crack_password_single(dictionary: &Vec<String>, hash_password: String) -> Option<String>{
    // Hash word using Sha256
    let mut hash_word: String = "ZZZZZZZZZZ".to_owned();
    for _ in 0..40 {
        let mut hasher = Sha256::new();
        hasher.input_str(&hash_word);
        hash_word = hasher.result_str();
    }

    Some(hash_word.clone())
}

//#[auto_parallelize]
fn main() {
    let now = Instant::now();

    let dictionary = load_dictionary();
    let hash_password = format!("be9f36142cf64f3804323c8f29bc5822d01e60f7849244c59ff42de38d11fa37");
    let password = crack_password_parallel(&dictionary, hash_password);
    match password {
        Some(word) => println!("Found password: {}", word),
        None => println!("Could not find password"),
    }

    let elapsed = now.elapsed();
    let sec = (elapsed.as_secs() as f64) + (elapsed.subsec_nanos() as f64 / 1000_000_000.0);
    println!("Seconds: {}", sec);
}
