mod resp;

use resp::parser::parse;
use resp::value::*;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

fn handler(mut stream: &TcpStream, db: Arc<Mutex<HashMap<Vec<u8>, Vec<u8>>>>) {
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
                    Value::Array(arr) => match &arr[0] {
                        Value::BulkString(string) => match string.as_str() {
                            "PING" => stream.write_all(b"+PONG\r\n").unwrap(),
                            "ECHO" => stream.write_all(&arr[1].to_resp()).unwrap(),
                            "SET" => {
                                let key = arr[1].to_resp();
                                let value = arr[2].to_resp();
                                db.lock().unwrap().entry(key).or_insert(value);

                                stream.write_all(b"+OK\r\n").unwrap()
                            }
                            "GET" => {
                                let key = arr[1].to_resp();
                                let value = db.lock().unwrap().get(&key).cloned();

                                match value {
                                    Some(value) => stream.write_all(&*value).unwrap(),
                                    None => {
                                        let null = Value::Null.to_resp();
                                        stream.write_all(&*null).unwrap()
                                    }
                                }
                                stream.write_all(&arr[1].to_resp()).unwrap()
                            }
                            _ => println!("Invalid command: {}", string),
                        },
                        _ => println!("Invalid command"),
                    },
                    _ => stream.write_all(b"+PONG\r\n").unwrap(),
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
    let db = Arc::new(Mutex::new(HashMap::new()));

    for stream in listener.incoming() {
        match stream {
            Ok(mut _stream) => {
                println!("accepted new connection");
                let store = Arc::clone(&db);
                thread::spawn(move || {
                    handler(&_stream, store);
                });
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
