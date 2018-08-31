extern crate regex;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate lazy_static;
extern crate itertools;
extern crate url;

use serde_json::Value;

mod context;
mod error;
mod resolver;
mod schemas;
mod unique;
mod util;
mod validators;

pub fn validate(instance: &Value, schema: &Value, draft: Option<&schemas::Draft>) -> validators::ValidatorResult {
    context::Context::from_schema(schema, draft)?.validate(instance, schema)
}

// pub fn validate_schema(schema: &Value) -> validators::ValidatorResult {

// }

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs;

    fn test_draft(path: &str, draft: &schemas::Draft) {
        let paths = fs::read_dir(path).unwrap();

        for entry in paths {
            let path = &entry.unwrap().path();
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
                            if !result.is_ok() {
                                println!("{:?}", result);
                            }
                            assert_eq!(result.is_ok(), *is_valid);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_draft6() {
        test_draft("../JSON-Schema-Test-Suite/tests/draft6", &schemas::Draft6);
    }

    // #[test]
    // fn test_draft4() {
    //     test_draft("../JSON-Schema-Test-Suite/tests/draft4", &schemas::Draft4);
    // }
}
