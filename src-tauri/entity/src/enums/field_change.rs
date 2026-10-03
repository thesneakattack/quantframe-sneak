use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
#[derive(Default)]
pub enum FieldChange<T> {
    #[default]
    Ignore, // don’t touch this field
    Value(T), // set to a new value
    Null,     // explicitly clear (if nullable)
}

impl<T> FieldChange<T> {
    pub fn get_value(&self, default: T) -> T
    where
        T: Clone,
    {
        match self {
            FieldChange::Value(v) => v.clone(),
            _ => default,
        }
    }
}
