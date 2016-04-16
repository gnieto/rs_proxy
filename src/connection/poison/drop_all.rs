use connection::Connection;
use connection::ConnectionAction;
use std::io::Read;
use std::io::Write;
use std::io::Result;
use mio::Token;
use mio::Evented;
use mio::EventSet;

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
    fn handle_read(&mut self) -> ConnectionAction {
        ConnectionAction::Noop
    }

    fn handle_write(&mut self) -> ConnectionAction {
        ConnectionAction::Noop
    }

    fn get_interest(&self) -> EventSet {
        self.connection.get_interest()
    }
}

impl Read for DropAllConnection {
    fn read(&mut self, _: &mut [u8]) -> Result<usize> {
        return Ok(0)
    }
}

impl Write for DropAllConnection {
    fn write(&mut self, _: &[u8]) -> Result<usize> {
        return Ok(0)
    }

    fn flush(&mut self) -> Result<()> {
        return Ok(())
    }
}
