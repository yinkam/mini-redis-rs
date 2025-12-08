mod cache;
mod event_loop;
mod handler;
mod resp;

use cache::Cache;
use event_loop::EventLoop;
use handler::tcp_handler;

fn main() {
    println!("Logs from your program will appear here!");

    let address = "127.0.0.1:6379";
    let db = Cache::new();
    let mut event_loop = EventLoop::new(address);

    match event_loop.run(db, tcp_handler) {
        Ok(()) => println!("The event_loop ran successfully!"),
        Err(e) => println!("Error running event_loop: {}", e),
    }
}
