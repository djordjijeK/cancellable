use std::ops::Deref;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread;
use std::thread::JoinHandle;

pub trait Cancellable {
    type Error;

    fn execute(&mut self) -> Result<LoopStep, Self::Error>;

    fn run(&mut self) -> Result<(), Self::Error> {
        loop {
            match self.execute() {
                Ok(LoopStep::Next) => {}
                Ok(LoopStep::Break) => break,
                Err(error) => return Err(error),
            }
        }

        Ok(())
    }

    fn spawn(mut self) -> Handle<Self::Error>
    where
        Self: Sized + Send + 'static,
        Self::Error: Send + 'static,
    {
        let keep_running = Arc::new(AtomicBool::new(true));

        let thread_handle = {
            let keep_running = keep_running.clone();
            thread::spawn(move || {
                while keep_running.load(Ordering::Relaxed) {
                    match self.execute() {
                        Ok(LoopStep::Next) => {}
                        Ok(LoopStep::Break) => break,
                        Err(error) => return Err(error),
                    }
                }

                Ok(())
            })
        };

        Handle {
            cancel_handle: CancelHandle { keep_running },
            thread_handle,
        }
    }
}

pub enum LoopStep {
    Next,
    Break,
}

pub struct Handle<E> {
    cancel_handle: CancelHandle,
    thread_handle: JoinHandle<Result<(), E>>,
}

pub struct CancelHandle {
    keep_running: Arc<AtomicBool>,
}

impl<E> Handle<E> {
    pub fn cancel_handle(&self) -> CancelHandle {
        CancelHandle {
            keep_running: self.cancel_handle.keep_running.clone(),
        }
    }

    pub fn wait(self) -> Result<(), E> {
        match self.thread_handle.join() {
            Ok(r) => r,
            Err(error) => std::panic::panic_any(error),
        }
    }
}

impl CancelHandle {
    pub fn cancel(&self) {
        self.keep_running.store(false, Ordering::Relaxed);
    }
}
