#![allow(non_snake_case)]

use regex;

use serde_json::{Map, Value};

use context::Context;
use error::ValidationError;
use unique;
use util;

pub struct ScopeStack<'a> {
    pub x: &'a Value,
    pub parent: Option<&'a ScopeStack<'a>>,
}

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
        Value::Bool(b) => {
            if *b {
                Ok(())
            } else {
                Err(ValidationError::new("False schema always fails"))
            }
        }
        Value::Object(schema_object) => {
            if schema_object.contains_key("$ref") {
                if let Some(validator) = ctx.get_validator("$ref") {
                    if let Err(err) = validator(
                        ctx,
                        instance,
                        schema_object.get("$ref").unwrap(),
                        schema_object,
                        stack,
                    ) {
                        return Err(err);
                    }
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

    if let Err(mut err) = run_validators(ctx, instance, schema, &ScopeStack { x: schema, parent: Some(stack) }) {
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

pub fn validate_patternProperties(
    ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    stack: &ScopeStack,
) -> ValidatorResult {
    if let Value::Object(instance) = instance {
        if let Value::Object(schema) = schema {
            for (pattern, subschema) in schema.iter() {
                let re = regex::Regex::new(pattern)?;
                for (k, v) in instance.iter() {
                    if re.is_match(k) {
                        descend(ctx, v, subschema, Some(k), Some(pattern), stack)?;
                    }
                }
            }
        }
    }
    Ok(())
}

pub fn validate_propertyNames(
    ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    stack: &ScopeStack,
) -> ValidatorResult {
    if let Value::Object(instance) = instance {
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
    lazy_static! {
        static ref EMPTY_OBJ: Value = Value::Object(Map::new());
    }
    let properties = schema.get("properties").unwrap_or_else(move || &EMPTY_OBJ);
    let pattern_properties = schema
        .get("patternProperties")
        .unwrap_or_else(move || &EMPTY_OBJ);
    if let Value::Object(properties) = properties {
        if let Value::Object(pattern_properties) = pattern_properties {
            let pattern_regexes_result: Result<Vec<regex::Regex>, regex::Error> =
                pattern_properties
                    .keys()
                    .map(|k| regex::Regex::new(k))
                    .collect();
            let pattern_regexes = pattern_regexes_result?;
            return Ok(Box::new(
                instance
                    .keys()
                    .filter(move |&property| !properties.contains_key(property))
                    .filter(move |&property| !pattern_regexes.iter().any(|x| x.is_match(property))),
            ));
        }
    }
    Ok(Box::new(instance.keys()))
}

pub fn validate_additionalProperties(
    ctx: &Context,
    instance: &Value,
    schema: &Value,
    parent_schema: &Map<String, Value>,
    stack: &ScopeStack,
) -> ValidatorResult {
    if let Value::Object(instance) = instance {
        let mut extras = find_additional_properties(instance, parent_schema)?;
        match schema {
            Value::Object(_) => {
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
            Value::Bool(bool) => {
                if !bool {
                    if let Some(_) = extras.next() {
                        return Err(ValidationError::new(
                            "Additional properties are not allowed",
                        ));
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}

// TODO: items_draft3/4

pub fn validate_items(
    ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    stack: &ScopeStack,
) -> ValidatorResult {
    if let Value::Array(instance) = instance {
        let items = util::bool_to_object_schema(schema);

        match items {
            Value::Object(_) => for (index, item) in instance.iter().enumerate() {
                descend(ctx, item, items, Some(&index.to_string()), None, stack)?;
            },
            Value::Array(items) => {
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

pub fn validate_additionalItems(
    ctx: &Context,
    instance: &Value,
    schema: &Value,
    parent_schema: &Map<String, Value>,
    stack: &ScopeStack,
) -> ValidatorResult {
    if !parent_schema.contains_key("items") {
        return Ok(());
    } else if let Value::Object(_) = parent_schema["items"] {
        return Ok(());
    }

    if let Value::Array(instance) = instance {
        let len_items = parent_schema.get("items").map_or(0, |x| match x {
            Value::Array(array) => array.len(),
            _ => 0,
        });
        match schema {
            Value::Object(_) => for i in len_items..instance.len() {
                descend(ctx, &instance[i], schema, Some(&i.to_string()), None, stack)?;
            },
            Value::Bool(b) => if !b && instance.len() > len_items {
                return Err(ValidationError::new("Additional items are not allowed"));
            },
            _ => {}
        }
    }
    Ok(())
}

pub fn validate_const(
    _ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    stack: &ScopeStack,
) -> ValidatorResult {
    if instance != schema {
        return Err(ValidationError::new("Invalid const"));
    }
    Ok(())
}

pub fn validate_contains(
    ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    stack: &ScopeStack,
) -> ValidatorResult {
    if let Value::Array(instance) = instance {
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

// TODO: minimum draft 3/4
// TODO: maximum draft 3/4

pub fn validate_exclusiveMinimum(
    _ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    stack: &ScopeStack,
) -> ValidatorResult {
    if let Value::Number(instance) = instance {
        if let Value::Number(schema) = schema {
            if instance.as_f64() <= schema.as_f64() {
                return Err(ValidationError::new("exclusiveMinimum"));
            }
        }
    }
    Ok(())
}

pub fn validate_exclusiveMaximum(
    _ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    stack: &ScopeStack,
) -> ValidatorResult {
    if let Value::Number(instance) = instance {
        if let Value::Number(schema) = schema {
            if instance.as_f64() >= schema.as_f64() {
                return Err(ValidationError::new("exclusiveMaximum"));
            }
        }
    }
    Ok(())
}

pub fn validate_minimum(
    _ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    stack: &ScopeStack,
) -> ValidatorResult {
    if let Value::Number(instance) = instance {
        if let Value::Number(schema) = schema {
            if instance.as_f64() < schema.as_f64() {
                return Err(ValidationError::new("minimum"));
            }
        }
    }
    Ok(())
}

pub fn validate_maximum(
    _ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    stack: &ScopeStack,
) -> ValidatorResult {
    if let Value::Number(instance) = instance {
        if let Value::Number(schema) = schema {
            if instance.as_f64() > schema.as_f64() {
                return Err(ValidationError::new("maximum"));
            }
        }
    }
    Ok(())
}

pub fn validate_multipleOf(
    _ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    stack: &ScopeStack,
) -> ValidatorResult {
    if let Value::Number(instance) = instance {
        if let Value::Number(schema) = schema {
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
    }
    Ok(())
}

pub fn validate_minItems(
    _ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    stack: &ScopeStack,
) -> ValidatorResult {
    if let Value::Array(instance) = instance {
        if let Value::Number(schema) = schema {
            if instance.len() < schema.as_u64().unwrap() as usize {
                return Err(ValidationError::new("minItems"));
            }
        }
    }
    Ok(())
}

pub fn validate_maxItems(
    _ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    stack: &ScopeStack,
) -> ValidatorResult {
    if let Value::Array(instance) = instance {
        if let Value::Number(schema) = schema {
            if instance.len() > schema.as_u64().unwrap() as usize {
                return Err(ValidationError::new("minItems"));
            }
        }
    }
    Ok(())
}

pub fn validate_uniqueItems(
    _ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    stack: &ScopeStack,
) -> ValidatorResult {
    if let Value::Array(instance) = instance {
        if let Value::Bool(b) = schema {
            if *b && !unique::has_unique_elements(&mut instance.iter()) {
                return Err(ValidationError::new("uniqueItems"));
            }
        }
    }
    Ok(())
}

pub fn validate_pattern(
    _ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    stack: &ScopeStack,
) -> ValidatorResult {
    if let Value::String(instance) = instance {
        if let Value::String(schema) = schema {
            if !regex::Regex::new(schema)?.is_match(instance) {
                return Err(ValidationError::new("pattern"));
            }
        }
    }
    Ok(())
}

// TODO format

pub fn validate_minLength(
    _ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    stack: &ScopeStack,
) -> ValidatorResult {
    if let Value::String(instance) = instance {
        if let Value::Number(schema) = schema {
            if instance.chars().count() < schema.as_u64().unwrap() as usize {
                return Err(ValidationError::new("minLength"));
            }
        }
    }
    Ok(())
}

pub fn validate_maxLength(
    _ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    stack: &ScopeStack,
) -> ValidatorResult {
    if let Value::String(instance) = instance {
        if let Value::Number(schema) = schema {
            if instance.chars().count() > schema.as_u64().unwrap() as usize {
                return Err(ValidationError::new("maxLength"));
            }
        }
    }
    Ok(())
}

pub fn validate_dependencies(
    ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    stack: &ScopeStack,
) -> ValidatorResult {
    if let Value::Object(object) = instance {
        if let Value::Object(schema) = schema {
            for (property, dependency) in schema.iter() {
                if !object.contains_key(property) {
                    continue;
                }

                let dep = util::bool_to_object_schema(dependency);
                match dep {
                    Value::Object(_) => descend(ctx, instance, dep, None, Some(property), stack)?,
                    _ => {
                        for dep0 in util::iter_or_once(dep) {
                            if let Value::String(key) = dep0 {
                                println!("key {}", key);
                                if !object.contains_key(key) {
                                    return Err(ValidationError::new("dependency"));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

pub fn validate_enum(
    _ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    stack: &ScopeStack,
) -> ValidatorResult {
    if let Value::Array(enums) = schema {
        if !enums.iter().any(|val| val == instance) {
            return Err(ValidationError::new("enum"));
        }
    }
    Ok(())
}

// TODO: ref

// TODO: type draft3
// TODO: properties draft3
// TODO: disallow draft3
// TODO: extends draft3

pub fn validate_single_type(instance: &Value, schema: &Value) -> ValidatorResult {
    if let Value::String(typename) = schema {
        match typename.as_ref() {
            "array" => {
                if let Value::Array(_) = instance {
                    return Ok(());
                } else {
                    return Err(ValidationError::new("array"));
                }
            }
            "object" => {
                if let Value::Object(_) = instance {
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
                if let Value::Number(_) = instance {
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
                if let Value::Number(number) = instance {
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
                if let Value::Bool(_) = instance {
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

pub fn validate_type(
    _ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    stack: &ScopeStack,
) -> ValidatorResult {
    if !util::iter_or_once(schema).any(|x| validate_single_type(instance, x).is_ok()) {
        return Err(ValidationError::new("type"));
    }
    Ok(())
}

pub fn validate_properties(
    ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    stack: &ScopeStack,
) -> ValidatorResult {
    if let Value::Object(instance) = instance {
        if let Value::Object(schema) = schema {
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
    }
    Ok(())
}

pub fn validate_required(
    _ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    stack: &ScopeStack,
) -> ValidatorResult {
    if let Value::Object(instance) = instance {
        if let Value::Array(schema) = schema {
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
    }
    Ok(())
}

pub fn validate_minProperties(
    _ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    stack: &ScopeStack,
) -> ValidatorResult {
    if let Value::Object(instance) = instance {
        if let Value::Number(schema) = schema {
            if instance.len() < schema.as_u64().unwrap() as usize {
                return Err(ValidationError::new("minProperties"));
            }
        }
    }
    Ok(())
}

pub fn validate_maxProperties(
    _ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    stack: &ScopeStack,
) -> ValidatorResult {
    if let Value::Object(instance) = instance {
        if let Value::Number(schema) = schema {
            if instance.len() > schema.as_u64().unwrap() as usize {
                return Err(ValidationError::new("maxProperties"));
            }
        }
    }
    Ok(())
}

pub fn validate_allOf(
    ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    stack: &ScopeStack,
) -> ValidatorResult {
    if let Value::Array(schema) = schema {
        for (index, subschema) in schema.iter().enumerate() {
            let subschema0 = util::bool_to_object_schema(subschema);
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

pub fn validate_anyOf(
    ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    stack: &ScopeStack,
) -> ValidatorResult {
    if let Value::Array(schema) = schema {
        for (index, subschema) in schema.iter().enumerate() {
            let subschema0 = util::bool_to_object_schema(subschema);
            // TODO Wrap up all errors into a list
            if descend(
                ctx,
                instance,
                subschema0,
                None,
                Some(&index.to_string()),
                stack,
            ).is_ok()
            {
                return Ok(());
            }
        }
        return Err(ValidationError::new("anyOf"));
    }
    Ok(())
}

pub fn validate_oneOf(
    ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    stack: &ScopeStack,
) -> ValidatorResult {
    if let Value::Array(schema) = schema {
        let mut oneOf = schema.into_iter();
        let mut found_one = false;
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
                found_one = true;
                break;
            }
        }

        if !found_one {
            return Err(ValidationError::new("Nothing matched in oneOf"));
        }

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
                return Err(ValidationError::new("More than one matched in oneOf"));
            }
        }
    }
    Ok(())
}

pub fn validate_not(
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

pub fn validate_ref(
    ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    stack: &ScopeStack,
) -> ValidatorResult {
    if let Value::String(sref) = schema {
        let (scope, resolved) = ctx.get_resolver().resolve_fragment(sref, stack, ctx.get_schema())?;
        println!("Resolved {:?}", resolved);
        let scope_schema = json!({"$id": scope.to_string()});
        let new_stack = ScopeStack {
            x: &scope_schema,
            parent: Some(stack)
        };
        descend(ctx, instance, resolved, None, None, &new_stack)?
    }
    Ok(())
}
