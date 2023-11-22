use std::io::{Write, self};

use http::Response;
use mio::Token;

use crate::{client::ClientInfo, http_parse::ParseBytes};


pub struct GlobalHandle {
    pub task_queue : TaskQueue,
    pub data : Vec<u8>,
    pub is_fresh : bool
}

pub struct TaskQueue {
    pub queue : Vec<Task>,
    pub serviceable : u32
}

pub struct Task {
    pub service : bool,
    pub handler : fn(&mut ClientInfo, Response<String>) -> usize,
    pub token : Token
}

impl GlobalHandle {
    pub fn new() -> GlobalHandle {
        GlobalHandle { task_queue: TaskQueue::new(), data: Vec::new(), is_fresh: false }
    }
}

impl TaskQueue {
    pub fn new() -> TaskQueue {
        TaskQueue { queue: Vec::new(), serviceable: 0 }
    }
}

impl Task {
    pub fn new(function : fn(&mut ClientInfo, Response<String>) -> usize, token : Token ) -> Task {
        Task { service: true, handler: function, token: token}
    }

    pub fn write_task(generic : &mut ClientInfo, response : Response<String>) -> usize {
        match generic.stream.write(&response.clone().parse_to_bytes()) {
            Ok(n) => n,
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => 0, // OS is not ready to write 
            Err(e) if e.kind() == io::ErrorKind::Interrupted => Task::write_task(generic, response), // Try again if read fails
            Err(e) if e.kind() == io::ErrorKind::ConnectionReset => 0, // Connection Reset
            Err(e) if e.kind() == io::ErrorKind::BrokenPipe => 0, // Connection probably was closed,
                                                                  // NEED TO CLEAN UP!!!
            Err(e) => panic!("{e}") // All other errors fatal
        }
    }
}


