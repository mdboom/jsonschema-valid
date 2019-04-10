extern crate regex;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate lazy_static;
extern crate itertools;
extern crate url;
extern crate chrono;

use serde_json::Value;

mod context;
mod error;
mod format;
mod resolver;
mod schemas;
mod unique;
mod util;
mod validators;

pub fn validate(instance: &Value, schema: &Value, draft: Option<&schemas::Draft>) -> Vec<error::ValidationError> {
    context::Context::from_schema(schema, draft).unwrap().validate(instance, schema)
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs;
    use std::path::PathBuf;

    // Test files we know will fail.
    const KNOWN_FAILURES: &'static [&'static str] = &["refRemote.json"];

    fn test_draft(path: &PathBuf, draft: &schemas::Draft) {
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
                        if let Value::Bool(is_valid) = valid {
                            let result = validate(&data, &schema, Some(draft));
                            if result.len() != 0 {
                                println!("{:?}", result);
                            }
                            assert_eq!(result.len() == 0, *is_valid);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_draft6() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("JSON-Schema-Test-Suite/tests/draft6");
        test_draft(&d, &schemas::Draft6);
    }

    #[test]
    fn test_draft4() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("JSON-Schema-Test-Suite/tests/draft4");
        test_draft(&d, &schemas::Draft4);
    }
}
