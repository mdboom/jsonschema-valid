use std::error::Error as StdError;
use std::fmt;
use std::iter::{empty, once};

use itertools::Itertools;
use serde_json::Value;

/// An error that can occur during validation.
#[derive(Default, Debug, Clone)]
pub struct ValidationError {
    /// The error message.
    pub msg: String,

    /// The JSON instance fragment that had the issue.
    pub instance: Option<serde_json::Value>,

    /// The JSON schema fragment that had the issue.
    pub schema: Option<serde_json::Value>,

    /// The path to the JSON instance fragment within the entire instance document.
    pub instance_path: Vec<String>,

    /// The path to the JSON schema fragment within the entire schema.
    pub schema_path: Vec<String>,
}

impl StdError for ValidationError {}

fn path_to_string(path: &[String]) -> String {
    if path.is_empty() {
        "/".to_string()
    } else {
        "/".to_owned() + &path.iter().rev().join("/")
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}", textwrap::fill(&self.msg, 78))?;

        if let Some(instance) = &self.instance {
            writeln!(
                f,
                "At instance path {}:",
                path_to_string(&self.instance_path)
            )?;

            let json_content =
                serde_json::to_string_pretty(&instance).unwrap_or_else(|_| "".to_string());
            writeln!(f, "{}", textwrap::indent(&json_content, "  "))?;
        }

        if let Some(schema) = &self.schema {
            writeln!(f, "At schema path {}:", path_to_string(&self.schema_path))?;

            let json_content =
                serde_json::to_string_pretty(&schema).unwrap_or_else(|_| "".to_string());
            writeln!(f, "{}", textwrap::indent(&json_content, "  "))?;

            if let Some(description) = schema.get("description").and_then(|x| x.as_str()) {
                writeln!(f, "Documentation for this node:")?;
                writeln!(f, "{}", textwrap::indent(description, "  "))?;
            };
        }

        Ok(())
    }
}

impl From<url::ParseError> for ValidationError {
    fn from(err: url::ParseError) -> ValidationError {
        ValidationError::new(&format!("Invalid URL: {:?}", err), None, None)
    }
}

/// Stores information about a single validation error.
impl ValidationError {
    /// Create a new validation error with the given error message.
    pub fn new(msg: &str, instance: Option<&Value>, schema: Option<&Value>) -> ValidationError {
        ValidationError {
            msg: String::from(msg),
            instance: instance.cloned(),
            schema: schema.cloned(),
            ..Default::default()
        }
    }

    /// Update the instance and schema context for the error.
    pub fn add_ctx(mut self, instance_context: String, schema_context: String) -> Self {
        self.instance_path.push(instance_context);
        self.schema_path.push(schema_context);
        self
    }

    /// Update the instance context for the error.
    pub fn instance_ctx(mut self, instance_context: String) -> Self {
        self.instance_path.push(instance_context);
        self
    }

    /// Update the schema context for the error.
    pub fn schema_ctx(mut self, schema_context: String) -> Self {
        self.schema_path.push(schema_context);
        self
    }
}

/// An `Iterator` over `ValidationError` objects. The main method by which
/// validation errors are returned to the user.
pub type ErrorIterator<'a> = Box<dyn Iterator<Item = ValidationError> + 'a>;

pub fn make_error<'a, O: Into<String>>(
    message: O,
    instance: Option<&Value>,
    schema: Option<&Value>,
) -> ErrorIterator<'a> {
    Box::new(once(ValidationError::new(
        &message.into(),
        instance,
        schema,
    )))
}

pub fn no_error<'a>() -> ErrorIterator<'a> {
    Box::new(empty())
}

#[cfg(test)]
mod tests {
    use crate::{schemas, Config};
    use serde_json::json;

    #[test]
    fn test_pretty_print_errors() {
        let schema = json!(
            { "properties": { "foo": { "type": "integer", "description": "HELLO" } } });
        let instance = json!({"foo": "string"});
        let cfg = Config::from_schema(&schema, Some(schemas::Draft::Draft6)).unwrap();
        let validation = cfg.validate(&instance);

        if let Err(errors) = validation {
            for error in errors {
                let formatted = format!("{}", error);
                println!("{}", formatted);
                assert!(error.instance_path == vec!("foo"));
                assert!(error.schema_path == vec!("type", "foo", "properties"));

                assert!(formatted.contains("At instance path /foo"));
                assert!(formatted.contains("At schema path /properties/foo/type"));
                assert!(formatted.contains("Invalid type"));
                assert!(formatted.contains("HELLO"));
            }
        }
    }
}
