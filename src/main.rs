#![allow(unused_imports)]

mod resp;

use std::borrow::Cow;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::str;
use std::thread;
use resp::parser::parse;
use resp::value::*;


fn handler(mut stream: &TcpStream) {
    println!("Connection from {}", stream.peer_addr().unwrap());
    let mut buffer = [0; 512];
    loop {
        match stream.read(&mut buffer) {
            Ok(bytes) => {
                if bytes == 0 {
                    break;
                }

                let (_, parsed_command) = parse(&buffer);
                match parsed_command {
                    Value::Array(arr)  => {
                        match &arr[0] {
                            Value::BulkString(string) => {
                                match string.as_str() {
                                    "PING" => stream.write_all(b"+PONG\r\n").unwrap(),
                                    "ECHO" => stream.write_all(&arr[1].to_resp()).unwrap(),
                                    _ => println!("Invalid command: {}", string),
                                }
                            }
                            _ => println!("Invalid command"),
                        }
                    }
                    _ => stream.write_all(b"+PONG\r\n").unwrap()
                }
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(mut _stream) => {
                println!("accepted new connection");
                thread::spawn(move || {
                    handler(&_stream);
                });
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
