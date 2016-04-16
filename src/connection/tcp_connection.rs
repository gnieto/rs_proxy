use mio::{Token, Evented, EventSet};
use mio::tcp::TcpStream;
use connection::Connection;
use connection::ConnectionAction;
use netbuf::Buf;
use std::io;
use std::cmp::min;

pub struct TcpConnection {
    input: Buf,
    output: Buf,
    stream: TcpStream,
    token: Token,
    interest: EventSet,
}

impl TcpConnection {
    pub fn new(stream: TcpStream, token: Token) -> Self {
        TcpConnection {
            input: Buf::new(),
            output: Buf::new(),
            stream: stream,
            token: token,
            interest: EventSet::all(),
        }
    }
}

impl io::Read for TcpConnection {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let buf_size = buf.len();
        let read_size = min(buf.len(), self.input.len());
        if self.input.len() > 0 {
            info!("{:?}: Buffer size: {} and input buffer: {}, read_size: {}", self.get_token(), buf_size, self.input.capacity(), read_size);
        }
        buf[0..read_size].clone_from_slice(&self.input[0..read_size]);
        self.input.consume(read_size);

        Ok(read_size)
    }
}

impl io::Write for TcpConnection {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let buf_size = buf.len();
        let write_size = min(buf.len(), self.output.capacity());
        if self.output.len() > 0 {
            info!("{:?}: Buffer size: {} and input buffer: {}, write_size: {}", self.get_token(), buf_size, self.input.capacity(), write_size);
        }
        self.output.extend(buf);

        Ok(write_size)
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
        let read_result = self.input.read_from(&mut self.stream);
        match read_result {
            Ok(amount) => {
                info!("Read to {:?} {} bytes on input", self.get_token(), amount);

                if amount > 0 {
                    ConnectionAction::Forward
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
        info!("Handling write on token: {:?}", self.get_token());
        let write_result = self.output.write_to(&mut self.stream);
        match write_result {
            Ok(amount) => {
                info!("Writting bytes: {}", amount);
                ()
            },
            Err(_) => {
                error!("Could not read on the connection with token {:?}", self.token);
            },
        };

        ConnectionAction::Noop
    }

    fn get_interest(&self) -> EventSet {
        self.interest
    }
}
