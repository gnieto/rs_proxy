use connection::Connection;
use std::io::Read;
use std::io::Write;
use std::io::Result;
use mio::Token;
use mio::Evented;

pub struct DropAllConnection {
    connection: Box<Connection>,
}

impl DropAllConnection {
    pub fn new(connection: Box<Connection>) -> Self {
        DropAllConnection {
            connection: connection,
        }
    }
}

impl Connection for DropAllConnection {
    fn get_evented(&self) -> &Evented {
        return &*self.connection.get_evented();
    }

    fn get_token(&self) -> Token {
        return self.connection.get_token();
    }

    fn get_mut_buffer(&mut self) -> &mut [u8] {
        return self.connection.get_mut_buffer()
    }
}

impl Read for DropAllConnection {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize> {
        return Ok(0)
    }
}

impl Write for DropAllConnection {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        return Ok(0)
    }

    fn flush(&mut self) -> Result<()> {
        return Ok(())
    }
}
