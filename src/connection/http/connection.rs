use connection::http::{HttpProxy, HttpAction};
use connection::tcp_connection::TcpConnection;
use connection::Connection;
use std::io;
use mio::{Token, Evented, EventSet};
use connection::ConnectionAction;
use httparse;
use httparse::Status;

pub struct HttpConnection<P: HttpProxy> {
    connection: TcpConnection,
    proxy: P,
}

impl<P: HttpProxy> HttpConnection<P> {
    pub fn new(connection: TcpConnection, proxy: P) -> Self {
        HttpConnection {
            connection: connection,
            proxy: proxy,
        }
    }
}

impl<P> io::Read for HttpConnection<P> where P: HttpProxy {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let action = {
            let mut headers = [httparse::EMPTY_HEADER; 16];
            let mut req = httparse::Request::new(&mut headers);
            let len = self.connection.get_input().len();

            let parse_result = req.parse(&self.connection.get_input()[0..]);
             match parse_result {
                Err(e) => {
                    error!("Error parsing http: {:?}", e);
                    HttpAction::Wait
                },
                Ok(Status::Complete(a)) => {
                    info!("Complete size: {}; buffer size: {}", a, len);
                    self.proxy.on_request(
                        req.method.unwrap_or("GET"),
                        req.path.unwrap_or("/"),
                        req.version.unwrap_or(1),
                        req.headers,
                        &self.connection.get_input()[a..]
                    )
                },
                Ok(Status::Partial) => {
                    info!("Partial response");
                    HttpAction::Wait
                }
            }
        };

        match action {
            HttpAction::Modify(b) => {
                let len = self.connection.get_input().len();
                self.connection.get_mut_input().consume(len);
                self.connection.get_mut_input().extend(b.as_slice());

                self.connection.read(buf)
            }
            HttpAction::Forward => self.connection.read(buf),
            _ => Ok(0),
        }
    }
}

impl<P> io::Write for HttpConnection<P> where P: HttpProxy {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut headers = [httparse::EMPTY_HEADER; 16];
        let mut resp = httparse::Response::new(&mut headers);

        let parse_result = resp.parse(buf);
        let action = match parse_result {
            Err(e) => {
                error!("Error parsing http: {:?}", e);
                HttpAction::Wait
            },
            Ok(Status::Complete(a)) => {
                warn!("Status complete. Read: {}", a);

                self.proxy.on_response(
                    resp.version.unwrap_or(1),
                    resp.code.unwrap_or(0),
                    resp.reason.unwrap_or("Unknown"),
                    resp.headers,
                    &buf[a..]
                )
            },
            Ok(Status::Partial) => {
                // Todo save internally to a buffer to parse it later
                info!("Partial response");
                HttpAction::Wait
            }
        };

        match action {
            HttpAction::Modify(b) => {
                self.connection.write(&b)
            }
            HttpAction::Forward => self.connection.write(buf),
            _ => Ok(0),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
 }

impl<P: HttpProxy> Connection for HttpConnection<P> {
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
