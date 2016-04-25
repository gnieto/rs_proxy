use mio::{Token, Evented, EventSet};
use connection::Connection;
use connection::tcp_connection::TcpConnection;
use connection::ConnectionAction;
use std::io;
use connection::redis::RedisProxy;
use resp::{Decoder, Value};
use std::ascii::AsciiExt;

pub struct RedisConnection<P> where P: RedisProxy {
    connection: TcpConnection,
    proxy: P,
}

impl<P> RedisConnection<P> where P: RedisProxy {
    pub fn new(connection: TcpConnection, proxy: P) -> Self {
        RedisConnection {
            connection: connection,
            proxy: proxy,
        }
    }
}

impl<P> io::Read for RedisConnection<P> where P: RedisProxy {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let len = self.connection.get_input().len();
        let mut decoder = Decoder::new();
        decoder.feed(&self.connection.get_mut_input()[0..len]).unwrap();
        let parsed = decoder.read();
        match parsed {
            Some(command) => {
                let command: Value = self.proxy.on_command(command);

                self.connection.get_mut_input().consume(len);
                self.connection.get_mut_input().extend(command.to_encoded_string().unwrap().as_str().as_bytes());

                self.connection.read(buf)
            }
            None => {
                Ok(0)
            },
        }
    }
}

impl<P> io::Write for RedisConnection<P> where P: RedisProxy {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut decoder = Decoder::new();
        decoder.feed(buf).unwrap();
        let parsed = decoder.read();

        match parsed {
            Some(response) => {
                info!("Parsed!");
                let response = self.proxy.on_response(response);

                self.connection.write(response.to_encoded_string().unwrap().as_str().as_bytes())
            },
            None => {
                info!("No parsed!");
                Ok(0)
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
 }

impl<P> Connection for RedisConnection<P> where P: RedisProxy {
    fn get_evented(&self) -> &Evented {
        return &*self.connection.get_evented();
    }

    fn get_token(&self) -> Token {
        return self.connection.get_token();
    }

    fn get_interest(&self) -> EventSet {
        return self.connection.get_interest();
    }

    fn handle_read(&mut self) -> ConnectionAction {
        let read_response = self.connection.handle_read();

        read_response
    }

    fn handle_write(&mut self) -> ConnectionAction {
        return self.connection.handle_write()
    }
}
