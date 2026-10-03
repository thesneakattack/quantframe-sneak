use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
#[derive(Default)]
pub enum FieldChange<T> {
    #[default]
    Ignore,   // don’t touch this field
    Value(T), // set to a new value
    Null,     // explicitly clear (if nullable)
}
