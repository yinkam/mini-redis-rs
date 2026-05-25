use crate::cache::Cache;
use crate::resp::value::Value::Integer;
use crate::{ServerInfo, WaitState};
use mio::net::{TcpListener, TcpStream};
use mio::{Events, Interest, Poll, Token};
use std::collections::HashMap;
use std::io::ErrorKind::WouldBlock;
use std::io::{Error, Write};
use std::net::SocketAddr;
use std::str::FromStr;
use std::time::Duration;

#[derive(Debug)]
pub struct EventLoop {
    listener: TcpListener,
    poll: Poll,
    events: Events,
    pub(crate) connections: HashMap<Token, TcpStream>,
    server_info: ServerInfo,
}

impl EventLoop {
    pub fn new(
        address: &str,
        server_info: ServerInfo,
        mut connections: HashMap<Token, TcpStream>,
    ) -> Self {
        let address = SocketAddr::from_str(address).unwrap();
        let listener = TcpListener::bind(address).unwrap();
        let poll = match Poll::new() {
            Ok(poll) => poll,
            Err(e) => panic!("Unable to create poll listener: {:?}", e),
        };

        let events = Events::with_capacity(1024);
        for (client, conn) in &mut connections {
            poll.registry()
                .register(conn, *client, Interest::READABLE | Interest::WRITABLE)
                .expect("Connection registration failed");
        }

        Self {
            listener,
            poll,
            events,
            connections,
            server_info,
        }
    }

    pub fn run<F>(&mut self, mut db: Cache, handler: F) -> Result<(), Error>
    where
        F: Fn(&mut Cache, &Token, &mut HashMap<Token, TcpStream>, &mut ServerInfo),
    {
        const SERVER: Token = Token(0);
        let mut next_token = if self.connections.is_empty() { 1 } else { 2 };

        self.poll
            .registry()
            .register(&mut self.listener, SERVER, Interest::READABLE)?;

        loop {
            let timeout = self.get_timeout();

            self.poll.poll(&mut self.events, timeout)?;

            if let Some(x) = &self.server_info.waiting.clone() {
                self.send_wait_timeout(x)
            }

            for event in self.events.iter() {
                match event.token() {
                    SERVER => loop {
                        match self.listener.accept() {
                            Ok((mut conn, addr)) => {
                                println!("new connection from {}", addr);
                                let client = Token(next_token);
                                next_token += 1;

                                self.poll.registry().register(
                                    &mut conn,
                                    client,
                                    Interest::READABLE | Interest::WRITABLE,
                                )?;
                                self.connections.insert(client, conn);
                            }
                            Err(ref err) if err.kind() == WouldBlock => break,
                            Err(err) => return Err(err),
                        }
                    },
                    client => {
                        match self.connections.get_mut(&client) {
                            Some(conn) => conn,
                            None => continue,
                        };
                        handler(
                            &mut db,
                            &client,
                            &mut self.connections,
                            &mut self.server_info,
                        );
                    }
                }
            }
        }
    }

    fn get_timeout(&self) -> Option<Duration> {
        match &self.server_info.waiting {
            Some(state) => {
                let elapsed = state.start_time.elapsed();
                if elapsed >= state.timeout {
                    Some(Duration::from_millis(0))
                } else {
                    Some(state.timeout - elapsed)
                }
            }
            None => None,
        }
    }

    fn send_wait_timeout(&mut self, state: &WaitState) {
        if state.start_time.elapsed() > state.timeout {
            let response = Integer(state.acks_received as i64);
            let client = &mut self.connections.get_mut(&state.client).unwrap();
            client
                .write_all(&response.to_resp().to_vec())
                .expect("Error: could not respond to WAIT after timeout");
            self.server_info.waiting = None;
        }
    }
}
