#![allow(non_snake_case)]

use regex;

use serde_json::{Map, Value, Value::Array, Value::Bool, Value::Number, Value::Object};

use context::Context;
use error::ValidationError;
use unique;
use util;

// TODO: Move this to another module???
pub struct ScopeStack<'a> {
    pub x: &'a Value,
    pub parent: Option<&'a ScopeStack<'a>>,
}

// TODO: Move this to another module???
pub type ValidatorResult = Result<(), ValidationError>;

pub type Validator = fn(
    ctx: &Context,
    instance: &Value,
    schema: &Value,
    parent_schema: &Map<String, Value>,
    stack: &ScopeStack,
) -> ValidatorResult;

pub fn run_validators<'a>(
    ctx: &Context,
    instance: &Value,
    schema: &Value,
    stack: &ScopeStack<'a>,
) -> ValidatorResult {
    match schema {
        Bool(b) => {
            if *b {
                Ok(())
            } else {
                Err(ValidationError::new("false schema always fails"))
            }
        }
        Object(schema_object) => {
            if schema_object.contains_key("$ref") {
                if let Some(validator) = ctx.get_validator("$ref") {
                    validator(
                        ctx,
                        instance,
                        schema_object.get("$ref").unwrap(),
                        schema_object,
                        stack,
                    )?;
                }
            } else {
                for (k, v) in schema_object.iter() {
                    if let Some(validator) = ctx.get_validator(k.as_ref()) {
                        if let Err(mut err) = validator(ctx, instance, v, schema_object, stack) {
                            err.add_schema_path(k);
                            return Err(err);
                        }
                    }
                }
            }
            Ok(())
        }
        _ => Err(ValidationError::new("Invalid schema")),
    }
}

pub fn is_valid(ctx: &Context, instance: &Value, schema: &Value) -> bool {
    run_validators(
        ctx,
        instance,
        schema,
        &ScopeStack {
            x: schema,
            parent: None,
        },
    ).is_ok()
}

fn descend(
    ctx: &Context,
    instance: &Value,
    schema: &Value,
    instance_key: Option<&String>,
    schema_key: Option<&String>,
    stack: &ScopeStack,
) -> ValidatorResult {
    if let Err(mut err) = run_validators(
        ctx,
        instance,
        schema,
        &ScopeStack {
            x: schema,
            parent: Some(stack),
        },
    ) {
        if let Some(instance_key) = instance_key {
            err.add_instance_path(instance_key);
        }
        if let Some(schema_key) = schema_key {
            err.add_schema_path(schema_key);
        }
        Err(err)
    } else {
        Ok(())
    }
}

pub fn patternProperties(
    ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    stack: &ScopeStack,
) -> ValidatorResult {
    if let (Object(instance), Object(schema)) = (instance, schema) {
        for (pattern, subschema) in schema.iter() {
            let re = regex::Regex::new(pattern)?;
            for (k, v) in instance.iter() {
                if re.is_match(k) {
                    descend(ctx, v, subschema, Some(k), Some(pattern), stack)?;
                }
            }
        }
    }
    Ok(())
}

pub fn propertyNames(
    ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    stack: &ScopeStack,
) -> ValidatorResult {
    if let Object(instance) = instance {
        for property in instance.keys() {
            descend(
                ctx,
                &Value::String(property.to_string()),
                schema,
                Some(property),
                None,
                stack,
            )?;
        }
    }
    Ok(())
}

fn find_additional_properties<'a>(
    instance: &'a Map<String, Value>,
    schema: &'a Map<String, Value>,
) -> Result<Box<Iterator<Item = &'a String> + 'a>, ValidationError> {
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
    Ok(Box::new(
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
    ))
}

pub fn additionalProperties(
    ctx: &Context,
    instance: &Value,
    schema: &Value,
    parent_schema: &Map<String, Value>,
    stack: &ScopeStack,
) -> ValidatorResult {
    if let Object(instance) = instance {
        let mut extras = find_additional_properties(instance, parent_schema)?;
        match schema {
            Object(_) => {
                for extra in extras {
                    descend(
                        ctx,
                        instance.get(extra).expect("Property gone missing."),
                        schema,
                        Some(extra),
                        None,
                        stack,
                    )?;
                }
            }
            Bool(bool) => {
                if !bool && extras.next().is_some() {
                    return Err(ValidationError::new(
                        "Additional properties are not allowed",
                    ));
                }
            }
            _ => {}
        }
    }
    Ok(())
}

pub fn items_draft4(
    ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    stack: &ScopeStack,
) -> ValidatorResult {
    if let Array(instance) = instance {
        match schema {
            Object(_) => for (index, item) in instance.iter().enumerate() {
                descend(ctx, item, schema, Some(&index.to_string()), None, stack)?;
            },
            Array(items) => {
                for ((index, item), subschema) in instance.iter().enumerate().zip(items.iter()) {
                    descend(
                        ctx,
                        item,
                        subschema,
                        Some(&index.to_string()),
                        Some(&index.to_string()),
                        stack,
                    )?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}

pub fn items(
    ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    stack: &ScopeStack,
) -> ValidatorResult {
    if let Array(instance) = instance {
        let items = util::bool_to_object_schema(schema);

        match items {
            Object(_) => for (index, item) in instance.iter().enumerate() {
                descend(ctx, item, items, Some(&index.to_string()), None, stack)?;
            },
            Array(items) => {
                for ((index, item), subschema) in instance.iter().enumerate().zip(items.iter()) {
                    descend(
                        ctx,
                        item,
                        subschema,
                        Some(&index.to_string()),
                        Some(&index.to_string()),
                        stack,
                    )?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}

pub fn additionalItems(
    ctx: &Context,
    instance: &Value,
    schema: &Value,
    parent_schema: &Map<String, Value>,
    stack: &ScopeStack,
) -> ValidatorResult {
    if !parent_schema.contains_key("items") {
        return Ok(());
    } else if let Object(_) = parent_schema["items"] {
        return Ok(());
    }

    if let Array(instance) = instance {
        let len_items = parent_schema
            .get("items")
            .and_then(|x| x.as_array())
            .map_or_else(|| 0, |x| x.len());
        match schema {
            Object(_) => for (i, item) in instance.iter().enumerate().skip(len_items) {
                descend(ctx, &item, schema, Some(&i.to_string()), None, stack)?;
            },
            Bool(b) => if !b && instance.len() > len_items {
                return Err(ValidationError::new("Additional items are not allowed"));
            },
            _ => {}
        }
    }
    Ok(())
}

pub fn const_(
    _ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    _stack: &ScopeStack,
) -> ValidatorResult {
    if instance != schema {
        return Err(ValidationError::new("Invalid const"));
    }
    Ok(())
}

pub fn contains(
    ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    _stack: &ScopeStack,
) -> ValidatorResult {
    if let Array(instance) = instance {
        if !instance
            .iter()
            .any(|element| is_valid(ctx, element, schema))
        {
            return Err(ValidationError::new(
                "Nothing is valid under the given schema",
            ));
        }
    }
    Ok(())
}

pub fn exclusiveMinimum(
    _ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    _stack: &ScopeStack,
) -> ValidatorResult {
    if let (Number(instance), Number(schema)) = (instance, schema) {
        if instance.as_f64() <= schema.as_f64() {
            return Err(ValidationError::new("exclusiveMinimum"));
        }
    }
    Ok(())
}

pub fn exclusiveMaximum(
    _ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    _stack: &ScopeStack,
) -> ValidatorResult {
    if let (Number(instance), Number(schema)) = (instance, schema) {
        if instance.as_f64() >= schema.as_f64() {
            return Err(ValidationError::new("exclusiveMaximum"));
        }
    }
    Ok(())
}

pub fn minimum_draft4(
    _ctx: &Context,
    instance: &Value,
    schema: &Value,
    parent_schema: &Map<String, Value>,
    _stack: &ScopeStack,
) -> ValidatorResult {
    if let (Number(instance), Number(minimum)) = (instance, schema) {
        let failed = if parent_schema
            .get("exclusiveMinimum")
            .and_then(|x| x.as_bool())
            .unwrap_or_else(|| false)
        {
            instance.as_f64() <= minimum.as_f64()
        } else {
            instance.as_f64() < minimum.as_f64()
        };
        if failed {
            return Err(ValidationError::new("minimum"));
        }
    }
    Ok(())
}

pub fn minimum(
    _ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    _stack: &ScopeStack,
) -> ValidatorResult {
    if let (Number(instance), Number(schema)) = (instance, schema) {
        if instance.as_f64() < schema.as_f64() {
            return Err(ValidationError::new("minimum"));
        }
    }
    Ok(())
}

pub fn maximum_draft4(
    _ctx: &Context,
    instance: &Value,
    schema: &Value,
    parent_schema: &Map<String, Value>,
    _stack: &ScopeStack,
) -> ValidatorResult {
    if let (Number(instance), Number(maximum)) = (instance, schema) {
        let failed = if parent_schema
            .get("exclusiveMaximum")
            .and_then(|x| x.as_bool())
            .unwrap_or_else(|| false)
        {
            instance.as_f64() >= maximum.as_f64()
        } else {
            instance.as_f64() > maximum.as_f64()
        };
        if failed {
            return Err(ValidationError::new("maximum"));
        }
    }
    Ok(())
}

pub fn maximum(
    _ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    _stack: &ScopeStack,
) -> ValidatorResult {
    if let (Number(instance), Number(schema)) = (instance, schema) {
        if instance.as_f64() > schema.as_f64() {
            return Err(ValidationError::new("maximum"));
        }
    }
    Ok(())
}

#[allow(clippy::float_cmp)]
pub fn multipleOf(
    _ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    _stack: &ScopeStack,
) -> ValidatorResult {
    if let (Number(instance), Number(schema)) = (instance, schema) {
        let failed = if schema.is_f64() {
            let quotient = instance.as_f64().unwrap() / schema.as_f64().unwrap();
            quotient.trunc() != quotient
        } else if schema.is_u64() {
            (instance.as_u64().unwrap() % schema.as_u64().unwrap()) != 0
        } else {
            (instance.as_i64().unwrap() % schema.as_i64().unwrap()) != 0
        };
        if failed {
            return Err(ValidationError::new("not multipleOf"));
        }
    }
    Ok(())
}

pub fn minItems(
    _ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    _stack: &ScopeStack,
) -> ValidatorResult {
    if let (Array(instance), Number(schema)) = (instance, schema) {
        if instance.len() < schema.as_u64().unwrap() as usize {
            return Err(ValidationError::new("minItems"));
        }
    }
    Ok(())
}

pub fn maxItems(
    _ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    _stack: &ScopeStack,
) -> ValidatorResult {
    if let (Array(instance), Number(schema)) = (instance, schema) {
        if instance.len() > schema.as_u64().unwrap() as usize {
            return Err(ValidationError::new("minItems"));
        }
    }
    Ok(())
}

pub fn uniqueItems(
    _ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    _stack: &ScopeStack,
) -> ValidatorResult {
    if let (Array(instance), Bool(schema)) = (instance, schema) {
        if *schema && !unique::has_unique_elements(&mut instance.iter()) {
            return Err(ValidationError::new("uniqueItems"));
        }
    }
    Ok(())
}

pub fn pattern(
    _ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    _stack: &ScopeStack,
) -> ValidatorResult {
    if let (Value::String(instance), Value::String(schema)) = (instance, schema) {
        if !regex::Regex::new(schema)?.is_match(instance) {
            return Err(ValidationError::new("pattern"));
        }
    }
    Ok(())
}

pub fn format(
    ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    _stack: &ScopeStack,
) -> ValidatorResult {
    if let (Value::String(instance), Value::String(schema)) = (instance, schema) {
        if let Some(checker) = ctx.get_format_checker(schema) {
            if !checker(ctx, instance) {
                return Err(ValidationError::new("format"));
            }
        }
    }
    Ok(())
}

pub fn minLength(
    _ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    _stack: &ScopeStack,
) -> ValidatorResult {
    if let (Value::String(instance), Number(schema)) = (instance, schema) {
        if instance.chars().count() < schema.as_u64().unwrap() as usize {
            return Err(ValidationError::new("minLength"));
        }
    }
    Ok(())
}

pub fn maxLength(
    _ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    _stack: &ScopeStack,
) -> ValidatorResult {
    if let (Value::String(instance), Number(schema)) = (instance, schema) {
        if instance.chars().count() > schema.as_u64().unwrap() as usize {
            return Err(ValidationError::new("maxLength"));
        }
    }
    Ok(())
}

pub fn dependencies(
    ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    stack: &ScopeStack,
) -> ValidatorResult {
    if let (Object(object), Object(schema)) = (instance, schema) {
        for (property, dependency) in schema.iter() {
            if !object.contains_key(property) {
                continue;
            }

            let dep = util::bool_to_object_schema(dependency);
            match dep {
                Object(_) => descend(ctx, instance, dep, None, Some(property), stack)?,
                _ => {
                    for dep0 in util::iter_or_once(dep) {
                        if let Value::String(key) = dep0 {
                            if !object.contains_key(key) {
                                return Err(ValidationError::new("dependency"));
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

pub fn enum_(
    _ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    _stack: &ScopeStack,
) -> ValidatorResult {
    if let Array(enums) = schema {
        if !enums.iter().any(|val| val == instance) {
            return Err(ValidationError::new("enum"));
        }
    }
    Ok(())
}

#[allow(clippy::float_cmp)]
fn single_type(instance: &Value, schema: &Value) -> ValidatorResult {
    if let Value::String(typename) = schema {
        match typename.as_ref() {
            "array" => {
                if let Array(_) = instance {
                    return Ok(());
                } else {
                    return Err(ValidationError::new("array"));
                }
            }
            "object" => {
                if let Object(_) = instance {
                    return Ok(());
                } else {
                    return Err(ValidationError::new("object"));
                }
            }
            "null" => {
                if let Value::Null = instance {
                    return Ok(());
                } else {
                    return Err(ValidationError::new("null"));
                }
            }
            "number" => {
                if let Number(_) = instance {
                    return Ok(());
                } else {
                    return Err(ValidationError::new("number"));
                }
            }
            "string" => {
                if let Value::String(_) = instance {
                    return Ok(());
                } else {
                    return Err(ValidationError::new("string"));
                }
            }
            "integer" => {
                if let Number(number) = instance {
                    if number.is_i64() || number.is_u64()
                        || (number.is_f64()
                            && number.as_f64().unwrap().trunc() == number.as_f64().unwrap())
                    {
                        return Ok(());
                    }
                }
                return Err(ValidationError::new("integer"));
            }
            "boolean" => {
                if let Bool(_) = instance {
                    return Ok(());
                } else {
                    return Err(ValidationError::new("boolean"));
                }
            }
            _ => return Ok(()),
        }
    }
    Ok(())
}

pub fn type_(
    _ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    _stack: &ScopeStack,
) -> ValidatorResult {
    if !util::iter_or_once(schema).any(|x| single_type(instance, x).is_ok()) {
        return Err(ValidationError::new("type"));
    }
    Ok(())
}

pub fn properties(
    ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    stack: &ScopeStack,
) -> ValidatorResult {
    if let (Object(instance), Object(schema)) = (instance, schema) {
        for (property, subschema) in schema.iter() {
            if instance.contains_key(property) {
                descend(
                    ctx,
                    instance.get(property).unwrap(),
                    subschema,
                    Some(property),
                    Some(property),
                    stack,
                )?;
            }
        }
    }
    Ok(())
}

pub fn required(
    _ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    _stack: &ScopeStack,
) -> ValidatorResult {
    if let (Object(instance), Array(schema)) = (instance, schema) {
        for property in schema.iter() {
            if let Value::String(key) = property {
                if !instance.contains_key(key) {
                    return Err(ValidationError::new(&format!(
                        "required property '{}' missing",
                        key
                    )));
                }
            }
        }
    }
    Ok(())
}

pub fn minProperties(
    _ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    _stack: &ScopeStack,
) -> ValidatorResult {
    if let (Object(instance), Number(schema)) = (instance, schema) {
        if instance.len() < schema.as_u64().unwrap() as usize {
            return Err(ValidationError::new("minProperties"));
        }
    }
    Ok(())
}

pub fn maxProperties(
    _ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    _stack: &ScopeStack,
) -> ValidatorResult {
    if let (Object(instance), Number(schema)) = (instance, schema) {
        if instance.len() > schema.as_u64().unwrap() as usize {
            return Err(ValidationError::new("maxProperties"));
        }
    }
    Ok(())
}

pub fn allOf(
    ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    stack: &ScopeStack,
) -> ValidatorResult {
    if let Array(schema) = schema {
        for (index, subschema) in schema.iter().enumerate() {
            let subschema0 = if ctx.get_draft_number() >= 6 {
                util::bool_to_object_schema(subschema)
            } else {
                subschema
            };
            descend(
                ctx,
                instance,
                subschema0,
                None,
                Some(&index.to_string()),
                stack,
            )?;
        }
    }
    Ok(())
}

pub fn anyOf(
    ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    stack: &ScopeStack,
) -> ValidatorResult {
    if let Array(schema) = schema {
        let mut errors: Vec<ValidationError> = Vec::new();
        errors.reserve(schema.len());
        for (index, subschema) in schema.iter().enumerate() {
            let subschema0 = if ctx.get_draft_number() >= 6 {
                util::bool_to_object_schema(subschema)
            } else {
                subschema
            };

            match descend(
                ctx,
                instance,
                subschema0,
                None,
                Some(&index.to_string()),
                stack,
            ) {
                Ok(_) => return Ok(()),
                Err(err) => errors.push(err)
            }
        }
        return Err(ValidationError::from_errors("anyOf", &errors));
    }
    Ok(())
}

pub fn oneOf(
    ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    stack: &ScopeStack,
) -> ValidatorResult {
    if let Array(schema) = schema {
        let mut oneOf = schema.iter();
        let mut found_one = false;
        let mut errors: Vec<ValidationError> = Vec::new();
        for (index, subschema) in oneOf.by_ref().enumerate() {
            let subschema0 = if ctx.get_draft_number() >= 6 {
                util::bool_to_object_schema(subschema)
            } else {
                subschema
            };
            match descend(
                ctx,
                instance,
                subschema0,
                None,
                Some(&index.to_string()),
                stack,
            ) {
                Ok(_) => { found_one = true; break },
                Err(err) => errors.push(err)
            }
        }

        if !found_one {
            return Err(ValidationError::from_errors("Nothing matched in oneOf", &errors));
        }

        let mut found_more = false;
        for (index, subschema) in oneOf.by_ref().enumerate() {
            let subschema0 = util::bool_to_object_schema(subschema);
            if descend(
                ctx,
                instance,
                subschema0,
                None,
                Some(&index.to_string()),
                stack,
            ).is_ok()
            {
                found_more = true;
            }
        }

        if found_more {
            return Err(ValidationError::from_errors("More than one matched in oneOf", &errors));
        }
    }
    Ok(())
}

pub fn not(
    ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    stack: &ScopeStack,
) -> ValidatorResult {
    if run_validators(ctx, instance, schema, stack).is_ok() {
        return Err(ValidationError::new("not"));
    }
    Ok(())
}

pub fn ref_(
    ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    stack: &ScopeStack,
) -> ValidatorResult {
    if let Value::String(sref) = schema {
        let (scope, resolved) = ctx
            .get_resolver()
            .resolve_fragment(sref, stack, ctx.get_schema())?;
        let scope_schema = json!({"$id": scope.to_string()});
        let new_stack = ScopeStack {
            x: &scope_schema,
            parent: Some(stack),
        };
        descend(ctx, instance, resolved, None, None, &new_stack)?
    }
    Ok(())
}
