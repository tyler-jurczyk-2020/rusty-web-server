use std::net::{TcpListener, TcpStream};
use std::io::{BufReader, BufRead};

fn main() {
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

fn process_stream(stream : TcpStream) {
    let buf_reader = BufReader::new(stream);
    let http_request : Vec<_> = buf_reader
        .lines()
        .map(|res| res.unwrap())
        .take_while(|line| !line.is_empty())
        .collect();
    println!("{:?}", http_request)
}
