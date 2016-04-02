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
    pub fn new(input: &str, stream: TcpStream) -> Self {
        let addr: SocketAddr = input.parse().unwrap();

        let downstream = TcpConnection::new(1024, stream, Token(1));
        let upstream = TcpConnection::new(1024, TcpStream::connect(&addr).unwrap(), Token(2));

        TcpProxy {
            downstream: Box::new(downstream),
            upstream: Box::new(upstream),
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
