use std::cell::RefCell;
use std::rc::Rc;
use std::fs::File;
use std::io::{Read, Write, self, Error, ErrorKind};
use std::time::Duration;
use http::{Response, Request};
use mio::event::Iter;
use mio::{Interest, Poll, Events, Token};
use mio::net::{TcpListener, TcpStream};
use std::net::SocketAddr;
use http::method::Method;

use crate::http_parse::{ParseBytes, ParseString};

pub trait Serviceable {
    fn read_from_client(&mut self) -> Result<Request<String>, Error>;
    fn write_to_client(&mut self, response : Response<String>) -> usize;
}


pub enum ConnType {
    Server,
    Client(usize)
}

pub enum Client {
    Browser(GenericConn, Rc<RefCell<Vec<u8>>>, Rc<RefCell<TaskQueue>>, Token),
    Python(GenericConn, Rc<RefCell<Vec<u8>>>),
    Unknown(GenericConn)
}

impl From<ConnType> for usize {
    fn from(connection: ConnType) -> usize {
        match connection {
            ConnType::Server => 0,
            ConnType::Client(identifier) => identifier
        }
    } 
}

pub struct IO_Handler {
    poll : Poll,
    events : Events,
    pub server : TcpListener
}

impl IO_Handler {
    pub fn poll_events(&mut self) -> io::Result<()> {
        self.poll.poll(&mut self.events, Some(Duration::new(5, 0)))
    }
    pub fn accept_connection(&self) -> io::Result<(TcpStream, SocketAddr)> {
        self.server.accept()
    }
    pub fn get_events<'a>(&'a self) -> Iter<'a> {
        self.events.iter()
    }
    pub fn register_connection(&self, connection : &mut TcpStream, client : usize) -> io::Result<()> {
        self.poll.registry().register(connection, Token(ConnType::Client(client).into()), Interest::READABLE)
    }
    pub fn reregister_connection(&self, connection : &mut TcpStream, client : usize, interest : Interest) -> io::Result<()> {
        self.poll.registry().reregister(connection, Token(ConnType::Client(client).into()), interest)
    }
    pub fn deregister_connection(&self, connection : &mut TcpStream) -> io::Result<()> {
        self.poll.registry().deregister(connection)
    }
}

pub fn initialize_poll() -> Result<IO_Handler, Box<dyn std::error::Error>> {
    let poll = Poll::new()?;
    let events = Events::with_capacity(10);
    let mut server = TcpListener::bind("127.0.0.1:7878".parse()?)?;
    poll.registry().register(&mut server, Token(ConnType::Server.into()), Interest::READABLE)?;
    Ok(IO_Handler { poll, events, server })
}

pub struct GenericConn {
    pub stream : TcpStream,
    pub addr : SocketAddr
}

impl Serviceable for GenericConn {
    fn read_from_client(&mut self) -> Result<Request<String>, Error> {
        let mut buffer = String::new();
        let bytes_read;
            bytes_read = match self.stream.read_to_string(&mut buffer) {
                Ok(n) => n,
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => buffer.len(),
                Err(e) if e.kind() == io::ErrorKind::ConnectionReset => 0, // Connection was reset,
                                                                           // unable to read
                Err(e) => panic!("{e}")
            };
        if bytes_read > 0 {
            return Ok(buffer.parse_to_struct())
        }
        Err(Error::new(io::ErrorKind::WriteZero, "Improper read of stream"))
    }
    fn write_to_client(&mut self, response : Response<String>) -> usize { // Returns the number of bytes written
        match self.stream.write(&response.clone().parse_to_bytes()) {
            Ok(n) => n,
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => 0, // OS is not ready to write 
            Err(e) if e.kind() == io::ErrorKind::Interrupted => self.write_to_client(response), // Try again if read fails
            Err(e) if e.kind() == io::ErrorKind::BrokenPipe => 0, // Connection probably was closed,
                                                              // NEED TO CLEAN UP!!! 
            Err(e) => panic!("{e}") // All other errors fatal
        }
    }
}

impl Client {
    pub fn handle_request(&mut self, optional_response : Option<Request<String>>) -> Result<Response<String>, io::Error> {
        if let Some(r) = optional_response {
            let mut contents : Option<Vec<u8>> = None;
            if r.method().eq(&http::method::Method::GET) {
                contents = match Client::load_file("./src/webpage/home.html".to_string(), None, None) {
                    Ok(c) => Some(c),
                    Err(_) => None 
                }
            }
            return Ok(Client::build_response(contents))
        }
        match self {
            Client::Python(g, d) => {
                if let Ok(r) = g.read_from_client() {
                    let mut data = d.borrow_mut(); 
                    data.clear();
                    println!("Body: {:?}", r.body().as_bytes());
                    data.extend_from_slice(r.body().as_bytes());
                }
                Ok(Response::default())
            },
            Client::Browser(g, d, t, tok) => {
                let mut file_contents : Option<Vec<u8>> = None;
                if let Ok(r) = g.read_from_client() {
                    let file_to_load = "./src/webpage".to_string() + &r.uri().to_string();
                    file_contents = match r.method() {
                        &Method::GET => match Client::load_file(file_to_load.clone(), Some(d), Some(t)) {
                            Ok(c) => Some(c),
                            Err(_) => None 
                        },
                        _ => panic!("Method currently not handled!") 
                    };
                    if file_to_load.eq("./src/webpage/stats") {
                        let mut borrow_task = t.borrow_mut(); 
                        borrow_task.queue.push(Task::new(write_task, *tok));
                        borrow_task.serviceable += 1;
                        return Err(Error::new(ErrorKind::WouldBlock, String::from_utf8(file_contents.unwrap()).unwrap()))
                    }
                };
                Ok(Client::build_response(file_contents))
            },
            _ => panic!("Unknown requests not supported!")
        }
    } 

    fn build_response(contents : Option<Vec<u8>>) -> Response<String> {
        match contents {
            Some(c) => {
                let contents_as_string = String::from_utf8(c).unwrap();
                Response::builder()
                .status(200)
                .header("Content-Length", contents_as_string.len())
                .body(contents_as_string)
                .unwrap()
            },
            None => {
                Response::builder()
                    .status(404)
                    .header("Content-Length", 0)
                    .body("".to_string())
                    .unwrap()
            }
        }
    }

    fn load_file(location : String, data : Option<&mut Rc<RefCell<Vec<u8>>>>,
                 tasklet : Option<&mut Rc<RefCell<TaskQueue>>>) -> Result<Vec<u8>, io::Error> {
        let mut contents = Vec::new();
        // stats is reserved as a special file that will instead load the data obtained from python
        match location.as_str() {
            "./src/webpage/stats" => {
                if let Some(d) = data {
                    println!("{:?}", d.borrow().to_vec());
                    Ok(d.borrow().to_vec()) 
                }
                else {
                    Err(io::Error::new(io::ErrorKind::Other, "Unable to access data"))
                }
            },
            _ =>  {
                let mut file = File::open(location)?;
                file.read_to_end(&mut contents)?; 
                Ok(contents)
            }
        }
    }
}

pub struct TaskQueue {
    pub queue : Vec<Task>,
    pub serviceable : u32
}

impl TaskQueue {
    pub fn new() -> TaskQueue {
        TaskQueue { queue: Vec::new(), serviceable: 0 }
    }
}

pub struct Task {
    pub service : bool,
    pub handler : fn(&mut GenericConn, Response<String>) -> usize,
    pub token : Token
}

impl Task {
    pub fn new(function : fn(&mut GenericConn, Response<String>) -> usize, token : Token ) -> Task {
        Task { service: true, handler: function, token: token}
    }
}

pub fn write_task(generic : &mut GenericConn, response : Response<String>) -> usize {
    match generic.stream.write(&response.clone().parse_to_bytes()) {
        Ok(n) => n,
        Err(e) if e.kind() == io::ErrorKind::WouldBlock => 0, // OS is not ready to write 
        Err(e) if e.kind() == io::ErrorKind::Interrupted => write_task(generic, response), // Try again if read fails
        Err(e) if e.kind() == io::ErrorKind::ConnectionReset => 0, // Connection Reset
        Err(e) if e.kind() == io::ErrorKind::BrokenPipe => 0, // Connection probably was closed,
                                                              // NEED TO CLEAN UP!!!
        Err(e) => panic!("{e}") // All other errors fatal
    }
}
