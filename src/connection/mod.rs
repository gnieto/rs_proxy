use mio::{Evented, Token, EventSet};
use std::io;

pub mod tcp_connection;
pub mod poison;

#[cfg(feature = "redis")]
pub mod redis;

#[cfg(feature = "http")]
pub mod http;

pub trait Connection: io::Read + io::Write {
    fn get_evented(&self) -> &Evented;
    fn get_token(&self) -> Token;
    fn get_interest(&self) -> EventSet;
    fn handle_read(&mut self) -> ConnectionAction;
    fn handle_write(&mut self) -> ConnectionAction;
}

pub trait Timer {
    fn handle_timer(&mut self) -> TimerAction;
    fn get_frequency(&self) -> u64;
}

#[derive(Copy,Clone,Debug)]
pub enum Role {
    Downstream,
    Upstream,
}

#[derive(Debug)]
pub enum ConnectionAction {
    Forward,
    Hold,
    Halt,
    Noop,
}

pub enum TimerAction {
    Continue,
    Stop,
}
