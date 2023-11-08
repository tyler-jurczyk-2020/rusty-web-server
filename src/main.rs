use std::collections::HashMap;
use std::net::{TcpListener, TcpStream};
use std::io::{BufReader, BufRead, Write, Read};
use std::time::Duration;

mod poller;

fn main() {
    let mut handler = poller::initialize_poll().unwrap();
    loop {
        handler.poll_events();
        for event in handler.get_events() {
            match event.token() {
                mio::Token(0) => {
                    println!("Client attempting to connect to the server!");
                    let mut connection = match handler.accept_connection() {
                        Ok((stream, addr)) => (stream, addr),
                        Err(e) => panic!("{e}") // Need to still handle WouldBlock
                    };
                    if let Err(_) = handler.register_connection(&mut connection.0, 1) {
                        println!("Unable to register the connection")
                    }


                    // Dead code, should be moved into separate function
                    let mut buffer = String::new();
                    connection.0.read_to_string(&mut buffer);
                    println!("{buffer}");
                    match connection.0.write_all("HTTP/1.1 200 OK\r\n Content-Type: text/plain\r\n\r\nb".as_bytes()) {
                        Ok(_) => println!("All is good!"),
                        Err(e) => panic!("{e}")
                    };
                }
                token => println!("We received client token: {token:?}"),
            }
        }
    }

    // Old code
    //let tcp_listener = match TcpListener::bind("127.0.0.1:7878") {
    //    Ok(v) => v,
    //    Err(e) => panic!("Unable to setup listener: {}", e)
    //}; 

    //for stream in tcp_listener.incoming() {
    //    println!("Processing Connection");
    //    match stream {
    //        Ok(v) => process_stream(v),
    //        Err(e) => panic!("Unable to aquire stream: {}", e)
    //    };
    //    println!("Processed Connection!");
    //}
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
