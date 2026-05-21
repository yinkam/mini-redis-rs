use crate::cache::Cache;
use crate::rdb::RDB;
use crate::resp::value::Value;
use crate::resp::{parser, value::Value::*};
use crate::ServerInfo;
use mio::net::TcpStream;
use mio::Token;
use std::collections::HashMap;
use std::io::ErrorKind::{NotFound, WouldBlock};
use std::io::{Error, Read, Write};
use std::time::{Duration, Instant};

pub fn tcp_handler(
    db: &mut Cache,
    client: &Token,
    connections: &mut HashMap<Token, TcpStream>,
    server_info: &mut ServerInfo,
) {
    let buffer = match read_buffer(client, connections) {
        Some(v) => v,
        None => return,
    };
    let mut bytes_offset = 0usize;
    while bytes_offset < buffer.len() {
        let (bytes_consumed, parsed_command) = parser::parse(buffer[bytes_offset..].to_vec());

        match process_command(db, &parsed_command, client, connections, server_info) {
            Ok(true) => {
                if server_info.role == "master" {
                    propagate_command(
                        server_info,
                        connections,
                        buffer[bytes_offset..bytes_consumed].to_vec(),
                    )
                    .expect("Error while propagating command");
                }
            }
            Ok(false) => {
                bytes_offset += bytes_consumed;
                continue;
            }
            Err(e) => {
                println!("Error processing command: {}", e);
                return;
            }
        };

        bytes_offset += bytes_consumed;
    }
}

fn read_buffer(client: &Token, connections: &mut HashMap<Token, TcpStream>) -> Option<Vec<u8>> {
    let stream = match connections.get_mut(&client) {
        Some(conn) => conn,
        None => {
            println!("Error getting stream");
            return None;
        }
    };

    let mut buffer = [0; 512];
    loop {
        match stream.read(&mut buffer) {
            // TODO: handle partial reads/failures of the byte stream
            Ok(bytes) => {
                if bytes == 0 {
                    break;
                }

                return Some(buffer[..bytes].to_vec());
            }
            Err(ref err) if err.kind() == WouldBlock => break,
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
    None
}

fn write_buffer(stream: &mut TcpStream, buffer: &[u8]) -> Result<(), Error> {
    stream.write_all(buffer)?;
    stream.flush()?;
    Ok(())
}

fn propagate_command(
    server_info: &mut ServerInfo,
    connections: &mut HashMap<Token, TcpStream>,
    buffer: Vec<u8>,
) -> Result<(), Error> {
    let mut errors: Vec<Error> = Vec::new();

    for replica in server_info.replicas.clone() {
        let stream = match connections.get_mut(&replica) {
            Some(s) => s,
            None => {
                errors.push(Error::new(
                    NotFound,
                    format!("connection {:?} not found", replica),
                ));
                continue;
            }
        };

        match write_buffer(stream, &buffer) {
            Ok(_) => continue,
            _ => {
                println!("replication to {:?} failed", replica);
                continue;
            }
        }
    }

    if !errors.is_empty() {
        println!("Error getting replicas connections: {:#?}", errors);
    }

    Ok(())
}

fn process_command(
    db: &mut Cache,
    command: &Value,
    client: &Token,
    connections: &mut HashMap<Token, TcpStream>,
    server_info: &mut ServerInfo,
) -> Result<bool, Error> {
    let stream = connections
        .get_mut(&client)
        .ok_or_else(|| Error::new(NotFound, "no such connection"))?;

    match command {
        Array(arr) => match &arr[0] {
            BulkString(string) => match string.to_uppercase().as_ref() {
                "PING" => {
                    write_buffer(stream, b"+PONG\r\n")?;
                    Ok(false)
                }
                "ECHO" => {
                    write_buffer(stream, &arr[1].to_resp())?;
                    Ok(false)
                }
                "SET" => {
                    let response = execute_set(arr, db);
                    if server_info.role != "slave" {
                        write_buffer(stream, &response)?;
                    }
                    Ok(true)
                }
                "GET" => {
                    execute_get(stream, arr, db)?;
                    Ok(false)
                }
                "INFO" => {
                    execute_info(stream, arr, db, server_info)?;
                    Ok(false)
                }
                "REPLCONF" => {
                    execute_replconf(stream, arr)?;
                    Ok(false)
                }
                "PSYNC" => {
                    execute_psync(stream, arr, client, server_info)?;
                    Ok(false)
                }
                _ => {
                    write_buffer(stream, b"-ERR Unknown Command\r\n")?;
                    Ok(false)
                }
            },
            _ => {
                write_buffer(stream, b"-Err Invalid Command\r\n")?;
                Ok(false)
            }
        },
        _ => {
            write_buffer(stream, b"-Err an error occured\r\n")?;
            Ok(false)
        }
    }
}

fn execute_set(arr: &Vec<Value>, db: &mut Cache) -> Vec<u8> {
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
                            Some(_) => b"+UPDATED\r\n".to_vec(),
                            None => b"+OK\r\n".to_vec(),
                        }
                    }
                    _ => panic!("INVALID VALUE TYPE {:?}", &arr[3]),
                },
                "EX" => match &arr[4] {
                    BulkString(x) => {
                        let time = x.parse::<u64>().unwrap();
                        let duration = Duration::from_secs(time);
                        let expiry_time = Instant::now() + duration;
                        match db.insert(key, value, Some(expiry_time)) {
                            Some(_) => b"+UPDATED\r\n".to_vec(),
                            None => b"+OK\r\n".to_vec(),
                        }
                    }
                    _ => panic!("INVALID VALUE {:?}", &arr[3]),
                },
                _ => panic!("INVALID SUBCOMMAND {:?}", &arr[3]),
            },
            _ => panic!("INVALID COMMAND STRUCTURE {:?}", &arr[2]),
        }
    } else {
        let res = db.insert(key, value, None);

        match res {
            Some(_) => b"+UPDATED\r\n".to_vec(),
            None => b"+OK\r\n".to_vec(),
        }
    }
}

fn execute_get(stream: &mut TcpStream, arr: &Vec<Value>, db: &mut Cache) -> Result<(), Error> {
    let key = arr[1].to_resp();
    let null_bulk_string = b"$-1\r\n".to_vec();

    let value = &db.get(&key).unwrap_or(null_bulk_string);
    write_buffer(stream, value)
}

fn execute_info(
    stream: &mut TcpStream,
    _arr: &Vec<Value>,
    _db: &mut Cache,
    server_info: &ServerInfo,
) -> Result<(), Error> {
    let server_info = BulkString(format!(
        "role:{}\nmaster_replid:{}\nmaster_repl_offset:{}",
        server_info.role, server_info.master_replid, server_info.master_repl_offset
    ));

    write_buffer(stream, &server_info.to_resp())
}

fn execute_replconf(stream: &mut TcpStream, arr: &Vec<Value>) -> Result<(), Error> {
    match &arr[1] {
        BulkString(string) => match string.to_lowercase().as_ref() {
            "listening-port" => match &arr[2] {
                BulkString(_x) => write_buffer(stream, b"+OK\r\n"),
                _ => panic!("INVALID listening port {:?}", &arr[2]),
            },
            "capa" => match &arr[2] {
                BulkString(x) => match x.to_lowercase().as_ref() {
                    "psync2" => write_buffer(stream, b"+OK\r\n"),
                    _ => panic!("INVALID CAPA COMMAND {:?}", &arr[2]),
                },
                _ => panic!("INVALID VALUE {:?}", &arr[3]),
            },
            "getack" => match &arr[2] {
                BulkString(x) => match x.to_lowercase().as_ref() {
                    "*" => write_buffer(stream, b"*3\r\n$8\r\nREPLCONF\r\n$3\r\nACK\r\n$1\r\n0\r\n"),
                    _ => panic!("INVALID GETACK COMMAND {:?}", &arr[2]),
                },
                _ => panic!("INVALID VALUE {:?}", &arr[3]),
            },
            _ => panic!("INVALID REPLCONF COMMAND {:?}", &arr[3]),
        },
        _ => panic!("INVALID COMMAND STRUCTURE {:?}", &arr[2]),
    }
}

fn execute_psync(
    stream: &mut TcpStream,
    arr: &Vec<Value>,
    client: &Token,
    server_info: &mut ServerInfo,
) -> Result<(), Error> {
    match &arr[1] {
        BulkString(string) => match string.to_lowercase().as_ref() {
            "?" => match &arr[2] {
                BulkString(_x) => {
                    let response = SimpleString(format!(
                        "FULLRESYNC {} {}",
                        server_info.master_replid, server_info.master_repl_offset
                    ));
                    write_buffer(stream, &response.to_resp())?;

                    let rdb_file = RDB::new().to_binary().unwrap();
                    let response = format!("${}\r\n", &rdb_file.len());
                    write_buffer(stream, &response.as_bytes())?;
                    write_buffer(stream, &rdb_file)?;
                    server_info.replicas.insert(*client);
                    Ok(())
                }
                _ => panic!("INVALID listening port {:?}", &arr[2]),
            },
            _ => panic!("INVALID PSYNC COMMAND {:?}", &arr[3]),
        },
        _ => panic!("INVALID COMMAND STRUCTURE {:?}", &arr[2]),
    }
}
