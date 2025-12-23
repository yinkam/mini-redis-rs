use std::io::Write;
mod cache;
mod event_loop;
mod handler;
mod resp;

use crate::resp::value::Value::{Array, BulkString};
use cache::Cache;
use clap::Parser;
use event_loop::EventLoop;
use handler::tcp_handler;
use mio::net::TcpStream;
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

fn connect_master(server_info: &ServerInfo) {
    let host = match &server_info.master_host {
        Some(host) => host,
        None => "localhost",
    };
    let port = match &server_info.master_port {
        Some(port) => port.parse::<u16>().unwrap(),
        None => 6379,
    };
    let addr = format!("{}:{}", host, port);
    let address = addr.to_socket_addrs().unwrap().next().unwrap();
    let mut stream = TcpStream::connect(address).unwrap();

    let message = Array(vec![BulkString("PING".to_string())]);
    stream.write_all(&message.to_resp()).unwrap()
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
        connect_master(&server_info)
    }
    let address = format!("127.0.0.1:{}", cli.port);
    let db = Cache::new();
    let mut event_loop = EventLoop::new(&address, server_info);

    match event_loop.run(db, tcp_handler) {
        Ok(()) => println!("The event_loop ran successfully!"),
        Err(e) => println!("Error running event_loop: {}", e),
    }
}
