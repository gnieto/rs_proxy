use std::net::SocketAddr;
use mio::tcp::{TcpListener, TcpStream};
use mio::Token;
use connection::Connection;
use connection::tcp_connection::TcpConnection;
use proxy::Proxy;

pub struct TcpProxy {
    downstream: Box<Connection>,
    upstream: Box<Connection>,
}

impl TcpProxy {
    pub fn new(downstream: Box<Connection>, upstream: Box<Connection>) -> Self {
        TcpProxy {
            downstream: downstream,
            upstream: upstream,
        }
    }
}

impl Proxy for TcpProxy {
    fn get_upstream(&mut self) -> &mut Connection {
        return &mut *self.upstream;
    }

    fn get_downstream(&mut self) -> &mut Connection {
        return &mut *self.downstream;
    }
}
