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
//! let cfg = jsonschema_valid::Config::from_schema(&schema, Some(&schemas::Draft6)).unwrap();
//!
//! assert!(jsonschema_valid::is_valid(&cfg, &data, &schema, false));
//!
//! # Ok(()) }
//! ````

#![warn(missing_docs)]

use std::iter::empty;

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
/// * `schema`: The JSON schema to validate against
/// * `validate_schema`: When `true`, validate the schema against the metaschema
///   first.
///
/// # Returns
///
/// * `errors`: An `Iterator` of `ValidationError` found during validation.
///
/// ## Example:
///
/// The following example validates some JSON data against a draft 6 JSON schema.
///
/// ```rust
/// # fn main() -> Result<(), Box<dyn std::error::Error + 'static>> {
/// # use serde_json::Value;
/// # use jsonschema_valid::{schemas, Config};
/// # let schema_json = "{}";
/// # let your_json_data = "{}";
/// let schema: Value = serde_json::from_str(schema_json)?;
/// let data: Value = serde_json::from_str(your_json_data)?;
/// let cfg = jsonschema_valid::Config::from_schema(&schema, Some(&schemas::Draft6)).unwrap();
///
/// let mut validation = jsonschema_valid::validate(&cfg, &data, &schema, false);
/// assert!(!validation.next().is_some());
///
/// # Ok(()) }
/// ````
pub fn validate<'a>(
    cfg: &'a config::Config<'a>,
    instance: &'a Value,
    schema: &'a Value,
    validate_schema: bool,
) -> ErrorIterator<'a> {
    Box::new(
        if validate_schema {
            validators::descend(
                cfg,
                schema,
                cfg.get_metaschema(),
                None,
                Context::new_from(cfg.get_metaschema()),
            )
        } else {
            Box::new(empty())
        }
        .chain(validators::descend(
            cfg,
            instance,
            schema,
            None,
            Context::new_from(schema),
        )),
    )
}

/// Validates a given JSON instance against a given JSON schema, returning true
/// if valid. This function is more efficient than [validate](fn.validate.html)
/// or [validate_to_stream](fn.validate_to_stream.html), because it stops at the
/// first error.
///
/// # Arguments
///
/// * `cfg`: The configuration object to use
/// * `instance`: The JSON document to validate
/// * `schema`: The JSON schema to validate against
/// * `validate_schema`: When `true`, validate the schema against the metaschema
///   first.
///
/// # Returns
///
/// * `true`: `instance` is valid against `schema`.
///
/// ## Example:
///
/// The following example validates some JSON data against a draft 6 JSON schema.
///
/// ```rust
/// # fn main() -> Result<(), Box<dyn std::error::Error + 'static>> {
/// # use serde_json::Value;
/// # use jsonschema_valid::schemas;
/// # let schema_json = "{}";
/// # let your_json_data = "{}";
/// let schema: Value = serde_json::from_str(schema_json)?;
/// let data: Value = serde_json::from_str(your_json_data)?;
/// let cfg = jsonschema_valid::Config::from_schema(&schema, Some(&schemas::Draft6)).unwrap();
///
/// assert!(jsonschema_valid::is_valid(&cfg, &data, &schema, false));
///
/// # Ok(()) }
/// ````
pub fn is_valid<'a>(
    cfg: &'a config::Config<'a>,
    instance: &Value,
    schema: &Value,
    validate_schema: bool,
) -> bool {
    validate(cfg, instance, schema, validate_schema)
        .next()
        .is_none()
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs;
    use std::path::PathBuf;

    // Test files we know will fail.
    const KNOWN_FAILURES: &'static [&'static str] = &["refRemote.json"];

    fn test_draft(dirname: &str, draft: &dyn schemas::Draft) {
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
                            let cfg = config::Config::from_schema(&schema, Some(draft)).unwrap();
                            let result = validate(&cfg, &data, &schema, true);
                            assert_eq!(
                                result.collect::<Vec<ValidationError>>().is_empty(),
                                *expected_valid
                            );
                            let result2 = is_valid(&cfg, &data, &schema, true);
                            assert_eq!(result2, *expected_valid);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_draft7() {
        test_draft("draft7", &schemas::Draft7);
    }

    #[test]
    fn test_draft6() {
        test_draft("draft6", &schemas::Draft6);
    }

    #[test]
    fn test_draft4() {
        test_draft("draft4", &schemas::Draft4);
    }
}
