use std::collections::HashMap;

use serde_json::Value;

use crate::context::Context;
use crate::error::ValidationError;
use crate::schemas::{self, Draft};
// TODO: Make the choice of resolver dynamic

const DOCUMENT_PROTOCOL: &str = "document:///";

fn id_of(draft: Draft, schema: &Value) -> Option<&str> {
    if let Value::Object(object) = schema {
        if draft == Draft::Draft4 {
            object.get("$id").or_else(|| object.get("id"))
        } else {
            object.get("$id")
        }
        .and_then(Value::as_str)
    } else {
        None
    }
}

pub struct Resolver<'a> {
    base_url: String,
    id_mapping: HashMap<String, &'a Value>,
}

/// Iterate through all of the document fragments with an assigned id, calling a
/// callback at each location.
fn find_ids<'a, F>(
    draft: Draft,
    schema: &'a Value,
    base_url: &url::Url,
    visitor: &mut F,
) -> Result<Option<&'a Value>, ValidationError>
where
    F: FnMut(String, &'a Value) -> Option<&'a Value>,
{
    match schema {
        Value::Object(object) => {
            if let Some(url) = id_of(draft, schema) {
                let new_url = base_url.join(url)?;
                if let Some(x) = visitor(new_url.to_string(), schema) {
                    return Ok(Some(x));
                }
                for (_k, v) in object {
                    let result = find_ids(draft, v, &new_url, visitor)?;
                    if result.is_some() {
                        return Ok(result);
                    }
                }
            } else {
                for (_k, v) in object {
                    let result = find_ids(draft, v, base_url, visitor)?;
                    if result.is_some() {
                        return Ok(result);
                    }
                }
            }
        }
        Value::Array(array) => {
            for v in array {
                let result = find_ids(draft, v, base_url, visitor)?;
                if result.is_some() {
                    return Ok(result);
                }
            }
        }
        _ => {}
    }
    Ok(None)
}

impl<'a> Resolver<'a> {
    pub fn from_schema(draft: Draft, schema: &'a Value) -> Result<Resolver<'a>, ValidationError> {
        let base_url = match id_of(draft, schema) {
            Some(url) => url.to_string(),
            None => DOCUMENT_PROTOCOL.to_string(),
        };

        let mut id_mapping: HashMap<String, &'a Value> = HashMap::new();

        find_ids(draft, schema, &url::Url::parse(&base_url)?, &mut |id, x| {
            id_mapping.insert(id, x);
            None
        })?;

        Ok(Resolver {
            base_url,
            id_mapping,
        })
    }

    pub fn join_url(
        &self,
        draft: Draft,
        url_ref: &str,
        ctx: &Context,
    ) -> Result<url::Url, ValidationError> {
        let mut urls: Vec<&str> = vec![url_ref];
        let mut frame = ctx;
        loop {
            if let Some(id) = id_of(draft, frame.x) {
                urls.push(id);
            }
            match frame.parent {
                Some(x) => frame = x,
                None => break,
            }
        }
        let base_url = url::Url::parse(&self.base_url)?;
        let url = urls.iter().rev().try_fold(base_url, |x, y| x.join(y));
        Ok(url?)
    }

    pub fn resolve_url(
        &self,
        url: &url::Url,
        instance: &'a Value,
    ) -> Result<&'a Value, ValidationError> {
        let url_str = url.as_str();
        match url_str {
            DOCUMENT_PROTOCOL => Ok(instance),
            _ => match schemas::draft_from_url(url_str) {
                Some(value) => Ok(value.get_schema()),
                _ => match self.id_mapping.get(url_str) {
                    Some(value) => Ok(value),
                    None => Err(ValidationError::new(
                        &format!("Can't resolve url {}", url_str),
                        None,
                        None,
                    )),
                },
            },
        }
    }

    pub fn resolve_fragment(
        &self,
        draft: Draft,
        url: &str,
        ctx: &Context,
        instance: &'a Value,
    ) -> Result<(url::Url, &'a Value), ValidationError> {
        let url = self.join_url(draft, url, ctx)?;
        let mut resource = url.clone();
        resource.set_fragment(None);
        let fragment = percent_encoding::percent_decode(url.fragment().unwrap_or("").as_bytes())
            .decode_utf8()
            .unwrap();

        if let Some(x) = find_ids(
            draft,
            instance,
            &url::Url::parse(DOCUMENT_PROTOCOL)?,
            &mut |id, x| {
                if id == url.as_str() {
                    Some(x)
                } else {
                    None
                }
            },
        )? {
            return Ok((resource, x));
        }

        let document = self.resolve_url(&resource, instance)?;

        // TODO Prevent infinite reference recursion
        match document.pointer(&fragment) {
            Some(x) => Ok((resource, x)),
            None => Err(ValidationError::new(
                &format!("Couldn't resolve JSON pointer {}", url),
                None,
                None,
            )),
        }
    }
}
