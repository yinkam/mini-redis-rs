use crate::resp::value::Value;
use crate::resp::{parser::parse, value::Value::*};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};

pub fn tcp_handler(mut stream: &TcpStream, db: Arc<Mutex<HashMap<Vec<u8>, Vec<u8>>>>) {
    println!("Connection from {}", stream.peer_addr().unwrap());
    let mut buffer = [0; 512];
    loop {
        match stream.read(&mut buffer) {
            Ok(bytes) => {
                if bytes == 0 {
                    break;
                }

                let (_, parsed_command) = parse(&buffer);
                process_command(stream, &db, parsed_command)
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}

fn process_command(
    mut stream: &TcpStream,
    db: &Arc<Mutex<HashMap<Vec<u8>, Vec<u8>>>>,
    command: Value,
) {
    match command {
        Array(arr) => match &arr[0] {
            BulkString(string) => match string.as_str() {
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
                        Some(value) => stream.write_all(&value).unwrap(),
                        None => {
                            let null = Null.to_resp();
                            stream.write_all(&null).unwrap()
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
