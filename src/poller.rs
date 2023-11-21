use std::fs::File;
use std::io::{Read, Write, self, Error};
use http::{Response, Request};
use mio::event::Iter;
use mio::{Interest, Poll, Events, Token};
use mio::net::{TcpListener, TcpStream};
use std::net::SocketAddr;

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
    Browser(GenericConn),
    Python(GenericConn),
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
        self.poll.poll(&mut self.events, None)
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
            Err(e) if e.kind() == io::ErrorKind::BrokenPipe => 0, // Connection probably was closed
            Err(e) => panic!("{e}") // All other errors fatal
        }
    }
}

impl Client {
    pub fn handle_request(&mut self, optional_response : Option<Request<String>>) -> Response<String> {
        if let Some(r) = optional_response {
            let mut contents = Vec::new();
            if r.method().eq(&http::method::Method::GET) {
                let mut file = match File::open("./src/webpage/home.html") {
                    Ok(f) => f,
                    Err(e) => panic!("{e}")
                };
                if let Err(e) = file.read_to_end(&mut contents) {
                    panic!("{e}");
                } 
            }
            let contents_as_string = String::from_utf8(contents).unwrap();
            return Response::builder()
                .status(200)
                .header("Content-Length", contents_as_string.len())
                .body(contents_as_string)
                .unwrap()
        }
        match self {
            Client::Python(g) => {
                if let Ok(r) = g.read_from_client() {
                    println!("{:?}", r);
                }
                Response::default()
            },
            Client::Browser(g) | Client::Unknown(g) => {
                let mut contents = Vec::new();
                if let Ok(r) = g.read_from_client() {
                    if r.method().eq(&http::method::Method::GET) {
                        let mut file = match File::open("./src/webpage/home.html") {
                            Ok(f) => f,
                            Err(e) => panic!("{e}")
                        };
                        if let Err(e) = file.read_to_end(&mut contents) {
                            panic!("{e}");
                        } 
                    }
                }
                let contents_as_string = String::from_utf8(contents).unwrap();
                Response::builder()
                    .status(200)
                    .header("Content-Length", contents_as_string.len())
                    .body(contents_as_string)
                    .unwrap()
            }
        } 
    } 
}
