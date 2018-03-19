#![feature(plugin)]
#![plugin(auto_parallelise)]

extern crate crypto;
use self::crypto::digest::Digest;
use self::crypto::sha2::Sha256;

use std::time::Instant;
use std::io::{BufRead, BufReader};
use std::fs::File;

#[autoparallelise]
fn load_dictionary() -> Vec<String> {
    let mut dict = vec![];

    // Put each line of dictionary.txt into the vector
    let f: File = File::open("passwords.txt").unwrap();
    let file = BufReader::new(f);
    for line in file.lines() {
        let l = line.unwrap();
        dict.push(l);
    }

    dict
}

#[autoparallelise]
fn hash(word: &String) -> String {
    let mut hash_word = word.clone();
    // Hash word using Sha256
    for i in (0..40).rev() {
        let mut hasher = Sha256::new();
        hasher.input_str(&hash_word);
        hash_word = hasher.result_str();
    }
    hash_word
}

#[autoparallelise]
fn main() {
    let now = Instant::now();
    println!("Start");

    let dictionary: Vec<String> = load_dictionary();
    let password_hash = format!("be9f36142cf64f3804323c8f29bc5822d01e60f7849244c59ff42de38d11fa37");

    for id in 0..dictionary.len() {
        let word = dictionary[id].clone();
        let hash_word = hash(&word);
        if hash_word == password_hash {
            println!("Password is {}", word);
        }
    }

    for _ in 0..4 {
        println!("Done");
    }
    let elapsed = now.elapsed();
    let sec = (elapsed.as_secs() as f64) + (elapsed.subsec_nanos() as f64 / 1000_000_000.0);
    println!("Seconds: {}", sec);
}
