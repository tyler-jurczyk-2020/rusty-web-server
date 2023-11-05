use std::error::Error;
use mio::event::Iter;
use mio::{Interest, Poll, Events, Token};
use mio::net::{TcpListener, TcpStream};
use std::net::SocketAddr;

pub enum ConnType {
    Server,
    Client(usize)
}

impl Into<usize> for ConnType {
    fn into(self) -> usize {
        match self {
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
    pub fn poll_events(&mut self) {
        self.poll.poll(&mut self.events, None);
    }
    pub fn accept_connection(&self) -> (TcpStream, SocketAddr) {
        match self.server.accept() {
            Ok((stream, addr)) => (stream, addr),
            Err(e) => panic!("")
        }
    }
    pub fn get_events<'a>(&'a self) -> Iter<'a> {
        self.events.iter()
    }
}

pub fn initialize_poll() -> Result<IO_Handler, Box<dyn Error>> {
    let poll = Poll::new()?;
    let events = Events::with_capacity(10);
    let mut server = TcpListener::bind("127.0.0.1:7878".parse()?)?;
    poll.registry().register(&mut server, Token(ConnType::Server.into()), Interest::READABLE)?;
    Ok(IO_Handler { poll, events, server })
}


