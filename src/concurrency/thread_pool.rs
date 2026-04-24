use std::sync::{Arc, Mutex, mpsc};
use std::thread::{self, JoinHandle};

type Job = Box<dyn FnOnce() + Send + 'static>;

/// Small fixed-size worker pool used to handle accepted TCP connections.
pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Job>>,
}

impl ThreadPool {
    pub fn new(size: usize) -> Self {
        assert!(size > 0, "thread pool size must be positive");

        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));
        let workers = (0..size)
            .map(|_| Worker::new(Arc::clone(&receiver)))
            .collect();

        Self {
            workers,
            sender: Some(sender),
        }
    }

    pub fn execute<F>(&self, job: F) -> Result<(), mpsc::SendError<Job>>
    where
        F: FnOnce() + Send + 'static,
    {
        self.sender
            .as_ref()
            .expect("thread pool sender should exist")
            .send(Box::new(job))
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        drop(self.sender.take());

        for worker in &mut self.workers {
            if let Some(thread) = worker.thread.take() {
                let _ = thread.join();
            }
        }
    }
}

struct Worker {
    thread: Option<JoinHandle<()>>,
}

impl Worker {
    fn new(receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Self {
        let thread = thread::spawn(move || {
            loop {
                let message = receiver
                    .lock()
                    .expect("worker receiver mutex should not be poisoned")
                    .recv();

                match message {
                    Ok(job) => job(),
                    Err(_) => break,
                }
            }
        });

        Self {
            thread: Some(thread),
        }
    }
}
