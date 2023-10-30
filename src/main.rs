use std::net::{TcpListener, TcpStream};
use std::io::{BufReader, BufRead, Write};

mod poller;

fn main() {
    let mut handler = poller::initialize_poll().unwrap();
    loop {
        handler.poll_events();
    }

    // Old code
    let tcp_listener = match TcpListener::bind("127.0.0.1:7878") {
        Ok(v) => v,
        Err(e) => panic!("Unable to setup listener: {}", e)
    }; 

    for stream in tcp_listener.incoming() {
        println!("Processing Connection");
        match stream {
            Ok(v) => process_stream(v),
            Err(e) => panic!("Unable to aquire stream: {}", e)
        };
        println!("Processed Connection!");
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
