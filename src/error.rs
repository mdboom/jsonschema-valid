use std::fmt;
use std::fmt::Debug;

use itertools::Itertools;

#[derive(Default)]
pub struct ValidationError {
  msg: String,
  instance_path: Vec<String>,
  schema_path: Vec<String>
}

impl Debug for ValidationError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    let instance_path = self.instance_path.iter().rev().join("/");
    let schema_path = self.schema_path.iter().rev().join("/");
    write!(f, "At {} in schema {}: {}",
           instance_path,
           schema_path,
           self.msg)
  }
}

impl ValidationError {
  pub fn new(msg: &str) -> ValidationError {
    ValidationError {
      msg: String::from(msg),
      ..Default::default()
    }
  }

  pub fn add_instance_path(&mut self, path: &str) {
    self.instance_path.push(String::from(path));
  }

  pub fn add_schema_path(&mut self, path: &str) {
    self.schema_path.push(String::from(path));
  }
}
