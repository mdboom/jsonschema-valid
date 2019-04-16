use serde_json::Value;

use context::Context;
use error::{ErrorRecorder, ValidationError};
use format::FormatChecker;
use resolver::Resolver;
use schemas;
use validators;
use validators::Validator;

pub struct Config<'a> {
    schema: &'a Value,
    resolver: Resolver<'a>,
    draft: &'a schemas::Draft,
}

impl<'a> Config<'a> {
    pub fn get_validator(&self, key: &str) -> Option<Validator> {
        self.draft.get_validator(key)
    }

    pub fn get_format_checker(&self, key: &str) -> Option<FormatChecker> {
        self.draft.get_format_checker(key)
    }

    pub fn get_draft_number(&self) -> u8 {
        self.draft.get_draft_number()
    }

    pub fn get_metaschema(&self) -> &Value {
        self.draft.get_schema()
    }

    pub fn validate(
        &self,
        instance: &Value,
        schema: &Value,
        errors: &mut ErrorRecorder,
        validate_schema: bool,
    ) -> Option<()> {
        if validate_schema {
            let metaschema = self.get_metaschema();
            validators::descend(
                self,
                schema,
                metaschema,
                &Context::new(),
                &Context::new(),
                &Context::new_from(metaschema),
                errors,
            );
            if errors.has_errors() {
                return None;
            }
        }

        validators::descend(
            self,
            instance,
            schema,
            &Context::new(),
            &Context::new(),
            &Context::new_from(schema),
            errors,
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
        draft: Option<&'a schemas::Draft>,
    ) -> Result<Config<'a>, ValidationError> {
        Ok(Config {
            schema,
            resolver: Resolver::from_schema(schema)?,
            draft: draft.unwrap_or_else(|| {
                schemas::draft_from_schema(schema).unwrap_or_else(|| &schemas::Draft7)
            }),
        })
    }
}
