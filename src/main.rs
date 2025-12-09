mod cache;
mod event_loop;
mod handler;
mod resp;

use cache::Cache;
use event_loop::EventLoop;
use handler::tcp_handler;
use clap::{Parser};


struct ServerInfo {
    role: String,
}
#[derive(Parser)]
struct Cli {

    #[arg(short = 'p', long = "port", default_value = "6379")]
    port: String,

    #[clap(long = "replicaof")]
    master_addr: Option<String>
}

fn main() {
    println!("Logs from your program will appear here!");
    let cli = Cli::parse();
    let role = match cli.master_addr {
        Some(_addr) => "slave".to_string(),
        None => "master".to_string()
    };

    let server_info = ServerInfo { role };

    let address = format!("127.0.0.1:{}", cli.port);
    let db = Cache::new();
    let mut event_loop = EventLoop::new(&address, server_info);

    match event_loop.run(db, tcp_handler) {
        Ok(()) => println!("The event_loop ran successfully!"),
        Err(e) => println!("Error running event_loop: {}", e),
    }
}
