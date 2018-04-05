#![feature(plugin)]
#![plugin(auto_parallelise)]

extern crate crypto;
use self::crypto::digest::Digest;
use self::crypto::sha2::Sha256;

use std::time::Instant;

#[autoparallelise]
fn hash(word: String) -> String {
    let mut hash_word: String = word;
    // Hash word using Sha256
    for _ in (0..10000).rev() { // Disable this for loop as type inference uses type str
        let mut hasher = Sha256::new();
        hasher.input_str(&hash_word);
        hash_word = String::from(hasher.result_str());
    }

    hash_word
}

#[autoparallelise]
fn main() {
    //let now = Instant::now();
    println!("Start");

    let password_hash = format!("108c25e139b930c86c3712e96cb199db970592443f82e239ea6705ab5018ad5b");

    let test_hash = hash("test".to_owned());
    let word_hash = hash("word".to_owned());

    println!("test_hash = {}", test_hash);
    println!("word_hash = {}", word_hash);

    if test_hash == password_hash {
        println!("password: test");
    }
    if word_hash == password_hash {
        println!("password: word");
    }

    println!("Done");
    //let elapsed = now.elapsed();
    //let sec = (elapsed.as_secs() as f64) + (elapsed.subsec_nanos() as f64 / 1000_000_000.0);
    //println!("Seconds: {}", sec);
}
