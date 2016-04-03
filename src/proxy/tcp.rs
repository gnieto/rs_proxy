use std::net::SocketAddr;
use mio::tcp::{TcpListener, TcpStream};
use mio::Token;
use connection::Connection;
use connection::tcp_connection::TcpConnection;
use proxy::Proxy;
use std::rc::Rc;
use std::cell::RefCell;

pub struct TcpProxy {
    downstream: Rc<RefCell<Connection>>,
    upstream: Rc<RefCell<Connection>>,
}

impl TcpProxy {
    pub fn new(downstream: Rc<RefCell<Connection>>, upstream: Rc<RefCell<Connection>>) -> Self {
        TcpProxy {
            downstream: downstream,
            upstream: upstream,
        }
    }
}

impl Proxy for TcpProxy {
    fn get_upstream(&self) -> Rc<RefCell<Connection>> {
        return self.upstream.clone();
    }

    fn get_downstream(&self) -> Rc<RefCell<Connection>> {
        return self.downstream.clone();
    }
}
