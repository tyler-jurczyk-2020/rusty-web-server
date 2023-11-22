use std::cell::RefCell;
use std::io::{self, ErrorKind};
use std::collections::HashMap;
use std::rc::Rc;

use http::{Response, Request};
use mio::Interest;
use poller::{Client, TaskQueue};
use poller::GenericConn;
use poller::Serviceable;

mod poller;
mod http_parse;

fn main() {
    let mut handler = poller::initialize_poll().unwrap();
    let mut clients : HashMap<mio::Token, Client> = HashMap::new();
    let tasklet : Rc<RefCell<TaskQueue>> = Rc::new(RefCell::new(TaskQueue::new()));
    let mut client_id = 1;
    let global_data : Rc<RefCell<Vec<u8>>> = Rc::new(RefCell::new(Vec::new()));
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
            let mut borrowed_tasklet = tasklet.borrow_mut();
            if borrowed_tasklet.serviceable > 0 {
                // Probably should have active and inactive queues, but number of tasks queued should
                // be small to none
                let mut tasks_to_remove = 0;
                for task in &borrowed_tasklet.queue {
                    if task.service {
                        let cl = clients.get_mut(&task.token).unwrap();
                        if let Client::Browser(g, _, _, _) = cl {
                            (task.handler)(g, http_response.clone()); 
                            tasks_to_remove += 1;
                        }
                    } 
                } 
                while tasks_to_remove > 0 {
                    borrowed_tasklet.queue.pop();
                    borrowed_tasklet.serviceable -= 1;
                    tasks_to_remove -= 1;
                }
            }
        }
        for event in handler.get_events() {
            match event.token() {
                mio::Token(0) => {
                    loop {
                        let mut connection = match handler.accept_connection() {
                            Ok((stream, addr)) => Client::Unknown(GenericConn { stream, addr }) ,
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
                                        if let Client::Unknown(g) = removed_item {
                                            clients.insert(token, Client::Python(g, Rc::clone(&global_data)));
                                        } 
                                    }
                                    _ => {
                                        if let Client::Unknown(g) = removed_item {
                                            clients.insert(token, Client::Browser(g, Rc::clone(&global_data),
                                            Rc::clone(&tasklet), token));
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
                                    Client::Browser(g, _, _, _) => {
                                        valid_write = g.write_to_client(http_response.clone());
                                        need_to_write = false;
                                    },
                                    Client::Python(g, _) => {
                                        valid_write = g.write_to_client(http_response1.clone());
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


