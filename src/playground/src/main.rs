fn main_parallel() -> ::std::thread::JoinHandle<()> {
    ::std::thread::spawn(move ||
                             {
                                 let (syncline_156_163_233_259_send,
                                      syncline_156_163_233_259_receive) =
                                     std::sync::mpsc::channel();
                                 let thread_85_99 =
                                     std::thread::spawn(move ||
                                                            {
                                                                let mut a = 4;
                                                                a += 1;
                                                                let (b,) =
                                                                    syncline_156_163_233_259_receive.recv().unwrap();
                                                                println!("{} + {}"
                                                                         , a ,
                                                                         b);
                                                            });
                                 let mut b = 3;
                                 b += 1;
                                 syncline_156_163_233_259_send.send((b,)).unwrap();
                                 thread_85_99.join().unwrap()
                             })
}
fn main() { main_parallel().join().unwrap() }
