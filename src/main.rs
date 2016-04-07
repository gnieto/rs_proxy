extern crate mio;
extern crate bit_set;
extern crate bytes;
#[macro_use]
extern crate log;
extern crate env_logger;

pub mod proxy;
pub mod connection;

use mio::*;
use mio::tcp::{TcpListener, TcpStream};
use std::thread;
use bit_set::BitSet;
use std::net::SocketAddr;
use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;

use std::env;
use log::{LogRecord, LogLevelFilter};
use env_logger::LogBuilder;
use proxy::Proxy;
use proxy::ProxyLocator;
use connection::{Connection, Role, BufferState};
use connection::tcp_connection::TcpConnection;

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
    proxy_locator: ProxyLocator,
    acceptors: HashMap<Token, TcpListener>,
    tokens: BitSet,
}

impl MyHandler {
    pub fn new() -> Self {
        MyHandler {
            tokens: BitSet::with_capacity(4096), // TODO: Read from EventLoop configuration
            proxy_locator: ProxyLocator::new(),
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

    pub fn proxy(&mut self, input: &str, stream: TcpStream) -> (Rc<RefCell<Connection>>, Rc<RefCell<Connection>>) {
        let addr: SocketAddr = input.parse().unwrap();

        let downstream_token = self.claim_token().unwrap();
        let upstream_token = self.claim_token().unwrap();

        let downstream = TcpConnection::new(1024, stream, downstream_token);
        let upstream = TcpConnection::new(1024, TcpStream::connect(&addr).unwrap(), upstream_token);

        (Rc::new(RefCell::new(downstream)), Rc::new(RefCell::new(upstream)))
    }

    pub fn handle_accept(&mut self, event_loop: &mut EventLoop<MyHandler>, token: Token) -> Option<(Token, Token)>{
        if !self.acceptors.contains_key(&token) {
            return None
        }

        info!("Inbound connection with token {:?}!", token);

        let tcp_stream = {
            let acceptor = self.acceptors.get_mut(&token).unwrap();
            let (tcp_stream, _) = acceptor.accept().unwrap().unwrap();
            tcp_stream
        };

        let (downstream, upstream) = self.proxy("127.0.0.1:8001", tcp_stream);
        let proxy = Proxy::new(downstream, upstream);
        let (downstream_token, upstream_token) = proxy.tokens();

        let ds = proxy.get_downstream();
        let us = proxy.get_upstream();

        event_loop.register(ds.borrow().get_evented(), downstream_token, EventSet::readable() | EventSet::hup() | EventSet::error(), PollOpt::edge()).unwrap();
        event_loop.register(us.borrow().get_evented(), upstream_token, EventSet::readable() | EventSet::hup() | EventSet::error(), PollOpt::edge()).unwrap();

        let bp = Rc::new(RefCell::new(proxy));
        self.proxy_locator.link(downstream_token, Role::Downstream, bp.clone());
        self.proxy_locator.link(upstream_token, Role::Upstream, bp.clone());

        info!("Registered downstream_connection {:?} and upstream_connection {:?}", downstream_token, upstream_token);

        Some((downstream_token, upstream_token))
    }

    pub fn handle_connection(&mut self, event_loop: &mut EventLoop<MyHandler>, token: Token, event_set: EventSet) {
        // info!("Handle connection with token {:?} with events {:?}", token, event_set);

        if event_set.is_writable() {
            let (role, ref_proxy) = self.proxy_locator.get(&token).unwrap();
            // info!("Handling writting on token {:?} with role {:?}", token, role);

            let proxy = ref_proxy.borrow();
            let ds = proxy.get_downstream();
            let us = proxy.get_upstream();

            let (read_borrow, mut write_borrow) = match role {
                Role::Downstream => {
                    let us_borrow = us.borrow();
                    let ds_borrow = ds.borrow_mut();

                    (us_borrow, ds_borrow)
                },
                Role::Upstream => {
                    let ds_borrow = ds.borrow();
                    let us_borrow = us.borrow_mut();

                    (ds_borrow, us_borrow)
                },
            };

            let buffer = read_borrow.get_buffer();
            write_borrow.handle_write(&buffer);

            let has_to_close = proxy.is_upstream_closed();

            if !has_to_close {
                event_loop.reregister(write_borrow.get_evented(), write_borrow.get_token(), EventSet::readable() | EventSet::hup() | EventSet::error(), PollOpt::edge()).unwrap();
            }

            self.handle_downstream_close(event_loop, &token);
        }

        if event_set.is_readable() {
            // 1 - Check if there's something to read
                // Something to read
                    // Read it
                    // Put the other connection on write interest
                    // Write and if it has not more data to write, remove the interest
                // Nothing to read
                    // Skip to the next token

            // 1- Recover connection

            let (role, ref_proxy) = self.proxy_locator.get(&token).unwrap();
            // info!("Handling token {:?} with role {:?}", token, role);

            let proxy = ref_proxy.borrow_mut();
            let ds = proxy.get_downstream();
            let us = proxy.get_upstream();

            let (mut read_borrow, write_borrow) = match role {
                Role::Downstream => {
                    // info!("Handle downstream. EventSet: {:?}", event_set);
                    (ds.borrow_mut(), us.borrow())
                },
                Role::Upstream => {
                    // info!("Handle upstream. EventSet: {:?}", event_set);
                    (us.borrow_mut(), ds.borrow())
                },
            };

            let bs = read_borrow.handle_read();
            match bs {
                BufferState::Full => {
                    event_loop.reregister(read_borrow.get_evented(), read_borrow.get_token(), EventSet::writable() | EventSet::hup() | EventSet::error(), PollOpt::edge()).unwrap();
                },
                _ => ()
            }

            // Add writable behaviour
            event_loop.reregister(write_borrow.get_evented(), write_borrow.get_token(), EventSet::readable() | EventSet::writable() | EventSet::hup() | EventSet::error(), PollOpt::edge()).unwrap();
        }

        if event_set.is_hup() || event_set.is_error() {
            let (role, ref_proxy) = self.proxy_locator.get(&token).unwrap();
            match role {
                Role::Upstream => {
                    // info!("Marked upstream connection with token {:?} as closed", token);
                    ref_proxy.borrow_mut().upstream_closed();
                },
                _ => ()
            }
        }
    }

    fn handle_downstream_close(&mut self, event_loop: &mut EventLoop<MyHandler>, token: &Token) {
        let (role, ref_proxy) = self.proxy_locator.get(&token).unwrap();

        match role {
            Role::Downstream => {
                let proxy = ref_proxy.borrow();
                if proxy.is_upstream_closed() {
                    self.remove_proxy(event_loop, token)
                }
            },
            _ => (),
        };
    }

    fn remove_proxy(&mut self, event_loop: &mut EventLoop<MyHandler>, token: &Token) {
        let tokens = {
            match self.proxy_locator.get(token)
            {
                Some((_, ref_proxy)) => {
                    let proxy = ref_proxy.borrow();
                    let ds = proxy.get_downstream();
                    event_loop.deregister(ds.borrow().get_evented()).unwrap();
                    let us = proxy.get_upstream();
                    event_loop.deregister(us.borrow().get_evented()).unwrap();

                    Some(proxy.tokens())
                },
                None => {
                    None
                }
            }
        };

        match tokens {
            Some((ds_token, us_token)) => {
                // info!("Cleaning connections with tokens {:?} and {:?} has been removed", ds_token, us_token);

                self.proxy_locator.unlink(&ds_token);
                self.proxy_locator.unlink(&us_token);

                self.return_token(ds_token);
                self.return_token(us_token);
            },
            None => {()}
        }
    }
}

impl Handler for MyHandler {
    type Timeout = ();
    type Message = u32;

    fn ready(&mut self, event_loop: &mut EventLoop<MyHandler>, token: Token, event_set: EventSet) {
        if self.proxy_locator.has(&token) {
            self.handle_connection(event_loop, token, event_set);
        } else {
            self.handle_accept(event_loop, token);
        }
    }

    fn notify(&mut self, event_loop: &mut EventLoop<MyHandler>, _: u32) {
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
