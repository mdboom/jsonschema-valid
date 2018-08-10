extern crate regex;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate lazy_static;
extern crate itertools;

use serde_json::Value;

mod error;
mod unique;
mod util;
mod validators;

pub fn validate(instance: &Value, schema: &Value) -> validators::ValidatorResult {
    validators::run_validators(instance, schema)
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs;

    #[test]
    fn suite() {
        let paths = fs::read_dir("../JSON-Schema-Test-Suite/tests/draft6").unwrap();

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
                            assert_eq!(validate(&data, &schema).is_ok(), *is_valid);
                        }
                    }
                }
            }
        }
    }
}
