use itertools::Itertools;
use serde_json::Value;

use std::error;
use std::fmt;
use std::io::prelude::*;
use url;

use context::Context;

#[derive(Default, Debug)]
pub struct ValidationError {
    msg: String,
    instance_path: Option<Vec<Value>>,
    schema_path: Option<Vec<Value>>,
}

fn simple_to_string(value: &Value) -> String {
    match value {
        Value::String(v) => v.as_str().to_string(),
        _ => value.to_string(),
    }
}

fn path_to_string(path: &[Value]) -> String {
    if path.is_empty() {
        ".".to_string()
    } else {
        path.iter().map(|x| simple_to_string(x)).join("/")
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let (Some(instance_path), Some(schema_path)) = (&self.instance_path, &self.schema_path) {
            write!(
                f,
                "At {} with schema at {}: {}",
                path_to_string(&instance_path),
                path_to_string(&schema_path),
                self.msg
            )
        } else if let Some(schema_path) = &self.schema_path {
            write!(
                f,
                "At schema {}: {}",
                path_to_string(&schema_path),
                self.msg
            )
        } else {
            write!(f, "{}", self.msg)
        }
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

/// Stores information about a single validation error.
impl ValidationError {
    pub fn new(msg: &str) -> ValidationError {
        ValidationError {
            msg: String::from(msg),
            ..Default::default()
        }
    }

    pub fn new_with_schema_context(msg: &str, schema_ctx: &Context) -> ValidationError {
        ValidationError {
            msg: String::from(msg),
            instance_path: None,
            schema_path: Some(schema_ctx.flatten()),
        }
    }

    pub fn new_with_context(
        msg: &str,
        instance_ctx: &Context,
        schema_ctx: &Context,
    ) -> ValidationError {
        ValidationError {
            msg: String::from(msg),
            instance_path: Some(instance_ctx.flatten()),
            schema_path: Some(schema_ctx.flatten()),
        }
    }
}

pub trait ErrorRecorder {
    fn record_error(&mut self, error: ValidationError) -> Option<()>;
    fn has_errors(&self) -> bool;
}

/// Stores the ValidationErrors from a validation run.
#[derive(Default)]
pub struct ValidationErrors {
    errors: Vec<ValidationError>,
}

impl ErrorRecorder for ValidationErrors {
    fn record_error(&mut self, error: ValidationError) -> Option<()> {
        self.errors.push(error);
        Some(())
    }

    fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

impl ValidationErrors {
    pub fn new() -> ValidationErrors {
        ValidationErrors {
            ..Default::default()
        }
    }

    pub fn get_errors(&self) -> &[ValidationError] {
        &self.errors
    }
}

#[derive(Default)]
pub struct FastFailErrorRecorder {
    error: Option<ValidationError>,
}

impl ErrorRecorder for FastFailErrorRecorder {
    fn record_error(&mut self, err: ValidationError) -> Option<()> {
        self.error = Some(err);
        None
    }

    fn has_errors(&self) -> bool {
        self.error.is_some()
    }
}

impl FastFailErrorRecorder {
    pub fn new() -> FastFailErrorRecorder {
        FastFailErrorRecorder {
            ..Default::default()
        }
    }
}

pub struct ErrorRecorderStream<'a> {
    stream: &'a mut Write,
    has_error: bool,
}

impl<'a> ErrorRecorder for ErrorRecorderStream<'a> {
    fn record_error(&mut self, err: ValidationError) -> Option<()> {
        self.has_error = true;
        if writeln!(self.stream, "{}", err.to_string()).is_err() {
            None
        } else {
            Some(())
        }
    }

    fn has_errors(&self) -> bool {
        self.has_error
    }
}

impl<'a> ErrorRecorderStream<'a> {
    pub fn new(stream: &'a mut Write) -> ErrorRecorderStream<'a> {
        ErrorRecorderStream {
            stream,
            has_error: false,
        }
    }
}
