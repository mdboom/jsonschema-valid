extern crate serde_json;
extern crate lazy_static;
extern crate jsonschema_valid_compat as jsonschema_valid;
use jsonschema_valid::{schemas, Config};
use lazy_static::lazy_static;
use serde_json::Value;

lazy_static! {
    // Create the schema and schema validator globally once, then re-use them in multiple threads
    // without problems.

    static ref SCHEMA: Value = serde_json::from_str("{}").unwrap();
    static ref SCHEMA_CFG: Config<'static> = Config::from_schema(&SCHEMA, Some(schemas::Draft::Draft6)).unwrap();
}

fn main() {
    {
        let data = serde_json::from_str("{}").unwrap();
        assert!(SCHEMA_CFG.validate(&data).is_ok());
    }

    std::thread::spawn(|| {
        let data = serde_json::from_str("{}").unwrap();
        assert!(SCHEMA_CFG.validate(&data).is_ok());
    });
}
