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
use std::net::SocketAddr;
use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;

use std::env;
use log::{LogRecord, LogLevelFilter};
use env_logger::LogBuilder;
use std::io::Read;
use proxy::Proxy;
use connection::Connection;
use connection::tcp_connection::TcpConnection;
use connection::poison::DropAllConnection;

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
    connections: HashMap<Token, Rc<RefCell<Proxy>>>,
    tokens: BitSet,
}

impl MyHandler {
    pub fn new() -> Self {
        MyHandler {
            listeners: Vec::new(),
            tokens: BitSet::with_capacity(4096), // TODO: Read from EventLoop configuration
            connections: HashMap::new(),
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

    pub fn proxy(&self, input: &str, stream: TcpStream) -> (Box<Connection>, Box<Connection>) {
        let addr: SocketAddr = input.parse().unwrap();

        let downstream = TcpConnection::new(1024, stream, Token(1));
        let upstream = TcpConnection::new(1024, TcpStream::connect(&addr).unwrap(), Token(2));

        (Box::new(downstream), Box::new(upstream))
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

                let (downstream, upstream) = self.proxy("127.0.0.1:8001", tcp_stream);
                let mut proxy = TcpProxy::new(downstream, upstream/*Box::new(DropAllConnection::new(upstream))*/);

                event_loop.register(proxy.get_downstream().get_evented(), read_token, EventSet::readable(), PollOpt::edge());
                event_loop.register(proxy.get_upstream().get_evented(), write_token, EventSet::writable(), PollOpt::edge());

                let bp = Rc::new(RefCell::new(proxy));
                self.connections.insert(Token(1), bp.clone());
                self.connections.insert(Token(2), bp.clone());
            },
            Token(1) => {
                info!("Read event!");
                let mut buffer = [0; 1024];

                let mut proxy = self.connections[&token].borrow_mut();
                let amount = proxy.get_downstream().read(&mut buffer[..]).unwrap();
                info!("Amount of bytes: {}", amount);
                proxy.get_upstream().write(&buffer).unwrap();

                event_loop.register(proxy.get_downstream().get_evented(), token, EventSet::readable(), PollOpt::edge());
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
