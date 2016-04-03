use std::io::Read;
use std::io::Write;
use mio::{Evented, Token};

pub mod tcp_connection;
// pub mod poison;

pub trait Connection {
    fn get_evented(&self) -> &Evented;
    fn get_token(&self) -> Token;
    fn handle_read(&mut self) -> BufferState;
    fn handle_write(&mut self, &[u8]);
    fn get_buffer(&self) -> &[u8];
}

#[derive(Debug)]
pub enum Role {
    Downstream,
    Upstream,
}

#[derive(Debug)]
pub enum BufferState {
    Full,
    Remaining(usize),
}
