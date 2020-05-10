use log::debug;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

// job is closure that consumes its environment -> FnOnce
// jobs must be sendable to other thread -> Send
// Job may outlive any scope within which it is defined -> static lifetime
type Job = Box<dyn FnOnce() + Send + 'static>;

// workers will receive two kinds of messages:
// 1 - new job to process
// 2 - terminate signal
enum Message {
    NewJob(Job),
    Terminate,
}

// thread pool is just vector of workers and
// sending part of channel to propagate the tasks
// to downstream workers
pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Message>,
}

impl ThreadPool {
    pub fn new(size: usize, running: Arc<AtomicBool>) -> ThreadPool {
        assert!(size > 0);

        let (sender, receiver) = mpsc::channel();

        // only one worker can receive the task from channel receiver at any given time -> Mutex
        // in order to propagate Mutex to all worker threada we must wrap it in thread safe reference counter -> Arc
        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver), running.clone()));
        }

        ThreadPool { workers, sender }
    }

    // this method will be called for every test (respective closure)
    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);
        self.sender.send(Message::NewJob(job)).unwrap();
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        debug!("Sending terminate message to all workers.");

        // ask to workers to terminate ...
        for _ in &mut self.workers {
            self.sender.send(Message::Terminate).unwrap();
        }

        debug!("Shutting down all workers.");

        for worker in &mut self.workers {
            debug!("Shutting down worker {}", worker.id);

            // ... wait until the really do so!
            if let Some(thread) = worker.thread.take()
            /* takes the value out of the option, leaving a None in its place. */
            {
                thread.join().unwrap();
            }
        }
    }
}

struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>, // handle to thread that can be joined
}

impl Worker {
    fn new(
        id: usize,
        receiver: Arc<Mutex<mpsc::Receiver<Message>>>,
        running: Arc<AtomicBool>,
    ) -> Worker {
        let thread = thread::spawn(move || {
            loop {
                // thread::sleep(std::time::Duration::from_millis(5000)); // just for debugging

                //
                // receive mutex lock: lock()
                // receive message from channel once available: recv()
                //
                let recv_res = receiver.lock().unwrap().recv();

                if let Err(_) = recv_res {
                    debug!(
                        "Sender for worker {} got disconnected, worker will terminate.",
                        id
                    );
                    break;
                }

                let message = recv_res.unwrap();

                if running.load(Ordering::SeqCst) == false {
                    debug!("Worker {} was told to terminate (ctrl+c pressed).", id);
                    break; // break the worker loop once asked to do so
                }

                match message {
                    Message::NewJob(job) => {
                        debug!("Worker {} got a job; executing.", id);
                        job(); //this will do the job, i.e. execute dialog test
                    }
                    Message::Terminate => {
                        debug!(
                            "Worker {} was told to terminate since thread pool is terminating.",
                            id
                        );
                        break; // break the worker loop once asked to do so
                    }
                }
            } // end loop
        });

        Worker {
            id,
            thread: Some(thread),
        }
    }
}
