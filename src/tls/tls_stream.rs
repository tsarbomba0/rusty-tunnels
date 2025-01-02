use log::{debug, error, info};
use rustls::{ClientConfig, ClientConnection, RootCertStore};
use std::io::{BufReader, BufWriter, Error, ErrorKind, Read, Write};
use std::net::TcpStream;
use std::sync::Arc;
use webpki_roots::TLS_SERVER_ROOTS;
type TLSResult<T> = Result<T, Error>;

pub struct TlsStream {
    pub(crate) conn: ClientConnection,
    pub(crate) buf_r: BufReader<TcpStream>,
    pub(crate) buf_w: BufWriter<TcpStream>,
    #[allow(dead_code)]
    pub(crate) sock: TcpStream,
}

impl TlsStream {
    pub fn new(config: Option<&Arc<ClientConfig>>, url: &str, addr: &str) -> TLSResult<Self> {
        info!("Creating DNS name for {}", url);
        let sock = TcpStream::connect(addr)?;
        let server_name = match url.to_string().try_into() {
            Ok(name) => name,
            Err(_) => panic!("Invalid DNS name!"),
        };

        // if supplied config
        // use that
        let cfg = if let Some(c) = config {
            Arc::clone(c)
        } else {
            let root_store: RootCertStore = RootCertStore {
                roots: TLS_SERVER_ROOTS.into(),
            };
            Arc::new(
                ClientConfig::builder()
                    .with_root_certificates(root_store)
                    .with_no_client_auth(),
            )
        };

        // tls connection
        let client_conn = match ClientConnection::new(cfg, server_name) {
            Ok(conn) => conn,
            Err(_) => return Err(Error::from(ErrorKind::ConnectionAborted)),
        };

        info!("Connected to {}", addr);

        Ok(Self {
            conn: client_conn,
            buf_r: BufReader::new(TcpStream::try_clone(&sock)?),
            buf_w: BufWriter::new(TcpStream::try_clone(&sock)?),
            sock,
        })
    }

    // Does IO for the connection.
    pub fn handshake(&mut self) -> TLSResult<(usize, usize)> {
        let mut eof = false;
        let mut read = 0;
        let mut write = 0;
        loop {
            let handshake = self.conn.is_handshaking();

            if !self.conn.wants_write() && !self.conn.wants_read() {
                return Ok((read, write));
            }

            while self.conn.wants_write() {
                match self.conn.write_tls(&mut self.buf_w)? {
                    0 => {
                        self.buf_w.flush()?;
                        return Ok((read, write));
                    }
                    n => write += n,
                }
            }
            self.buf_w.flush()?;

            if !handshake && write > 0 {
                return Ok((read, write));
            }

            while !eof && self.conn.wants_read() {
                debug!("reading in handshake fn");
                let r = match self.conn.read_tls(&mut self.buf_r) {
                    Ok(0) => {
                        eof = true;
                        info!("EOF while doing IO!");
                        Some(0)
                    }
                    Ok(n) => {
                        read += n;
                        debug!("Read: {} bytes", n);
                        Some(n)
                    }
                    Err(ref e) if e.kind() == ErrorKind::Interrupted => None,
                    Err(e) => return Err(e),
                };
                if r.is_some() {
                    break;
                }
            }

            match self.conn.process_new_packets() {
                Ok(io) => debug!("{:#?}", io),
                Err(e) => return Err(Error::new(ErrorKind::Interrupted, e.to_string())),
            }

            if !self.conn.is_handshaking() && handshake && self.conn.wants_write() {
                continue;
            }

            match (eof, handshake, self.conn.is_handshaking()) {
                (_, true, false) => return Ok((read, write)),
                (_, false, _) => return Ok((read, write)),
                (true, true, true) => {
                    return Err(Error::new(ErrorKind::ConnectionAborted, "Not good!"))
                }
                (..) => debug!(
                    "eof?: {}, handshaking earlier?: {}, handshaking now?: {}",
                    eof,
                    handshake,
                    self.conn.is_handshaking()
                ),
            };
        }
    }
}
impl Read for TlsStream {
    // Reads once
    fn read(&mut self, buf: &mut [u8]) -> TLSResult<usize> {
        if self.conn.is_handshaking() {
            self.handshake()?;
        }
        if self.conn.wants_write() {
            self.handshake()?;
        }
        while self.conn.wants_read() {
            if self.handshake()?.0 == 0 {
                break;
            };
        }
        match self.conn.process_new_packets() {
            Ok(io) => {
                debug!(
                    "Bytes to read: {}, Bytes to write: {}, Closed?: {}",
                    io.plaintext_bytes_to_read(),
                    io.tls_bytes_to_write(),
                    io.peer_has_closed()
                );
            }
            Err(ref e) => return Err(Error::new(ErrorKind::ConnectionAborted, e.to_string())),
        };

        debug!("Finished reading");
        match self.conn.reader().read(buf) {
            Ok(u) => return Ok(u),
            Err(ref e) if e.kind() == ErrorKind::UnexpectedEof => return Ok(0),
            Err(e) => return Err(e),
        }
    }

    // Reads till EOF
    // Tweaked a bit to work nicely
    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> TLSResult<usize> {
        let mut tmp: [u8; 4096] = [0; 4096];
        let mut wrlen = 0;

        loop {
            match self.read(&mut tmp) {
                Ok(0) => {
                    debug!("EOF at read_to_end()");
                    break;
                }
                Ok(n) => {
                    if let Err(e) = buf.try_reserve_exact(n) {
                        error!("Failed allocation for Vec, due to an error: {}", e)
                    };
                    wrlen += n;
                    buf.extend_from_slice(&tmp[0..n]);
                }
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                    debug!("WouldBlock occured!");
                    break;
                }
                Err(e) => return Err(e),
            }
        }
        debug!("Finished reading to end");
        Ok(wrlen)
    }
}

impl Write for TlsStream {
    // Writes encrypted data to the socket
    fn write(&mut self, buf: &[u8]) -> TLSResult<usize> {
        if self.conn.is_handshaking() {
            self.handshake()?;
        };
        if self.conn.wants_write() {
            self.handshake()?;
        };

        let len = self.conn.writer().write(buf)?;
        self.conn.writer().flush()?;
        self.conn.write_tls(&mut self.buf_w)?;

        self.buf_w.flush()?;

        debug!("Finished writing");
        Ok(len)
    }
    // Flushes all buffers
    fn flush(&mut self) -> TLSResult<()> {
        self.handshake()?;
        self.conn.writer().flush()?;
        if self.conn.wants_write() {
            self.handshake()?;
        }
        Ok(())
    }
}
