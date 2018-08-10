use std::collections::HashSet;
use std::hash::Hash;
use std::hash::Hasher;

use serde_json::{Value};

struct ValueWrapper<'a> {
  x: &'a Value
}

impl<'a> Hash for ValueWrapper<'a> {
  fn hash<H: Hasher>(&self, state: &mut H) {
    match self.x {
      Value::Array(array) =>
        for element in array {
          ValueWrapper { x: element }.hash(state);
        },
      Value::Object(object) =>
        for (key, val) in object {
          key.hash(state);
          ValueWrapper { x: val }.hash(state);
        },
      Value::String(string) => string.hash(state),
      Value::Number(number) => {
        if number.is_f64() {
          number.as_f64().unwrap().to_bits().hash(state);
        } else if number.is_u64() {
          number.as_u64().unwrap().hash(state);
        } else {
          number.as_i64().unwrap().hash(state);
        }
      },
      Value::Bool(bool) => bool.hash(state),
      Value::Null => 0.hash(state)
    }
  }
}

impl<'a> PartialEq for ValueWrapper<'a> {
  fn eq(&self, other: &ValueWrapper<'a>) -> bool {
    self.x == other.x
  }
}

impl<'a> Eq for ValueWrapper<'a> {}

pub fn has_unique_elements(iter: &mut Iterator<Item=&Value>) -> bool
{
  let mut uniq = HashSet::new();
  iter
    .map(|x| ValueWrapper {x: &x})
    .all(move |x| uniq.insert(x))
}
