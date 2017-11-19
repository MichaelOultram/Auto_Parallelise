use std::sync::Mutex;
use std::thread::{self, Thread};
use std::sync::mpsc::{channel, Receiver, Sender};

/*struct Task<D: Send + 'static> {
    function: Box<TaskFunction<D,R>>,
    children: Vec<Task<R>>,
}

// From: https://github.com/reem/rust-scoped-pool/blob/master/src/lib.rs
trait TaskFunction<D,R>: Send {
    fn run(self: Box<Self>, arg: Option<D>) -> R;
}
impl<F: FnOnce(Option<D>) -> R + Send, D: Send + 'static + Sized, R: Send + 'static + Sized> TaskFunction<D,R> for F {
    fn run(self: Box<Self>, arg: Option<D>) -> R { (*self)(arg) }
}

impl<D> Task<D> where D: Send + 'static {
    pub fn new<F, R>(f: F) -> Self
    where F: FnOnce() -> R, F: Send + 'static, R: Send + 'static
    {
        unimplemented!()
    }
}*/

// Adapted From: https://github.com/reem/rust-scoped-pool/blob/master/src/lib.rs
trait TaskFunction<D,R>: Send {
    fn run(self: Box<Self>, arg: Option<D>) -> R;
    fn dependency<G: FnOnce(Option<R>) -> Q + Send, Q: Send + 'static + Sized>(self: Box<Self>, g: G) -> Self;
}
impl<F: FnOnce(Option<D>) -> R + Send, D: Send + 'static + Sized, R: Send + 'static + Sized> TaskFunction<D,R> for F {
    fn run(self: Box<Self>, arg: Option<D>) -> R { (*self)(arg) }

    fn dependency<G: FnOnce(Option<R>) -> Q + Send, Q: Send + 'static + Sized> (self: Box<Self>, g: G) -> Self
    {
        let r = self.run(None);
        g(Some(r));
        unimplemented!()
    }

}



#[test]
pub fn fib_test(n: u32)
{

}
