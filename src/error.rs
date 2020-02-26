use std::fmt;
use std::iter::{empty, once};
use url;

/// An error that can occur during validation.
///
/// It holds:
///
/// * a message describing the validation failure.
/// * An optional path to the field in the data where the validation failure occured.
/// * An optional path to the item in the schema that caused the validation failure.
#[derive(Default, Debug, Clone)]
pub struct ValidationError {
    msg: String,
    instance_path: Vec<String>,
    schema_path: Vec<String>,
}

fn path_to_string(path: &[String]) -> String {
    if path.is_empty() {
        "/".to_string()
    } else {
        "/".to_owned() + &path.join("/")
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "At {} with schema at {}: {}",
            path_to_string(&self.instance_path),
            path_to_string(&self.schema_path),
            self.msg
        )
    }
}

impl From<url::ParseError> for ValidationError {
    fn from(err: url::ParseError) -> ValidationError {
        ValidationError::new(&format!("Invalid URL: {:?}", err))
    }
}

/// Stores information about a single validation error.
impl ValidationError {
    /// Create a new validation error with the given error message.
    pub fn new(msg: &str) -> ValidationError {
        ValidationError {
            msg: String::from(msg),
            ..Default::default()
        }
    }

    /// Create a new validation error with the given error message,
    /// providing context for the schema and data.
    pub fn add_ctx(mut self, instance_context: String, schema_context: String) -> Self {
        self.instance_path.push(instance_context);
        self.schema_path.push(schema_context);
        self
    }

    /// Create a new validation error with the given error message, providing
    /// the instance context.
    pub fn instance_ctx(mut self, instance_context: String) -> Self {
        self.instance_path.push(instance_context);
        self
    }

    /// Create a new validation error with the given error message, providing
    /// the schema context.
    pub fn schema_ctx(mut self, schema_context: String) -> Self {
        self.schema_path.push(schema_context);
        self
    }
}

/// An `Iterator` over `ValidationError` objects. The main method by which
/// validation errors are returned to the user.
pub type ErrorIterator<'a> = Box<dyn Iterator<Item = ValidationError> + 'a>;

pub fn make_error<'a, O: Into<String>>(message: O) -> ErrorIterator<'a> {
    Box::new(once(ValidationError::new(&message.into())))
}

pub fn no_error<'a>() -> ErrorIterator<'a> {
    Box::new(empty())
}
