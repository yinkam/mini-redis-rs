mod cache;
mod event_loop;
mod handler;
mod resp;

use cache::Cache;
use clap::Parser;
use event_loop::EventLoop;
use handler::tcp_handler;

struct ServerInfo {
    role: String,
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

fn main() {
    println!("Logs from your program will appear here!");
    let cli = Cli::parse();
    let role = match cli.master_addr {
        Some(_addr) => "slave".to_string(),
        None => "master".to_string(),
    };
    let master_replid = "8371b4fb1155b71f4a04d3e1bc3e18c4a990aeeb".to_string();
    let master_repl_offset = "0".to_string();

    let server_info = ServerInfo {
        role,
        master_replid,
        master_repl_offset,
    };

    let address = format!("127.0.0.1:{}", cli.port);
    let db = Cache::new();
    let mut event_loop = EventLoop::new(&address, server_info);

    match event_loop.run(db, tcp_handler) {
        Ok(()) => println!("The event_loop ran successfully!"),
        Err(e) => println!("Error running event_loop: {}", e),
    }
}
