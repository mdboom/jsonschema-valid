//! # jsonschema-valid
//!
//! A simple crate to perform json schema validation.
//!
//! Supports JSON Schema drafts 4, 6, and 7.

extern crate regex;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate lazy_static;
extern crate chrono;
extern crate iri_string;
extern crate itertools;
extern crate json_pointer;
extern crate url;

use serde_json::Value;
use std::io::prelude::*;

mod config;
mod context;
mod error;
mod format;
mod resolver;
pub mod schemas;
mod unique;
mod util;
mod validators;

pub use error::ValidationError;
pub use error::ValidationErrors;

/// Validates a given JSON instance against a given JSON schema, returning the
/// errors, if any. draft may provide the schema draft to use. If not provided,
/// it will be determined automatically from the schema.
///
/// # Arguments
///
/// * `instance`: The JSON document to validate
/// * `schema`: The JSON schema to validate against
/// * `draft`: The draft of the JSON schema specification to use. If `None`, the
///   draft will be automatically determined from the `schema`.
/// * `validate_schema`: When `true`, validate the schema against the metaschema
///   first.
///
/// # Returns
///
/// * `errors`: A vector of `ValidationError` found during validation.
pub fn validate(
    instance: &Value,
    schema: &Value,
    draft: Option<&schemas::Draft>,
    validate_schema: bool,
) -> error::ValidationErrors {
    let mut errors = error::ValidationErrors::new();
    config::Config::from_schema(schema, draft)
        .unwrap()
        .validate(instance, schema, &mut errors, validate_schema);
    errors
}

/// Validates a given JSON instance against a given JSON schema, writing the
/// errors to the given stream. draft may provide the schema draft to use. If
/// not provided, it will be determined automatically from the schema.
///
/// # Arguments
///
/// * `stream`: An object to write errors to.
/// * `instance`: The JSON document to validate
/// * `schema`: The JSON schema to validate against
/// * `draft`: The draft of the JSON schema specification to use. If `None`, the
///   draft will be automatically determined from the `schema`.
/// * `validate_schema`: When `true`, validate the schema against the metaschema
///   first.
///
/// # Returns
///
/// * `Some(())`: No errors were found.
/// * `None`: At least one error was found.
pub fn validate_to_stream(
    stream: &mut Write,
    instance: &Value,
    schema: &Value,
    draft: Option<&schemas::Draft>,
    validate_schema: bool,
) -> Option<()> {
    let mut errors = error::ErrorRecorderStream::new(stream);
    config::Config::from_schema(schema, draft)
        .unwrap()
        .validate(instance, schema, &mut errors, validate_schema)
}

/// Validates a given JSON instance against a given JSON schema, returning true
/// if valid. This function is more efficient than [validate](fn.validate.html)
/// or [validate_to_stream](fn.validate_to_stream.html), because it stops at the
/// first error.
///
/// # Arguments
///
/// * `instance`: The JSON document to validate
/// * `schema`: The JSON schema to validate against
/// * `draft`: The draft of the JSON schema specification to use. If `None`, the
///   draft will be automatically determined from the `schema`.
/// * `validate_schema`: When `true`, validate the schema against the metaschema
///   first.
///
/// # Returns
///
/// * `true`: `instance` is valid against `schema`.
pub fn is_valid(
    instance: &Value,
    schema: &Value,
    draft: Option<&schemas::Draft>,
    validate_schema: bool,
) -> bool {
    config::Config::from_schema(schema, draft)
        .unwrap()
        .validate(
            instance,
            schema,
            &mut error::FastFailErrorRecorder::new(),
            validate_schema,
        )
        .is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    use error::ErrorRecorder;
    use std::fs;
    use std::path::PathBuf;

    // Test files we know will fail.
    const KNOWN_FAILURES: &'static [&'static str] = &["refRemote.json"];

    fn test_draft(dirname: &str, draft: &schemas::Draft) {
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
                            let result = validate(&data, &schema, Some(draft), true);
                            assert_eq!(!result.has_errors(), *expected_valid);
                            let result2 = is_valid(&data, &schema, Some(draft), true);
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
