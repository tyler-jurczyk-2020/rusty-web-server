use std::{io::{Error, Read, self, Write, ErrorKind}, rc::Rc, cell::RefCell, fs::File};
use http::{Request, Response, Method};
use mio::{net::TcpStream, Token};
use crate::{global::{GlobalHandle, TaskQueue, Task}, http_parse::{ParseString, ParseBytes}};

pub enum Client {
    Browser(ClientInfo, Rc<RefCell<GlobalHandle>>),
    Python(ClientInfo, Rc<RefCell<GlobalHandle>>),
    Unknown(ClientInfo)
}

pub struct ClientInfo {
    pub stream : TcpStream,
    pub token : Token
}

pub trait Serviceable {
    fn read_from_client(&mut self) -> Result<Request<String>, Error>;
    fn write_to_client(&mut self, response : Response<String>) -> usize;
}

impl Serviceable for ClientInfo {
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
                contents = match Client::load_file("./src/webpage/home.html".to_string(), None) {
                    Ok(c) => Some(c),
                    Err(_) => None 
                }
            }
            return Ok(Client::build_response(contents))
        }
        match self {
            Client::Python(i, d) => {
                if let Ok(r) = i.read_from_client() {
                    let mut handle = d.borrow_mut(); 
                    handle.data.clear();
                    println!("Body: {:?}", r.body().as_bytes());
                    handle.data.extend_from_slice(r.body().as_bytes());
                }
                Ok(Response::default())
            },
            Client::Browser(i, d) => {
                let mut file_contents : Option<Vec<u8>> = None;
                if let Ok(r) = i.read_from_client() {
                    let file_to_load = "./src/webpage".to_string() + &r.uri().to_string();
                    file_contents = match r.method() {
                        &Method::GET => match Client::load_file(file_to_load.clone(), Some(d)) {
                            Ok(c) => Some(c),
                            Err(_) => None 
                        },
                        _ => panic!("Method currently not handled!") 
                    };
                    if file_to_load.eq("./src/webpage/stats") {
                        let mut borrow_task = d.borrow_mut(); 
                        borrow_task.task_queue.queue.push(Task::new(Task::write_task, i.token));
                        borrow_task.task_queue.serviceable += 1;
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

    fn load_file(location : String, handle : Option<&mut Rc<RefCell<GlobalHandle>>>) -> Result<Vec<u8>, io::Error> {
        let mut contents = Vec::new();
        // stats is reserved as a special file that will instead load the data obtained from python
        match location.as_str() {
            "./src/webpage/stats" => {
                if let Some(h) = handle {
                    println!("{:?}", h.borrow().data.to_vec());
                    Ok(h.borrow().data.to_vec()) 
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
