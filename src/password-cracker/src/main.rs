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
    let f = File::open("passwords.txt").unwrap();
    let file = BufReader::new(&f);
    for line in file.lines(){
        let l = line.unwrap();
        dict.push(l);
    }

    return dict;
}

#[autoparallelise]
fn hash(word: String) -> String {
    let mut hash_word = word;
    // Hash word using Sha256
    for _ in 0..40 {
        let mut hasher = Sha256::new();
        hasher.input_str(&hash_word);
        hash_word = hasher.result_str();
    }

    return hash_word;
}

#[autoparallelise]
fn crack_password(dictionary: Vec<String>, password_hash: String) -> Option<String> {
    let mut hashes = vec![];
    for id in 0..dictionary.len() {
        let word = dictionary[id];
        let word_hash = hash(word);
        hashes.push(word_hash);
    }

    for id in 0..dictionary.len() {
        let word_hash = hashes[id];
        if word_hash == password_hash {
            let word = dictionary[id];
            return Some(word);
        }
    }
    return None;
}

#[autoparallelise]
fn main() {
    let now = Instant::now();
    println!("Start");

    let dictionary = load_dictionary();
    let password_hash = format!("be9f36142cf64f3804323c8f29bc5822d01e60f7849244c59ff42de38d11fa37");

    crack_password(dictionary, password_hash);

    println!("Done");
    let elapsed = now.elapsed();
    let sec = (elapsed.as_secs() as f64) + (elapsed.subsec_nanos() as f64 / 1000_000_000.0);
    println!("Seconds: {}", sec);
}
