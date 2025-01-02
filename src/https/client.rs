use super::request::RequestBuilder;
use crate::tls::tls_stream::TlsStream;
use crate::Url;
use std::collections::HashMap;
use std::io;
use std::io::{Read, Write};

#[allow(clippy::upper_case_acronyms)]
#[allow(dead_code)]
pub enum Methods {
    GET,
    POST,
    PUT,
    PATCH,
    DELETE,
    HEAD,
    CONNECT,
    OPTIONS,
}

type HeaderMap<'a> = HashMap<&'a str, &'a str>;

pub struct HttpsClient<'b> {
    headers: HashMap<&'b str, &'b str>,
}

impl<'b> HttpsClient<'b> {
    pub fn new(agent: &'b str, extra_headers: Option<&HeaderMap<'b>>) -> Self {
        let mut headers: HeaderMap<'b> = HashMap::new();
        headers.insert("User-Agent", agent);
        if let Some(h) = extra_headers {
            headers.extend(h.iter().map(|(k, v)| (*k, *v)));
        }

        Self { headers }
    }
    fn request(
        &self,
        method: Methods,
        url: &str,
        content: Option<Vec<u8>>,
        headers: Option<HeaderMap>,
    ) -> io::Result<Vec<u8>> {
        println!("{}", url);
        let url_parts = Url::new(url).unwrap();

        let mut req = RequestBuilder::new();

        req.http_method(method)
            .headers(&self.headers)
            .route(url_parts.route())
            .host(url_parts.domain());

        if let Some(c) = content {
            req.content(c);
        }
        if let Some(h) = headers {
            req.headers(&h);
        }

        let bytes = req.build();

        println!("{:?}", std::str::from_utf8(&bytes).unwrap());
        let mut stream = TlsStream::new(None, url_parts.domain(), &url_parts.socket_addr())?;
        let mut buf = vec![];
        let _ = stream.write(&bytes)?;
        let _ = stream.read_to_end(&mut buf)?;
        Ok(buf)
    }

    pub fn get(&mut self, url: &str, extra_headers: Option<HeaderMap>) -> io::Result<Vec<u8>> {
        self.request(Methods::GET, url, None, extra_headers)
    }

    pub fn post(
        &mut self,
        url: &str,
        content: Option<Vec<u8>>,
        extra_headers: Option<HeaderMap>,
    ) -> io::Result<Vec<u8>> {
        self.request(Methods::POST, url, content, extra_headers)
    }
}
