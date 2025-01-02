use super::client::Methods;
use super::request::RequestBuilder;
use super::response::Response;
use super::url::Url;
use crate::tls::tls_stream::TlsStream;
use std::collections::HashMap;
use std::io::{Error, ErrorKind, Read, Write};

type HeaderMap<'p> = HashMap<&'p str, &'p str>;
type OptHeaders<'p> = Option<HeaderMap<'p>>;
type TLSResult<T> = Result<T, Error>;

pub struct PersistentClient<'p> {
    io: TlsStream,
    head: HeaderMap<'p>,
}

impl<'p> PersistentClient<'p> {
    pub fn new(a: &'p str, eh: OptHeaders<'p>, url: &'p str) -> TLSResult<Self> {
        let mut head: HeaderMap<'p> = HashMap::new();
        let p_url = Url::new(url).unwrap();
        head.insert("User-Agent", a);
        if let Some(h) = eh {
            head.extend(h.iter())
        };

        Ok(Self {
            io: TlsStream::new(None, p_url.domain(), &p_url.socket_addr())?,
            head,
        })
    }

    pub fn request(
        &mut self,
        m: Methods,
        url: &'p str,
        content: Option<Vec<u8>>,
        extra_headers: Option<HeaderMap<'p>>,
    ) -> TLSResult<Vec<u8>> {
        let mut req = RequestBuilder::new();
        let split_url = match Url::new(url) {
            Ok(u) => u,
            Err(e) => return Err(Error::new(ErrorKind::InvalidData, e.to_string())),
        };

        req.http_method(m)
            .headers(&self.head)
            .route(split_url.route())
            .host(split_url.domain());

        if let Some(c) = content {
            req.content(c);
        };
        if let Some(h) = extra_headers {
            req.headers(&h);
        };

        let b_req = req.build();
        let mut buf = vec![];
        let _ = self.io.write(&b_req);
        let _ = self.io.read_to_end(&mut buf);
        Ok(buf)
    }

    pub fn get(&mut self, url: &'p str, headers: Option<HeaderMap<'p>>) -> TLSResult<Vec<u8>> {
        self.request(Methods::GET, url, None, headers)
    }
}
