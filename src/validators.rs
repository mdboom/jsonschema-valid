#![allow(non_snake_case)]

use regex;

use serde_json::{Map, Value};

use error::ValidationError;
use unique;
use util;

pub type ValidatorResult = Result<(), ValidationError>;

type Validator =
    fn(instance: &Value, schema: &Value, parent_schema: &Map<String, Value>) -> ValidatorResult;

fn get_validator(key: &str) -> Option<Validator> {
    match key {
        "patternProperties" => Some(validate_patternProperties as Validator),
        "pattern" => Some(validate_pattern as Validator),
        "propertyNames" => Some(validate_propertyNames as Validator),
        "additionalProperties" => Some(validate_additionalProperties as Validator),
        "items" => Some(validate_items as Validator),
        "additionalItems" => Some(validate_additionalItems as Validator),
        "const" => Some(validate_const as Validator),
        "contains" => Some(validate_contains as Validator),
        "exclusiveMinimum" => Some(validate_exclusiveMinimum as Validator),
        "exclusiveMaximum" => Some(validate_exclusiveMaximum as Validator),
        "minimum" => Some(validate_minimum as Validator),
        "maximum" => Some(validate_maximum as Validator),
        "multipleOf" => Some(validate_multipleOf as Validator),
        "minItems" => Some(validate_minItems as Validator),
        "maxItems" => Some(validate_maxItems as Validator),
        "uniqueItems" => Some(validate_uniqueItems as Validator),
        "minLength" => Some(validate_minLength as Validator),
        "maxLength" => Some(validate_maxLength as Validator),
        "dependencies" => Some(validate_dependencies as Validator),
        "enum" => Some(validate_enum as Validator),
        "type" => Some(validate_type as Validator),
        "properties" => Some(validate_properties as Validator),
        "required" => Some(validate_required as Validator),
        "minProperties" => Some(validate_minProperties as Validator),
        "maxProperties" => Some(validate_maxProperties as Validator),
        "allOf" => Some(validate_allOf as Validator),
        "anyOf" => Some(validate_anyOf as Validator),
        "oneOf" => Some(validate_oneOf as Validator),
        "not" => Some(validate_not as Validator),
        _ => None,
    }
}

pub fn run_validators(instance: &Value, schema: &Value) -> ValidatorResult {
    match schema {
        Value::Bool(b) => {
            if *b {
                Ok(())
            } else {
                Err(ValidationError::new("False schema always fails"))
            }
        }
        Value::Object(schema_object) => {
            if let Some(_sref) = schema_object.get("$ref") {
                Ok(()) // validate_ref(instance, sref, schema);
            } else {
                for (k, v) in schema_object.iter() {
                    if let Some(validator) = get_validator(k.as_ref()) {
                        if let Err(mut err) = validator(instance, v, schema_object) {
                            err.add_schema_path(k);
                            return Err(err);
                        }
                    }
                }
                Ok(())
            }
        }
        _ => Err(ValidationError::new("Invalid schema")),
    }
}

pub fn is_valid(instance: &Value, schema: &Value) -> bool {
    run_validators(instance, schema).is_ok()
}

fn descend(
    instance: &Value,
    schema: &Value,
    instance_key: Option<&String>,
    schema_key: Option<&String>,
) -> ValidatorResult {
    if let Err(mut err) = run_validators(instance, schema) {
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

fn validate_patternProperties(
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
) -> ValidatorResult {
    if let Value::Object(instance) = instance {
        if let Value::Object(schema) = schema {
            for (pattern, subschema) in schema.iter() {
                let re = util::get_regex(pattern)?;
                for (k, v) in instance.iter() {
                    if re.is_match(k) {
                        descend(v, subschema, Some(k), Some(pattern))?;
                    }
                }
            }
        }
    }
    Ok(())
}

fn validate_propertyNames(
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
) -> ValidatorResult {
    if let Value::Object(instance) = instance {
        for property in instance.keys() {
            descend(
                &Value::String(property.to_string()),
                schema,
                Some(property),
                None,
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
    let properties = schema
        .get("properties")
        .unwrap_or_else(move || &EMPTY_OBJ);
    let pattern_properties = schema
        .get("patternProperties")
        .unwrap_or_else(move || &EMPTY_OBJ);
    if let Value::Object(properties) = properties {
        if let Value::Object(pattern_properties) = pattern_properties {
            let pattern_regexes_result: Result<Vec<regex::Regex>, ValidationError> =
                pattern_properties
                    .keys()
                    .map(|k| util::get_regex(k))
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

fn validate_additionalProperties(
    instance: &Value,
    schema: &Value,
    parent_schema: &Map<String, Value>,
) -> ValidatorResult {
    if let Value::Object(instance) = instance {
        let mut extras = find_additional_properties(instance, parent_schema)?;
        match schema {
            Value::Object(_) => {
                for extra in extras {
                    descend(
                        instance.get(extra).expect("Property gone missing."),
                        schema,
                        Some(extra),
                        None,
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

fn validate_items(
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
) -> ValidatorResult {
    if let Value::Array(instance) = instance {
        let items = util::bool_to_object_schema(schema);

        match items {
            Value::Object(_) => for (index, item) in instance.iter().enumerate() {
                descend(item, items, Some(&index.to_string()), None)?;
            },
            Value::Array(items) => {
                for ((index, item), subschema) in instance.iter().enumerate().zip(items.iter()) {
                    descend(
                        item,
                        subschema,
                        Some(&index.to_string()),
                        Some(&index.to_string()),
                    )?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn validate_additionalItems(
    instance: &Value,
    schema: &Value,
    parent_schema: &Map<String, Value>,
) -> ValidatorResult {
    if !parent_schema.contains_key("items") {
        return Ok(());
    } else if let Value::Object(_) = parent_schema["items"] {
        return Ok(());
    }

    if let Value::Array(instance) = instance {
        let len_items = parent_schema
            .get("items")
            .map_or(0, |x| match x { Value::Array(array) => array.len(), _ => 0, });
        match schema {
            Value::Object(_) => for i in len_items..instance.len() {
                descend(&instance[i], schema, Some(&i.to_string()), None)?;
            },
            Value::Bool(b) => if !b && instance.len() > len_items {
                return Err(ValidationError::new("Additional items are not allowed"));
            },
            _ => {}
        }
    }
    Ok(())
}

fn validate_const(
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
) -> ValidatorResult {
    if instance != schema {
        return Err(ValidationError::new("Invalid const"));
    }
    Ok(())
}

fn validate_contains(
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
) -> ValidatorResult {
    if let Value::Array(instance) = instance {
        if !instance.iter().any(|element| is_valid(element, schema)) {
            return Err(ValidationError::new(
                "Nothing is valid under the given schema",
            ));
        }
    }
    Ok(())
}

// TODO: minimum draft 3/4
// TODO: maximum draft 3/4

fn validate_exclusiveMinimum(
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
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

fn validate_exclusiveMaximum(
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
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

fn validate_minimum(
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
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

fn validate_maximum(
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
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

fn validate_multipleOf(
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
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

fn validate_minItems(
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
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

fn validate_maxItems(
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
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

fn validate_uniqueItems(
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
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

fn validate_pattern(
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
) -> ValidatorResult {
    if let Value::String(instance) = instance {
        if let Value::String(schema) = schema {
            if !util::get_regex(schema)?.is_match(instance) {
                return Err(ValidationError::new("pattern"));
            }
        }
    }
    Ok(())
}

// TODO format

fn validate_minLength(
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
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

fn validate_maxLength(
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
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

fn validate_dependencies(
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
) -> ValidatorResult {
    if let Value::Object(object) = instance {
        if let Value::Object(schema) = schema {
            for (property, dependency) in schema.iter() {
                if !object.contains_key(property) {
                    continue;
                }

                let dep = util::bool_to_object_schema(dependency);
                match dep {
                    Value::Object(_) => descend(instance, dep, None, Some(property))?,
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

fn validate_enum(
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
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

fn validate_single_type(instance: &Value, schema: &Value) -> ValidatorResult {
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

fn validate_type(
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
) -> ValidatorResult {
    if !util::iter_or_once(schema).any(|x| validate_single_type(instance, x).is_ok()) {
        return Err(ValidationError::new("type"));
    }
    Ok(())
}

fn validate_properties(
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
) -> ValidatorResult {
    if let Value::Object(instance) = instance {
        if let Value::Object(schema) = schema {
            for (property, subschema) in schema.iter() {
                if instance.contains_key(property) {
                    descend(
                        instance.get(property).unwrap(),
                        subschema,
                        Some(property),
                        Some(property),
                    )?;
                }
            }
        }
    }
    Ok(())
}

fn validate_required(
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
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

fn validate_minProperties(
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
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

fn validate_maxProperties(
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
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

fn validate_allOf(
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
) -> ValidatorResult {
    if let Value::Array(schema) = schema {
        for (index, subschema) in schema.iter().enumerate() {
            let subschema0 = util::bool_to_object_schema(subschema);
            descend(instance, subschema0, None, Some(&index.to_string()))?;
        }
    }
    Ok(())
}

fn validate_anyOf(
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
) -> ValidatorResult {
    if let Value::Array(schema) = schema {
        for (index, subschema) in schema.iter().enumerate() {
            let subschema0 = util::bool_to_object_schema(subschema);
            // TODO Wrap up all errors into a list
            if descend(instance, subschema0, None, Some(&index.to_string())).is_ok() {
                return Ok(());
            }
        }
        return Err(ValidationError::new("anyOf"));
    }
    Ok(())
}

fn validate_oneOf(
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
) -> ValidatorResult {
    if let Value::Array(schema) = schema {
        let mut oneOf = schema.into_iter();
        let mut found_one = false;
        for (index, subschema) in oneOf.by_ref().enumerate() {
            let subschema0 = util::bool_to_object_schema(subschema);
            if descend(instance, subschema0, None, Some(&index.to_string())).is_ok() {
                found_one = true;
                break;
            }
        }

        if !found_one {
            return Err(ValidationError::new("Nothing matched in oneOf"));
        }

        for (index, subschema) in oneOf.by_ref().enumerate() {
            let subschema0 = util::bool_to_object_schema(subschema);
            if descend(instance, subschema0, None, Some(&index.to_string())).is_ok() {
                return Err(ValidationError::new("More than one matched in oneOf"));
            }
        }
    }
    Ok(())
}

fn validate_not(
    instance: &Value,
    schema: &Value,
    _parent_schema: &Map<String, Value>,
) -> ValidatorResult {
    if run_validators(instance, schema).is_ok() {
        return Err(ValidationError::new("not"));
    }
    Ok(())
}
