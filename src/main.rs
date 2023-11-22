use std::cell::RefCell;
use std::io::{self, ErrorKind};
use std::collections::HashMap;
use std::rc::Rc;

use client::{Client, Serviceable, ClientInfo};
use global::{TaskQueue, GlobalHandle};
use http::{Response, Request};
use mio::Interest;

mod poller;
mod http_parse;
mod global;
mod client;

fn main() {
    let mut handler = poller::initialize_poll().unwrap();
    let mut clients : HashMap<mio::Token, Client> = HashMap::new();
    let mut client_id = 1;
    let global_data : Rc<RefCell<GlobalHandle>> = Rc::new(RefCell::new(GlobalHandle::new()));
    //let http_response = "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 13\r\n\r\nHi from Rust!";
    let mut http_response = Response::builder()
                                .status(200)
                                .header("Content-Length", 9)
                                .body("OK OK OK!"
                                .to_string()).unwrap();
    let http_response1 = Response::builder()
                            .status(200)
                            .header("Content-Length", 9)
                            .body("eK eK eK!"
                            .to_string()).unwrap();
    loop {
        handler.poll_events().unwrap();
        {
            let mut borrowed_handle = global_data.borrow_mut();
            if borrowed_handle.task_queue.serviceable > 0 {
                if !borrowed_handle.task_queue.queue.is_empty() {
                    let handle_task = borrowed_handle.task_queue.queue.pop();
                    if let Some(t) = handle_task {
                        let client_info = clients.get_mut(&t.token).unwrap();
                        if let Client::Browser(i, _) = client_info {
                            (t.handler)(i, borrowed_handle.data.clone());
                            borrowed_handle.task_queue.serviceable -= 1;
                            borrowed_handle.is_fresh = false;
                        }
                    }
                }
            }
        }
        for event in handler.get_events() {
            match event.token() {
                mio::Token(0) => {
                    loop {
                        let mut connection = match handler.accept_connection() {
                            Ok((stream, _)) => Client::Unknown(ClientInfo { stream, token: mio::Token(client_id) }) ,
                            Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                            Err(e) => panic!("{e}")
                        };
                        if let Client::Unknown(g) = &mut connection {
                            handler.register_connection(&mut g.stream, client_id);
                        }
                        clients.insert(mio::Token(client_id), connection);
                        client_id += 1;
                    }
                }
                token => {
                    let client = match clients.get_mut(&token) {
                        Some(c) => c,
                        None => panic!("Unable to get client, here was the passed token: {token:?}")
                    };
                    // We will always upgrade the unknown client before proceeding with any other
                    // operations
                    if let Client::Unknown(g) = client {
                        if event.is_readable() {
                            if let Ok(r) = g.read_from_client() {
                                //println!("{:?}", r);
                                handler.reregister_connection(&mut g.stream, token.into(), Interest::READABLE | Interest::WRITABLE);
                                // Follow check allows us to load browser page even through we
                                // consume the readable event
                                if let Some(s) = r.headers().get("User-Agent") {
                                    if s.ne("python-requests/2.31.0") {
                                        http_response = match client.handle_request(Some(r.clone())) {
                                            Ok(r) => r,
                                            Err(e) => panic!("Wut")
                                        }
                                    }
                                    //println!("Response: {http_response:?}");
                                }
                                let removed_item = clients.remove(&token).unwrap();
                                match r.headers().get("User-Agent").unwrap().to_str().unwrap() {
                                    "python-requests/2.31.0" => {
                                        if let Client::Unknown(i) = removed_item {
                                            clients.insert(token, Client::Python(i, Rc::clone(&global_data)));
                                        } 
                                    }
                                    _ => {
                                        if let Client::Unknown(i) = removed_item {
                                            clients.insert(token, Client::Browser(i, Rc::clone(&global_data)));
                                        }
                                    }
                                };
                            } 
                        }
                    }
                    else {
                        let mut need_to_write = true;
                        loop {
                            let mut valid_write = 0;
                            let mut valid_read = 0;
                            if event.is_readable() {
                                http_response = match client.handle_request(None) {
                                    Ok(r) => r,
                                    Err(e) if e.kind() == ErrorKind::WouldBlock => {
                                        need_to_write = false;
                                        println!("{e}");
                                        Response::builder()
                                        .status(200)
                                        .header("Content-Length", e.to_string().len())
                                        .body(e.to_string())
                                        .unwrap()
                                    },
                                    Err(e) => panic!("Don't know how to handle this...")
                                };
                            }
                            if event.is_writable() && need_to_write {
                                match client {
                                    Client::Browser(i, _) => {
                                        valid_write = i.write_to_client(http_response.clone());
                                        need_to_write = false;
                                    },
                                    Client::Python(i, _) => {
                                        valid_write = i.write_to_client(http_response1.clone());
                                        need_to_write = false;
                                    },
                                    _ => panic!("Attempting to write to potentially unknown client!")
                                } 
                            }
                            // Break out of loop once there is no more data to read nor write
                            if valid_read == 0 && valid_write == 0 {
                                break;
                            }
                        }
                    }
                }            
            }
        }
    }
}


