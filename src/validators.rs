#![allow(non_snake_case)]
#![allow(clippy::too_many_arguments)]

use itertools::Itertools;
use regex;

use serde_json::{Map, Number, Value, Value::Array, Value::Bool, Value::Object};

use config::Config;
use context::Context;
use error::{ErrorRecorder, FastFailErrorRecorder, ValidationError, ValidationErrors};
use unique;
use util;

pub type Validator = fn(
    cfg: &Config,
    instance: &Value,
    schema: &Value,
    parent_schema: &Map<String, Value>,
    instance_ctx: &Context,
    schema_ctx: &Context,
    ref_ctx: &Context,
    errors: &mut ErrorRecorder,
) -> Option<()>;

pub fn descend(
    cfg: &Config,
    instance: &Value,
    schema: &Value,
    instance_ctx: &Context,
    schema_ctx: &Context,
    ref_ctx: &Context,
    errors: &mut ErrorRecorder,
) -> Option<()> {
    match schema {
        Bool(b) => {
            if !*b {
                errors.record_error(ValidationError::new_with_schema_context(
                    "false schema always fails",
                    schema_ctx,
                ))?
            }
        }
        Object(schema_object) => {
            if schema_object.contains_key("$ref") {
                if let Some(validator) = cfg.get_validator("$ref") {
                    validator(
                        cfg,
                        instance,
                        schema_object.get("$ref").unwrap(),
                        schema_object,
                        instance_ctx,
                        &schema_ctx.push(&Value::String("$ref".to_string())),
                        ref_ctx,
                        errors,
                    )?;
                }
            } else {
                for (k, v) in schema_object.iter() {
                    if let Some(validator) = cfg.get_validator(k.as_ref()) {
                        validator(
                            cfg,
                            instance,
                            v,
                            schema_object,
                            instance_ctx,
                            &schema_ctx.push(&Value::String(k.to_string())),
                            ref_ctx,
                            errors,
                        )?
                    }
                }
            }
        }
        _ => errors.record_error(ValidationError::new_with_schema_context(
            format!("Invalid schema. Must be Bool or Object, got '{:?}'", schema).as_str(),
            schema_ctx,
        ))?,
    }
    Some(())
}

pub fn patternProperties(
    cfg: &Config,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    instance_ctx: &Context,
    schema_ctx: &Context,
    ref_ctx: &Context,
    errors: &mut ErrorRecorder,
) -> Option<()> {
    if let (Object(instance), Object(schema)) = (instance, schema) {
        for (pattern, subschema) in schema.iter() {
            match regex::Regex::new(pattern) {
                Ok(re) => {
                    for (k, v) in instance.iter() {
                        if re.is_match(k) {
                            descend(
                                cfg,
                                v,
                                subschema,
                                &instance_ctx.push(&Value::String(k.to_string())),
                                &schema_ctx.push(&Value::String(pattern.to_string())),
                                ref_ctx,
                                errors,
                            )?;
                        }
                    }
                }
                Err(err) => errors.record_error(ValidationError::new_with_schema_context(
                    format!("Invalid regular expression '{}': ({})", pattern, err).as_str(),
                    schema_ctx,
                ))?,
            }
        }
    }
    Some(())
}

pub fn propertyNames(
    cfg: &Config,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    instance_ctx: &Context,
    schema_ctx: &Context,
    ref_ctx: &Context,
    errors: &mut ErrorRecorder,
) -> Option<()> {
    if let Object(instance) = instance {
        for property in instance.keys() {
            descend(
                cfg,
                &Value::String(property.to_string()),
                schema,
                &instance_ctx.push(&Value::String(property.to_string())),
                schema_ctx,
                ref_ctx,
                errors,
            )?;
        }
    }
    Some(())
}

fn find_additional_properties<'a>(
    instance: &'a Map<String, Value>,
    schema: &'a Map<String, Value>,
) -> Box<Iterator<Item = &'a String> + 'a> {
    let properties = schema.get("properties").and_then(|x| x.as_object());
    let pattern_regexes = schema
        .get("patternProperties")
        .and_then(|x| x.as_object())
        .and_then(|x| {
            Some(
                x.keys()
                    .filter_map(|k| regex::Regex::new(k).ok())
                    .collect::<Vec<regex::Regex>>(),
            )
        });
    Box::new(
        instance
            .keys()
            .filter(move |&property| {
                !properties.map_or_else(|| false, |x| x.contains_key(property))
            })
            .filter(move |&property| {
                !pattern_regexes
                    .as_ref()
                    .map_or_else(|| false, |x| x.iter().any(|y| y.is_match(property)))
            }),
    )
}

pub fn additionalProperties(
    cfg: &Config,
    instance: &Value,
    schema: &Value,
    parent_schema: &Map<String, Value>,
    instance_ctx: &Context,
    schema_ctx: &Context,
    ref_ctx: &Context,
    errors: &mut ErrorRecorder,
) -> Option<()> {
    if let Object(instance) = instance {
        let extras = find_additional_properties(instance, parent_schema);
        {
            match schema {
                Object(_) => {
                    for extra in extras {
                        descend(
                            cfg,
                            instance.get(extra).expect("Property gone missing."),
                            schema,
                            &instance_ctx.push(&Value::String(extra.to_string())),
                            schema_ctx,
                            ref_ctx,
                            errors,
                        )?;
                    }
                }
                Bool(bool) => {
                    if !bool && extras.peekable().peek().is_some() {
                        let mut extraProps = find_additional_properties(instance, parent_schema);
                        errors.record_error(ValidationError::new_with_context(
                            format!("Additional properties are not allowed. Found {}", extraProps.join(", "))
                                .as_str(),
                            instance_ctx,
                            schema_ctx,
                        ))?;
                    }
                }
                _ => {}
            }
        }
    }
    Some(())
}

pub fn items_draft4(
    cfg: &Config,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    instance_ctx: &Context,
    schema_ctx: &Context,
    ref_ctx: &Context,
    errors: &mut ErrorRecorder,
) -> Option<()> {
    if let Array(instance) = instance {
        match schema {
            Object(_) => {
                for (index, item) in instance.iter().enumerate() {
                    descend(
                        cfg,
                        item,
                        schema,
                        &instance_ctx.push(&Value::Number(Number::from(index))),
                        schema_ctx,
                        ref_ctx,
                        errors,
                    )?;
                }
            }
            Array(items) => {
                for ((index, item), subschema) in instance.iter().enumerate().zip(items.iter()) {
                    descend(
                        cfg,
                        item,
                        subschema,
                        &instance_ctx.push(&Value::Number(Number::from(index))),
                        &schema_ctx.push(&Value::Number(Number::from(index))),
                        ref_ctx,
                        errors,
                    )?;
                }
            }
            _ => {}
        }
    }
    Some(())
}

pub fn items(
    cfg: &Config,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    instance_ctx: &Context,
    schema_ctx: &Context,
    ref_ctx: &Context,
    errors: &mut ErrorRecorder,
) -> Option<()> {
    if let Array(instance) = instance {
        let items = util::bool_to_object_schema(schema);

        match items {
            Object(_) => {
                for (index, item) in instance.iter().enumerate() {
                    descend(
                        cfg,
                        item,
                        items,
                        &instance_ctx.push(&Value::Number(Number::from(index))),
                        schema_ctx,
                        ref_ctx,
                        errors,
                    )?;
                }
            }
            Array(items) => {
                for ((index, item), subschema) in instance.iter().enumerate().zip(items.iter()) {
                    descend(
                        cfg,
                        item,
                        subschema,
                        &instance_ctx.push(&Value::Number(Number::from(index))),
                        &schema_ctx.push(&Value::Number(Number::from(index))),
                        ref_ctx,
                        errors,
                    )?;
                }
            }
            _ => {}
        }
    }
    Some(())
}

pub fn additionalItems(
    cfg: &Config,
    instance: &Value,
    schema: &Value,
    parent_schema: &Map<String, Value>,
    instance_ctx: &Context,
    schema_ctx: &Context,
    ref_ctx: &Context,
    errors: &mut ErrorRecorder,
) -> Option<()> {
    if !parent_schema.contains_key("items") {
        return Some(());
    } else if let Object(_) = parent_schema["items"] {
        return Some(());
    }

    if let Array(instance) = instance {
        let len_items = parent_schema
            .get("items")
            .and_then(|x| x.as_array())
            .map_or_else(|| 0, |x| x.len());
        match schema {
            Object(_) => {
                for (i, item) in instance.iter().enumerate().skip(len_items) {
                    descend(
                        cfg,
                        &item,
                        schema,
                        &instance_ctx.push(&Value::Number(Number::from(i))),
                        schema_ctx,
                        ref_ctx,
                        errors,
                    )?;
                }
            }
            Bool(b) => {
                if !b && instance.len() > len_items {
                    errors.record_error(ValidationError::new_with_context(
                        "Additional items are not allowed",
                        instance_ctx,
                        schema_ctx,
                    ))?;
                }
            }
            _ => {}
        }
    }
    Some(())
}

pub fn const_(
    _cfg: &Config,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    instance_ctx: &Context,
    schema_ctx: &Context,
    _ref_ctx: &Context,
    errors: &mut ErrorRecorder,
) -> Option<()> {
    if instance != schema {
        errors.record_error(ValidationError::new_with_context(
            format!("const doesn't match. Got {}, expected {}", instance.to_string(), schema.to_string()).as_str(),
            instance_ctx,
            schema_ctx,
        ))?;
    }
    Some(())
}

pub fn contains(
    cfg: &Config,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    instance_ctx: &Context,
    schema_ctx: &Context,
    ref_ctx: &Context,
    errors: &mut ErrorRecorder,
) -> Option<()> {
    if let Array(instance) = instance {
        for (i, item) in instance.iter().enumerate() {
            if descend(
                cfg,
                item,
                schema,
                &instance_ctx.push(&Value::Number(Number::from(i))),
                schema_ctx,
                ref_ctx,
                &mut FastFailErrorRecorder::new(),
            ).is_some() {
                return Some(())
            }
        }
        errors.record_error(ValidationError::new_with_context(
            "No items in array valid under the given schema",
            instance_ctx,
            schema_ctx,
        ))?;
    }
    Some(())
}

pub fn exclusiveMinimum(
    _cfg: &Config,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    instance_ctx: &Context,
    schema_ctx: &Context,
    _ref_ctx: &Context,
    errors: &mut ErrorRecorder,
) -> Option<()> {
    if let (Value::Number(instance), Value::Number(schema)) = (instance, schema) {
        if instance.as_f64() <= schema.as_f64() {
            errors.record_error(ValidationError::new_with_context(
                format!("{} <= exclusiveMinimum {}", instance, schema).as_str(),
                instance_ctx,
                schema_ctx,
            ))?;
        }
    }
    Some(())
}

pub fn exclusiveMaximum(
    _cfg: &Config,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    instance_ctx: &Context,
    schema_ctx: &Context,
    _ref_ctx: &Context,
    errors: &mut ErrorRecorder,
) -> Option<()> {
    if let (Value::Number(instance), Value::Number(schema)) = (instance, schema) {
        if instance.as_f64() >= schema.as_f64() {
            errors.record_error(ValidationError::new_with_context(
                format!("{} >= exclusiveMaximum {}", instance, schema).as_str(),
                instance_ctx,
                schema_ctx,
            ))?;
        }
    }
    Some(())
}

pub fn minimum_draft4(
    _cfg: &Config,
    instance: &Value,
    schema: &Value,
    parent_schema: &Map<String, Value>,
    instance_ctx: &Context,
    schema_ctx: &Context,
    _ref_ctx: &Context,
    errors: &mut ErrorRecorder,
) -> Option<()> {
    if let (Value::Number(instance), Value::Number(minimum)) = (instance, schema) {
        if parent_schema
            .get("exclusiveMinimum")
            .and_then(|x| x.as_bool())
            .unwrap_or_else(|| false)
        {
            if instance.as_f64() <= minimum.as_f64() {
                errors.record_error(ValidationError::new_with_context(
                    format!("{} <= exclusiveMinimum {}", instance, schema).as_str(),
                    instance_ctx,
                    schema_ctx,
                ))?;
            }
        } else if instance.as_f64() < minimum.as_f64() {
            errors.record_error(ValidationError::new_with_context(
                format!("{} <= minimum {}", instance, schema).as_str(),
                instance_ctx,
                schema_ctx,
            ))?;
        }
    }
    Some(())
}

pub fn minimum(
    _cfg: &Config,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    instance_ctx: &Context,
    schema_ctx: &Context,
    _ref_ctx: &Context,
    errors: &mut ErrorRecorder,
) -> Option<()> {
    if let (Value::Number(instance), Value::Number(schema)) = (instance, schema) {
        if instance.as_f64() < schema.as_f64() {
            errors.record_error(ValidationError::new_with_context(
                format!("{} < minimum {}", instance, schema).as_str(),
                instance_ctx,
                schema_ctx,
            ))?;
        }
    }
    Some(())
}

pub fn maximum_draft4(
    _cfg: &Config,
    instance: &Value,
    schema: &Value,
    parent_schema: &Map<String, Value>,
    instance_ctx: &Context,
    schema_ctx: &Context,
    _ref_ctx: &Context,
    errors: &mut ErrorRecorder,
) -> Option<()> {
    if let (Value::Number(instance), Value::Number(maximum)) = (instance, schema) {
        if parent_schema
            .get("exclusiveMaximum")
            .and_then(|x| x.as_bool())
            .unwrap_or_else(|| false)
        {
            if instance.as_f64() >= maximum.as_f64() {
                errors.record_error(ValidationError::new_with_context(
                    format!("{} >= exclusiveMaximum {}", instance, schema).as_str(),
                    instance_ctx,
                    schema_ctx,
                ))?;
            }
        } else if instance.as_f64() > maximum.as_f64() {
            errors.record_error(ValidationError::new_with_context(
                format!("{} > maximum {}", instance, schema).as_str(),
                instance_ctx,
                schema_ctx,
            ))?;
        }
    }
    Some(())
}

pub fn maximum(
    _cfg: &Config,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    instance_ctx: &Context,
    schema_ctx: &Context,
    _ref_ctx: &Context,
    errors: &mut ErrorRecorder,
) -> Option<()> {
    if let (Value::Number(instance), Value::Number(schema)) = (instance, schema) {
        if instance.as_f64() > schema.as_f64() {
            errors.record_error(ValidationError::new_with_context(
                format!("{} > maximum {}", instance, schema).as_str(),
                instance_ctx,
                schema_ctx,
            ))?;
        }
    }
    Some(())
}

#[allow(clippy::float_cmp)]
pub fn multipleOf(
    _cfg: &Config,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    instance_ctx: &Context,
    schema_ctx: &Context,
    _ref_ctx: &Context,
    errors: &mut ErrorRecorder,
) -> Option<()> {
    if let (Value::Number(instance), Value::Number(schema)) = (instance, schema) {
        let failed = if schema.is_f64() {
            let quotient = instance.as_f64().unwrap() / schema.as_f64().unwrap();
            quotient.trunc() != quotient
        } else if schema.is_u64() {
            (instance.as_u64().unwrap() % schema.as_u64().unwrap()) != 0
        } else {
            (instance.as_i64().unwrap() % schema.as_i64().unwrap()) != 0
        };
        if failed {
            errors.record_error(ValidationError::new_with_context(
                format!("{} not multipleOf {}", instance, schema).as_str(),
                instance_ctx,
                schema_ctx,
            ))?;
        }
    }
    Some(())
}

pub fn minItems(
    _cfg: &Config,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    instance_ctx: &Context,
    schema_ctx: &Context,
    _ref_ctx: &Context,
    errors: &mut ErrorRecorder,
) -> Option<()> {
    if let (Array(instance), Value::Number(schema)) = (instance, schema) {
        if instance.len() < schema.as_u64().unwrap() as usize {
            errors.record_error(ValidationError::new_with_context(
                format!("{} < minItems {}", instance.len(), schema).as_str(),
                instance_ctx,
                schema_ctx,
            ))?;
        }
    }
    Some(())
}

pub fn maxItems(
    _cfg: &Config,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    instance_ctx: &Context,
    schema_ctx: &Context,
    _ref_ctx: &Context,
    errors: &mut ErrorRecorder,
) -> Option<()> {
    if let (Array(instance), Value::Number(schema)) = (instance, schema) {
        if instance.len() > schema.as_u64().unwrap() as usize {
            errors.record_error(ValidationError::new_with_context(
                format!("{} > maxItems {}", instance.len(), schema).as_str(),
                instance_ctx,
                schema_ctx,
            ))?;
        }
    }
    Some(())
}

pub fn uniqueItems(
    _cfg: &Config,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    instance_ctx: &Context,
    schema_ctx: &Context,
    _ref_ctx: &Context,
    errors: &mut ErrorRecorder,
) -> Option<()> {
    if let (Array(instance), Bool(schema)) = (instance, schema) {
        if *schema && !unique::has_unique_elements(&mut instance.iter()) {
            errors.record_error(ValidationError::new_with_context(
                "items are not unique",
                instance_ctx,
                schema_ctx,
            ))?;
        }
    }
    Some(())
}

pub fn pattern(
    _cfg: &Config,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    instance_ctx: &Context,
    schema_ctx: &Context,
    _ref_ctx: &Context,
    errors: &mut ErrorRecorder,
) -> Option<()> {
    if let (Value::String(instance), Value::String(schema)) = (instance, schema) {
        match regex::Regex::new(schema) {
            Ok(re) => {
                if !re.is_match(instance) {
                    errors.record_error(ValidationError::new_with_context(
                        format!("{} does not match pattern {}", instance.to_string(), schema.to_string()).as_str(),
                        instance_ctx,
                        schema_ctx,
                    ))?;
                }
            }
            Err(err) => errors.record_error(ValidationError::new_with_schema_context(
                format!("Invalid regular expression '{}': ({})", schema, err).as_str(),
                schema_ctx,
            ))?,
        }
    }
    Some(())
}

pub fn format(
    cfg: &Config,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    instance_ctx: &Context,
    schema_ctx: &Context,
    _ref_ctx: &Context,
    errors: &mut ErrorRecorder,
) -> Option<()> {
    if let (Value::String(instance), Value::String(schema)) = (instance, schema) {
        if let Some(checker) = cfg.get_format_checker(schema) {
            if !checker(cfg, instance) {
                errors.record_error(ValidationError::new_with_context(
                    format!("{} invalid for {} format", instance.to_string(), schema.to_string()).as_str(),
                    instance_ctx,
                    schema_ctx,
                ))?;
            }
        }
    }
    Some(())
}

pub fn minLength(
    _cfg: &Config,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    instance_ctx: &Context,
    schema_ctx: &Context,
    _ref_ctx: &Context,
    errors: &mut ErrorRecorder,
) -> Option<()> {
    if let (Value::String(instance), Value::Number(schema)) = (instance, schema) {
        let count = instance.chars().count();
        if count < schema.as_u64().unwrap() as usize {
            errors.record_error(ValidationError::new_with_context(
                format!("{} < minLength {}", instance.chars().count(), schema).as_str(),
                instance_ctx,
                schema_ctx,
            ))?;
        }
    }
    Some(())
}

pub fn maxLength(
    _cfg: &Config,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    instance_ctx: &Context,
    schema_ctx: &Context,
    _ref_ctx: &Context,
    errors: &mut ErrorRecorder,
) -> Option<()> {
    if let (Value::String(instance), Value::Number(schema)) = (instance, schema) {
        let count = instance.chars().count();
        if instance.chars().count() > schema.as_u64().unwrap() as usize {
            errors.record_error(ValidationError::new_with_context(
                format!("{} < maxLength {}", count, schema).as_str(),
                instance_ctx,
                schema_ctx,
            ))?;
        }
    }
    Some(())
}

pub fn dependencies(
    cfg: &Config,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    instance_ctx: &Context,
    schema_ctx: &Context,
    ref_ctx: &Context,
    errors: &mut ErrorRecorder,
) -> Option<()> {
    if let (Object(object), Object(schema)) = (instance, schema) {
        for (property, dependency) in schema.iter() {
            if !object.contains_key(property) {
                continue;
            }

            let dep = util::bool_to_object_schema(dependency);
            match dep {
                Object(_) => descend(
                    cfg,
                    instance,
                    dep,
                    instance_ctx,
                    &schema_ctx.push(&Value::String(property.to_string())),
                    ref_ctx,
                    errors,
                )?,
                _ => {
                    for dep0 in util::iter_or_once(dep) {
                        if let Value::String(key) = dep0 {
                            if !object.contains_key(key) {
                                errors.record_error(ValidationError::new_with_context(
                                    "dependency",
                                    instance_ctx,
                                    schema_ctx,
                                ))?;
                            }
                        }
                    }
                }
            }
        }
    }
    Some(())
}

pub fn enum_(
    _cfg: &Config,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    instance_ctx: &Context,
    schema_ctx: &Context,
    _ref_ctx: &Context,
    errors: &mut ErrorRecorder,
) -> Option<()> {
    if let Array(enums) = schema {
        if !enums.iter().any(|val| val == instance) {
            errors.record_error(ValidationError::new_with_context(
                format!("{} is not one of enum {}", instance.to_string(), schema.to_string()).as_str(),
                instance_ctx,
                schema_ctx,
            ))?;
        }
    }
    Some(())
}

#[allow(clippy::float_cmp)]
fn single_type(instance: &Value, schema: &Value) -> bool {
    if let Value::String(typename) = schema {
        return match typename.as_ref() {
            "array" => {
                if let Array(_) = instance {
                    true
                } else {
                    false
                }
            }
            "object" => {
                if let Object(_) = instance {
                    true
                } else {
                    false
                }
            }
            "null" => {
                if let Value::Null = instance {
                    true
                } else {
                    false
                }
            }
            "number" => {
                if let Value::Number(_) = instance {
                    true
                } else {
                    false
                }
            }
            "string" => {
                if let Value::String(_) = instance {
                    true
                } else {
                    false
                }
            }
            "integer" => {
                if let Value::Number(number) = instance {
                    number.is_i64()
                        || number.is_u64()
                        || (number.is_f64()
                            && number.as_f64().unwrap().trunc() == number.as_f64().unwrap())
                } else {
                    false
                }
            }
            "boolean" => {
                if let Bool(_) = instance {
                    true
                } else {
                    false
                }
            }
            _ => true,
        };
    }
    true
}

pub fn type_(
    _cfg: &Config,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    instance_ctx: &Context,
    schema_ctx: &Context,
    _ref_ctx: &Context,
    errors: &mut ErrorRecorder,
) -> Option<()> {
    if !util::iter_or_once(schema).any(|x| single_type(instance, x)) {
        errors.record_error(ValidationError::new_with_context(
            format!("{} is not of type {}", instance.to_string(), schema.to_string()).as_str(),
            instance_ctx,
            schema_ctx,
        ))?;
    }
    Some(())
}

pub fn properties(
    cfg: &Config,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    instance_ctx: &Context,
    schema_ctx: &Context,
    ref_ctx: &Context,
    errors: &mut ErrorRecorder,
) -> Option<()> {
    if let (Object(instance), Object(schema)) = (instance, schema) {
        for (property, subschema) in schema.iter() {
            if instance.contains_key(property) {
                descend(
                    cfg,
                    instance.get(property).unwrap(),
                    subschema,
                    &instance_ctx.push(&Value::String(property.to_string())),
                    &schema_ctx.push(&Value::String(property.to_string())),
                    ref_ctx,
                    errors,
                )?;
            }
        }
    }
    Some(())
}

pub fn required(
    _cfg: &Config,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    instance_ctx: &Context,
    schema_ctx: &Context,
    _ref_ctx: &Context,
    errors: &mut ErrorRecorder,
) -> Option<()> {
    // TODO: Report all in a single error
    if let (Object(instance), Array(schema)) = (instance, schema) {
        for property in schema.iter() {
            if let Value::String(key) = property {
                if !instance.contains_key(key) {
                    errors.record_error(ValidationError::new_with_context(
                        &format!("required property {} is missing", property.to_string()),
                        instance_ctx,
                        schema_ctx,
                    ))?;
                }
            }
        }
    }
    Some(())
}

pub fn minProperties(
    _cfg: &Config,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    instance_ctx: &Context,
    schema_ctx: &Context,
    _ref_ctx: &Context,
    errors: &mut ErrorRecorder,
) -> Option<()> {
    if let (Object(instance), Value::Number(schema)) = (instance, schema) {
        if instance.len() < schema.as_u64().unwrap() as usize {
            errors.record_error(ValidationError::new_with_context(
                format!("{} < minProperties {}", instance.len(), schema).as_str(),
                instance_ctx,
                schema_ctx,
            ))?;
        }
    }
    Some(())
}

pub fn maxProperties(
    _cfg: &Config,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    instance_ctx: &Context,
    schema_ctx: &Context,
    _ref_ctx: &Context,
    errors: &mut ErrorRecorder,
) -> Option<()> {
    if let (Object(instance), Value::Number(schema)) = (instance, schema) {
        if instance.len() > schema.as_u64().unwrap() as usize {
            errors.record_error(ValidationError::new_with_context(
                format!("{} > maxProperties {}", instance.len(), schema).as_str(),
                instance_ctx,
                schema_ctx,
            ))?;
        }
    }
    Some(())
}

pub fn allOf(
    cfg: &Config,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    instance_ctx: &Context,
    schema_ctx: &Context,
    ref_ctx: &Context,
    errors: &mut ErrorRecorder,
) -> Option<()> {
    if let Array(schema) = schema {
        for (index, subschema) in schema.iter().enumerate() {
            let subschema0 = if cfg.get_draft_number() >= 6 {
                util::bool_to_object_schema(subschema)
            } else {
                subschema
            };
            descend(
                cfg,
                instance,
                subschema0,
                instance_ctx,
                &schema_ctx.push(&Value::Number(Number::from(index))),
                ref_ctx,
                errors,
            )?;
        }
    }
    Some(())
}

pub fn anyOf(
    cfg: &Config,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    instance_ctx: &Context,
    schema_ctx: &Context,
    ref_ctx: &Context,
    errors: &mut ErrorRecorder,
) -> Option<()> {
    if let Array(schema) = schema {
        for (index, subschema) in schema.iter().enumerate() {
            let mut local_errors = ValidationErrors::new();

            let subschema0 = if cfg.get_draft_number() >= 6 {
                util::bool_to_object_schema(subschema)
            } else {
                subschema
            };

            descend(
                cfg,
                instance,
                subschema0,
                instance_ctx,
                &schema_ctx.push(&Value::Number(Number::from(index))),
                ref_ctx,
                &mut local_errors,
            );
            if !local_errors.has_errors() {
                return Some(());
            }
        }
        // TODO: Wrap local_errors into a single error
        errors.record_error(ValidationError::new_with_context(
            "anyOf",
            instance_ctx,
            schema_ctx,
        ))?;
    }
    Some(())
}

pub fn oneOf(
    cfg: &Config,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    instance_ctx: &Context,
    schema_ctx: &Context,
    ref_ctx: &Context,
    errors: &mut ErrorRecorder,
) -> Option<()> {
    if let Array(schema) = schema {
        let mut oneOf = schema.iter();
        let mut found_one = false;
        // TODO: Gather errors
        for (index, subschema) in oneOf.by_ref().enumerate() {
            let subschema0 = if cfg.get_draft_number() >= 6 {
                util::bool_to_object_schema(subschema)
            } else {
                subschema
            };
            if descend(
                cfg,
                instance,
                subschema0,
                instance_ctx,
                &schema_ctx.push(&Value::Number(Number::from(index))),
                ref_ctx,
                &mut FastFailErrorRecorder::new()
            ).is_some() {
                found_one = true;
                break;
            }
        }

        if !found_one {
            errors.record_error(ValidationError::new_with_context(
                "Nothing matched in oneOf",
                instance_ctx,
                schema_ctx,
            ))?;
            return Some(())
        }

        let mut found_more = false;
        for (index, subschema) in oneOf.by_ref().enumerate() {
            let subschema0 = util::bool_to_object_schema(subschema);
            if descend(
                cfg,
                instance,
                subschema0,
                instance_ctx,
                &schema_ctx.push(&Value::Number(Number::from(index))),
                ref_ctx,
                &mut FastFailErrorRecorder::new(),
            ).is_some() {
                found_more = true;
                break;
            }
        }

        if found_more {
            errors.record_error(ValidationError::new_with_context(
                "More than one matched in oneOf",
                instance_ctx,
                schema_ctx,
            ))?;
        }
    }
    Some(())
}

pub fn not(
    cfg: &Config,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    instance_ctx: &Context,
    schema_ctx: &Context,
    ref_ctx: &Context,
    errors: &mut ErrorRecorder,
) -> Option<()> {
    if descend(
        cfg,
        instance,
        schema,
        instance_ctx,
        schema_ctx,
        ref_ctx,
        &mut FastFailErrorRecorder::new(),
    ).is_some() {
        errors.record_error(ValidationError::new_with_context(
            "not",
            instance_ctx,
            schema_ctx,
        ))?;
    }
    Some(())
}

pub fn ref_(
    cfg: &Config,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    instance_ctx: &Context,
    schema_ctx: &Context,
    ref_ctx: &Context,
    errors: &mut ErrorRecorder,
) -> Option<()> {
    if let Value::String(sref) = schema {
        match cfg
            .get_resolver()
            .resolve_fragment(sref, ref_ctx, cfg.get_schema())
        {
            Ok((scope, resolved)) => {
                let scope_schema = json!({"$id": scope.to_string()});
                descend(
                    cfg,
                    instance,
                    resolved,
                    instance_ctx,
                    schema_ctx,
                    &ref_ctx.push(&scope_schema),
                    errors,
                )?;
            }
            Err(err) => errors.record_error(err)?,
        }
    }
    Some(())
}

pub fn if_(
    cfg: &Config,
    instance: &Value,
    schema: &Value,
    parent_schema: &Map<String, Value>,
    instance_ctx: &Context,
    schema_ctx: &Context,
    ref_ctx: &Context,
    errors: &mut ErrorRecorder,
) -> Option<()> {
    match descend(
        cfg,
        instance,
        schema,
        instance_ctx,
        schema_ctx,
        ref_ctx,
        &mut FastFailErrorRecorder::new()
    ) {
        Some(_) => {
            if parent_schema.contains_key("then") {
                let then = &parent_schema["then"];
                if let Object(_) = then {
                    descend(
                        cfg,
                        instance,
                        &then,
                        instance_ctx,
                        &schema_ctx.parent.unwrap().push(&Value::String("then".to_string())),
                        ref_ctx,
                        errors
                    )?
                }
            }
        }
        None => {
            if parent_schema.contains_key("else") {
                let else_ = &parent_schema["else"];
                if let Object(_) = else_ {
                    descend(
                        cfg,
                        instance,
                        &else_,
                        instance_ctx,
                        &schema_ctx.parent.unwrap().push(&Value::String("else".to_string())),
                        ref_ctx,
                        errors
                    )?
                }
            }
        }
    }
    Some(())
}
