use serde_json::Value;

use crate::error::ValidationError;
use crate::format::FormatChecker;
use crate::resolver::Resolver;
use crate::schemas;
use crate::validators::Validator;

/// A structure to hold configuration for a single validation run.
pub struct Config<'a> {
    schema: &'a Value,
    resolver: Resolver<'a>,
    draft: &'a dyn schemas::Draft,
}

impl<'a> Config<'a> {
    /// Get the validator object for the draft in use.
    pub fn get_validator(&self, key: &str) -> Option<Validator> {
        self.draft.get_validator(key)
    }

    /// Get the string format checker for the draft in use.
    pub fn get_format_checker(&self, key: &str) -> Option<FormatChecker> {
        self.draft.get_format_checker(key)
    }

    /// Get the draft number in use.
    pub fn get_draft_number(&self) -> u8 {
        self.draft.get_draft_number()
    }

    /// Get the metaschema associated with the draft in use.
    pub fn get_metaschema(&self) -> &Value {
        self.draft.get_schema()
    }

    /// Get the resolver for the parsing context.
    pub fn get_resolver(&self) -> &Resolver<'a> {
        &self.resolver
    }

    /// Get the schema currently being checked against.
    pub fn get_schema(&self) -> &Value {
        &self.schema
    }

    /// Create a new Config object from a given schema.
    ///
    /// Will use the Draft of JSON schema specified by `draft`. If `draft` is
    /// `None`, it will be automatically determined from the `$schema` entry in
    /// the given `shema`. If no `$schema` entry is present Draft 7 will be used
    /// by default.
    pub fn from_schema(
        schema: &'a Value,
        draft: Option<&'a dyn schemas::Draft>,
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
