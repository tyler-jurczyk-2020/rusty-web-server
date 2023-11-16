use std::net::TcpStream;
use std::io::{BufReader, BufRead, Write, Read, self};
use std::collections::HashMap;

use poller::Client;

mod poller;

fn main() {
    let mut handler = poller::initialize_poll().unwrap();
    let mut clients = HashMap::new();
    let mut client_id = 1;
    let http_response = "HTTP/1.1 200 OK\r\n Content-Type: text/plain\r\n\r\nHi from Rust!";
    loop {
        handler.poll_events().unwrap();
        for event in handler.get_events() {
            match event.token() {
                mio::Token(0) => {
                    loop {
                        let mut connection = match handler.accept_connection() {
                            Ok((stream, addr)) => Client { stream, addr },
                            Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                            Err(e) => panic!("{e}")
                        };
                        handler.register_connection(&mut connection.stream, client_id).unwrap();
                        client_id += 1;
                        clients.insert(mio::Token(1), connection);
                    }
                }
                token => {
                    let client = clients.get_mut(&token).unwrap();
                    loop {
                        let mut valid_write = 0;
                        let mut valid_read = 0;
                        if event.is_writable() {
                            valid_write = client.write_to_client(http_response.to_string());
                        } 
                        if event.is_readable() {
                            valid_read = client.read_from_client().len();
                        }
                        // Break out of loop once there is no more data to read nor write
                        if valid_read == 0 && valid_write == 0 {
                            break;
                        }
                    }
                    handler.deregister_connection(&mut client.stream).unwrap();
                    clients.remove(&token);
                }            
            }
        }
    }
}
