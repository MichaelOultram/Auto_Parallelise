
extern crate crypto;

use self::crypto::digest::Digest;
use self::crypto::sha2::Sha256;
use std::time::Instant;
use std::io::{BufRead, BufReader};
use std::fs::File;
fn load_dictionary() -> Vec<String> {
    let (syncline_417_446_451_536_file_send,
         syncline_417_446_451_536_file_receive) = std::sync::mpsc::channel();
    let thread_361_412 =
        std::thread::spawn(move ||
                               {
                                   let f: File =
                                       File::open("passwords.txt").unwrap();
                                   let return_value =
                                       {
                                           let file = BufReader::new(f);
                                           syncline_417_446_451_536_file_send.send((file,)).unwrap()
                                       };
                                   return_value
                               });
    let return_value =
        {
            let mut dict = vec!();
            let return_value =
                {
                    let (file,) =
                        syncline_417_446_451_536_file_receive.recv().unwrap();
                    for line in file.lines() {
                        let return_value =
                            {
                                let l = line.unwrap();
                                let return_value = { dict.push(l); };
                                return_value
                            };
                        return_value
                    }
                    let return_value = { dict };
                    return_value
                };
            return_value
        };
    thread_361_412.join().unwrap();
    return_value
}

fn hash(word: &String) -> String {
    let return_value =
        {
            let mut hash_word = word.clone();
            let return_value =
                {
                    for i in (0..1000).rev() {
                        let return_value =
                            {
                                let mut hasher = Sha256::new();
                                let return_value =
                                    {
                                        hasher.input_str(&hash_word);
                                        let return_value =
                                            {
                                                hash_word =
                                                    hasher.result_str();
                                            };
                                        return_value
                                    };
                                return_value
                            };
                        return_value
                    }
                    let return_value = { hash_word };
                    return_value
                };
            return_value
        };
    return_value
}

fn main() {
    let (syncline_962_1058_1064_1277_password_hash_send,
         syncline_962_1058_1064_1277_password_hash_receive) =
        std::sync::mpsc::channel();
    let thread_962_1058 =
        std::thread::spawn(move ||
                               {
                                   let password_hash =
                                       format!("0954229bd82060f9d55ccc310b315ea831b9ba8faee1b76f66a19cc71140dfd7");
                                   syncline_962_1058_1064_1277_password_hash_send.send((password_hash,)).unwrap()
                               });
    let thread_909_957 =
        std::thread::spawn(move ||
                               {
                                   let dictionary: Vec<String> =
                                       load_dictionary();
                                   let return_value =
                                       {
                                           let (password_hash,) =
                                               syncline_962_1058_1064_1277_password_hash_receive.recv().unwrap();
                                           let (dictionary, password_hash) =
                                               {
                                                   let (syncline_1064_1277_1184_1271_password_hash_send_0,
                                                        syncline_1064_1277_1184_1271_password_hash_receive_0) =
                                                       std::sync::mpsc::channel();
                                                   let mut syncline_1064_1277_1184_1271_password_hash_receive_i =
                                                       syncline_1064_1277_1184_1271_password_hash_receive_0;
                                                   let (syncline_1064_1277_1104_1138_dictionary_send_0,
                                                        syncline_1064_1277_1104_1138_dictionary_receive_0) : (std::sync::mpsc::Sender<(Vec<String>,)>, std::sync::mpsc::Receiver<(Vec<String>,)>) =
                                                       std::sync::mpsc::channel();
                                                   let mut syncline_1064_1277_1104_1138_dictionary_receive_i =
                                                       syncline_1064_1277_1104_1138_dictionary_receive_0;
                                                   for id in
                                                       0..dictionary.len() {
                                                       let (syncline_1064_1277_1184_1271_password_hash_send,
                                                            syncline_1064_1277_1184_1271_password_hash_receive_new) =
                                                           std::sync::mpsc::channel();
                                                       let syncline_1064_1277_1184_1271_password_hash_receive =
                                                           syncline_1064_1277_1184_1271_password_hash_receive_i;
                                                       syncline_1064_1277_1184_1271_password_hash_receive_i
                                                           =
                                                           syncline_1064_1277_1184_1271_password_hash_receive_new;
                                                       let (syncline_1064_1277_1104_1138_dictionary_send,
                                                            syncline_1064_1277_1104_1138_dictionary_receive_new) =
                                                           std::sync::mpsc::channel();
                                                       let syncline_1064_1277_1104_1138_dictionary_receive =
                                                           syncline_1064_1277_1104_1138_dictionary_receive_i;
                                                       syncline_1064_1277_1104_1138_dictionary_receive_i
                                                           =
                                                           syncline_1064_1277_1104_1138_dictionary_receive_new;
                                                       ::std::thread::spawn(move
                                                                                ||
                                                                                {
                                                                                    let return_value =
                                                                                        {
                                                                                            {
                                                                                                let return_value =
                                                                                                    {
                                                                                                        let (dictionary,) =
                                                                                                            syncline_1064_1277_1104_1138_dictionary_receive.recv().unwrap();
                                                                                                        let word =
                                                                                                            dictionary[id].clone();
                                                                                                        syncline_1064_1277_1104_1138_dictionary_send.send((dictionary,)).unwrap();
                                                                                                        let return_value =
                                                                                                            {
                                                                                                                let hash_word =
                                                                                                                    hash(&word);
                                                                                                                let return_value =
                                                                                                                    {
                                                                                                                        let (password_hash,) =
                                                                                                                            syncline_1064_1277_1184_1271_password_hash_receive.recv().unwrap();
                                                                                                                        if hash_word
                                                                                                                               ==
                                                                                                                               password_hash
                                                                                                                           {
                                                                                                                            let return_value =
                                                                                                                                {
                                                                                                                                    println!("Password is {}"
                                                                                                                                             ,
                                                                                                                                             word);
                                                                                                                                };
                                                                                                                            return_value
                                                                                                                        }
                                                                                                                        syncline_1064_1277_1184_1271_password_hash_send.send((password_hash,)).unwrap()
                                                                                                                    };
                                                                                                                return_value
                                                                                                            };
                                                                                                        return_value
                                                                                                    };
                                                                                                return_value
                                                                                            }
                                                                                        };
                                                                                    return_value
                                                                                });
                                                       ()
                                                   }
                                                   syncline_1064_1277_1184_1271_password_hash_send_0.send((password_hash,)).unwrap();
                                                   syncline_1064_1277_1104_1138_dictionary_send_0.send((dictionary,)).unwrap();
                                                   let (password_hash,) =
                                                       syncline_1064_1277_1184_1271_password_hash_receive_i.recv().unwrap();
                                                   let (dictionary,) =
                                                       syncline_1064_1277_1104_1138_dictionary_receive_i.recv().unwrap();
                                                   (dictionary, password_hash)
                                               };
                                       };
                                   return_value
                               });
    let return_value =
        {
            let now = Instant::now();
            let return_value =
                {
                    let elapsed = now.elapsed();
                    let return_value =
                        {
                            let sec =
                                (elapsed.as_secs() as f64) +
                                    (elapsed.subsec_nanos() as f64 /
                                         1000000000.0);
                            let return_value =
                                { println!("Seconds: {}" , sec); };
                            return_value
                        };
                    return_value
                };
            return_value
        };
    thread_962_1058.join().unwrap();
    thread_909_957.join().unwrap();
    return_value
}
