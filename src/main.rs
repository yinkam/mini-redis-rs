mod handler;
mod resp;

use handler::tcp_handler;
use std::collections::HashMap;
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();
    let db = Arc::new(Mutex::new(HashMap::new()));

    for stream in listener.incoming() {
        match stream {
            Ok(mut _stream) => {
                println!("accepted new connection");
                let db = Arc::clone(&db);
                thread::spawn(move || {
                    tcp_handler(&_stream, db);
                });
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
