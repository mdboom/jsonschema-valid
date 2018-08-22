use serde_json;
use serde_json::Value;

use validators;
use validators::Validator;

pub trait Draft {
    fn get_validator(&self, key: &str) -> Option<Validator>;
    fn get_schema(&self) -> &'static Value;
}

pub struct Draft6;

impl Draft for Draft6 {
    fn get_validator(&self, key: &str) -> Option<Validator> {
        match key {
            "patternProperties" => Some(validators::validate_patternProperties as Validator),
            "pattern" => Some(validators::validate_pattern as Validator),
            "propertyNames" => Some(validators::validate_propertyNames as Validator),
            "additionalProperties" => Some(validators::validate_additionalProperties as Validator),
            "items" => Some(validators::validate_items as Validator),
            "additionalItems" => Some(validators::validate_additionalItems as Validator),
            "const" => Some(validators::validate_const as Validator),
            "contains" => Some(validators::validate_contains as Validator),
            "exclusiveMinimum" => Some(validators::validate_exclusiveMinimum as Validator),
            "exclusiveMaximum" => Some(validators::validate_exclusiveMaximum as Validator),
            "minimum" => Some(validators::validate_minimum as Validator),
            "maximum" => Some(validators::validate_maximum as Validator),
            "multipleOf" => Some(validators::validate_multipleOf as Validator),
            "minItems" => Some(validators::validate_minItems as Validator),
            "maxItems" => Some(validators::validate_maxItems as Validator),
            "uniqueItems" => Some(validators::validate_uniqueItems as Validator),
            "minLength" => Some(validators::validate_minLength as Validator),
            "maxLength" => Some(validators::validate_maxLength as Validator),
            "dependencies" => Some(validators::validate_dependencies as Validator),
            "enum" => Some(validators::validate_enum as Validator),
            "type" => Some(validators::validate_type as Validator),
            "properties" => Some(validators::validate_properties as Validator),
            "required" => Some(validators::validate_required as Validator),
            "minProperties" => Some(validators::validate_minProperties as Validator),
            "maxProperties" => Some(validators::validate_maxProperties as Validator),
            "allOf" => Some(validators::validate_allOf as Validator),
            "anyOf" => Some(validators::validate_anyOf as Validator),
            "oneOf" => Some(validators::validate_oneOf as Validator),
            "not" => Some(validators::validate_not as Validator),
            "$ref" => Some(validators::validate_ref as Validator),
            _ => None,
        }
    }

    fn get_schema(&self) -> &'static Value {
        lazy_static! {
            static ref DRAFT6: Value = serde_json::from_str(include_str!("draft6.json")).unwrap();
        }
        &DRAFT6
    }
}

pub fn draft_from_url(url: &str) -> Option<Box<Draft>> {
    match url {
        "http://json-schema.org/draft-06/schema" => Some(Box::new(Draft6 {})),
        _ => None,
    }
}

pub fn draft_from_schema(schema: &Value) -> Option<Box<Draft>> {
    schema
        .as_object()
        .and_then(|x| x.get("$schema"))
        .and_then(|x| x.as_str())
        .and_then(|x| draft_from_url(x))
}
