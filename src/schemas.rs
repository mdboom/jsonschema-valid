use serde_json;
use serde_json::Value;

use format;
use format::FormatChecker;
use validators;
use validators::Validator;

pub trait Draft {
    fn get_validator(&self, key: &str) -> Option<Validator>;
    fn get_schema(&self) -> &'static Value;
    fn get_format_checker(&self, format: &str) -> Option<FormatChecker>;
    fn get_draft_number(&self) -> u8;
}

pub struct Draft7;

impl Draft for Draft7 {
    fn get_validator(&self, key: &str) -> Option<Validator> {
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

    fn get_schema(&self) -> &'static Value {
        lazy_static! {
            static ref DRAFT7: Value = serde_json::from_str(include_str!("draft7.json")).unwrap();
        }
        &DRAFT7
    }

    fn get_format_checker(&self, key: &str) -> Option<FormatChecker> {
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

    fn get_draft_number(&self) -> u8 {
        7
    }
}

pub struct Draft6;

impl Draft for Draft6 {
    fn get_validator(&self, key: &str) -> Option<Validator> {
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

    fn get_schema(&self) -> &'static Value {
        lazy_static! {
            static ref DRAFT6: Value = serde_json::from_str(include_str!("draft6.json")).unwrap();
        }
        &DRAFT6
    }

    fn get_format_checker(&self, key: &str) -> Option<FormatChecker> {
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

    fn get_draft_number(&self) -> u8 {
        6
    }
}

pub struct Draft4;

impl Draft for Draft4 {
    fn get_validator(&self, key: &str) -> Option<Validator> {
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

    fn get_schema(&self) -> &'static Value {
        lazy_static! {
            static ref DRAFT4: Value = serde_json::from_str(include_str!("draft4.json")).unwrap();
        }
        &DRAFT4
    }

    fn get_format_checker(&self, key: &str) -> Option<FormatChecker> {
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

    fn get_draft_number(&self) -> u8 {
        4
    }
}

pub fn draft_from_url(url: &str) -> Option<&'static dyn Draft> {
    match url {
        "http://json-schema.org/draft-07/schema" => Some(&Draft7),
        "http://json-schema.org/draft-06/schema" => Some(&Draft6),
        "http://json-schema.org/draft-04/schema" => Some(&Draft4),
        _ => None,
    }
}

pub fn draft_from_schema(schema: &Value) -> Option<&'static dyn Draft> {
    schema
        .as_object()
        .and_then(|x| x.get("$schema"))
        .and_then(Value::as_str)
        .and_then(|x| draft_from_url(x))
}
