use std::net::SocketAddr;

pub struct TcpProxy {
    pub input: SocketAddr,
}

impl TcpProxy {
    pub fn new(input: &str) -> Self {
        TcpProxy {
            input: input.parse().unwrap(),
        }
    }
}
