use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[derive(Default)]
pub struct ItemId {
    #[serde(
        rename = "$oid",
        skip_serializing_if = "Option::is_none",
        alias = "oid"
    )]
    pub id: Option<String>,
}

