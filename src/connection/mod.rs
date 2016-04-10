use mio::{Evented, Token};
use std::io;

pub mod tcp_connection;
// pub mod poison;
pub mod buffer;

pub trait Connection: io::Read + io::Write {
    fn get_evented(&self) -> &Evented;
    fn get_token(&self) -> Token;
    fn handle_read(&mut self) -> ConnectionAction;
    fn handle_write(&mut self) -> ConnectionAction;
}

#[derive(Copy,Clone,Debug)]
pub enum Role {
    Downstream,
    Upstream,
}

#[derive(Debug)]
pub enum ConnectionAction {
    ForwardAll,
    Forward(usize),
    Hold,
    Halt,
    Noop,
}
