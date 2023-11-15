use std::net::TcpStream;
use std::io::{BufReader, BufRead, Write, Read, self};
use std::collections::HashMap;

mod poller;

fn main() {
    let mut handler = poller::initialize_poll().unwrap();
    let mut clients = HashMap::new();
    loop {
        handler.poll_events().unwrap();
        for event in handler.get_events() {
            match event.token() {
                mio::Token(0) => {
                    loop {
                        let mut connection = match handler.accept_connection() {
                            Ok((stream, addr)) => (stream, addr),
                            Err(e) if e.kind() == io::ErrorKind::WouldBlock => break, // Need to still handle WouldBlock
                            Err(e) => panic!("{e}")
                        };
                        handler.register_connection(&mut connection.0, 1).unwrap();
                        clients.insert(mio::Token(1), connection);
                    }
                }
                    token => {
                        let mut client = clients.get_mut(&token).unwrap();
                        let mut buffer = String::new();
                        client.0.read_to_string(&mut buffer);
                        println!("{buffer}");
                        match client.0.write_all("HTTP/1.1 200 OK\r\n Content-Type: text/plain\r\n\r\nHi from Rust!".as_bytes()) {
                            Ok(_) => println!("{}", event.is_writable()),
                            Err(e) => panic!("{e}")
                        };
                        handler.deregister_connection(&mut client.0);
                        clients.remove(&token);
                    }            
            }
        }
    }
}

fn process_stream(mut stream : TcpStream) {
    let buf_reader = BufReader::new(&stream);
    let http_request : Vec<_> = buf_reader
        .lines()
        .map(|res| res.unwrap())
        .take_while(|line| !line.is_empty())
        .collect();
    for string in http_request {
        println!("{}", string);
    }
    let greeting = "Hello from the rusty web server!";
    let greeting_len = greeting.len();
    stream.write_all(format!("{}Content-Length: {greeting_len}\r\n\r\n{greeting}", generate_response()).as_bytes()).unwrap();
}

fn generate_response() -> &'static str {
    "HTTP/1.1 200 OK\r\n"
}
