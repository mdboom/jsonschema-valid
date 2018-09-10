use std::net::Ipv4Addr;
use std::net::Ipv6Addr;
use std::str::FromStr;

use chrono::datetime::DateTime;
use regex::Regex;
use url::{ Host, Url };

use context::Context;

pub type FormatChecker = fn(
    ctx: &Context,
    value: &str,
) -> bool;

pub fn format_email(_ctx: &Context, value: &str) -> bool {
    value.contains("@")
}

pub fn format_ipv4(_ctx: &Context, value: &str) -> bool {
    Ipv4Addr::from_str(value).is_ok()
}

pub fn format_ipv6(_ctx: &Context, value: &str) -> bool {
    Ipv6Addr::from_str(value).is_ok()
}

pub fn format_hostname(_ctx: &Context, value: &str) -> bool {
    Host::parse(value).is_ok()
}

pub fn format_uri(_ctx: &Context, value: &str) -> bool {
    Url::parse(value).is_ok()
}

pub fn format_uri_reference(_ctx: &Context, value: &str) -> bool {
    // TODO: This is not correct
    Url::parse(value).is_ok()
}

pub fn format_datetime(_ctx: &Context, value: &str) -> bool {
    DateTime::parse_from_rfc3339(value).is_ok()
}

pub fn format_regex(_ctx: &Context, value: &str) -> bool {
    Regex::new(value).is_ok()
}

pub fn format_date(_ctx: &Context, value: &str) -> bool {
    DateTime::parse_from_str(value, "%Y-%m-%d").is_ok()
}

pub fn format_time(_ctx: &Context, value: &str) -> bool {
    DateTime::parse_from_str(value, "%H:%M:%S").is_ok()
}
