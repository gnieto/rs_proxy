pub use self::connection::HttpConnection;
use httparse;
use httparse::Header;

mod connection;

pub trait HttpProxy {
    fn on_request_headers(&mut self, method: &str, path: &str, version: u8, headers: &[Header]);
    fn on_request(&mut self, _: &str, _: &str, _: u8, _: &[Header], body: &[u8]) -> HttpAction;

    fn on_response_headers(&mut self);
    fn on_response(&mut self, version: u8, code: u16, reason: &str, header: &[Header], body: &[u8]) -> HttpAction;
}

pub struct NoopProxy;

impl HttpProxy for NoopProxy {
    fn on_request_headers(&mut self, _: &str, _: &str, _: u8, _: &[Header]) {}
    fn on_request(&mut self, _: &str, _: &str, _: u8, _: &[Header], body: &[u8]) -> HttpAction {
        HttpAction::Forward
    }

    fn on_response_headers(&mut self) {}
    fn on_response(&mut self, version: u8, code: u16, reason: &str, header: &[Header], body: &[u8]) -> HttpAction {
        HttpAction::Forward
    }
}

pub struct LoggerProxy;

impl HttpProxy for LoggerProxy {
    fn on_request_headers(&mut self, method: &str, path: &str, version: u8, headers: &[Header]) {
        warn!("Method {}", method);
        warn!("Path {}", path);
        warn!("Version {}", version);

        for h in headers {
            warn!("Header: {:?}", h);
        }
    }

    fn on_request(&mut self, method: &str, path: &str, version: u8, headers: &[Header], body: &[u8]) -> HttpAction {
        warn!("Method {}", method);
        warn!("Path {}", path);
        warn!("Version {}", version);
        for h in headers {
            warn!("Header: {:?}", h);
        }
        warn!("Body: {:?}", body);

        HttpAction::Forward
    }

    fn on_response_headers(&mut self) {}
    fn on_response(&mut self, version: u8, code: u16, reason: &str, headers: &[Header], body: &[u8]) -> HttpAction {
        warn!("Code {}", code);
        warn!("Reason {}", reason);

        for h in headers {
            warn!("Header: {:?}", h);
        }

        warn!("Body: {:?}", body);

        HttpAction::Forward
    }
}

pub enum HttpAction {
    Abort,
    Forward,
    Wait,
    Modify(Vec<u8>)
}

pub struct AddHeaderProxy;

impl HttpProxy for AddHeaderProxy {
    fn on_request_headers(&mut self, method: &str, path: &str, version: u8, headers: &[Header]) {
        warn!("Method {}", method);
        warn!("Path {}", path);
        warn!("Version {}", version);

        for h in headers {
            warn!("Header: {:?}", h);
        }
    }

    fn on_request(&mut self, method: &str, path: &str, version: u8, headers: &[Header], body: &[u8]) -> HttpAction {
        warn!("Method {}", method);
        warn!("Path {}", path);
        warn!("Version {}", version);
        for h in headers {
            warn!("Header: {:?}", h);
        }
        warn!("Body: {:?}", body);

        let mut req = Vec::<u8>::new();
        req.extend_from_slice(method.as_bytes());
        req.extend_from_slice(" ".as_bytes());
        req.extend_from_slice(path.as_bytes());
        req.extend_from_slice(" HTTP/1.1\r\n".as_bytes());

        for h in headers {
            req.extend_from_slice(h.name.as_bytes());
            req.extend_from_slice(": ".as_bytes());
            req.extend_from_slice(h.value);
            req.extend_from_slice("\r\n".as_bytes());
        };

        req.extend_from_slice("X-RSPROXY: v0.1\r\n".as_bytes());

        req.extend("\r\n".as_bytes());
        req.extend(body);

        HttpAction::Modify(req)
    }

    fn on_response_headers(&mut self) {}

    fn on_response(&mut self, version: u8, code: u16, reason: &str, header: &[Header], body: &[u8]) -> HttpAction {
        HttpAction::Forward
    }
}
