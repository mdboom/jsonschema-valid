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
            "$ref" => Some(validators::validate_ref as Validator),
            "additionalItems" => Some(validators::validate_additionalItems as Validator),
            "additionalProperties" => Some(validators::validate_additionalProperties as Validator),
            "allOf" => Some(validators::validate_allOf as Validator),
            "anyOf" => Some(validators::validate_anyOf as Validator),
            "const" => Some(validators::validate_const as Validator),
            "contains" => Some(validators::validate_contains as Validator),
            "dependencies" => Some(validators::validate_dependencies as Validator),
            "enum" => Some(validators::validate_enum as Validator),
            "exclusiveMaximum" => Some(validators::validate_exclusiveMaximum as Validator),
            "exclusiveMinimum" => Some(validators::validate_exclusiveMinimum as Validator),
            "items" => Some(validators::validate_items as Validator),
            "maxItems" => Some(validators::validate_maxItems as Validator),
            "maxLength" => Some(validators::validate_maxLength as Validator),
            "maxProperties" => Some(validators::validate_maxProperties as Validator),
            "maximum" => Some(validators::validate_maximum as Validator),
            "minItems" => Some(validators::validate_minItems as Validator),
            "minLength" => Some(validators::validate_minLength as Validator),
            "minProperties" => Some(validators::validate_minProperties as Validator),
            "minimum" => Some(validators::validate_minimum as Validator),
            "multipleOf" => Some(validators::validate_multipleOf as Validator),
            "not" => Some(validators::validate_not as Validator),
            "oneOf" => Some(validators::validate_oneOf as Validator),
            "pattern" => Some(validators::validate_pattern as Validator),
            "patternProperties" => Some(validators::validate_patternProperties as Validator),
            "properties" => Some(validators::validate_properties as Validator),
            "propertyNames" => Some(validators::validate_propertyNames as Validator),
            "required" => Some(validators::validate_required as Validator),
            "type" => Some(validators::validate_type as Validator),
            "uniqueItems" => Some(validators::validate_uniqueItems as Validator),
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

pub struct Draft4;

impl Draft for Draft4 {
    fn get_validator(&self, key: &str) -> Option<Validator> {
        match key {
            "$ref" => Some(validators::validate_ref as Validator),
            "additionalItems" => Some(validators::validate_additionalItems as Validator),
            "additionalProperties" => Some(validators::validate_additionalProperties as Validator),
            "allOf" => Some(validators::validate_allOf_draft4 as Validator),
            "anyOf" => Some(validators::validate_anyOf_draft4 as Validator),
            "dependencies" => Some(validators::validate_dependencies as Validator),
            "enum" => Some(validators::validate_enum as Validator),
            "items" => Some(validators::validate_items_draft4 as Validator),
            "maxItems" => Some(validators::validate_maxItems as Validator),
            "maxLength" => Some(validators::validate_maxLength as Validator),
            "maxProperties" => Some(validators::validate_maxProperties as Validator),
            "maximum" => Some(validators::validate_maximum_draft4 as Validator),
            "minItems" => Some(validators::validate_minItems as Validator),
            "minLength" => Some(validators::validate_minLength as Validator),
            "minProperties" => Some(validators::validate_minProperties as Validator),
            "minimum" => Some(validators::validate_minimum_draft4 as Validator),
            "multipleOf" => Some(validators::validate_multipleOf as Validator),
            "not" => Some(validators::validate_not as Validator),
            "oneOf" => Some(validators::validate_oneOf_draft4 as Validator),
            "pattern" => Some(validators::validate_pattern as Validator),
            "patternProperties" => Some(validators::validate_patternProperties as Validator),
            "properties" => Some(validators::validate_properties as Validator),
            "required" => Some(validators::validate_required as Validator),
            "type" => Some(validators::validate_type as Validator),
            "uniqueItems" => Some(validators::validate_uniqueItems as Validator),
            _ => None,
        }
    }

    fn get_schema(&self) -> &'static Value {
        lazy_static! {
            static ref DRAFT4: Value = serde_json::from_str(include_str!("draft4.json")).unwrap();
        }
        &DRAFT4
    }
}

pub fn draft_from_url(url: &str) -> Option<&Draft> {
    match url {
        "http://json-schema.org/draft-06/schema" => Some(&Draft6),
        "http://json-schema.org/draft-04/schema" => Some(&Draft4),
        _ => None,
    }
}

pub fn draft_from_schema(schema: &Value) -> Option<&Draft> {
    schema
        .as_object()
        .and_then(|x| x.get("$schema"))
        .and_then(|x| x.as_str())
        .and_then(|x| draft_from_url(x))
}
