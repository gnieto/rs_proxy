use connection::Connection;

pub mod tcp;

pub trait Proxy {
    fn get_upstream(&mut self) -> &mut Connection;
    fn get_downstream(&mut self) -> &mut Connection;
}
