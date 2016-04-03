use connection::Connection;
use connection::Role;
use mio::Token;
use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;

pub mod tcp;

pub trait Proxy {
    fn get_upstream(&self) -> Rc<RefCell<Connection>>;
    fn get_downstream(&self) -> Rc<RefCell<Connection>>;

    /*fn get_mut_upstream(&mut self) -> &mut Connection;
    fn get_mut_downstream(&self) -> &mut Box<Connection>;*/

    fn tokens(&self) -> (Token, Token) {
        let ds = self.get_downstream();
        let ds_borrow = ds.borrow();
        let downstream_token = ds_borrow.get_token();

        let us = self.get_upstream();
        let us_borrow = us.borrow();
        let upstream_token = us_borrow.get_token();

        (downstream_token, upstream_token)
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

    pub fn get(&self, token: &Token) -> Option<&(Role, Rc<RefCell<Proxy>>)> {
        self.proxies.get(&token)
    }
}
