use bytes::{Bytes, BytesMut};
use log::{debug, info, warn};
use std::collections::HashMap;
use std::error;
use std::fmt;
use std::num::ParseIntError;
use std::str;
use std::str::Utf8Error;

#[derive(Debug)]
pub enum HttpResponseError {
    Empty,
    ParseStrError(Utf8Error),
    ParseError(ParseIntError),
    NoHeaders,
    InvalidHeader,
}

impl fmt::Display for HttpResponseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            HttpResponseError::Empty => write!(f, "the given byte slice was empty"),
            HttpResponseError::ParseError(..) => {
                write!(f, "there was a error during parsing a number")
            }
            HttpResponseError::ParseStrError(..) => {
                write!(f, "there was an error with parsing a string")
            }
            HttpResponseError::NoHeaders => {
                write!(f, "no headers were found")
            }
            HttpResponseError::InvalidHeader => {
                write!(f, "invalid header")
            }
        }
    }
}

impl error::Error for HttpResponseError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            HttpResponseError::Empty => None,
            HttpResponseError::ParseError(ref e) => Some(e),
            HttpResponseError::ParseStrError(ref e) => Some(e),
            HttpResponseError::NoHeaders => None,
            HttpResponseError::InvalidHeader => None,
        }
    }
}

pub type HttpResult<T> = Result<T, HttpResponseError>;
pub type HeaderMap<'r> = HashMap<&'r str, &'r str>;

fn read_chunk_length(buf: &[u8]) -> HttpResult<usize> {
    let str = match str::from_utf8(buf) {
        Ok(o) => o,
        Err(e) => return Err(HttpResponseError::ParseStrError(e)),
    };
    match usize::from_str_radix(str, 16) {
        Ok(num) => Ok(num),
        Err(e) => Err(HttpResponseError::ParseError(e)),
    }
}

#[derive(Debug)]
pub struct Response<'r> {
    pub status_code: u16,
    pub headers: HeaderMap<'r>,
    pub content: Bytes,
}

impl<'r> Response<'r> {
    pub fn from_slice(d: &'r [u8]) -> HttpResult<Self> {
        if d.is_empty() {
            return Err(HttpResponseError::Empty);
        };

        let mut headers: HashMap<&'r str, &'r str> = HashMap::new();

        let mut iter = d.iter();

        // loop to split into lines
        let mut lines: Vec<&'r str> = Vec::with_capacity(10);
        let mut line_break = false;
        let mut count = 0;
        let mut t_count: usize = 0;

        while let Some(byte) = iter.next() {
            match byte {
                b'\r' => {
                    if iter.next().is_some_and(|v| v == &b'\n') {
                        line_break = true;
                        count += 2;
                        continue;
                    }
                }
                _ => {
                    if line_break {
                        let line = match str::from_utf8(&d[t_count..t_count + count - 2]) {
                            Ok(o) => o,
                            Err(e) => return Err(HttpResponseError::ParseStrError(e)),
                        };
                        lines.push(line);
                        line_break = false;
                        t_count += count;
                        count = 0;
                    };
                    count += 1
                }
            }
        }

        let mut lines_iter = lines.into_iter();

        // status code
        let status_code = match lines_iter.next() {
            Some(line) => match line[9..12].parse::<u16>() {
                Ok(str) => str,
                Err(e) => return Err(HttpResponseError::ParseError(e)),
            },
            None => return Err(HttpResponseError::Empty),
        };

        // headers
        for line in lines_iter {
            let mut header = line.split(": ");
            match header.next() {
                None => return Err(HttpResponseError::NoHeaders),
                Some(k) => {
                    if let Some(v) = header.next() {
                        headers.insert(k, v);
                    } else {
                        return Err(HttpResponseError::InvalidHeader);
                    }
                }
            }
        }

        // content length
        let content_len = match headers.get("Content-Length") {
            Some(l) => match l.parse::<usize>() {
                Ok(n) => Some(n),
                Err(e) => return Err(HttpResponseError::ParseError(e)),
            },
            None => None,
        };

        // transfer encoding
        let mut transfer_encoding = None;
        if let Some(len) = content_len {
            return Ok(Self {
                status_code,
                headers,
                content: Bytes::copy_from_slice(&d[t_count..t_count + len]),
            });
        } else {
            match headers.get("Transfer-Encoding") {
                None => {
                    warn!("No Content-Length and Transfer-Encoding found! Reading everything");
                }
                Some(v) => transfer_encoding = Some(*v),
            }
        };

        match transfer_encoding {
            Some("chunked") => {
                let mut data_read = false;
                let mut length_read = true;
                let mut read_length = 0;
                let mut l_count = 0;
                let mut c_buf = BytesMut::with_capacity(368);
                t_count += 2;

                while let Some(byte) = iter.next() {
                    if data_read {
                        c_buf.extend_from_slice(&d[0..1 + read_length]);
                        t_count += read_length + 2;
                        data_read = false;
                    }
                    if length_read {
                        match byte {
                            b'\r' => {
                                iter.nth(0);
                                length_read = false;
                                data_read = true;

                                read_length = read_chunk_length(&d[t_count..t_count + l_count])?;
                                l_count = 0;

                                debug!("Chunk length: {}", read_length);

                                if read_length == 0 {
                                    break;
                                }
                            }
                            _ => {
                                l_count += 1;
                                continue;
                            }
                        }
                    }
                }

                Ok(Self {
                    status_code,
                    headers,
                    content: c_buf.freeze(),
                })
            }
            Some(_) => unimplemented!(),
            None => Ok(Self {
                status_code,
                headers,
                content: Bytes::copy_from_slice(&d[t_count..]),
            }),
        }
    }
}
