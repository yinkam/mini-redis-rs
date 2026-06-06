use std::collections::HashMap;

mod cache;
mod event_loop;
mod handler;
mod resp;
mod replication;
mod persistence;

use cache::Cache;
use clap::Parser;
use event_loop::EventLoop;
use handler::tcp_handler;
use mio::Token;
use std::net::TcpStream;
use std::net::ToSocketAddrs;
use std::time::{Duration, Instant};
use crate::replication::handshake::handshake;

#[derive(Debug)]
struct Config {
    dir: String,
    dbfilename: String,
}

#[derive(Debug, Clone)]
struct WaitState {
    client: Token,
    min_replicas: usize,
    timeout: Duration,
    start_time: Instant,
    acks_received: usize,
}
#[derive(Debug)]
struct ServerInfo {
    role: String,
    master_host: Option<String>,
    master_port: Option<String>,
    master_replid: String,
    master_repl_offset: usize,
    replicas: HashMap<Token, usize>,
    waiting: Option<WaitState>,
    config: Config,
}
#[derive(Parser)]
struct Cli {
    #[arg(short = 'p', long = "port", default_value = "6379")]
    port: String,

    #[clap(long = "replicaof")]
    master_addr: Option<String>,

    #[arg(long = "dir", default_value = "/tmp/redis")]
    dir: String,

    #[arg(long = "dbfilename", default_value = "dump.rdb")]
    dbfilename: String,
}

fn connect_master(server_info: &ServerInfo) -> Result<TcpStream, std::io::Error> {
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
    let mut stream: TcpStream = TcpStream::connect(address)?;

    stream.set_nonblocking(false)?;
    stream = handshake(stream)?;
    stream.set_nonblocking(true)?;

    Ok(stream)
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
    let master_repl_offset = 0usize;
    let config = Config {
        dir: cli.dir,
        dbfilename: cli.dbfilename,
    };

    let server_info = ServerInfo {
        role,
        master_host,
        master_port,
        master_replid,
        master_repl_offset,
        replicas: HashMap::new(),
        waiting: None,
        config,
    };

    let master_connection = match server_info.role.as_str() {
        "slave" => match connect_master(&server_info) {
            Ok(conn) => Some(conn),
            Err(_e) => None,
        },
        _ => None,
    };

    let address = format!("127.0.0.1:{}", cli.port);
    let db = Cache::new();
    let connections = match master_connection {
        Some(conn) => HashMap::from([(Token(1), mio::net::TcpStream::from_std(conn))]),
        None => HashMap::new(),
    };
    let mut event_loop = EventLoop::new(&address, server_info, connections);

    match event_loop.run(db, tcp_handler) {
        Ok(()) => println!("The event_loop ran successfully!"),
        Err(e) => println!("Error running event_loop: {}", e),
    }
}
