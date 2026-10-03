use sea_orm::Set;
use serde::{Deserialize, Serialize};

use crate::{enums::*, trade_entry::*};
use utils::SubType;
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct UpdateTradeEntry {
    pub id: i64,
    #[serde(default)]
    pub price: FieldChange<i64>,
    #[serde(default)]
    pub tags: FieldChange<Vec<String>>,
    #[serde(default)]
    pub sub_type: FieldChange<Option<SubType>>,
}

impl UpdateTradeEntry {
    pub fn apply_to(self, mut item: trade_entry::ActiveModel) -> trade_entry::ActiveModel {
        use FieldChange::*;

        if let Value(v) = self.price {
            item.price = Set(v)
        }
        if let Value(v) = self.tags {
            item.tags = Set(v.join(","))
        }
        if let Value(v) = self.sub_type {
            item.sub_type = Set(v)
        }
        item
    }
    pub fn new(id: i64) -> Self {
        UpdateTradeEntry {
            id,
            price: FieldChange::Ignore,
            tags: FieldChange::Ignore,
            sub_type: FieldChange::Ignore,
        }
    }
}
