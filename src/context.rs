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
        Context { x: x, parent: None }
    }

    pub fn push(&'a self, x: &'a Value) -> Context<'a> {
        Context {
            x: x,
            parent: Some(self),
        }
    }

    // TODO: Read out in reverse
}
