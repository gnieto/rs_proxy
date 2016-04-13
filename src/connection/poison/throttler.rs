use connection::{Connection, Timer};
use connection::{ConnectionAction, TimerAction};
use std::io::Read;
use std::io::Write;
use std::io::Result;
use mio::Token;
use mio::Evented;
use std::cmp::min;

pub struct Throttler {
    connection: Box<Connection>,
    size: usize,
}

impl Throttler {
    // TODO: How to handle upstream + downstream throttling
    pub fn new(connection: Box<Connection>, size: usize) -> Self {
        Throttler {
            connection: connection,
            size: size,
        }
    }
}

impl Connection for Throttler {
    fn get_evented(&self) -> &Evented {
        return &*self.connection.get_evented();
    }

    fn get_token(&self) -> Token {
        return self.connection.get_token();
    }

    fn handle_read(&mut self) -> ConnectionAction {
        return self.connection.handle_read()
    }

    fn handle_write(&mut self) -> ConnectionAction {
        return self.connection.handle_write()
    }
}

impl Timer for Throttler {
    fn handle_timer(&mut self) -> TimerAction {
        info!("Tick!");
        TimerAction::Continue
    }

    fn get_frequency(&self) -> u64 {
        50
    }
}

impl Read for Throttler {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.connection.read(buf)
    }
}

impl Write for Throttler {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.connection.write(buf)
    }

    fn flush(&mut self) -> Result<()> {
        return Ok(())
    }
}
