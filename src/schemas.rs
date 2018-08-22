use serde_json;
use serde_json::Value;


pub fn get_draft6_schema() -> &'static Value {
    lazy_static! {
        static ref draft6_string: &'static str = include_str!("draft6.json");
        static ref DRAFT6: Value = serde_json::from_str(&draft6_string).unwrap();
    }
    &DRAFT6
}
