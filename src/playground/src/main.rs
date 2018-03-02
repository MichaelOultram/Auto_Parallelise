extern crate crypto;
use self::crypto::digest::Digest;
use self::crypto::sha2::Sha256;

use std::time::Instant;
use std::io::{BufRead, BufReader};
use std::fs::File;

fn load_dictionary() -> Vec<String> {
    let (syncline_411_441_446_530_send, syncline_411_441_446_530_receive) =
        std::sync::mpsc::channel();
    let thread_278_300 =
        std::thread::spawn(move ||
                               {
                                   let mut dict = vec!();
                                   let (file,) =
                                       syncline_411_441_446_530_receive.recv().unwrap();
                                   for line in file.lines() {
                                       {
                                           let l = line.unwrap();
                                           dict.push(l);
                                       }
                                   }
                                   return dict;
                               });
    let f = File::open("passwords.txt").unwrap();
    let file = BufReader::new(&f);
    syncline_411_441_446_530_send.send((file,)).unwrap();
    thread_278_300.join().unwrap()
}

fn hash(word: String) -> String {
    for _ in 0..40 {
        {
            let mut hasher = Sha256::new();
            hasher.input_str(&word);
            word = hasher.result_str();
        }
    }
    return word;
}

fn crack_password(dictionary: Vec<String>, password_hash: String)
 -> Option<String> {
    let thread_901_925 =
        std::thread::spawn(move ||
                               {
                                   let mut hashes = vec!();
                                   for id in 0..dictionary.len() {
                                       {
                                           let word = dictionary[id];
                                           let word_hash = hash(word);
                                           hashes.push(word_hash);
                                       }
                                   }
                                   for id in 0..dictionary.len() {
                                       {
                                           let word_hash = hashes[id];
                                           if word_hash == password_hash {
                                               {
                                                   let word = dictionary[id];
                                                   return Some(word);
                                               }
                                           }
                                       }
                                   }
                                   return None;
                               });
    thread_901_925.join().unwrap()
}

fn main() {
    let (syncline_1419_1515_1521_1563_send,
         syncline_1419_1515_1521_1563_receive) = std::sync::mpsc::channel();
    let thread_1325_1350 =
        std::thread::spawn(move ||
                               {
                                   let now = Instant::now();
                                   let elapsed = now.elapsed();
                                   let sec =
                                       (elapsed.as_secs() as f64) +
                                           (elapsed.subsec_nanos() as f64 /
                                                1000000000.0);
                                   println!("Seconds: {}" , sec);
                               });
    let thread_1355_1373 = std::thread::spawn(move || { println!("Start"); });
    let thread_1379_1414 =
        std::thread::spawn(move ||
                               {
                                   let dictionary = load_dictionary();
                                   let (password_hash,) =
                                       syncline_1419_1515_1521_1563_receive.recv().unwrap();
                                   crack_password(dictionary, password_hash);
                               });
    let thread_1419_1515 =
        std::thread::spawn(move ||
                               {
                                   let password_hash =
                                       format!("be9f36142cf64f3804323c8f29bc5822d01e60f7849244c59ff42de38d11fa37");
                                   syncline_1419_1515_1521_1563_send.send((password_hash,)).unwrap()
                               });
    println!("Done");
    thread_1325_1350.join().unwrap();
    thread_1355_1373.join().unwrap();
    thread_1379_1414.join().unwrap();
    thread_1419_1515.join().unwrap()
}
