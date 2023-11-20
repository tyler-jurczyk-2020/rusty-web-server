use std::io::{BufReader, BufRead, Write, Read, self};
use std::collections::HashMap;

use http::{Response, Request};
use mio::Interest;
use poller::Client;
use poller::GenericConn;
use poller::Serviceable;

mod poller;
mod http_parse;

fn main() {
    let mut handler = poller::initialize_poll().unwrap();
    let mut clients = HashMap::new();
    let mut client_id = 1;
    //let http_response = "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 13\r\n\r\nHi from Rust!";
    let http_response = Response::builder().status(200).header("Content-Length", 9).body("OK OK OK!".to_string()).unwrap();
    loop {
        handler.poll_events().unwrap();
        for event in handler.get_events() {
            match event.token() {
                mio::Token(0) => {
                    loop {
                        let mut connection = match handler.accept_connection() {
                            Ok((stream, addr)) => Client::Browser(GenericConn { stream, addr }) ,
                            Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                            Err(e) => panic!("{e}")
                        };
                        match &mut connection {
                            Client::Browser(b) => handler.register_connection(&mut b.stream, client_id).unwrap(),
                            Client::Python() => panic!("Not yet implemented")
                        };
                        clients.insert(mio::Token(client_id), connection);
                        client_id += 1;
                    }
                }
                token => {
                    let client = match clients.get_mut(&token) {
                        Some(c) => c,
                        None => panic!("Unable to get client, here was the passed token: {token:?}")
                    };
                    let mut need_to_write = true;
                    loop {
                        let mut valid_write = 0;
                        let mut valid_read = 0;
                        if event.is_writable() && need_to_write {
                            valid_write = client.write_to_client(http_response.clone());
                            //handler.reregister_connection(&mut client.stream, token.into(), Interest::READABLE); 
                            need_to_write = false;
                        } 
                        if event.is_readable() {
                            let def = match client.read_from_client() {
                                Ok(r) => r,
                                Err(e) => Request::default()
                            };
                            valid_read = 0; // Likely broken here, probably need to get rid of this line
                        }
                        // Break out of loop once there is no more data to read nor write
                        if valid_read == 0 && valid_write == 0 {
                            break;
                        }
                    }
                    //handler.deregister_connection(&mut client.stream).unwrap();
                    //clients.remove(&token);
                }            
            }
        }
    }
}
