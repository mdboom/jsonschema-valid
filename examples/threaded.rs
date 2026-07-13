use std::sync::LazyLock;

use jsonschema_valid::{Config, schemas};
use serde_json::Value;

// Create the schema and schema validator globally once, then re-use them in multiple threads
// without problems.
static SCHEMA: LazyLock<Value> = LazyLock::new(|| serde_json::from_str("{}").unwrap());
static SCHEMA_CFG: LazyLock<Config<'static>> =
    LazyLock::new(|| Config::from_schema(&SCHEMA, Some(schemas::Draft::Draft6)).unwrap());

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
