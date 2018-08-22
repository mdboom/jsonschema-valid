use serde_json::Value;

use error::ValidationError;
use resolver::Resolver;
use schemas;
use validators;
use validators::Validator;

pub struct Context<'a> {
    schema: &'a Value,
    resolver: Resolver<'a>,
    draft: Box<schemas::Draft>
}

impl<'a> Context<'a> {
    pub fn get_validator(&self, key: &str) -> Option<Validator> {
        self.draft.get_validator(key)
    }

    pub fn validate(&self, instance: &Value, schema: &Value) -> validators::ValidatorResult {
        validators::run_validators(self, instance, schema, &validators::ScopeStack { x: &schema, parent: None })
    }

    pub fn get_resolver(&self) -> &Resolver {
        &self.resolver
    }

    pub fn get_schema(&self) -> &Value {
        &self.schema
    }

    pub fn from_schema(schema: &'a Value) -> Result<Context, ValidationError> {
        Ok(
            Context {
                schema: schema,
                resolver: Resolver::from_schema(schema)?,
                draft: schemas::draft_from_schema(schema).unwrap_or_else(|| Box::new(schemas::Draft6))
            }
        )
    }
}
