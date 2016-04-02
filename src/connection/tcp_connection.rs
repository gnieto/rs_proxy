use mio::{Token, Evented};
use mio::tcp::TcpStream;
use connection::Connection;
use std::io::Read;
use std::io::Write;
use std::io::Result;

pub struct TcpConnection {
    buffer: Vec<u8>,
    stream: TcpStream,
    token: Token,
}

impl TcpConnection {
    pub fn new(size: usize, stream: TcpStream, token: Token) -> Self {
        TcpConnection {
            buffer: Vec::with_capacity(size),
            stream: stream,
            token: token,
        }
    }
}

impl Connection for TcpConnection {
    fn get_evented(&self) -> &Evented {
        return &self.stream;
    }

    fn get_token(&self) -> Token {
        return self.token;
    }
}

impl Read for TcpConnection {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize> {
        return self.stream.read(buffer)
    }
}

impl Write for TcpConnection {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        return self.stream.write(buf)
    }

    fn flush(&mut self) -> Result<()> {
        return self.stream.flush()
    }
}
