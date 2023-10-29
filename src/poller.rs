use std::io;

use mio::{Interest, Poll, Events, Token};
use mio::net::TcpStream;

pub fn initialize_poll() -> Result<Poll, io::Error> {
    let mut poll = Poll::new();
}


