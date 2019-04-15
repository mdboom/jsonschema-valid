use std::collections::HashMap;

use serde_json::Value;
use url;

use context::Context;
use error::ValidationError;
use schemas;
// TODO: Make the choice of resolver dynamic

fn id_of(schema: &Value) -> Option<&str> {
    if let Value::Object(object) = schema {
        object
            .get("$id")
            .or_else(|| object.get("id"))
            .and_then(Value::as_str)
    } else {
        None
    }
}

pub struct Resolver<'a> {
    base_url: String,
    id_mapping: HashMap<String, &'a Value>,
}

fn find_ids<'a>(
    schema: &'a Value,
    id_mapping: &mut HashMap<String, &'a Value>,
    base_url: &url::Url,
) -> Result<(), ValidationError> {
    match schema {
        Value::Object(object) => {
            if let Some(url) = id_of(schema) {
                id_mapping.insert(url.to_string(), schema);
                let new_url = base_url.join(url)?;
                for (_k, v) in object {
                    find_ids(v, id_mapping, &new_url)?;
                }
            } else {
                for (_k, v) in object {
                    find_ids(v, id_mapping, base_url)?;
                }
            }
        }
        Value::Array(array) => {
            for v in array {
                find_ids(v, id_mapping, base_url)?;
            }
        }
        _ => {}
    }
    Ok(())
}

impl<'a> Resolver<'a> {
    pub fn from_schema(schema: &'a Value) -> Result<Resolver<'a>, ValidationError> {
        let base_url = match id_of(schema) {
            Some(url) => url.to_string(),
            None => "document:///".to_string(),
        };

        let mut id_mapping: HashMap<String, &'a Value> = HashMap::new();

        find_ids(schema, &mut id_mapping, &url::Url::parse(&base_url)?)?;

        Ok(Resolver {
            base_url,
            id_mapping,
        })
    }

    pub fn join_url(&self, url_ref: &str, ctx: &Context) -> Result<url::Url, ValidationError> {
        let mut urls: Vec<&str> = Vec::new();
        urls.push(url_ref);
        let mut frame = ctx;
        loop {
            if let Some(id) = id_of(frame.x) {
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
            "document:///" => Ok(instance),
            _ => match schemas::draft_from_url(url_str) {
                Some(value) => Ok(value.get_schema()),
                _ => match self.id_mapping.get(url_str) {
                    Some(value) => Ok(value),
                    None => Err(ValidationError::new("Can't fetch document")),
                },
            },
        }
    }

    pub fn resolve_fragment(
        &self,
        url: &str,
        ctx: &Context,
        instance: &'a Value,
    ) -> Result<(url::Url, &'a Value), ValidationError> {
        let url = self.join_url(url, ctx)?;
        let mut resource = url.clone();
        resource.set_fragment(None);
        let document = self.resolve_url(&resource, instance)?;
        let fragment =
            url::percent_encoding::percent_decode(url.fragment().unwrap_or_else(|| "").as_bytes())
                .decode_utf8()
                .unwrap();
        // TODO Prevent infinite reference recursion
        match document.pointer(&fragment) {
            Some(x) => Ok((resource, x)),
            None => Err(ValidationError::new("Couldn't resolve JSON pointer")),
        }
    }
}
