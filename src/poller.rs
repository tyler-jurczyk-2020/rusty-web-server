use std::error::Error;
use mio::{Interest, Poll, Events, Token};
use mio::net::TcpListener;

pub struct IO_Handler {
    poll : Poll,
    pub events : Events,
    pub server : TcpListener
}

impl IO_Handler {
    pub fn poll_events(&mut self) {
        self.poll.poll(&mut self.events, None);
    }
}

pub fn initialize_poll() -> Result<IO_Handler, Box<dyn Error>> {
    let poll = Poll::new()?;
    let events = Events::with_capacity(10);
    let mut server = TcpListener::bind("127.0.0.1:7878".parse()?)?;
    poll.registry().register(&mut server, Token(0), Interest::READABLE)?;
    Ok(IO_Handler { poll, events, server })
}


