extern crate crypto;
use self::crypto::digest::Digest;
use self::crypto::sha2::Sha256;

use std::time::Instant;
use std::io::{BufRead, BufReader};
use std::fs::File;


fn load_dictionary() -> Vec<String> {
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

fn main() {
    let now = Instant::now();

    let dictionary = load_dictionary();
    let hash_password = format!("10c2d630409d5a4b8132f21478f40a030b8aefb29ab1a541da6d884a0286a6dc");
    let password = crack_password_single(&dictionary, hash_password);
    match password {
        Some(word) => println!("Found password: {}", word),
        None => println!("Could not find password"),
    }

    let elapsed = now.elapsed();
    let sec = (elapsed.as_secs() as f64) + (elapsed.subsec_nanos() as f64 / 1000_000_000.0);
    println!("Seconds: {}", sec);
}
