use std::io::Read;
use std::io::Write;
use mio::{Evented, Token};

pub mod tcp_connection;
pub mod poison;

pub trait Connection: Read + Write {
    fn get_evented(&self) -> &Evented;
    fn get_token(&self) -> Token;
}
