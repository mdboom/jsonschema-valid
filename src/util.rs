use std::iter;

use serde_json::{Map, Value};

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

pub fn iter_or_once<'a>(instance: &'a Value) -> Box<Iterator<Item = &'a Value> + 'a> {
    match instance {
        Value::Array(array) => Box::new(array.iter()),
        _ => Box::new(iter::once(instance)),
    }
}
