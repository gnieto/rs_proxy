pub use self::round_robin::*;

use connection::Connection;
use connection::Role;
use mio::Token;
use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;

mod round_robin;

pub struct Proxy {
    downstream: Rc<RefCell<Connection>>,
    upstream: Rc<RefCell<Connection>>,
    upstream_closed: bool,
}

impl Proxy {
    pub fn new(downstream: Rc<RefCell<Connection>>, upstream: Rc<RefCell<Connection>>) -> Self {
        Proxy {
            downstream: downstream,
            upstream: upstream,
            upstream_closed: false,
        }
    }

    pub fn get_upstream(&self) -> Rc<RefCell<Connection>> {
        return self.upstream.clone();
    }

    pub fn get_downstream(&self) -> Rc<RefCell<Connection>> {
        return self.downstream.clone();
    }

    pub fn tokens(&self) -> (Token, Token) {
        let ds = self.get_downstream();
        let ds_borrow = ds.borrow();
        let downstream_token = ds_borrow.get_token();

        let us = self.get_upstream();
        let us_borrow = us.borrow();
        let upstream_token = us_borrow.get_token();

        (downstream_token, upstream_token)
    }

    pub fn upstream_closed(&mut self) {
        self.upstream_closed = true;
    }

    pub fn is_upstream_closed(&self) -> bool {
        self.upstream_closed
    }

    pub fn forward(&mut self, role: Role) {
        let ds = self.get_downstream();
        let us = self.get_upstream();

        let (mut read_borrow, mut write_borrow) = match role {
            Role::Upstream => {
                let us_borrow = us.borrow_mut();
                let ds_borrow = ds.borrow_mut();

                (us_borrow, ds_borrow)
            },
            Role::Downstream => {
                let ds_borrow = ds.borrow_mut();
                let us_borrow = us.borrow_mut();

                (ds_borrow, us_borrow)
            },
        };

        let mut buf: &mut [u8] = &mut [0u8; 1024];
        let read_result = read_borrow.read(&mut buf);
        match read_result {
            Ok(amount) => {
                if amount > 0 {
                    info!("Read result with amount: {}", amount);
                }
                write_borrow.write(&buf[0..amount]).unwrap();
            },
            Err(_) => {
                error!("Could not read from input buffer (token: {:?})", read_borrow.get_token());
            }
        };
    }

    pub fn get_from_token(&self, token: Token) -> Option<Rc<RefCell<Connection>>> {
        let tokens = self.tokens();
        if tokens.0 == token {
            return Some(self.get_downstream())
        }

        if tokens.1 == token {
            return Some(self.get_upstream())
        }

        None
    }
}

pub struct ProxyLocator {
    proxies: HashMap<Token, (Role, Rc<RefCell<Proxy>>)>,
}

impl ProxyLocator {
    pub fn new() -> Self {
        ProxyLocator {
            proxies: HashMap::new(),
        }
    }

    pub fn link(&mut self, token: Token, role: Role, proxy: Rc<RefCell<Proxy>>) {
        self.proxies.insert(token, (role, proxy));
    }

    pub fn unlink(&mut self, token: &Token) {
        self.proxies.remove(token);
    }

    pub fn has(&self, token: &Token) -> bool {
        self.proxies.contains_key(token)
    }

    pub fn get(&self, token: &Token) -> Option<(Role, Rc<RefCell<Proxy>>)> {
        match self.proxies.get(&token) {
            Some(&(ref role, ref refer)) => {
                Some((*role, refer.clone()))
            },
            None => None,
        }
    }
}
