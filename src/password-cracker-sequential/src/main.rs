extern crate crypto;
use self::crypto::digest::Digest;
use self::crypto::sha2::Sha256;

use std::time::Instant;
use std::io::{BufRead, BufReader};
use std::fs::File;

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

fn hash(word: &String) -> String {
    let mut hash_word = word.clone();
    // Hash word using Sha256
    for i in (0..1000).rev() {
        let mut hasher = Sha256::new();
        hasher.input_str(&hash_word);
        hash_word = hasher.result_str();
    }
    hash_word
}

fn main() {
    let now = Instant::now();

    let dictionary: Vec<String> = load_dictionary();
    let password_hash = format!("0954229bd82060f9d55ccc310b315ea831b9ba8faee1b76f66a19cc71140dfd7");

    for id in 0..dictionary.len() {
        let word = dictionary[id].clone();
        let hash_word = hash(&word);
        if hash_word == password_hash {
            println!("Password is {}", word);
        }
    }

    let elapsed = now.elapsed();
    let sec = (elapsed.as_secs() as f64) + (elapsed.subsec_nanos() as f64 / 1000_000_000.0);
    println!("Seconds: {}", sec);
}
