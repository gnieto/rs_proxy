use mio::{Token, Evented};
use mio::tcp::TcpStream;
use connection::Connection;
use bytes::RingBuf;
use connection::BufferState;

pub struct TcpConnection {
    buffer: RingBuf,
    stream: TcpStream,
    token: Token,
}

impl TcpConnection {
    pub fn new(size: usize, stream: TcpStream, token: Token) -> Self {
        TcpConnection {
            buffer: RingBuf::new(size),
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

    fn handle_read(&mut self) -> BufferState {
        use bytes::buf::MutBuf;
        use std::io::Read;

        let a = unsafe{ self.stream.read(&mut self.buffer.mut_bytes()) };

        match a {
            Ok(amount) => {
                unsafe {MutBuf::advance(&mut self.buffer, amount)};
            },
            _ => (),
        };

        info!("Vector size: {}; Capacity: {};", self.buffer.remaining(), self.buffer.capacity());

        return BufferState::Remaining(self.buffer.remaining());
    }

    fn handle_write(&mut self, buffer: &[u8]) {
        use std::io::Write;

        let result = self.stream.write(buffer);
        info!("Write result {:?}", result)
    }

    fn get_buffer(&self) -> &[u8] {
        use bytes::buf::Buf;
        return self.buffer.bytes();
    }
}
