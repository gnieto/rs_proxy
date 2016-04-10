use mio::{Token, Evented};
use mio::tcp::TcpStream;
use connection::Connection;
use connection::ConnectionAction;
use connection::buffer::Buffer;
use std::io;

pub struct TcpConnection {
    input: Buffer,
    output: Buffer,
    stream: TcpStream,
    token: Token,
}

impl TcpConnection {
    pub fn new(size: usize, stream: TcpStream, token: Token) -> Self {
        TcpConnection {
            input: Buffer::new(size),
            output: Buffer::new(size),
            stream: stream,
            token: token,
        }
    }
}

impl io::Read for TcpConnection {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let pending = self.input.pending();
        let mut buf = &mut buf[0..pending];

        {
            let read_slice = self.input.get_read_slice();
            buf.clone_from_slice(&read_slice[0..pending]);
        }
        self.input.advance(pending);

        Ok(pending)
    }
}

impl io::Write for TcpConnection {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let pending = self.output.pending();
        {
            let write_slice = self.output.get_write_slice();
            if write_slice.len() > 0 {
                info!("Somethin to write!");
            }
            write_slice.clone_from_slice(buf);
        }

        self.output.advance(pending);
        Ok(pending)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Connection for TcpConnection {
    fn get_evented(&self) -> &Evented {
        return &self.stream;
    }

    fn get_token(&self) -> Token {
        return self.token;
    }

    fn handle_read(&mut self) -> ConnectionAction {
        use std::io::Read;

        let read_result = self.stream.read(self.input.get_write_slice());
        match read_result {
            Ok(amount) => {
                if amount > 0 {
                    ConnectionAction::Forward(amount)
                } else {
                    ConnectionAction::Noop
                }
            },
            Err(_) => {
                ConnectionAction::Halt
            }
        }
    }

    fn handle_write(&mut self) -> ConnectionAction {
        use std::io::Write;

        if self.output.pending() > 0 {
            info!("Pending information to be written");
        }

        let write_result = self.stream.write(&self.output.get_read_slice());
        match write_result {
            Ok(amount) => {
                self.output.advance(amount);
            },
            Err(_) => {
                error!("Could not read on the connection with token {:?}", self.token);
            },
        };

        ConnectionAction::Noop
    }
}
