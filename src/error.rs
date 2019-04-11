use itertools::{join, Itertools};
use serde_json::Value;
use std::error;
use std::fmt;
use url;

pub struct ScopeStack<'a> {
    pub x: &'a Value,
    pub parent: Option<&'a ScopeStack<'a>>,
}

#[derive(Default, Debug)]
pub struct ValidationError {
    msg: String,
    instance_path: Vec<String>,
    schema_path: Vec<String>,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let instance_path = self.instance_path.iter().rev().join("/");
        let schema_path = self.schema_path.iter().rev().join("/");
        write!(
            f,
            "At {} in schema {}: {}",
            instance_path, schema_path, self.msg
        )
    }
}

impl error::Error for ValidationError {
    fn cause(&self) -> Option<&error::Error> {
        None
    }
}

impl From<url::ParseError> for ValidationError {
    fn from(err: url::ParseError) -> ValidationError {
        ValidationError::new(&format!("Invalid URL: {:?}", err))
    }
}

impl ValidationError {
    pub fn new(msg: &str) -> ValidationError {
        ValidationError {
            msg: String::from(msg),
            ..Default::default()
        }
    }

    pub fn from_errors(
        msg: &str,
        errors: &[ValidationError],
        _stack: &ScopeStack,
    ) -> ValidationError {
        ValidationError {
            msg: format!(
                "{}: [{}\n]",
                msg,
                join(errors.iter().map(|x| x.msg.as_str()), "\n    ")
            ),
            ..Default::default()
        }
    }
}

pub trait ErrorRecorder {
    fn record_error(&mut self, error: ValidationError);
    fn has_errors(&self) -> bool;
}

#[derive(Default)]
pub struct VecErrorRecorder {
    errors: Vec<ValidationError>,
}

impl ErrorRecorder for VecErrorRecorder {
    fn record_error(&mut self, error: ValidationError) {
        self.errors.push(error)
    }

    fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

impl VecErrorRecorder {
    pub fn new() -> VecErrorRecorder {
        VecErrorRecorder {
            ..Default::default()
        }
    }
}
