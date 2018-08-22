use serde_json::Value;

use error::ValidationError;
use resolver::Resolver;
use validators;
use validators::{Validator, ValidatorResult};

pub trait Context {
    fn get_validator(&self, key: &str) -> Option<Validator>;
    fn validate(&self, instance: &Value, schema: &Value) -> ValidatorResult;
    fn get_resolver(&self) -> &Resolver;
    fn get_schema(&self) -> &Value;
}

pub struct Draft6Context<'a> {
    schema: &'a Value,
    resolver: Resolver<'a>
}

impl<'a> Context for Draft6Context<'a> {
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

    fn validate(&self, instance: &Value, schema: &Value) -> validators::ValidatorResult {
        validators::run_validators(self, instance, schema, &validators::ScopeStack { x: &schema, parent: None })
    }

    fn get_resolver(&self) -> &Resolver {
        &self.resolver
    }

    fn get_schema(&self) -> &Value {
        &self.schema
    }
}

impl<'a> Draft6Context<'a> {
    pub fn from_schema(schema: &'a Value) -> Result<Draft6Context, ValidationError> {
        Ok(
            Draft6Context {
                schema: schema,
                resolver: Resolver::from_schema(schema)?
            }
        )
    }
}
