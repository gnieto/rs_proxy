extern crate mio;
extern crate bit_set;
extern crate netbuf;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate ansi_term;
// #[cfg(redis)]
extern crate resp;

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
use ansi_term::Colour::{ Red, Green, Yellow, Blue, Purple};
use ansi_term::Style;
use std::env;
use log::{LogRecord, LogLevelFilter, LogLevel};
use env_logger::LogBuilder;
use proxy::Proxy;
use proxy::ProxyLocator;
use connection::{Connection, Role, ConnectionAction, Timer, TimerAction};
use connection::tcp_connection::TcpConnection;
use connection::poison::Throttler;
// #[cfg(redis)]
use connection::redis::RedisConnection;
use connection::redis::{ComposedProxy, LogProxy, PrefixProxy};

fn main() {
    initialize_logger();

    let mut event_loop = EventLoop::new().expect("Could not initialize the event_loop");
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
    timers: HashMap<Token, Rc<RefCell<Timer>>>,
    tokens: BitSet,
}

impl MyHandler {
    pub fn new() -> Self {
        MyHandler {
            tokens: BitSet::with_capacity(4096), // TODO: Read from EventLoop configuration
            proxy_locator: ProxyLocator::new(),
            acceptors: HashMap::new(),
            timers: HashMap::new(),
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


    pub fn handle_accept(&mut self, event_loop: &mut EventLoop<MyHandler>, token: Token) -> Result<(Token, Token), &str> {
        info!("Inbound connection with token {:?}!", token);

        let tcp_stream = {
            let acceptor = try!{self.acceptors.get_mut(&token).ok_or("Called handle accept on a non-accept token")};
            let accept_result = try!{acceptor.accept().or(Err("Could not accept"))};
            let (tcp_stream, _) = try!{accept_result.ok_or("Could not get a tcp stream")};
            tcp_stream
        };

        // let (downstream, upstream) = self.proxy("127.0.0.1:6379", tcp_stream).unwrap();
        let addr: SocketAddr = try!("127.0.0.1:6379".parse().or(Err("Could not parse the listening address")));
        let downstream_token = try!(self.claim_token().ok_or("No more tokens available for downstream"));
        let upstream_token = try!(self.claim_token().ok_or("No more tokens available for upstream"));
        let downstream = TcpConnection::new(tcp_stream, downstream_token);
        let stream = try!(TcpStream::connect(&addr).or(Err("Could not connect to upstream")));
        let upstream = TcpConnection::new(stream, upstream_token);
        let log = ComposedProxy::new(LogProxy, PrefixProxy);
        let downstream = RedisConnection::new(downstream, log);
        // let downstream = RedisPrefixConnection::new(downstream);

        let downstream = Rc::new(RefCell::new(downstream));
        let upstream = Rc::new(RefCell::new(upstream));

        let proxy = Proxy::new(downstream, upstream.clone());
        let (downstream_token, upstream_token) = proxy.tokens();

        let ds = proxy.get_downstream();
        let us = proxy.get_upstream();

        event_loop.register(ds.borrow().get_evented(), downstream_token, ds.borrow().get_interest(), PollOpt::edge()).unwrap();
        event_loop.register(us.borrow().get_evented(), upstream_token, us.borrow().get_interest(), PollOpt::edge()).unwrap();

        let bp = Rc::new(RefCell::new(proxy));
        self.proxy_locator.link(downstream_token, Role::Downstream, bp.clone());
        self.proxy_locator.link(upstream_token, Role::Upstream, bp.clone());

        info!("Registered downstream_connection {:?} and upstream_connection {:?}", downstream_token, upstream_token);

        Ok((downstream_token, upstream_token))
    }

    pub fn handle_connection(&mut self, event_loop: &mut EventLoop<MyHandler>, token: Token, event_set: EventSet) -> Result<(), &str> {
        if event_set.is_writable() {
            let has_to_close = {
                let (role, ref_proxy) = try!{self.proxy_locator.get(&token).ok_or("Token not found")};

                let proxy = ref_proxy.borrow_mut();
                let ds = proxy.get_downstream();
                let us = proxy.get_upstream();

                let (_, mut write_borrow) = match role {
                    Role::Upstream => {
                        (ds.borrow(), us.borrow_mut())
                    },
                    Role::Downstream => {
                        (us.borrow(), ds.borrow_mut())
                    },
                };

                write_borrow.handle_write();

                let has_to_close = proxy.is_upstream_closed();

                if !has_to_close {
                    try!{event_loop.reregister(write_borrow.get_evented(), write_borrow.get_token(), EventSet::readable() | EventSet::hup() | EventSet::error(), PollOpt::edge()).or(Err("Could not reregister the token"))};
                }

                has_to_close
            };

            info!("Has to close: {}", has_to_close);
            if has_to_close {
                match self.handle_downstream_close(event_loop, &token) {
                    Err(e) => error!("Error closing connection with error: {}", e),
                    _ => (),
                }
            }
        }

        if event_set.is_readable() {
            info!("Token {:?} is readable", token);
            let (role, ref_proxy) = try!(self.proxy_locator.get(&token).ok_or("Token not found"));

            let mut proxy = ref_proxy.borrow_mut();
            let ds = proxy.get_downstream();
            let us = proxy.get_upstream();

            let (mut read_borrow, write_borrow) = match role {
                Role::Downstream => {
                    (ds.borrow_mut(), us.borrow())
                },
                Role::Upstream => {
                    (us.borrow_mut(), ds.borrow())
                },
            };

            let action = read_borrow.handle_read();

            // Add writable behaviour
            try!{event_loop.reregister(write_borrow.get_evented(), write_borrow.get_token(), EventSet::readable() | EventSet::writable() | EventSet::hup() | EventSet::error(), PollOpt::edge()).or(Err("Could not reregister the token"))};

            drop(read_borrow);
            drop(write_borrow);

            match action {
                ConnectionAction::Forward => {
                    proxy.forward(role);
                },
                _ => {
                    ()
                }
            }
        }

        if event_set.is_hup() || event_set.is_error() {
            let (role, ref_proxy) = try!{self.proxy_locator.get(&token).ok_or("Token not found")};
            match role {
                Role::Upstream => {
                    info!("Upstream cloased!");
                    ref_proxy.borrow_mut().upstream_closed();
                },
                _ => {
                    info!("Downstream closed!");
                    self.remove_proxy(event_loop, &token);
                }
            }
        };

        Ok(())
    }

    fn handle_downstream_close(&mut self, event_loop: &mut EventLoop<MyHandler>, token: &Token) -> Result<(), &str> {
        let (role, ref_proxy) = try!{self.proxy_locator.get(&token).ok_or("Token not found")};

        match role {
            Role::Downstream => {
                let proxy = ref_proxy.borrow();
                if proxy.is_upstream_closed() {
                    self.remove_proxy(event_loop, token)
                }
            },
            _ => (),
        };

        Ok(())
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
                self.proxy_locator.unlink(&ds_token);
                self.proxy_locator.unlink(&us_token);

                self.return_token(ds_token);
                self.return_token(us_token);
            },
            None => {()}
        }
    }

    pub fn handle_timer(&mut self, event_loop: &mut EventLoop<MyHandler>, token: Token) -> Result<(), &str> {
        match self.timers.get(&token) {
            Some(ref timer) => {
                let mut timer = timer.borrow_mut();
                match timer.handle_timer() {
                    TimerAction::Continue => {
                        event_loop.timeout_ms(token, timer.get_frequency());
                    },
                    _ => (),
                }
            },
            None => {
                info!("Tick on an unexisting timer");
            }
        };

        try!{self.register_token(event_loop, token)};
        Ok(())
    }

    fn register_token(&mut self, event_loop: &mut EventLoop<MyHandler>, token: Token) -> Result<(), &str> {
        match self.proxy_locator.get(&token)
        {
            Some((_, ref_proxy)) => {
                let proxy = ref_proxy.borrow();
                let connection = try!{proxy.get_from_token(token).ok_or("Token not found on proxy")};

                info!("Reregistering token {:?} with interest: {:?}", token, connection.borrow().get_interest());
                event_loop.register(connection.borrow().get_evented(), token, connection.borrow().get_interest(), PollOpt::edge());
            },
            None => {
                ()
            }
        };

        Ok(())
    }
}

impl Handler for MyHandler {
    type Timeout = Token;
    type Message = u32;

    fn ready(&mut self, event_loop: &mut EventLoop<MyHandler>, token: Token, event_set: EventSet) {
        if self.proxy_locator.has(&token) {
            let handle_result = self.handle_connection(event_loop, token, event_set);
            match handle_result {
                Err(e) => {
                    error!("Error found handling connection {:?} with reason: {}", token, e)
                },
                _ => (),
            }
        } else {
            match self.handle_accept(event_loop, token) {
                Err(s) => error!("{}", s),
                _ => (),
            }
        }
    }

    fn timeout(&mut self, event_loop: &mut EventLoop<MyHandler>, mut token: Token) {
        /*info!("Timeout at 50ms");
        event_loop.timeout_ms(token, 50);*/
        self.handle_timer(event_loop, token);
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
        let level = match record.level() {
            LogLevel::Info => format!("{}", Blue.bold().paint(format!("{}", record.level()))),
            LogLevel::Error => format!("{}", Red.bold().paint(format!("{}", record.level()))),
            LogLevel::Debug => format!("{}", Green.bold().paint(format!("{}", record.level()))),
            LogLevel::Warn => format!("{}", Yellow.bold().paint(format!("{}", record.level()))),
            LogLevel::Trace => format!("{}", Purple.bold().paint(format!("{}", record.level()))),
        };

        let location = format!("{}", Style::default().bold().paint(format!("{}:{}", record.location().file(), record.location().line())));
        format!("{} - {} - {}", level, location, record.args())
    };

    let mut builder = LogBuilder::new();
    builder.format(format).filter(None, LogLevelFilter::Info);

    if env::var("RUST_LOG").is_ok() {
       builder.parse(&env::var("RUST_LOG").unwrap());
    }

    builder.init().unwrap();

}
