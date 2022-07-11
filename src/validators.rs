#![allow(non_snake_case)]
#![allow(clippy::too_many_arguments)]

use serde_json::{json, Map, Value, Value::Array, Value::Bool, Value::Object};

use crate::config::Config;
use crate::context::Context;
use crate::error::{make_error, no_error, ErrorIterator, ValidationError};
use crate::unique;
use crate::util;

/// The type of the individual validation functions.
///
/// # Arguments
///
/// * `cfg`: Settings for the current validation run that don't change
///   during the run.
/// * `instance`: The part of the JSON document being validated.
/// * `schema`: The part of the JSON schema that the JSON document is being
///   validated against.
/// * `parent_schema`: The parent node of the `schema`.  Used to look up
///   sibling attributes, such as `if`/`then`/`else`.
/// * `ref_context`: The context in which to look up `$ref` elements. This is a
///   stack that is pushed/popped when entering `$ref` contexts.  It is always
///   the top element in which JSON path references are resolved.
/// * `errors`: An object to report errors. Depending on the concrete
///   implementation, this might store all errors in a `Vec`, or it might print
///   them to a stream immediately.
///
/// # Returns
///
/// * `Iterator` over `ValidationError`
pub type Validator<'a> = fn(
    cfg: &'a Config<'a>,
    instance: &'a Value,
    schema: &'a Value,
    parent_schema: Option<&'a Value>,
    ref_context: Context<'a>,
) -> ErrorIterator<'a>;

/// The top-level validation function that performs all of the concrete
/// validation functions at a given instance/schema pair.

pub fn descend<'a>(
    cfg: &'a Config<'a>,
    instance: &'a Value,
    schema: &'a Value,
    _parent_schema: Option<&'a Value>,
    ref_context: Context<'a>,
) -> ErrorIterator<'a> {
    match schema {
        Bool(b) => {
            if *b {
                no_error()
            } else {
                make_error("false schema always fails", Some(instance), Some(schema))
            }
        }
        Object(schema_object) => {
            if let (Some(ref_), Some(validator)) =
                (schema_object.get("$ref"), cfg.get_validator("$ref"))
            {
                Box::new(validator(cfg, instance, ref_, Some(schema), ref_context))
            } else {
                Box::new(
                    schema_object
                        .iter()
                        .flat_map(move |(k, v)| -> ErrorIterator<'a> {
                            if let Some(validator) = cfg.get_validator(k) {
                                Box::new(
                                    validator(cfg, instance, v, Some(schema), ref_context)
                                        .map(move |err| err.schema_ctx(k.to_string())),
                                )
                            } else {
                                no_error()
                            }
                        }),
                )
            }
        }
        _ => make_error(
            "Invalid schema. Must be boolean or object.",
            None,
            Some(schema),
        ),
    }
}

// The validation functions below all correspond to individual schema checks
// defined in the JSON schema specification.

pub fn patternProperties<'a>(
    cfg: &'a Config<'a>,
    instance: &'a Value,
    schema: &'a Value,
    _parent_schema: Option<&'a Value>,
    ref_context: Context<'a>,
) -> ErrorIterator<'a> {
    if let (Object(instance_object), Object(schema_object)) = (instance, schema) {
        Box::new(schema_object.iter().flat_map(move |(pattern, subschema)| {
            if let Ok(re) = regex::Regex::new(pattern) {
                Box::new(
                    instance_object
                        .iter()
                        .flat_map(move |(k, v)| {
                            if re.is_match(k) {
                                Box::new(
                                    descend(cfg, v, subschema, Some(schema), ref_context)
                                        .map(move |err| err.instance_ctx(k.clone())),
                                )
                            } else {
                                no_error()
                            }
                        })
                        .map(move |err| err.schema_ctx(pattern.clone())),
                )
            } else {
                no_error()
            }
        }))
    } else {
        no_error()
    }
}

pub fn propertyNames<'a>(
    cfg: &'a Config<'a>,
    instance: &'a Value,
    schema: &'a Value,
    parent_schema: Option<&'a Value>,
    ref_context: Context<'a>,
) -> ErrorIterator<'a> {
    struct PropertyNameIter<'a> {
        instance_cursor: Box<dyn Iterator<Item = &'a String> + 'a>,
        cfg: &'a Config<'a>,
        schema: &'a Value,
        parent_schema: Option<&'a Value>,
        ref_context: Context<'a>,
        collected_errors: Vec<ValidationError>,
        error_i: usize,
    }

    impl<'a> Iterator for PropertyNameIter<'a> {
        type Item = ValidationError;

        fn next(&mut self) -> Option<Self::Item> {
            loop {
                if self.error_i < self.collected_errors.len() {
                    self.error_i += 1;
                    return Some(self.collected_errors[self.error_i - 1].clone());
                } else if let Some(instance) = self.instance_cursor.next() {
                    let key = Value::String(instance.to_string());
                    self.collected_errors = descend(
                        self.cfg,
                        &key,
                        self.schema,
                        self.parent_schema,
                        self.ref_context,
                    )
                    .collect();
                    self.error_i = 0;
                } else {
                    return None;
                }
            }
        }
    }

    if let Object(instance) = instance {
        Box::new(PropertyNameIter {
            instance_cursor: Box::new(instance.keys()),
            cfg,
            schema,
            parent_schema,
            ref_context,
            collected_errors: Vec::new(),
            error_i: 0,
        })
    } else {
        no_error()
    }
}

fn find_additional_properties<'a>(
    instance: &'a Map<String, Value>,
    schema: &'a Map<String, Value>,
) -> Box<dyn Iterator<Item = &'a str> + 'a> {
    let properties = schema.get("properties").and_then(Value::as_object);
    let pattern_regexes = schema
        .get("patternProperties")
        .and_then(Value::as_object)
        .map(|x| {
            x.keys()
                .filter_map(|k| regex::Regex::new(k).ok())
                .collect::<Vec<regex::Regex>>()
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
            })
            .map(|x| x.as_str()),
    )
}

pub fn additionalProperties<'a>(
    cfg: &'a Config<'a>,
    instance: &'a Value,
    schema: &'a Value,
    parent_schema: Option<&'a Value>,
    ref_context: Context<'a>,
) -> ErrorIterator<'a> {
    if let Object(instance_map) = instance {
        let extras = parent_schema
            .and_then(|x| x.as_object())
            .map(|x| find_additional_properties(instance_map, x));

        if let Some(mut extras) = extras {
            match schema {
                Object(_) => {
                    return Box::new(extras.flat_map(move |extra| {
                        Box::new(
                            descend(
                                cfg,
                                instance.get(extra).unwrap(),
                                schema,
                                parent_schema,
                                ref_context,
                            )
                            .map(move |err| err.instance_ctx(extra.to_string())),
                        )
                    }));
                }
                Bool(bool) => {
                    if !bool {
                        let extra_string = util::format_list(&mut extras);
                        if !extra_string.is_empty() {
                            return make_error(
                                format!(
                                    "Additional properties are not allowed. Found {}.",
                                    extra_string
                                ),
                                Some(instance),
                                parent_schema,
                            );
                        }
                    }
                }
                _ => {}
            }
        }
    }
    no_error()
}

pub fn items<'a>(
    cfg: &'a Config<'a>,
    instance: &'a Value,
    schema: &'a Value,
    _parent_schema: Option<&'a Value>,
    ref_context: Context<'a>,
) -> ErrorIterator<'a> {
    if let Array(instance) = instance {
        let items = if cfg.get_draft_number() >= 6 {
            util::bool_to_object_schema(schema)
        } else {
            schema
        };

        match items {
            Object(_) => Box::new(instance.iter().enumerate().flat_map(move |(index, item)| {
                Box::new(
                    descend(cfg, item, items, Some(schema), ref_context)
                        .map(move |err| err.instance_ctx(index.to_string())),
                )
            })),
            Array(items) => Box::new(instance.iter().enumerate().zip(items.iter()).flat_map(
                move |((index, item), subschema)| {
                    Box::new(
                        descend(cfg, item, subschema, Some(schema), ref_context)
                            .map(move |err| err.add_ctx(index.to_string(), index.to_string())),
                    )
                },
            )),
            _ => no_error(),
        }
    } else {
        no_error()
    }
}

pub fn additionalItems<'a>(
    cfg: &'a Config<'a>,
    instance: &'a Value,
    schema: &'a Value,
    parent_schema: Option<&'a Value>,
    ref_context: Context<'a>,
) -> ErrorIterator<'a> {
    if let Some(parent_schema) = parent_schema {
        if let (Array(instance_array), Some(Array(items))) = (instance, parent_schema.get("items"))
        {
            match schema {
                Object(_) => {
                    return Box::new(
                        instance_array
                            .iter()
                            .enumerate()
                            .skip(items.len())
                            .flat_map(move |(index, item)| {
                                Box::new(
                                    descend(cfg, item, schema, Some(parent_schema), ref_context)
                                        .map(move |err| err.instance_ctx(index.to_string())),
                                )
                            }),
                    )
                }
                Bool(b) => {
                    if !b && instance_array.len() > items.len() {
                        return make_error(
                            "Additional items are not allowed.",
                            Some(instance),
                            Some(parent_schema),
                        );
                    }
                }
                _ => {}
            }
        }
    }
    no_error()
}

pub fn const_<'a>(
    _cfg: &'a Config<'a>,
    instance: &'a Value,
    schema: &'a Value,
    _parent_schema: Option<&'a Value>,
    _ref_context: Context<'a>,
) -> ErrorIterator<'a> {
    if !util::json_equal(instance, schema) {
        make_error("const doesn't match.", Some(instance), Some(schema))
    } else {
        no_error()
    }
}

pub fn contains<'a>(
    cfg: &'a Config<'a>,
    instance: &'a Value,
    schema: &'a Value,
    parent_schema: Option<&'a Value>,
    ref_context: Context<'a>,
) -> ErrorIterator<'a> {
    if let Array(instance_array) = instance {
        for item in instance_array {
            if descend(cfg, item, schema, parent_schema, ref_context)
                .next()
                .is_none()
            {
                return no_error();
            }
        }
        return make_error(
            "No items in array valid under the given schema.",
            Some(instance),
            Some(schema),
        );
    }
    no_error()
}

pub fn exclusiveMinimum<'a>(
    _cfg: &'a Config<'a>,
    instance: &'a Value,
    schema: &'a Value,
    _parent_schema: Option<&'a Value>,
    _ref_context: Context<'a>,
) -> ErrorIterator<'a> {
    if let (Value::Number(instance_number), Value::Number(schema_number)) = (instance, schema) {
        if instance_number.as_f64() <= schema_number.as_f64() {
            return make_error(
                format!("{} <= exclusiveMinimum {}", instance_number, schema_number),
                Some(instance),
                Some(schema),
            );
        }
    }
    no_error()
}

pub fn exclusiveMaximum<'a>(
    _cfg: &'a Config<'a>,
    instance: &'a Value,
    schema: &'a Value,
    _parent_schema: Option<&'a Value>,
    _ref_context: Context<'a>,
) -> ErrorIterator<'a> {
    if let (Value::Number(instance_number), Value::Number(schema_number)) = (instance, schema) {
        if instance_number.as_f64() >= schema_number.as_f64() {
            return make_error(
                format!("{} >= exclusiveMaximum {}", instance_number, schema_number),
                Some(instance),
                Some(schema),
            );
        }
    }
    no_error()
}

pub fn minimum_draft4<'a>(
    _cfg: &'a Config<'a>,
    instance: &'a Value,
    schema: &'a Value,
    parent_schema: Option<&'a Value>,
    _ref_context: Context<'a>,
) -> ErrorIterator<'a> {
    if let (Value::Number(instance_number), Value::Number(minimum)) = (instance, schema) {
        if parent_schema
            .and_then(|x| x.get("exclusiveMinimum"))
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            if instance_number.as_f64() <= minimum.as_f64() {
                return make_error(
                    format!("{} <= exclusiveMinimum {}", instance_number, minimum),
                    Some(instance),
                    Some(schema),
                );
            }
        } else if instance_number.as_f64() < minimum.as_f64() {
            return make_error(
                format!("{} <= minimum {}", instance_number, minimum),
                Some(instance),
                Some(schema),
            );
        }
    }
    no_error()
}

pub fn minimum<'a>(
    _cfg: &'a Config<'a>,
    instance: &'a Value,
    schema: &'a Value,
    _parent_schema: Option<&'a Value>,
    _ref_context: Context<'a>,
) -> ErrorIterator<'a> {
    if let (Value::Number(instance_number), Value::Number(schema_number)) = (instance, schema) {
        if instance.as_f64() < schema_number.as_f64() {
            return make_error(
                format!("{} < minimum {}", instance_number, schema_number),
                Some(instance),
                Some(schema),
            );
        }
    }
    no_error()
}

pub fn maximum_draft4<'a>(
    _cfg: &'a Config<'a>,
    instance: &'a Value,
    schema: &'a Value,
    parent_schema: Option<&'a Value>,
    _ref_context: Context<'a>,
) -> ErrorIterator<'a> {
    if let (Value::Number(instance_number), Value::Number(maximum)) = (instance, schema) {
        if parent_schema
            .and_then(|x| x.get("exclusiveMaximum"))
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            if instance_number.as_f64() >= maximum.as_f64() {
                return make_error(
                    format!("{} >= exclusiveMaximum {}", instance_number, maximum),
                    Some(instance),
                    Some(schema),
                );
            }
        } else if instance_number.as_f64() > maximum.as_f64() {
            return make_error(
                format!("{} > maximum {}", instance_number, maximum),
                Some(instance),
                Some(schema),
            );
        }
    }
    no_error()
}

pub fn maximum<'a>(
    _cfg: &'a Config<'a>,
    instance: &'a Value,
    schema: &'a Value,
    _parent_schema: Option<&'a Value>,
    _ref_context: Context<'a>,
) -> ErrorIterator<'a> {
    if let (Value::Number(instance_number), Value::Number(maximum)) = (instance, schema) {
        if instance_number.as_f64() > maximum.as_f64() {
            return make_error(
                format!("{} > maximum {}", instance_number, maximum),
                Some(instance),
                Some(schema),
            );
        }
    }
    no_error()
}

#[allow(clippy::float_cmp)]
pub fn multipleOf<'a>(
    _cfg: &'a Config<'a>,
    instance: &'a Value,
    schema: &'a Value,
    _parent_schema: Option<&'a Value>,
    _ref_context: Context<'a>,
) -> ErrorIterator<'a> {
    if let (Value::Number(instance_number), Value::Number(schema_number)) = (instance, schema) {
        let failed = if schema_number.is_f64() {
            let quotient = instance_number.as_f64().unwrap() / schema_number.as_f64().unwrap();
            quotient.trunc() != quotient
        } else if schema_number.is_u64() {
            (instance_number.as_u64().unwrap() % schema_number.as_u64().unwrap()) != 0
        } else {
            (instance_number.as_i64().unwrap() % schema_number.as_i64().unwrap()) != 0
        };
        if failed {
            return make_error(
                format!("{} not multipleOf {}", instance_number, schema_number),
                Some(instance),
                Some(schema),
            );
        }
    }
    no_error()
}

pub fn minItems<'a>(
    _cfg: &'a Config<'a>,
    instance: &'a Value,
    schema: &'a Value,
    _parent_schema: Option<&'a Value>,
    _ref_context: Context<'a>,
) -> ErrorIterator<'a> {
    if let (Array(instance_array), Value::Number(schema_number)) = (instance, schema) {
        if instance_array.len() < schema_number.as_u64().unwrap() as usize {
            return make_error(
                format!("{} < minItems {}", instance_array.len(), schema_number),
                Some(instance),
                Some(schema),
            );
        }
    }
    no_error()
}

pub fn maxItems<'a>(
    _cfg: &'a Config<'a>,
    instance: &'a Value,
    schema: &'a Value,
    _parent_schema: Option<&'a Value>,
    _ref_context: Context<'a>,
) -> ErrorIterator<'a> {
    if let (Array(instance_array), Value::Number(schema_number)) = (instance, schema) {
        if instance_array.len() > schema_number.as_u64().unwrap() as usize {
            return make_error(
                format!("{} > maxItems {}", instance_array.len(), schema_number),
                Some(instance),
                Some(schema),
            );
        }
    }
    no_error()
}

pub fn uniqueItems<'a>(
    _cfg: &'a Config<'a>,
    instance: &'a Value,
    schema: &'a Value,
    _parent_schema: Option<&'a Value>,
    _ref_context: Context<'a>,
) -> ErrorIterator<'a> {
    if let (Array(instance_array), Bool(schema)) = (instance, schema) {
        if *schema && !unique::has_unique_elements(&mut instance_array.iter()) {
            return make_error("Items are not unique", Some(instance), None);
        }
    }
    no_error()
}

pub fn pattern<'a>(
    _cfg: &'a Config<'a>,
    instance: &'a Value,
    schema: &'a Value,
    _parent_schema: Option<&'a Value>,
    _ref_context: Context<'a>,
) -> ErrorIterator<'a> {
    if let (Value::String(instance_string), Value::String(schema_string)) = (instance, schema) {
        if let Ok(re) = regex::Regex::new(schema_string) {
            if !re.is_match(instance_string) {
                return make_error("Does not match pattern.", Some(instance), Some(schema));
            }
        } else {
            return make_error("Invalid regex.", None, Some(schema));
        }
    }
    no_error()
}

pub fn format<'a>(
    cfg: &'a Config<'a>,
    instance: &'a Value,
    schema: &'a Value,
    _parent_schema: Option<&'a Value>,
    _ref_context: Context<'a>,
) -> ErrorIterator<'a> {
    if let (Value::String(instance_string), Value::String(schema_string)) = (instance, schema) {
        if let Some(checker) = cfg.get_format_checker(schema_string) {
            if !checker(cfg, instance_string) {
                return make_error("Invalid for format.", Some(instance), Some(schema));
            }
        }
    }
    no_error()
}

pub fn minLength<'a>(
    _cfg: &'a Config<'a>,
    instance: &'a Value,
    schema: &'a Value,
    _parent_schema: Option<&'a Value>,
    _ref_context: Context<'a>,
) -> ErrorIterator<'a> {
    if let (Value::String(instance_string), Value::Number(schema_number)) = (instance, schema) {
        let count = instance_string.chars().count();
        if count < schema_number.as_u64().unwrap() as usize {
            return make_error(
                format!("{} < minLength {}", count, schema_number),
                Some(instance),
                Some(schema),
            );
        }
    }
    no_error()
}

pub fn maxLength<'a>(
    _cfg: &'a Config<'a>,
    instance: &'a Value,
    schema: &'a Value,
    _parent_schema: Option<&'a Value>,
    _ref_context: Context<'a>,
) -> ErrorIterator<'a> {
    if let (Value::String(instance_string), Value::Number(schema_number)) = (instance, schema) {
        let count = instance_string.chars().count();
        if count > schema_number.as_u64().unwrap() as usize {
            return make_error(
                format!("{} < maxLength {}", count, schema_number),
                Some(instance),
                Some(schema),
            );
        }
    }
    no_error()
}

pub fn dependencies<'a>(
    cfg: &'a Config<'a>,
    instance: &'a Value,
    schema: &'a Value,
    _parent_schema: Option<&'a Value>,
    ref_context: Context<'a>,
) -> ErrorIterator<'a> {
    if let (Object(instance_object), Object(schema_object)) = (instance, schema) {
        Box::new(
            schema_object
                .iter()
                .filter(move |(property, _dependency)| {
                    instance_object.contains_key(property.as_str())
                })
                .flat_map(move |(property, dependency)| -> ErrorIterator<'a> {
                    let dep = util::bool_to_object_schema(dependency);
                    if let Object(_) = dep {
                        return Box::new(
                            descend(cfg, instance, dep, Some(schema), ref_context)
                                .map(move |err| err.schema_ctx(property.clone())),
                        );
                    } else {
                        for dep0 in util::iter_or_once(dep) {
                            if let Value::String(key) = dep0 {
                                if !instance_object.contains_key(key) {
                                    return make_error(
                                        "Invalid dependencies",
                                        Some(instance),
                                        Some(schema),
                                    );
                                }
                            }
                        }
                    }
                    no_error()
                }),
        )
    } else {
        no_error()
    }
}

pub fn enum_<'a>(
    _cfg: &'a Config<'a>,
    instance: &'a Value,
    schema: &'a Value,
    _parent_schema: Option<&'a Value>,
    _ref_context: Context<'a>,
) -> ErrorIterator<'a> {
    if let Array(enums) = schema {
        if !enums.iter().any(|val| util::json_equal(val, instance)) {
            return make_error("Value is not in enum.", Some(instance), Some(schema));
        }
    }
    no_error()
}

#[allow(clippy::float_cmp)]
fn single_type(instance: &Value, schema: &Value) -> bool {
    if let Value::String(typename) = schema {
        return match typename.as_ref() {
            "array" => matches!(instance, Array(_)),
            "object" => matches!(instance, Object(_)),
            "null" => matches!(instance, Value::Null),
            "number" => matches!(instance, Value::Number(_)),
            "string" => matches!(instance, Value::String(_)),
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
            "boolean" => matches!(instance, Bool(_)),
            _ => true,
        };
    }
    true
}

pub fn type_<'a>(
    _cfg: &'a Config<'a>,
    instance: &'a Value,
    schema: &'a Value,
    parent_schema: Option<&'a Value>,
    _ref_context: Context<'a>,
) -> ErrorIterator<'a> {
    if !util::iter_or_once(schema).any(|x| single_type(instance, x)) {
        return make_error("Invalid type.", Some(instance), parent_schema);
    }
    no_error()
}

pub fn properties<'a>(
    cfg: &'a Config<'a>,
    instance: &'a Value,
    schema: &'a Value,
    _parent_schema: Option<&'a Value>,
    ref_context: Context<'a>,
) -> ErrorIterator<'a> {
    if let (Object(instance_object), Object(schema_object)) = (instance, schema) {
        Box::new(schema_object.iter().flat_map(move |(property, subschema)| {
            if let Some(property_value) = instance_object.get(property) {
                Box::new(
                    descend(cfg, property_value, subschema, Some(schema), ref_context)
                        .map(move |err| err.add_ctx(property.clone(), property.clone())),
                )
            } else {
                no_error()
            }
        }))
    } else {
        no_error()
    }
}

pub fn required<'a>(
    _cfg: &'a Config<'a>,
    instance: &'a Value,
    schema: &'a Value,
    _parent_schema: Option<&'a Value>,
    _ref_context: Context<'a>,
) -> ErrorIterator<'a> {
    if let (Object(instance_object), Array(schema_array)) = (instance, schema) {
        let missing_properties: Vec<&str> = schema_array
            .iter()
            .filter_map(Value::as_str)
            .filter(|&x| !instance_object.contains_key(&x.to_string()))
            .collect();

        if !missing_properties.is_empty() {
            return make_error(
                format!(
                    "Required properties {} are missing",
                    util::format_list(&mut missing_properties.iter().copied())
                ),
                Some(instance),
                Some(schema),
            );
        }
    }
    no_error()
}

pub fn minProperties<'a>(
    _cfg: &'a Config<'a>,
    instance: &'a Value,
    schema: &'a Value,
    _parent_schema: Option<&'a Value>,
    _ref_context: Context<'a>,
) -> ErrorIterator<'a> {
    if let (Object(instance_object), Value::Number(schema_number)) = (instance, schema) {
        if instance_object.len() < schema_number.as_u64().unwrap() as usize {
            return make_error(
                format!(
                    "{} < minProperties {}",
                    instance_object.len(),
                    schema_number
                ),
                Some(instance),
                Some(schema),
            );
        }
    }
    no_error()
}

pub fn maxProperties<'a>(
    _cfg: &'a Config<'a>,
    instance: &'a Value,
    schema: &'a Value,
    _parent_schema: Option<&'a Value>,
    _ref_context: Context<'a>,
) -> ErrorIterator<'a> {
    if let (Object(instance_object), Value::Number(schema_number)) = (instance, schema) {
        if instance_object.len() > schema_number.as_u64().unwrap() as usize {
            return make_error(
                format!(
                    "{} > maxProperties {}",
                    instance_object.len(),
                    schema_number
                ),
                Some(instance),
                Some(schema),
            );
        }
    }
    no_error()
}

pub fn allOf<'a>(
    cfg: &'a Config<'a>,
    instance: &'a Value,
    schema: &'a Value,
    _parent_schema: Option<&'a Value>,
    ref_context: Context<'a>,
) -> ErrorIterator<'a> {
    if let Array(schema_array) = schema {
        Box::new(
            schema_array
                .iter()
                .enumerate()
                .flat_map(move |(index, subschema)| {
                    let subschema0 = if cfg.get_draft_number() >= 6 {
                        util::bool_to_object_schema(subschema)
                    } else {
                        subschema
                    };
                    Box::new(
                        descend(cfg, instance, subschema0, Some(schema), ref_context)
                            .map(move |err| err.schema_ctx(index.to_string())),
                    )
                }),
        )
    } else {
        no_error()
    }
}

pub fn anyOf<'a>(
    cfg: &'a Config<'a>,
    instance: &'a Value,
    schema: &'a Value,
    _parent_schema: Option<&'a Value>,
    ref_context: Context<'a>,
) -> ErrorIterator<'a> {
    if let Array(schema_array) = schema {
        for subschema in schema_array.iter() {
            let subschema0 = if cfg.get_draft_number() >= 6 {
                util::bool_to_object_schema(subschema)
            } else {
                subschema
            };
            if descend(cfg, instance, subschema0, Some(schema), ref_context)
                .next()
                .is_none()
            {
                return no_error();
            }
        }
        return make_error("anyOf failed", Some(instance), Some(schema));
    }
    no_error()
}

pub fn oneOf<'a>(
    cfg: &'a Config<'a>,
    instance: &'a Value,
    schema: &'a Value,
    _parent_schema: Option<&'a Value>,
    ref_context: Context<'a>,
) -> ErrorIterator<'a> {
    if let Array(schema_array) = schema {
        let mut oneOf = schema_array.iter().enumerate();
        let mut found_one = false;
        for (_, subschema) in oneOf.by_ref() {
            let subschema0 = if cfg.get_draft_number() >= 6 {
                util::bool_to_object_schema(subschema)
            } else {
                subschema
            };
            if descend(cfg, instance, subschema0, Some(schema), ref_context)
                .next()
                .is_none()
            {
                found_one = true;
                break;
            }
        }

        if !found_one {
            return make_error("nothing matched in oneOf", Some(instance), Some(schema));
        }

        let mut found_more = false;
        for (_, subschema) in oneOf.by_ref() {
            let subschema0 = if cfg.get_draft_number() >= 6 {
                util::bool_to_object_schema(subschema)
            } else {
                subschema
            };
            if descend(cfg, instance, subschema0, Some(schema), ref_context)
                .next()
                .is_none()
            {
                found_more = true;
                break;
            }
        }

        if found_more {
            return make_error(
                "More than one matched in oneOf",
                Some(instance),
                Some(schema),
            );
        }
    }
    no_error()
}

pub fn not<'a>(
    cfg: &'a Config<'a>,
    instance: &'a Value,
    schema: &'a Value,
    parent_schema: Option<&'a Value>,
    ref_context: Context<'a>,
) -> ErrorIterator<'a> {
    if descend(cfg, instance, schema, parent_schema, ref_context)
        .next()
        .is_none()
    {
        make_error("not", Some(instance), Some(schema))
    } else {
        no_error()
    }
}

pub fn ref_<'a>(
    cfg: &'a Config<'a>,
    instance: &'a Value,
    schema: &'a Value,
    _parent_schema: Option<&'a Value>,
    ref_context: Context<'a>,
) -> ErrorIterator<'a> {
    if let Value::String(sref) = schema {
        struct RefIter {
            collected_errors: Vec<ValidationError>,
            error_i: usize,
        }

        impl Iterator for RefIter {
            type Item = ValidationError;

            fn next(&mut self) -> Option<Self::Item> {
                if self.error_i < self.collected_errors.len() {
                    self.error_i += 1;
                    Some(self.collected_errors[self.error_i - 1].clone())
                } else {
                    None
                }
            }
        }

        match cfg
            .get_resolver()
            .resolve_fragment(cfg.draft, sref, &ref_context, cfg.get_schema())
        {
            Ok((scope, resolved)) => {
                let scope_schema = json!({"$id": scope.to_string()});
                return Box::new(RefIter {
                    collected_errors: descend(
                        cfg,
                        instance,
                        resolved,
                        Some(schema),
                        ref_context.push(&scope_schema),
                    )
                    .collect(),
                    error_i: 0,
                });
            }
            Err(_err) => {
                return make_error(
                    format!("Couldn't resolve reference {}", sref),
                    Some(instance),
                    None,
                )
            }
        }
    }
    no_error()
}

pub fn if_<'a>(
    cfg: &'a Config<'a>,
    instance: &'a Value,
    schema: &'a Value,
    parent_schema: Option<&'a Value>,
    ref_context: Context<'a>,
) -> ErrorIterator<'a> {
    if descend(cfg, instance, schema, parent_schema, ref_context)
        .next()
        .is_none()
    {
        if let Some(then) = parent_schema.and_then(|x| x.get("then")) {
            if then.is_object() {
                return Box::new(
                    descend(cfg, instance, then, Some(schema), ref_context)
                        .map(move |err| err.schema_ctx("then".to_string())),
                );
            }
        }
    } else if let Some(else_) = parent_schema.and_then(|x| x.get("else")) {
        if else_.is_object() {
            return Box::new(
                descend(cfg, instance, else_, Some(schema), ref_context)
                    .map(move |err| err.schema_ctx("else".to_string())),
            );
        }
    }
    no_error()
}

#[cfg(test)]
mod tests {
    use crate::{schemas, Config};
    use serde_json::json;

    #[test]
    fn test_additional_properties_errors() {
        let schema = json!({
            "properties": { "foo": { "type": "integer" } },
            "additionalProperties": false
        });
        let instance = json!({
            "foo": 42,
            "bar": "additional",
            "baz": "another additional"
        });
        let cfg = Config::from_schema(&schema, Some(schemas::Draft::Draft6)).unwrap();
        let validation = cfg.validate(&instance);

        if let Err(errors) = validation {
            for error in errors {
                let formatted = format!("{}", error);
                println!("{}", formatted);

                assert!(error.instance_path == (Vec::<String>::new()));
                assert!(error.schema_path == vec!("additionalProperties"));

                assert!(formatted
                    .contains("Additional properties are not allowed. Found \"bar\", \"baz\"."));
                assert!(formatted.contains("At instance path /:"));
                assert!(formatted.contains("At schema path /additionalProperties"));
            }
        }
    }
}
