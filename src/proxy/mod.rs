use connection::Connection;
use mio::Token;

pub mod tcp;

pub trait Proxy {
    fn get_upstream(&self) -> &Connection;
    fn get_downstream(&self) -> &Connection;

    fn get_mut_upstream(&mut self) -> &mut Connection;
    fn get_mut_downstream(&mut self) -> &mut Connection;

    fn tokens(&self) -> (Token, Token) {
        let downstream_token = self.get_downstream().get_token();
        let upstream_token = self.get_upstream().get_token();

        (downstream_token, upstream_token)
    }
}
