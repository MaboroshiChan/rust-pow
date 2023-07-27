use std::sync::mpsc;
use std::thread;

pub trait Task {
    type Output: Send;
    fn run(&self) -> Option<Self::Output>;
}

pub struct WorkQueue<TaskType: 'static + Task + Send> {
    send_tasks: Option<spmc::Sender<TaskType>>, // Option because it will be set to None to close the queue
    recv_tasks: spmc::Receiver<TaskType>,
    //send_output: mpsc::Sender<TaskType::Output>, // not need in the struct: each worker will have its own clone.
    recv_output: mpsc::Receiver<TaskType::Output>,
    workers: Vec<thread::JoinHandle<()>>,
}

impl<TaskType: 'static + Task + Send> WorkQueue<TaskType> {
    pub fn new(n_workers: usize) -> WorkQueue<TaskType> {
        // create the channels; start the worker threads; record their JoinHandles
        let (tx, rx) = spmc::channel();
        let (tx2, rx2) = mpsc::channel();
        let mut workers_queue = Vec::new();
        for _ in 0..n_workers {
            let rx = rx.clone();
            let tx2 = tx2.clone();
            // standby for a task
            workers_queue.push( thread::spawn(move || {
                WorkQueue::run(rx, tx2);
            }));
        }
        WorkQueue {
            send_tasks: Some(tx),
            recv_tasks: rx, // create a spmc channel
            recv_output: rx2, // create a mpsc channel
            workers: workers_queue,   // create the worker threads
        }
    }

    fn run(recv_tasks: spmc::Receiver<TaskType>, send_output: mpsc::Sender<TaskType::Output>) {
        // TODO: the main logic for a worker [thread]
        // task == Ok(t) || task == Err(e)
        loop {
            let task = recv_tasks.recv();
            // if connection is still alive, it waits here.
            match task {
                Ok(t) => {

                    let result = t.run();
                    match result {
                        Some(result) => {
                            send_output.send(result).unwrap();
                        },
                        None => {
                            // handle the case where the task has no result
                        }
                    }
                }
                Err(_) => {
                    break;
                }
            }
            // task_result will be Err() if the spmc::Sender has been destroyed and no more messages can be received here
        }
    }

    pub fn enqueue(&mut self, t: TaskType) -> Result<(), spmc::SendError<TaskType>> {
        // &Option<T> -> Option<&T> -> &T ->
        self.send_tasks.as_mut().unwrap().send(t)
    }

    // Helper methods that let you receive results in various ways
    pub fn iter(&mut self) -> mpsc::Iter<TaskType::Output> {
        self.recv_output.iter()
    }
    pub fn recv(&mut self) -> TaskType::Output {
        self.recv_output
            .recv()
            .expect("I have been shutdown incorrectly")
    }
    pub fn try_recv(&mut self) -> Result<TaskType::Output, mpsc::TryRecvError> {
        self.recv_output.try_recv()
    }
    pub fn recv_timeout(
        &self,
        timeout: std::time::Duration,
    ) -> Result<TaskType::Output, mpsc::RecvTimeoutError> {
        self.recv_output.recv_timeout(timeout)
    }

    pub fn shutdown(&mut self) {
        // Destroy the spmc::Sender so everybody knows no more tasks are incoming;
        // drain any pending tasks in the queue; wait for each worker thread to finish.
        // HINT: Vec.drain(..)
        self.send_tasks = None;
        while let Ok(_) = self.recv_tasks.try_recv() {}
        for handle in self.workers.drain(..) {
            handle.join().expect("The thread being joined has panicked");
        }
    }
}

impl<TaskType: 'static + Task + Send> Drop for WorkQueue<TaskType> {
    fn drop(&mut self) {
        // "Finalisation in destructors" pattern: https://rust-unofficial.github.io/patterns/idioms/dtor-finally.html
        match self.send_tasks {
            None => {} // already shut down
            Some(_) => self.shutdown(),
        }
    }
}
