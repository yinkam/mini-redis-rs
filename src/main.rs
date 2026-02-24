use std::io::{Read, Write};
mod cache;
mod event_loop;
mod handler;
mod resp;

use cache::Cache;
use clap::Parser;
use event_loop::EventLoop;
use handler::tcp_handler;
use std::net::TcpStream;
use std::net::ToSocketAddrs;

#[derive(Debug)]
struct ServerInfo {
    role: String,
    master_host: Option<String>,
    master_port: Option<String>,
    master_replid: String,
    master_repl_offset: String,
}
#[derive(Parser)]
struct Cli {
    #[arg(short = 'p', long = "port", default_value = "6379")]
    port: String,

    #[clap(long = "replicaof")]
    master_addr: Option<String>,
}

fn connect_master(server_info: &ServerInfo) -> Result<(), std::io::Error> {
    let host = match &server_info.master_host {
        Some(host) => host,
        None => "localhost",
    };
    let port = match &server_info.master_port {
        Some(port) => port.parse::<u16>().unwrap(),
        None => 6379,
    };
    let addr = format!("{}:{}", host, port);
    let address = addr.to_socket_addrs()?.next().unwrap();
    let mut stream = TcpStream::connect(address)?;
    stream.set_nonblocking(false)?;

    // HANDSHAKE
    handshake(&mut stream)?;
    Ok(())
}

fn handshake(stream: &mut TcpStream) -> Result<(), std::io::Error> {
    stream.write_all(b"*1\r\n$4\r\nPING\r\n")?;
    let response = read_response(stream)?;
    if response != b"+PONG\r\n" {
        println!("Invalid response {:?}", response);
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Expected PONG",
        ));
    }
    stream.write_all(b"*3\r\n$8\r\nREPLCONF\r\n$14\r\nlistening-port\r\n$4\r\n6380\r\n")?;
    let response = read_response(stream)?;
    if response != b"+OK\r\n" {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "REPLCONF listening-port failed",
        ));
    }

    stream.write_all(b"*3\r\n$8\r\nREPLCONF\r\n$4\r\ncapa\r\n$6\r\npsync2\r\n")?;
    let response = read_response(stream)?;
    if response != b"+OK\r\n" {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "REPLCONF capa failed",
        ));
    }

    Ok(())
}

fn read_response(stream: &mut TcpStream) -> Result<Vec<u8>, std::io::Error> {
    let mut buffer = [0; 512];
    println!("READ RESPONSE: {:?}", buffer);
    let n = stream.read(&mut buffer)?;
    Ok(buffer[..n].to_vec())
}

fn main() {
    println!("Logs from your program will appear here!");
    let cli = Cli::parse();
    let mut master_host = None;
    let mut master_port = None;
    let role = match cli.master_addr {
        Some(addr) => {
            let master_addr = addr.split(" ").collect::<Vec<&str>>();
            master_host = Some(master_addr[0].to_string());
            master_port = Some(master_addr[1].to_string());
            "slave".to_string()
        }
        None => "master".to_string(),
    };
    let master_replid = "8371b4fb1155b71f4a04d3e1bc3e18c4a990aeeb".to_string();
    let master_repl_offset = "0".to_string();

    let server_info = ServerInfo {
        role,
        master_host,
        master_port,
        master_replid,
        master_repl_offset,
    };

    if server_info.role == "slave" {
        match connect_master(&server_info) {
            Ok(()) => println!("Successfully connected to master server"),
            Err(e) => {
                println!("Failed to connect to master: {}", e);
            }
        }
    }
    let address = format!("127.0.0.1:{}", cli.port);
    let db = Cache::new();
    let mut event_loop = EventLoop::new(&address, server_info);

    match event_loop.run(db, tcp_handler) {
        Ok(()) => println!("The event_loop ran successfully!"),
        Err(e) => println!("Error running event_loop: {}", e),
    }
}
