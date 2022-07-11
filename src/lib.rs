//! # jsonschema-valid
//!
//! A simple crate to perform [JSON Schema](https://json-schema.org/) validation.
//!
//! Supports JSON Schema drafts 4, 6, and 7.
//!
//! ## Example:
//!
//! The following example validates some JSON data against a draft 6 JSON schema.
//!
//! ```rust
//! # fn main() -> Result<(), Box<dyn std::error::Error + 'static>> {
//! # use serde_json::Value;
//! # use jsonschema_valid::schemas;
//! # let schema_json = "{}";
//! # let your_json_data = "{}";
//! let schema: Value = serde_json::from_str(schema_json)?;
//! let data: Value = serde_json::from_str(your_json_data)?;
//! let cfg = jsonschema_valid::Config::from_schema(&schema, Some(schemas::Draft::Draft6))?;
//! // Validate the schema itself
//! assert!(cfg.validate_schema().is_ok());
//! // Validate a JSON instance against the schema
//! assert!(cfg.validate(&data).is_ok());
//!
//! # Ok(()) }
//! ````

#![warn(missing_docs)]

use serde_json::Value;

mod config;
mod context;
mod error;
mod format;
mod resolver;
pub mod schemas;
mod unique;
mod util;
mod validators;

pub use crate::config::Config;
use crate::context::Context;
pub use crate::error::{ErrorIterator, ValidationError};

/// Validates a given JSON instance against a given JSON schema, returning the
/// errors, if any. draft may provide the schema draft to use. If not provided,
/// it will be determined automatically from the schema.
///
/// # Arguments
///
/// * `cfg`: The configuration object to use
/// * `instance`: The JSON document to validate
///
/// # Returns
///
/// * `errors`: A `Result` indicating whether there were any validation errors.
///   If `Ok(())`, the `instance` is valid against `schema`. If `Err(e)`, `e` is
///   an iterator over all of the validation errors.
///
/// ## Example:
///
/// The following example validates some JSON data against a draft 6 JSON schema.
///
/// ```rust
/// # fn main() -> Result<(), Box<dyn std::error::Error + 'static>> {
/// # use serde_json::Value;
/// # use jsonschema_valid::{schemas, Config};
/// # let schema_json = "{\"type\": \"integer\"}";
/// # let your_json_data = "\"string\"";
/// let schema: Value = serde_json::from_str(schema_json)?;
/// let data: Value = serde_json::from_str(your_json_data)?;
/// let cfg = jsonschema_valid::Config::from_schema(&schema, Some(schemas::Draft::Draft6))?;
///
/// let mut validation = jsonschema_valid::validate(&cfg, &data);
/// if let Err(errors) = validation {
///     for error in errors {
///         println!("Error: {}", error);
///     }
/// }
///
/// # Ok(()) }
/// ````
pub fn validate<'a>(
    cfg: &'a config::Config<'a>,
    instance: &'a Value,
) -> Result<(), ErrorIterator<'a>> {
    let mut errors = validators::descend(
        cfg,
        instance,
        cfg.get_schema(),
        None,
        Context::new_from(cfg.get_schema()),
    )
    .peekable();

    if errors.peek().is_none() {
        Ok(())
    } else {
        Err(Box::new(errors))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs;
    use std::path::PathBuf;

    // Test files we know will fail.
    const KNOWN_FAILURES: &[&str] = &["refRemote.json"];

    fn test_draft(dirname: &str, draft: schemas::Draft) {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("JSON-Schema-Test-Suite/tests");
        path.push(dirname);

        let paths = fs::read_dir(path).unwrap();

        for entry in paths {
            let dir_entry = &entry.unwrap();
            if KNOWN_FAILURES.contains(&dir_entry.file_name().to_str().unwrap()) {
                continue;
            }

            let path = dir_entry.path();
            if path.extension().map_or_else(|| "", |x| x.to_str().unwrap()) == "json" {
                println!("Testing {:?}", path.display());
                let file = fs::File::open(path).unwrap();
                let json: Value = serde_json::from_reader(file).unwrap();
                for testset in json.as_array().unwrap().iter() {
                    println!(
                        "  Test set {}",
                        testset.get("description").unwrap().as_str().unwrap()
                    );
                    let schema = testset.get("schema").unwrap();
                    let tests = testset.get("tests").unwrap();
                    for test in tests.as_array().unwrap().iter() {
                        println!(
                            "    Test {}",
                            test.get("description").unwrap().as_str().unwrap()
                        );
                        let data = test.get("data").unwrap();
                        let valid = test.get("valid").unwrap();
                        if let Value::Bool(expected_valid) = valid {
                            let cfg = config::Config::from_schema(schema, Some(draft)).unwrap();
                            assert!(cfg.validate_schema().is_ok());
                            let result = validate(&cfg, data);
                            assert_eq!(result.is_ok(), *expected_valid);
                            let cfg2 = config::Config::from_schema(schema, Some(draft)).unwrap();
                            let result2 = cfg2.validate(data);
                            assert!(cfg2.validate_schema().is_ok());
                            assert_eq!(result2.is_ok(), *expected_valid);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_draft7() {
        test_draft("draft7", schemas::Draft::Draft7);
    }

    #[test]
    fn test_draft6() {
        test_draft("draft6", schemas::Draft::Draft6);
    }

    #[test]
    fn test_draft4() {
        test_draft("draft4", schemas::Draft::Draft4);
    }
}
