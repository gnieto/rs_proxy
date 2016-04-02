extern crate mio;
extern crate bit_set;
#[macro_use]
extern crate log;
extern crate env_logger;

pub mod proxy;
pub mod connection;

use mio::*;
use mio::tcp::{TcpListener, TcpStream};
use std::thread;
use std::sync::mpsc::channel;
use std::sync::mpsc::{Receiver};
use proxy::tcp::TcpProxy;
use bit_set::BitSet;

use std::env;
use log::{LogRecord, LogLevelFilter};
use env_logger::LogBuilder;
use std::io::Read;
use proxy::Proxy;

fn main() {
    initialize_logger();

    let mut event_loop = EventLoop::new().unwrap();
    let sender = event_loop.channel();

    // Send the notification from another thread
    thread::spawn(move || {
        let _ = sender.send(123);
    });

    let mut handler = MyHandler::new();
    let _ = event_loop.run(&mut handler);
}

struct MyHandler {
    listeners: Vec<TcpListener>,
    connections: Vec<Box<Proxy>>,
    tokens: BitSet,
}

impl MyHandler {
    pub fn new() -> Self {
        MyHandler {
            listeners: Vec::new(),
            connections: Vec::new(),
            tokens: BitSet::with_capacity(4096), // TODO: Read from EventLoop configuration
        }
    }

    pub fn claim_token(&mut self) -> Option<Token> {
        let mut i = 0;

        while self.tokens.contains(i) {
            i = i + 1;
        }

        if i < 4096 {
            self.tokens.insert(i);
            Some(Token(i))
        } else {
            None
        }
    }

    pub fn return_token(&mut self, token: Token) {
        self.tokens.remove(token.as_usize());
    }
}

impl Handler for MyHandler {
    type Timeout = ();
    type Message = u32;

    fn ready(&mut self, event_loop: &mut EventLoop<MyHandler>, token: Token, _: EventSet) {
        match token {
            Token(0) => {
                info!("Inbound connection!");
                let read_token = self.claim_token().unwrap();
                let write_token = self.claim_token().unwrap();

                let ref acceptor = self.listeners[0];
                let (tcp_stream, _) = acceptor.accept().unwrap().unwrap();

                let proxy = TcpProxy::new("127.0.0.1:8001", tcp_stream);
                self.connections.push(Box::new(proxy));

                event_loop.register(self.connections[0].get_downstream().get_evented(), read_token, EventSet::readable(), PollOpt::edge());
                event_loop.register(self.connections[0].get_upstream().get_evented(), write_token, EventSet::writable(), PollOpt::edge());
            },
            Token(1) => {
                info!("Read event!");
                let mut buffer = [0; 1024];
                // let amount = self.connections[0].input_read().read(&mut buffer[..]).unwrap();
                let amount = self.connections[0].get_downstream().read(&mut buffer[..]).unwrap();
                info!("Amount of bytes: {}", amount);

                // info!("Read: {}", buffer);
                self.connections[0].get_upstream().write(&buffer).unwrap();
            },
            Token(2) => {
                info!("Socket has some event!");
            }
            _ => {
                println!("Unknown token");
            }
        }
    }

    fn notify(&mut self, event_loop: &mut EventLoop<MyHandler>, msg: u32) {
        let addr = "127.0.0.1:8000".parse().unwrap();
        let server = TcpListener::bind(&addr).unwrap();
        let token = self.claim_token().unwrap();

        info!("Open listener at port 8000 with token {}", token.as_usize());

        event_loop.register(
            &server,
            token,
            EventSet::readable(),
            PollOpt::edge()
        ).unwrap();

        self.listeners.push(server);
    }
}


fn initialize_logger() {
	let format = |record: &LogRecord| {
        format!("{} - {}:{} - {}", record.level(), record.location().file(), record.location().line(), record.args())
    };

    let mut builder = LogBuilder::new();
    builder.format(format).filter(None, LogLevelFilter::Info);

    if env::var("RUST_LOG").is_ok() {
       builder.parse(&env::var("RUST_LOG").unwrap());
    }

    builder.init().unwrap();

}
