use crate::https::client::Methods;
use std::collections::HashMap;

const CRLF: &[u8] = "\r\n".as_bytes();

pub struct RequestBuilder<'a> {
    method: Option<Methods>,
    route: Option<&'a str>,
    headers: HashMap<&'a str, &'a str>,
    content: Option<Vec<u8>>,
    content_len: usize,
}

impl<'a> RequestBuilder<'a> {
    pub fn new() -> Self {
        Self {
            method: None,
            route: None,
            headers: HashMap::new(),
            content: None,
            content_len: 0,
        }
    }
    pub fn http_method(&mut self, m: Methods) -> &mut Self {
        self.method = Some(m);
        self
    }
    pub fn headers(&mut self, h: &HashMap<&'a str, &'a str>) -> &mut Self {
        let add_headers = &h;
        self.headers
            .extend(add_headers.iter().map(|(k, v)| (*k, *v)));
        self
    }
    pub fn host(&mut self, ht: &'a str) -> &mut Self {
        self.headers.insert("Host", ht);
        self
    }
    pub fn content(&mut self, c: Vec<u8>) -> &mut Self {
        self.content_len = c.len();
        self.content = Some(c);
        self
    }
    pub fn route(&mut self, r: &'a str) -> &mut Self {
        self.route = Some(r);
        self
    }

    pub fn build(self) -> Vec<u8> {
        let mut buf = vec![];
        let method = match self.method {
            None => Methods::GET,
            Some(m) => m,
        };

        let b_method = match method {
            Methods::GET => "GET".as_bytes(),
            Methods::POST => "POST".as_bytes(),
            Methods::PATCH => "PATCH".as_bytes(),
            Methods::OPTIONS => "OPTIONS".as_bytes(),
            Methods::CONNECT => "CONNECT".as_bytes(),
            Methods::HEAD => "HEAD".as_bytes(),
            Methods::PUT => "PUT".as_bytes(),
            Methods::DELETE => "DELETE".as_bytes(),
        };

        buf.extend_from_slice(b_method);
        buf.extend_from_slice(&[32]);

        // route
        match self.route {
            Some(r) => buf.extend_from_slice(r.as_bytes()),
            None => buf.extend_from_slice(r"\".as_bytes()),
        };
        buf.push(32);

        buf.extend_from_slice("HTTP/1.1".as_bytes());
        buf.extend_from_slice(CRLF);

        for (k, v) in self.headers {
            buf.extend_from_slice(k.as_bytes());
            buf.extend_from_slice(": ".as_bytes());
            buf.extend_from_slice(v.as_bytes());
            buf.extend_from_slice(CRLF);
        }
        buf.extend_from_slice(CRLF);

        // Content-Length and Content
        if self.content_len > 0 {
            buf.extend_from_slice("Content-Length: ".as_bytes());
            buf.extend_from_slice(format!("{}\r\n", self.content_len).as_bytes());
            if let Some(c) = self.content {
                buf.extend_from_slice(&c);
            };
        }
        buf
    }
}
