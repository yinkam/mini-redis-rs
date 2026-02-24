use crate::cache::Cache;
use crate::resp::value::Value;
use crate::resp::{parser::parse, value::Value::*};
use crate::ServerInfo;
use mio::net::TcpStream;
use std::io::ErrorKind::WouldBlock;
use std::io::{Read, Write};
use std::time::{Duration, Instant};

pub fn tcp_handler(mut stream: &TcpStream, db: &mut Cache, server_info: &ServerInfo) {
    let mut buffer = [0; 512];
    loop {
        match stream.read(&mut buffer) {
            Ok(bytes) => {
                if bytes == 0 {
                    break;
                }

                let (_, parsed_command) = parse(&buffer);
                process_command(stream, db, &parsed_command, server_info);
            }
            Err(ref err) if err.kind() == WouldBlock => break,
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}

fn process_command(
    mut stream: &TcpStream,
    db: &mut Cache,
    command: &Value,
    server_info: &ServerInfo,
) {
    match command {
        Array(arr) => match &arr[0] {
            BulkString(string) => match string.to_uppercase().as_ref() {
                "PING" => stream.write_all(b"+PONG\r\n").unwrap(),
                "ECHO" => stream.write_all(&arr[1].to_resp()).unwrap(),
                "SET" => execute_set(stream, arr, db),
                "GET" => execute_get(stream, arr, db),
                "INFO" => execute_info(stream, arr, db, server_info),
                "REPLCONF" => execute_replconf(stream, arr),
                _ => stream.write_all(b"-ERR Unknown command\r\n").unwrap(),
            },
            _ => println!("Invalid command"),
        },
        _ => stream.write_all(b"-Err an error occured\r\n").unwrap(),
    }
}

fn execute_set(mut stream: &TcpStream, arr: &Vec<Value>, db: &mut Cache) {
    let key = arr[1].to_resp();
    let value = arr[2].to_resp();

    if arr.len() > 3 {
        match &arr[3] {
            BulkString(string) => match string.to_uppercase().as_ref() {
                "PX" => match &arr[4] {
                    BulkString(x) => {
                        let time = x.parse::<u64>().unwrap();
                        let duration = Duration::from_millis(time);
                        let expiry_time = Instant::now() + duration;
                        match db.insert(key, value, Some(expiry_time)) {
                            Some(_) => stream.write_all(b"+UPDATED\r\n").unwrap(),
                            None => stream.write_all(b"+OK\r\n").unwrap(),
                        }
                    }
                    _ => println!("INVALID VALUE TYPE {:?}", &arr[3]),
                },
                "EX" => match &arr[4] {
                    BulkString(x) => {
                        let time = x.parse::<u64>().unwrap();
                        let duration = Duration::from_secs(time);
                        let expiry_time = Instant::now() + duration;
                        match db.insert(key, value, Some(expiry_time)) {
                            Some(_) => stream.write_all(b"+UPDATED\r\n").unwrap(),
                            None => stream.write_all(b"+OK\r\n").unwrap(),
                        }
                    }
                    _ => println!("INVALID VALUE {:?}", &arr[3]),
                },
                _ => println!("INVALID SUBCOMMAND {:?}", &arr[3]),
            },
            _ => println!("INVALID COMMAND STRUCTURE {:?}", &arr[2]),
        }
    } else {
        let res = db.insert(key, value, None);

        match res {
            Some(_) => stream.write_all(b"+UPDATED\r\n").unwrap(),
            None => stream.write_all(b"+OK\r\n").unwrap(),
        }
    }
}

fn execute_get(mut stream: &TcpStream, arr: &Vec<Value>, db: &mut Cache) {
    let key = arr[1].to_resp();
    let null_bulk_string = b"$-1\r\n".to_vec();

    let value = &db.get(&key).unwrap_or(null_bulk_string);
    stream.write_all(value).unwrap()
}

fn execute_info(
    mut stream: &TcpStream,
    _arr: &Vec<Value>,
    _db: &mut Cache,
    server_info: &ServerInfo,
) {
    let server_info = BulkString(format!(
        "role:{}\nmaster_replid:{}\nmaster_repl_offset:{}",
        server_info.role, server_info.master_replid, server_info.master_repl_offset
    ));

    stream.write_all(&server_info.to_resp()).unwrap()
}

fn execute_replconf(mut stream: &TcpStream, arr: &Vec<Value>) {
    println!("EXECUTE REPLCONF");

    match &arr[1] {
        BulkString(string) => match string.to_lowercase().as_ref() {
            "listening-port" => match &arr[2] {
                BulkString(_x) => stream.write_all(b"+OK\r\n").unwrap(),
                _ => println!("INVALID listening port {:?}", &arr[2]),
            },
            "capa" => match &arr[2] {
                BulkString(x) => match x.to_lowercase().as_ref() {
                    "psync2" => stream.write_all(b"+OK\r\n").unwrap(),
                    _ => println!("INVALID CAPA COMMAND {:?}", &arr[2]),
                },
                _ => println!("INVALID VALUE {:?}", &arr[3]),
            },
            _ => println!("INVALID REPLCONF COMMAND {:?}", &arr[3]),
        },
        _ => println!("INVALID COMMAND STRUCTURE {:?}", &arr[2]),
    }
}
