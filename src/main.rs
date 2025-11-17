#![allow(unused_imports)]
use std::net::TcpListener;
use std::io::{Read, Write};

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(mut _stream) => {
                println!("accepted new connection");
                let mut buffer = [0; 1024];
                loop {
                    match _stream.read(&mut buffer) {
                        Ok(bytes) => {
                            if bytes == 0 {
                                break;
                            }
                            let received_data = String::from_utf8_lossy(&buffer[..bytes]);
                            match received_data.to_string().as_str() {
                                "PING\n" => {
                                    _stream.write_all(b"+PONG\r\n").unwrap();
                                }
                                _ => {
                                    _stream.write_all(b"+PONG\r\n").unwrap();
                                }
                            }
                        }
                        Err(e) => {
                            println!("error: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
