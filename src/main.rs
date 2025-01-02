mod https;
mod tls;
use crate::https::response::Response;
use crate::https::url::Url;
use https::client::HttpsClient;
use https::persistent_client::PersistentClient;
use log::{Level, Metadata, Record};
use std::collections::HashMap;

struct Logger;
const LOGGER: Logger = Logger;
impl log::Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Debug
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            println!("[{}] {}", record.level(), record.args());
        }
    }

    fn flush(&self) {}
}

fn main() {
    // Init logger
    log::set_logger(&LOGGER)
        .map(|()| log::set_max_level(log::LevelFilter::Debug))
        .unwrap();

    // headers
    let mut headers: HashMap<&str, &str> = HashMap::new();
    headers.insert("Connection", "close");
    headers.insert("Accept-Encoding", "identity");
    headers.insert("Accept", "*/*");

    let mut client =
        PersistentClient::new("Verdun/0.2.2", Some(headers), "https://www.rust-lang.org/").unwrap();
    let first = client
        .get("https://prev.rust-lang.org/en-US/", None)
        .unwrap();

    println!("{:#?}", Response::from_slice(&first).unwrap());
}
