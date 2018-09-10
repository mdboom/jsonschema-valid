use serde_json::Value;

use error::ValidationError;
use format::FormatChecker;
use resolver::Resolver;
use schemas;
use validators;
use validators::Validator;

pub struct Context<'a> {
    schema: &'a Value,
    resolver: Resolver<'a>,
    draft: &'a schemas::Draft,
}

impl<'a> Context<'a> {
    pub fn get_validator(&self, key: &str) -> Option<Validator> {
        self.draft.get_validator(key)
    }

    pub fn get_format_checker(&self, key: &str) -> Option<FormatChecker> {
        self.draft.get_format_checker(key)
    }

    pub fn validate(&self, instance: &Value, schema: &Value) -> validators::ValidatorResult {
        validators::run_validators(
            self,
            instance,
            schema,
            &validators::ScopeStack {
                x: &schema,
                parent: None,
            },
        )
    }

    pub fn get_resolver(&self) -> &Resolver<'a> {
        &self.resolver
    }

    pub fn get_schema(&self) -> &Value {
        &self.schema
    }

    pub fn from_schema(
        schema: &'a Value,
        draft: Option<&'a schemas::Draft>
    ) -> Result<Context<'a>, ValidationError> {
        Ok(Context {
            schema,
            resolver: Resolver::from_schema(schema)?,
            draft: schemas::draft_from_schema(schema)
                .unwrap_or_else(|| draft.unwrap_or_else(|| &schemas::Draft6)),
        })
    }
}
