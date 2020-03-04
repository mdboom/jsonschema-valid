use std::iter;

use itertools::Itertools;
use lazy_static::lazy_static;
use serde_json::{json, Map, Value};

pub fn bool_to_object_schema(schema: &Value) -> &Value {
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

pub fn iter_or_once<'a>(instance: &'a Value) -> Box<dyn Iterator<Item = &'a Value> + 'a> {
    match instance {
        Value::Array(array) => Box::new(array.iter()),
        _ => Box::new(iter::once(instance)),
    }
}

pub fn format_list<'a, T: Iterator<Item = &'a str>>(iter: &mut T) -> String {
    iter.map(|x| format!("\"{}\"", x)).join(", ")
}
