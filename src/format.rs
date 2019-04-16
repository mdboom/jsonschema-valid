use std::net::Ipv4Addr;
use std::net::Ipv6Addr;
use std::str::FromStr;

use chrono::DateTime;
use iri_string;
use json_pointer;
use regex::Regex;
use url::{Host, Url};

use config::Config;

pub type FormatChecker = fn(cfg: &Config, value: &str) -> bool;

pub fn email(_cfg: &Config, value: &str) -> bool {
    value.contains('@')
}

pub fn ipv4(_cfg: &Config, value: &str) -> bool {
    Ipv4Addr::from_str(value).is_ok()
}

pub fn ipv6(_cfg: &Config, value: &str) -> bool {
    Ipv6Addr::from_str(value).is_ok()
}

pub fn hostname(_cfg: &Config, value: &str) -> bool {
    Host::parse(value).is_ok()
}

pub fn uri(_cfg: &Config, value: &str) -> bool {
    Url::parse(value).is_ok()
}

pub fn uri_reference(_cfg: &Config, _value: &str) -> bool {
    true
    // TODO: This is not correct
    // Url::parse(value).is_ok()
}

pub fn iri(_cfg: &Config, value: &str) -> bool {
    iri_string::AbsoluteIri::new(value).is_ok()
}

pub fn iri_reference(_cfg: &Config, value: &str) -> bool {
    iri_string::AbsoluteIri::new(value).is_ok()
}

pub fn datetime(_cfg: &Config, value: &str) -> bool {
    DateTime::parse_from_rfc3339(value).is_ok()
}

pub fn regex(_cfg: &Config, value: &str) -> bool {
    Regex::new(value).is_ok()
}

pub fn date(_cfg: &Config, value: &str) -> bool {
    DateTime::parse_from_str(value, "%Y-%m-%d").is_ok()
}

pub fn time(_cfg: &Config, value: &str) -> bool {
    DateTime::parse_from_str(value, "%H:%M:%S").is_ok()
}

pub fn json_pointer(_cfg: &Config, value: &str) -> bool {
    value.parse::<json_pointer::JsonPointer<_, _>>().is_ok()
}

pub fn uri_template(_cfg: &Config, _value: &str) -> bool {
    // It seems like pretty much anything can be a URI template
    true
}
