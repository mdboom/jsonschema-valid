use serde_json::Value;

use std::error;
use std::fmt;
use url;

use context::Context;

#[derive(Default, Debug)]
pub struct ValidationError {
    msg: String,
    instance_path: Option<Vec<Value>>,
    schema_path: Option<Vec<Value>>,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // let instance_path = self.instance_path.iter().rev().join("/");
        // let schema_path = self.schema_path.iter().rev().join("/");
        // write!(
        //     f,
        //     "At {} in schema {}: {}",
        //     instance_path, schema_path, self.msg
        // )
        write!(f, "{}", self.msg)
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

    // pub fn from_errors(
    //     msg: &str,
    //     errors: &[ValidationError],
    //     _stack: &ScopeStack,
    // ) -> ValidationError {
    //     ValidationError {
    //         msg: format!(
    //             "{}: [{}\n]",
    //             msg,
    //             join(errors.iter().map(|x| x.msg.as_str()), "\n    ")
    //         ),
    //         ..Default::default()
    //     }
    // }
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
    error: Option<ValidationError>
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
