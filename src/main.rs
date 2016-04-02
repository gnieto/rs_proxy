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
use connection::{Connection, Role};
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
    connections: HashMap<Token, (Role, Rc<RefCell<Proxy>>)>,
    acceptors: HashMap<Token, TcpListener>,
    tokens: BitSet,
}

impl MyHandler {
    pub fn new() -> Self {
        MyHandler {
            tokens: BitSet::with_capacity(4096), // TODO: Read from EventLoop configuration
            connections: HashMap::new(),
            acceptors: HashMap::new(),
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

    pub fn handle_accept(&mut self, event_loop: &mut EventLoop<MyHandler>, token: Token) -> Option<(Token, Token)>{
        if !self.acceptors.contains_key(&token) {
            return None
        }

        info!("Inbound connection with token {:?}!", token);

        let downstream_token = self.claim_token().unwrap();
        let upstream_token = self.claim_token().unwrap();

        let tcp_stream = {
            let acceptor = self.acceptors.get_mut(&token).unwrap();
            let (tcp_stream, _) = acceptor.accept().unwrap().unwrap();
            tcp_stream
        };

        let (downstream, upstream) = self.proxy("127.0.0.1:8001", tcp_stream);
        let mut proxy = TcpProxy::new(downstream, upstream);

        event_loop.register(proxy.get_downstream().get_evented(), downstream_token, EventSet::readable() | EventSet::hup() | EventSet::error(), PollOpt::edge());
        event_loop.register(proxy.get_upstream().get_evented(), upstream_token, EventSet::writable() | EventSet::hup() | EventSet::error(), PollOpt::edge());

        let bp = Rc::new(RefCell::new(proxy));
        self.connections.insert(downstream_token, (Role::Downstream, bp.clone()));
        self.connections.insert(upstream_token, (Role::Upstream, bp.clone()));

        info!("Registered downstream_connection {:?} and upstream_connection {:?}", downstream_token, upstream_token);

        Some((downstream_token, upstream_token))
    }

    pub fn handle_connection(&mut self, event_loop: &mut EventLoop<MyHandler>, token: Token, event_set: EventSet) {
        info!("Handle connection with token {:?}", token);

        if event_set.is_hup() || event_set.is_error() {
            let tokens = {
                match self.connections.get_mut(&token) {
                    Some(&mut(ref role, ref mut ref_proxy)) => {
                        Some(ref_proxy.borrow().tokens())
                    },
                    None => {
                        None
                    }
                }
            };

            match tokens {
                Some((ds_token, us_token)) => {
                    info!("Downstream connection has hung up. Freeing tokens {:?} and {:?}", ds_token, us_token);
                    self.connections.remove(&ds_token);
                    self.connections.remove(&us_token);

                    self.return_token(ds_token);
                    self.return_token(us_token);
                },
                None => {()}
            }
        } else {
            match self.connections.get_mut(&token) {
                None => (),
                Some(&mut(ref role, ref mut ref_proxy)) => {
                    match role {
                        &Role::Downstream => {
                            // Handle downstream connection
                            info!("Handle downstream. EventSet: {:?}", event_set);

                            if event_set.is_readable() {
                                let mut buffer = [0; 4];
                                let mut proxy = ref_proxy.borrow_mut();

                                loop {
                                    let read_response = proxy.get_mut_downstream().read(&mut buffer[..]);

                                    let amount = match read_response {
                                        Err(_) => 0,
                                        Ok(amount) => amount,
                                    };

                                    if amount == 0 {
                                        break;
                                    }

                                    info!("Amount of bytes: {}", amount);
                                    proxy.get_mut_upstream().write(&buffer).unwrap();
                                }

                                info!("Reregistering");
                                event_loop.register(proxy.get_downstream().get_evented(), token, EventSet::readable(), PollOpt::edge());
                            }
                        },
                        &Role::Upstream => {
                            // Handle upstream connection
                            info!("Handle upstream. EventSet: {:?}", event_set)
                        }
                    };
                    ()
                }
            }
        }
    }
}

impl Handler for MyHandler {
    type Timeout = ();
    type Message = u32;

    fn ready(&mut self, event_loop: &mut EventLoop<MyHandler>, token: Token, event_set: EventSet) {
        if self.connections.contains_key(&token) {
            self.handle_connection(event_loop, token, event_set);
        } else {
            self.handle_accept(event_loop, token);
        }

        /*match self.connections.contains_key(&token) {
            false => {

            },
            true => {

            }
            Some(&mut (ref role, ref mut connection)) => {
                match role {
                    &Role::Downstream => {
                        info!("Read event!");
                        let mut buffer = [0; 4];
                        let mut proxy = connection.borrow_mut();

                        loop {
                            let read_response = proxy.get_downstream().read(&mut buffer[..]);

                            let amount = match read_response {
                                Err(_) => 0,
                                Ok(amount) => amount,
                            };

                            if amount == 0 {
                                break;
                            }

                            info!("Amount of bytes: {}", amount);
                            proxy.get_upstream().write(&buffer).unwrap();
                        }

                        event_loop.register(proxy.get_downstream().get_evented(), token, EventSet::readable(), PollOpt::edge());
                    },
                    &Role::Upstream => {
                        info!("Socket has some event!");
                    }
                }
            }
        }*/
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

        self.acceptors.insert(token, server);
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
