use std::iter;

use serde_json::{Map, Value};

use error::ValidationError;
use regex;

pub fn get_regex(pattern: &String) -> Result<regex::Regex, ValidationError> {
    match regex::Regex::new(pattern) {
        Ok(re) => Ok(re),
        Err(err) => match err {
            regex::Error::Syntax(msg) => Err(ValidationError::new(&msg)),
            regex::Error::CompiledTooBig(_) => Err(ValidationError::new("regex too big")),
            _ => Err(ValidationError::new("Unknown regular expression error")),
        },
    }
}

pub fn bool_to_object_schema<'a>(schema: &'a Value) -> &'a Value {
    lazy_static! {
        static ref EMPTY_SCHEMA: Value = Value::Object(Map::new());
        static ref INVERSE_SCHEMA: Value = json!({"not": {}});
    }

    match schema {
        Value::Bool(bool) => {
            if *bool {
                &EMPTY_SCHEMA
            } else {
                &INVERSE_SCHEMA
            }
        }
        _ => schema,
    }
}

pub fn iter_or_once<'a>(instance: &'a Value) -> Box<Iterator<Item = &'a Value> + 'a> {
    match instance {
        Value::Array(array) => Box::new(array.iter()),
        _ => Box::new(iter::once(instance)),
    }
}
