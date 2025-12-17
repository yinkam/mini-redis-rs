use crate::cache::Cache;
use crate::ServerInfo;
use mio::net::{TcpListener, TcpStream};
use mio::{Events, Interest, Poll, Token};
use std::collections::HashMap;
use std::io::Error;
use std::io::ErrorKind::WouldBlock;
use std::net::SocketAddr;
use std::str::FromStr;

pub struct EventLoop {
    listener: TcpListener,
    poll: Poll,
    events: Events,
    connections: HashMap<Token, TcpStream>,
    server_info: ServerInfo,
}

impl EventLoop {
    pub fn new(address: &str, server_info: ServerInfo) -> Self {
        let address = SocketAddr::from_str(address).unwrap();
        let listener = TcpListener::bind(address).unwrap();
        let poll = match Poll::new() {
            Ok(poll) => poll,
            Err(e) => panic!("Unable to create poll listener: {:?}", e),
        };

        let events = Events::with_capacity(1024);
        let connections = HashMap::new();

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
        F: Fn(&TcpStream, &mut Cache, &ServerInfo),
    {
        const SERVER: Token = Token(0);
        let mut next_token = 1;

        self.poll
            .registry()
            .register(&mut self.listener, SERVER, Interest::READABLE)?;

        loop {
            self.poll.poll(&mut self.events, None)?;

            for event in self.events.iter() {
                match event.token() {
                    SERVER => loop {
                        match self.listener.accept() {
                            Ok((mut socket, addr)) => {
                                println!("new connection from {}", addr);
                                let client = Token(next_token);
                                next_token += 1;

                                self.poll.registry().register(
                                    &mut socket,
                                    client,
                                    Interest::READABLE | Interest::WRITABLE,
                                )?;
                                self.connections.insert(client, socket);
                            }
                            Err(ref err) if err.kind() == WouldBlock => break,
                            Err(err) => return Err(err),
                        }
                    },
                    client => {
                        let socket = match self.connections.get_mut(&client) {
                            Some(socket) => socket,
                            None => continue,
                        };

                        handler(socket, &mut db, &self.server_info);
                    }
                }
            }
        }
    }
}
