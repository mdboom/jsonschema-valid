/// Utility to determine whether a JSON array has all unique elements.
use std::collections::HashSet;
use std::hash::Hash;
use std::hash::Hasher;

use serde_json::Value;

struct ValueWrapper<'a> {
    x: &'a Value,
}

impl<'a> Hash for ValueWrapper<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self.x {
            Value::Array(array) => {
                0.hash(state);
                for element in array {
                    ValueWrapper { x: element }.hash(state);
                }
            }
            Value::Object(object) => {
                1.hash(state);
                for (key, val) in object {
                    key.hash(state);
                    ValueWrapper { x: val }.hash(state);
                }
            }
            Value::String(string) => {
                2.hash(state);
                string.hash(state)
            }
            Value::Number(number) => {
                if number.is_f64() {
                    3.hash(state);
                    number.as_f64().unwrap().to_bits().hash(state);
                } else if number.is_u64() {
                    4.hash(state);
                    number.as_u64().unwrap().hash(state);
                } else {
                    5.hash(state);
                    number.as_i64().unwrap().hash(state);
                }
            }
            Value::Bool(bool) => {
                6.hash(state);
                bool.hash(state)
            }
            Value::Null => 0.hash(state),
        }
    }
}

impl<'a> PartialEq for ValueWrapper<'a> {
    fn eq(&self, other: &ValueWrapper<'a>) -> bool {
        self.x == other.x
    }
}

impl<'a> Eq for ValueWrapper<'a> {}

pub fn has_unique_elements(iter: &mut Iterator<Item = &Value>) -> bool {
    let mut uniq = HashSet::new();
    iter.map(|x| ValueWrapper { x: &x })
        .all(move |x| uniq.insert(x))
}
