#![allow(non_snake_case)]

use regex;

use serde_json::{Map, Value, Value::Array, Value::Bool, Value::Number, Value::Object};

use context::Context;
use error::{ErrorRecorder, ScopeStack, ValidationError, VecErrorRecorder};
use unique;
use util;

pub type Validator = fn(
    ctx: &Context,
    instance: &Value,
    schema: &Value,
    parent_schema: &Map<String, Value>,
    stack: &ScopeStack,
    errors: &mut ErrorRecorder,
);

pub fn run_validators<'a>(
    ctx: &Context,
    instance: &Value,
    schema: &Value,
    stack: &ScopeStack<'a>,
    errors: &mut ErrorRecorder,
) {
    match schema {
        Bool(b) => {
            if !*b {
                errors.record_error(ValidationError::new("false schema always fails"))
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
                        errors,
                    );
                }
            } else {
                for (k, v) in schema_object.iter() {
                    if let Some(validator) = ctx.get_validator(k.as_ref()) {
                        validator(ctx, instance, v, schema_object, stack, errors)
                    }
                }
            }
        }
        _ => errors.record_error(ValidationError::new("Invalid schema")),
    }
}

pub fn is_valid(ctx: &Context, instance: &Value, schema: &Value) -> bool {
    let mut errors = VecErrorRecorder::new();
    run_validators(
        ctx,
        instance,
        schema,
        &ScopeStack {
            x: schema,
            parent: None,
        },
        &mut errors,
    );
    !errors.has_errors()
}

fn descend(
    ctx: &Context,
    instance: &Value,
    schema: &Value,
    _instance_key: Option<&String>,
    _schema_key: Option<&String>,
    stack: &ScopeStack,
    errors: &mut ErrorRecorder,
) {
    // TODO: Remove this indirection
    // TODO: Adjust scope stack accordingly
    run_validators(
        ctx,
        instance,
        schema,
        &ScopeStack {
            x: schema,
            parent: Some(stack),
        },
        errors,
    )
}

pub fn patternProperties(
    ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    stack: &ScopeStack,
    errors: &mut ErrorRecorder,
) {
    if let (Object(instance), Object(schema)) = (instance, schema) {
        for (pattern, subschema) in schema.iter() {
            match regex::Regex::new(pattern) {
                Ok(re) => {
                    for (k, v) in instance.iter() {
                        if re.is_match(k) {
                            descend(ctx, v, subschema, Some(k), Some(pattern), stack, errors);
                        }
                    }
                }
                Err(err) => {
                    errors.record_error(ValidationError::new(format!("{:?}", err).as_str()))
                }
            }
        }
    }
}

pub fn propertyNames(
    ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    stack: &ScopeStack,
    errors: &mut ErrorRecorder,
) {
    if let Object(instance) = instance {
        for property in instance.keys() {
            descend(
                ctx,
                &Value::String(property.to_string()),
                schema,
                Some(property),
                None,
                stack,
                errors,
            );
        }
    }
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
    ctx: &Context,
    instance: &Value,
    schema: &Value,
    parent_schema: &Map<String, Value>,
    stack: &ScopeStack,
    errors: &mut ErrorRecorder,
) {
    if let Object(instance) = instance {
        let extras = find_additional_properties(instance, parent_schema);
        {
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
                            errors,
                        );
                    }
                }
                Bool(bool) => {
                    if !bool && extras.peekable().peek().is_some() {
                        errors.record_error(ValidationError::new(
                            "Additional properties are not allowed",
                        ));
                    }
                }
                _ => {}
            }
        }
    }
}

pub fn items_draft4(
    ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    stack: &ScopeStack,
    errors: &mut ErrorRecorder,
) {
    if let Array(instance) = instance {
        match schema {
            Object(_) => {
                for (index, item) in instance.iter().enumerate() {
                    descend(
                        ctx,
                        item,
                        schema,
                        Some(&index.to_string()),
                        None,
                        stack,
                        errors,
                    );
                }
            }
            Array(items) => {
                for ((index, item), subschema) in instance.iter().enumerate().zip(items.iter()) {
                    descend(
                        ctx,
                        item,
                        subschema,
                        Some(&index.to_string()),
                        Some(&index.to_string()),
                        stack,
                        errors,
                    );
                }
            }
            _ => {}
        }
    }
}

pub fn items(
    ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    stack: &ScopeStack,
    errors: &mut ErrorRecorder,
) {
    if let Array(instance) = instance {
        let items = util::bool_to_object_schema(schema);

        match items {
            Object(_) => {
                for (index, item) in instance.iter().enumerate() {
                    descend(
                        ctx,
                        item,
                        items,
                        Some(&index.to_string()),
                        None,
                        stack,
                        errors,
                    );
                }
            }
            Array(items) => {
                for ((index, item), subschema) in instance.iter().enumerate().zip(items.iter()) {
                    descend(
                        ctx,
                        item,
                        subschema,
                        Some(&index.to_string()),
                        Some(&index.to_string()),
                        stack,
                        errors,
                    );
                }
            }
            _ => {}
        }
    }
}

pub fn additionalItems(
    ctx: &Context,
    instance: &Value,
    schema: &Value,
    parent_schema: &Map<String, Value>,
    stack: &ScopeStack,
    errors: &mut ErrorRecorder,
) {
    if !parent_schema.contains_key("items") {
        return;
    } else if let Object(_) = parent_schema["items"] {
        return;
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
                        ctx,
                        &item,
                        schema,
                        Some(&i.to_string()),
                        None,
                        stack,
                        errors,
                    );
                }
            }
            Bool(b) => {
                if !b && instance.len() > len_items {
                    errors.record_error(ValidationError::new("Additional items are not allowed"));
                }
            }
            _ => {}
        }
    }
}

pub fn const_(
    _ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    _stack: &ScopeStack,
    errors: &mut ErrorRecorder,
) {
    if instance != schema {
        errors.record_error(ValidationError::new("Invalid const"));
    }
}

pub fn contains(
    ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    _stack: &ScopeStack,
    errors: &mut ErrorRecorder,
) {
    if let Array(instance) = instance {
        if !instance
            .iter()
            .any(|element| is_valid(ctx, element, schema))
        {
            errors.record_error(ValidationError::new(
                "Nothing is valid under the given schema",
            ));
        }
    }
}

pub fn exclusiveMinimum(
    _ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    _stack: &ScopeStack,
    errors: &mut ErrorRecorder,
) {
    if let (Number(instance), Number(schema)) = (instance, schema) {
        if instance.as_f64() <= schema.as_f64() {
            errors.record_error(ValidationError::new("exclusiveMinimum"));
        }
    }
}

pub fn exclusiveMaximum(
    _ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    _stack: &ScopeStack,
    errors: &mut ErrorRecorder,
) {
    if let (Number(instance), Number(schema)) = (instance, schema) {
        if instance.as_f64() >= schema.as_f64() {
            errors.record_error(ValidationError::new("exclusiveMaximum"));
        }
    }
}

pub fn minimum_draft4(
    _ctx: &Context,
    instance: &Value,
    schema: &Value,
    parent_schema: &Map<String, Value>,
    _stack: &ScopeStack,
    errors: &mut ErrorRecorder,
) {
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
            errors.record_error(ValidationError::new("minimum"));
        }
    }
}

pub fn minimum(
    _ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    _stack: &ScopeStack,
    errors: &mut ErrorRecorder,
) {
    if let (Number(instance), Number(schema)) = (instance, schema) {
        if instance.as_f64() < schema.as_f64() {
            errors.record_error(ValidationError::new("minimum"));
        }
    }
}

pub fn maximum_draft4(
    _ctx: &Context,
    instance: &Value,
    schema: &Value,
    parent_schema: &Map<String, Value>,
    _stack: &ScopeStack,
    errors: &mut ErrorRecorder,
) {
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
            errors.record_error(ValidationError::new("maximum"));
        }
    }
}

pub fn maximum(
    _ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    _stack: &ScopeStack,
    errors: &mut ErrorRecorder,
) {
    if let (Number(instance), Number(schema)) = (instance, schema) {
        if instance.as_f64() > schema.as_f64() {
            errors.record_error(ValidationError::new("maximum"));
        }
    }
}

#[allow(clippy::float_cmp)]
pub fn multipleOf(
    _ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    _stack: &ScopeStack,
    errors: &mut ErrorRecorder,
) {
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
            errors.record_error(ValidationError::new("not multipleOf"));
        }
    }
}

pub fn minItems(
    _ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    _stack: &ScopeStack,
    errors: &mut ErrorRecorder,
) {
    if let (Array(instance), Number(schema)) = (instance, schema) {
        if instance.len() < schema.as_u64().unwrap() as usize {
            errors.record_error(ValidationError::new("minItems"));
        }
    }
}

pub fn maxItems(
    _ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    _stack: &ScopeStack,
    errors: &mut ErrorRecorder,
) {
    if let (Array(instance), Number(schema)) = (instance, schema) {
        if instance.len() > schema.as_u64().unwrap() as usize {
            errors.record_error(ValidationError::new("minItems"));
        }
    }
}

pub fn uniqueItems(
    _ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    _stack: &ScopeStack,
    errors: &mut ErrorRecorder,
) {
    if let (Array(instance), Bool(schema)) = (instance, schema) {
        if *schema && !unique::has_unique_elements(&mut instance.iter()) {
            errors.record_error(ValidationError::new("uniqueItems"));
        }
    }
}

pub fn pattern(
    _ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    _stack: &ScopeStack,
    errors: &mut ErrorRecorder,
) {
    if let (Value::String(instance), Value::String(schema)) = (instance, schema) {
        if let Ok(re) = regex::Regex::new(schema) {
            if !re.is_match(instance) {
                errors.record_error(ValidationError::new("pattern"));
            }
        }
    }
}

pub fn format(
    ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    _stack: &ScopeStack,
    errors: &mut ErrorRecorder,
) {
    if let (Value::String(instance), Value::String(schema)) = (instance, schema) {
        if let Some(checker) = ctx.get_format_checker(schema) {
            if !checker(ctx, instance) {
                errors.record_error(ValidationError::new("format"));
            }
        }
    }
}

pub fn minLength(
    _ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    _stack: &ScopeStack,
    errors: &mut ErrorRecorder,
) {
    if let (Value::String(instance), Number(schema)) = (instance, schema) {
        if instance.chars().count() < schema.as_u64().unwrap() as usize {
            errors.record_error(ValidationError::new("minLength"));
        }
    }
}

pub fn maxLength(
    _ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    _stack: &ScopeStack,
    errors: &mut ErrorRecorder,
) {
    if let (Value::String(instance), Number(schema)) = (instance, schema) {
        if instance.chars().count() > schema.as_u64().unwrap() as usize {
            errors.record_error(ValidationError::new("maxLength"));
        }
    }
}

pub fn dependencies(
    ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    stack: &ScopeStack,
    errors: &mut ErrorRecorder,
) {
    if let (Object(object), Object(schema)) = (instance, schema) {
        for (property, dependency) in schema.iter() {
            if !object.contains_key(property) {
                continue;
            }

            let dep = util::bool_to_object_schema(dependency);
            match dep {
                Object(_) => descend(ctx, instance, dep, None, Some(property), stack, errors),
                _ => {
                    for dep0 in util::iter_or_once(dep) {
                        if let Value::String(key) = dep0 {
                            if !object.contains_key(key) {
                                errors.record_error(ValidationError::new("dependency"));
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn enum_(
    _ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    _stack: &ScopeStack,
    errors: &mut ErrorRecorder,
) {
    if let Array(enums) = schema {
        if !enums.iter().any(|val| val == instance) {
            errors.record_error(ValidationError::new("enum"));
        }
    }
}

#[allow(clippy::float_cmp)]
fn single_type(instance: &Value, schema: &Value) -> bool {
    if let Value::String(typename) = schema {
        return match typename.as_ref() {
            "array" => {
                if let Array(_) = instance { true } else { false }
            }
            "object" => {
                if let Object(_) = instance { true } else { false }
            }
            "null" => {
                if let Value::Null = instance { true } else { false }
            }
            "number" => {
                if let Number(_) = instance { true } else { false }
            }
            "string" => {
                if let Value::String(_) = instance { true } else { false }
            }
            "integer" => {
                if let Number(number) = instance {
                    number.is_i64()
                        || number.is_u64()
                        || (number.is_f64()
                            && number.as_f64().unwrap().trunc() == number.as_f64().unwrap())
                } else {
                    false
                }
            }
            "boolean" => {
                if let Bool(_) = instance { true } else { false }
            }
            _ => true,
        }
    }
    true
}

pub fn type_(
    _ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    _stack: &ScopeStack,
    errors: &mut ErrorRecorder,
) {
    if !util::iter_or_once(schema).any(|x| single_type(instance, x)) {
        errors.record_error(ValidationError::new("type"));
    }
}

pub fn properties(
    ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    stack: &ScopeStack,
    errors: &mut ErrorRecorder,
) {
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
                    errors,
                );
            }
        }
    }
}

pub fn required(
    _ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    _stack: &ScopeStack,
    errors: &mut ErrorRecorder,
) {
    if let (Object(instance), Array(schema)) = (instance, schema) {
        for property in schema.iter() {
            if let Value::String(key) = property {
                if !instance.contains_key(key) {
                    errors.record_error(ValidationError::new(&format!(
                        "required property '{}' missing",
                        key
                    )));
                }
            }
        }
    }
}

pub fn minProperties(
    _ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    _stack: &ScopeStack,
    errors: &mut ErrorRecorder,
) {
    if let (Object(instance), Number(schema)) = (instance, schema) {
        if instance.len() < schema.as_u64().unwrap() as usize {
            errors.record_error(ValidationError::new("minProperties"));
        }
    }
}

pub fn maxProperties(
    _ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    _stack: &ScopeStack,
    errors: &mut ErrorRecorder,
) {
    if let (Object(instance), Number(schema)) = (instance, schema) {
        if instance.len() > schema.as_u64().unwrap() as usize {
            errors.record_error(ValidationError::new("maxProperties"));
        }
    }
}

pub fn allOf(
    ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    stack: &ScopeStack,
    errors: &mut ErrorRecorder,
) {
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
                errors,
            );
        }
    }
}

pub fn anyOf(
    ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    stack: &ScopeStack,
    errors: &mut ErrorRecorder,
) {
    if let Array(schema) = schema {
        for (index, subschema) in schema.iter().enumerate() {
            let mut local_errors = VecErrorRecorder::new();

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
                &mut local_errors,
            );
            if !local_errors.has_errors() {
                return;
            }
        }
        // TODO: Wrap local_errors into a single error
        errors.record_error(ValidationError::new("anyOf"));
    }
}

pub fn oneOf(
    ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    stack: &ScopeStack,
    errors: &mut ErrorRecorder,
) {
    if let Array(schema) = schema {
        let mut oneOf = schema.iter();
        let mut found_one = false;
        // TODO: Gather errors
        for (index, subschema) in oneOf.by_ref().enumerate() {
            let subschema0 = if ctx.get_draft_number() >= 6 {
                util::bool_to_object_schema(subschema)
            } else {
                subschema
            };
            let mut local_errors = VecErrorRecorder::new();
            descend(
                ctx,
                instance,
                subschema0,
                None,
                Some(&index.to_string()),
                stack,
                &mut local_errors,
            );
            if !local_errors.has_errors() {
                found_one = true;
                break;
            }
        }

        if !found_one {
            errors.record_error(ValidationError::new("Nothing matched in oneOf"));
            return;
        }

        let mut found_more = false;
        for (index, subschema) in oneOf.by_ref().enumerate() {
            let subschema0 = util::bool_to_object_schema(subschema);
            let mut local_errors = VecErrorRecorder::new();
            descend(
                ctx,
                instance,
                subschema0,
                None,
                Some(&index.to_string()),
                stack,
                &mut local_errors,
            );
            if !local_errors.has_errors() {
                found_more = true;
                break;
            }
        }

        if found_more {
            errors.record_error(ValidationError::new("More than one matched in oneOf"));
        }
    }
}

pub fn not(
    ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    stack: &ScopeStack,
    errors: &mut ErrorRecorder,
) {
    let mut local_errors = VecErrorRecorder::new();
    run_validators(ctx, instance, schema, stack, &mut local_errors);
    if !local_errors.has_errors() {
        errors.record_error(ValidationError::new("not"));
    }
}

pub fn ref_(
    ctx: &Context,
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
    stack: &ScopeStack,
    errors: &mut ErrorRecorder,
) {
    if let Value::String(sref) = schema {
        match ctx
            .get_resolver()
            .resolve_fragment(sref, stack, ctx.get_schema())
        {
            Ok((scope, resolved)) => {
                let scope_schema = json!({"$id": scope.to_string()});
                let new_stack = ScopeStack {
                    x: &scope_schema,
                    parent: Some(stack),
                };
                descend(ctx, instance, resolved, None, None, &new_stack, errors);
            }
            Err(err) => errors.record_error(err),
        }
    }
}
