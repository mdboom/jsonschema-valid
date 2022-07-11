//! Implementations of the different drafts of JSON schema.
//!

use lazy_static::lazy_static;
use serde_json::Value;

use crate::format;
use crate::format::FormatChecker;
use crate::validators;
use crate::validators::Validator;

/// The validator can validate JSON data against different versions of JSON Schema.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Draft {
    /// JSONSchema [Draft 4](https://json-schema.org/specification-links.html#draft-4)
    Draft4,

    /// JSONSchema [Draft 6](https://json-schema.org/specification-links.html#draft-6)
    Draft6,

    /// JSONSchema [Draft 7](https://json-schema.org/specification-links.html#draft-7)
    Draft7,
}

impl Draft {
    pub(crate) fn get_validator(self, key: &str) -> Option<Validator> {
        match self {
            Draft::Draft4 => draft4::get_validator(key),
            Draft::Draft6 => draft6::get_validator(key),
            Draft::Draft7 => draft7::get_validator(key),
        }
    }

    /// Get the JSON representation of the schema document.
    pub fn get_schema(self) -> &'static Value {
        match self {
            Draft::Draft7 => draft7::get_schema(),
            Draft::Draft6 => draft6::get_schema(),
            Draft::Draft4 => draft4::get_schema(),
        }
    }

    /// Get a format check function.
    pub(crate) fn get_format_checker(self, format: &str) -> Option<FormatChecker> {
        match self {
            Draft::Draft4 => draft4::get_format_checker(format),
            Draft::Draft6 => draft6::get_format_checker(format),
            Draft::Draft7 => draft7::get_format_checker(format),
        }
    }

    /// Return the draft's number.
    pub fn get_draft_number(self) -> u8 {
        match self {
            Draft::Draft4 => 4,
            Draft::Draft6 => 6,
            Draft::Draft7 => 7,
        }
    }
}

mod draft7 {
    use super::*;

    pub(super) fn get_validator(key: &str) -> Option<Validator> {
        match key {
            "$ref" => Some(validators::ref_ as Validator),
            "additionalItems" => Some(validators::additionalItems as Validator),
            "additionalProperties" => Some(validators::additionalProperties as Validator),
            "allOf" => Some(validators::allOf as Validator),
            "anyOf" => Some(validators::anyOf as Validator),
            "const" => Some(validators::const_ as Validator),
            "contains" => Some(validators::contains as Validator),
            "dependencies" => Some(validators::dependencies as Validator),
            "enum" => Some(validators::enum_ as Validator),
            "exclusiveMaximum" => Some(validators::exclusiveMaximum as Validator),
            "exclusiveMinimum" => Some(validators::exclusiveMinimum as Validator),
            "format" => Some(validators::format as Validator),
            "if" => Some(validators::if_ as Validator),
            "items" => Some(validators::items as Validator),
            "maxItems" => Some(validators::maxItems as Validator),
            "maxLength" => Some(validators::maxLength as Validator),
            "maxProperties" => Some(validators::maxProperties as Validator),
            "maximum" => Some(validators::maximum as Validator),
            "minItems" => Some(validators::minItems as Validator),
            "minLength" => Some(validators::minLength as Validator),
            "minProperties" => Some(validators::minProperties as Validator),
            "minimum" => Some(validators::minimum as Validator),
            "multipleOf" => Some(validators::multipleOf as Validator),
            "not" => Some(validators::not as Validator),
            "oneOf" => Some(validators::oneOf as Validator),
            "pattern" => Some(validators::pattern as Validator),
            "patternProperties" => Some(validators::patternProperties as Validator),
            "properties" => Some(validators::properties as Validator),
            "propertyNames" => Some(validators::propertyNames as Validator),
            "required" => Some(validators::required as Validator),
            "type" => Some(validators::type_ as Validator),
            "uniqueItems" => Some(validators::uniqueItems as Validator),
            _ => None,
        }
    }

    pub(super) fn get_schema() -> &'static Value {
        lazy_static! {
            static ref DRAFT7: Value = serde_json::from_str(include_str!("draft7.json")).unwrap();
        }
        &DRAFT7
    }

    pub(super) fn get_format_checker(key: &str) -> Option<FormatChecker> {
        match key {
            "date" => Some(format::date as FormatChecker),
            "date-time" => Some(format::datetime as FormatChecker),
            "email" => Some(format::email as FormatChecker),
            "hostname" => Some(format::hostname as FormatChecker),
            "idn-email" => Some(format::email as FormatChecker),
            "ipv4" => Some(format::ipv4 as FormatChecker),
            "ipv6" => Some(format::ipv6 as FormatChecker),
            "iri" => Some(format::iri as FormatChecker),
            "iri-reference" => Some(format::iri_reference as FormatChecker),
            "json-pointer" => Some(format::json_pointer as FormatChecker),
            "regex" => Some(format::regex as FormatChecker),
            "time" => Some(format::time as FormatChecker),
            "uri" => Some(format::uri as FormatChecker),
            "uri-reference" => Some(format::uri_reference as FormatChecker),
            "uri-template" => Some(format::uri_template as FormatChecker),
            _ => None,
        }
    }
}

mod draft6 {
    use super::*;

    pub(super) fn get_validator(key: &str) -> Option<Validator> {
        match key {
            "$ref" => Some(validators::ref_ as Validator),
            "additionalItems" => Some(validators::additionalItems as Validator),
            "additionalProperties" => Some(validators::additionalProperties as Validator),
            "allOf" => Some(validators::allOf as Validator),
            "anyOf" => Some(validators::anyOf as Validator),
            "const" => Some(validators::const_ as Validator),
            "contains" => Some(validators::contains as Validator),
            "dependencies" => Some(validators::dependencies as Validator),
            "enum" => Some(validators::enum_ as Validator),
            "exclusiveMaximum" => Some(validators::exclusiveMaximum as Validator),
            "exclusiveMinimum" => Some(validators::exclusiveMinimum as Validator),
            "format" => Some(validators::format as Validator),
            "items" => Some(validators::items as Validator),
            "maxItems" => Some(validators::maxItems as Validator),
            "maxLength" => Some(validators::maxLength as Validator),
            "maxProperties" => Some(validators::maxProperties as Validator),
            "maximum" => Some(validators::maximum as Validator),
            "minItems" => Some(validators::minItems as Validator),
            "minLength" => Some(validators::minLength as Validator),
            "minProperties" => Some(validators::minProperties as Validator),
            "minimum" => Some(validators::minimum as Validator),
            "multipleOf" => Some(validators::multipleOf as Validator),
            "not" => Some(validators::not as Validator),
            "oneOf" => Some(validators::oneOf as Validator),
            "pattern" => Some(validators::pattern as Validator),
            "patternProperties" => Some(validators::patternProperties as Validator),
            "properties" => Some(validators::properties as Validator),
            "propertyNames" => Some(validators::propertyNames as Validator),
            "required" => Some(validators::required as Validator),
            "type" => Some(validators::type_ as Validator),
            "uniqueItems" => Some(validators::uniqueItems as Validator),
            _ => None,
        }
    }

    pub(super) fn get_schema() -> &'static Value {
        lazy_static! {
            static ref DRAFT6: Value = serde_json::from_str(include_str!("draft6.json")).unwrap();
        }
        &DRAFT6
    }

    pub(super) fn get_format_checker(key: &str) -> Option<FormatChecker> {
        match key {
            "date" => Some(format::date as FormatChecker),
            "date-time" => Some(format::datetime as FormatChecker),
            "email" => Some(format::email as FormatChecker),
            "hostname" => Some(format::hostname as FormatChecker),
            "ipv4" => Some(format::ipv4 as FormatChecker),
            "ipv6" => Some(format::ipv6 as FormatChecker),
            "json-pointer" => Some(format::json_pointer as FormatChecker),
            "regex" => Some(format::regex as FormatChecker),
            "time" => Some(format::time as FormatChecker),
            "uri" => Some(format::uri as FormatChecker),
            "uri-reference" => Some(format::uri_reference as FormatChecker),
            "uri-template" => Some(format::uri_template as FormatChecker),
            _ => None,
        }
    }
}

mod draft4 {
    use super::*;

    pub(super) fn get_validator(key: &str) -> Option<Validator> {
        match key {
            "$ref" => Some(validators::ref_ as Validator),
            "additionalItems" => Some(validators::additionalItems as Validator),
            "additionalProperties" => Some(validators::additionalProperties as Validator),
            "allOf" => Some(validators::allOf as Validator),
            "anyOf" => Some(validators::anyOf as Validator),
            "dependencies" => Some(validators::dependencies as Validator),
            "enum" => Some(validators::enum_ as Validator),
            "format" => Some(validators::format as Validator),
            "items" => Some(validators::items as Validator),
            "maxItems" => Some(validators::maxItems as Validator),
            "maxLength" => Some(validators::maxLength as Validator),
            "maxProperties" => Some(validators::maxProperties as Validator),
            "maximum" => Some(validators::maximum_draft4 as Validator),
            "minItems" => Some(validators::minItems as Validator),
            "minLength" => Some(validators::minLength as Validator),
            "minProperties" => Some(validators::minProperties as Validator),
            "minimum" => Some(validators::minimum_draft4 as Validator),
            "multipleOf" => Some(validators::multipleOf as Validator),
            "not" => Some(validators::not as Validator),
            "oneOf" => Some(validators::oneOf as Validator),
            "pattern" => Some(validators::pattern as Validator),
            "patternProperties" => Some(validators::patternProperties as Validator),
            "properties" => Some(validators::properties as Validator),
            "required" => Some(validators::required as Validator),
            "type" => Some(validators::type_ as Validator),
            "uniqueItems" => Some(validators::uniqueItems as Validator),
            _ => None,
        }
    }

    pub(super) fn get_schema() -> &'static Value {
        lazy_static! {
            static ref DRAFT4: Value = serde_json::from_str(include_str!("draft4.json")).unwrap();
        }
        &DRAFT4
    }

    pub(super) fn get_format_checker(key: &str) -> Option<FormatChecker> {
        match key {
            "date-time" => Some(format::datetime as FormatChecker),
            "email" => Some(format::email as FormatChecker),
            "hostname" => Some(format::hostname as FormatChecker),
            "ipv4" => Some(format::ipv4 as FormatChecker),
            "ipv6" => Some(format::ipv6 as FormatChecker),
            "regex" => Some(format::regex as FormatChecker),
            "uri" => Some(format::uri as FormatChecker),
            _ => None,
        }
    }
}

/// Get the `Draft` from a JSON Schema URL.
pub fn draft_from_url(url: &str) -> Option<Draft> {
    match url {
        "http://json-schema.org/draft-07/schema" => Some(Draft::Draft7),
        "http://json-schema.org/draft-06/schema" => Some(Draft::Draft6),
        "http://json-schema.org/draft-04/schema" => Some(Draft::Draft4),
        _ => None,
    }
}

/// Get the `Draft` from a JSON Schema.
pub fn draft_from_schema(schema: &Value) -> Option<Draft> {
    schema
        .as_object()
        .and_then(|x| x.get("$schema"))
        .and_then(Value::as_str)
        .and_then(draft_from_url)
}
