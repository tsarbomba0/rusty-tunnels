use regex::Regex;
use std::fmt::{Debug, Formatter};
// parsing urls.

pub struct Url<'a> {
    route: &'a str,
    domain: &'a str,
    scheme: &'a str,
    port: u16,
    query: &'a str,
}

impl Debug for Url<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Url")
            .field("route", &self.route)
            .field("domain", &self.domain)
            .field("scheme", &self.scheme)
            .field("port", &self.port)
            .field("query", &self.query)
            .finish()
    }
}

impl<'a> Url<'a> {
    pub fn new(u: &'a str) -> Result<Url<'a>, Box<dyn std::error::Error>> {
        let reg =
            Regex::new(r"(?<scheme>.*?)://(?<domain>.*?[^\\]?)(?<route>/.*?[^\?]*)(?<query>.*)")?;
        let matches = match reg.captures(u) {
            None => return Err("Invalid URL")?,
            Some(o) => o,
        };

        let scheme = match &matches.name("scheme") {
            None => return Err("Schema not found, url invalid.")?,
            Some(o) => o.as_str(),
        };
        let route = match &matches.name("route") {
            None => "/",
            Some(o) => o.as_str(),
        };
        let domain = match &matches.name("domain") {
            None => return Err("Domain not found!")?,
            Some(o) => o.as_str(),
        };
        let query = &matches.name("query").map_or("", |o| o.as_str());
        let port = match scheme {
            "http" => 80,
            "https" => 443,
            "ftp" => 21,
            _ => 1919, // IDK
        };

        Ok(Url {
            route,
            domain,
            scheme,
            port,
            query,
        })
    }
    pub fn socket_addr(&self) -> String {
        format!("{}:{}", self.domain, self.port)
    }
    pub fn route(&self) -> &'a str {
        self.route
    }
    pub fn domain(&self) -> &'a str {
        self.domain
    }
    pub fn query(&self) -> &'a str {
        self.query
    }
    pub fn scheme(&self) -> &'a str {
        self.scheme
    }
    pub fn port(&self) -> u16 {
        self.port
    }
}
