/// Utilities to track the location within a JSON document
use serde_json::Value;

pub struct Context<'a> {
    pub x: &'a Value,
    pub parent: Option<&'a Context<'a>>,
}

impl<'a> Context<'a> {
    pub fn new() -> Context<'static> {
        Context {
            x: &Value::Null,
            parent: None,
        }
    }

    pub fn new_from(x: &'a Value) -> Context<'a> {
        Context { x, parent: None }
    }

    pub fn push(&'a self, x: &'a Value) -> Context<'a> {
        Context {
            x,
            parent: Some(self),
        }
    }

    pub fn replace(&'a self, x: &'a Value) -> Context<'a> {
        Context {
            x,
            parent: self.parent,
        }
    }

    pub fn flatten(&'a self) -> Vec<Value> {
        let mut result = Vec::new();
        let mut ptr = self;
        if !ptr.x.is_null() {
            result.push(ptr.x.clone())
        }
        while ptr.parent.is_some() {
            ptr = ptr.parent.unwrap();
            if !ptr.x.is_null() {
                result.push(ptr.x.clone())
            }
        }
        result.reverse();
        result
    }
}

impl<'a> Default for Context<'a> {
    fn default() -> Self {
        Self::new()
    }
}
