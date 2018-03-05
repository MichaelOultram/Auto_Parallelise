fn main_parallel() -> ::std::thread::JoinHandle<()> {
    ::std::thread::spawn(move ||
                             {
                                 let (syncline_166_220_225_251_send,
                                      syncline_166_220_225_251_receive) =
                                     std::sync::mpsc::channel();
                                 let (syncline_225_251_256_282_send,
                                      syncline_225_251_256_282_receive) =
                                     std::sync::mpsc::channel();
                                 let thread_85_99 =
                                     std::thread::spawn(move ||
                                                            {
                                                                let mut a = 4;
                                                                a += 1;
                                                                for i in 0..a
                                                                    {
                                                                    {
                                                                        println!("Hello world");
                                                                    }
                                                                }
                                                                syncline_166_220_225_251_send.send((a,)).unwrap()
                                                            });
                                 let thread_104_118 =
                                     std::thread::spawn(move ||
                                                            {
                                                                let mut b = 3;
                                                                b += 1;
                                                                let (::std::Display::fmt,
                                                                     __arg0,
                                                                     __arg1,
                                                                     a) =
                                                                    syncline_225_251_256_282_receive.recv().unwrap();
                                                                println!("{} + {}"
                                                                         , a ,
                                                                         b);
                                                            });
                                 let thread_123_137 =
                                     std::thread::spawn(move ||
                                                            {
                                                                let mut c = 5;
                                                                let (a,) =
                                                                    syncline_166_220_225_251_receive.recv().unwrap();
                                                                println!("{} * {}"
                                                                         , a ,
                                                                         c);
                                                                syncline_225_251_256_282_send.send((::std::Display::fmt,
                                                                                                    __arg0,
                                                                                                    __arg1,
                                                                                                    a)).unwrap()
                                                            });
                                 println!("End of program");
                                 thread_85_99.join().unwrap();
                                 thread_104_118.join().unwrap();
                                 thread_123_137.join().unwrap()
                             })
}
fn main() { main_parallel().join().unwrap() }
