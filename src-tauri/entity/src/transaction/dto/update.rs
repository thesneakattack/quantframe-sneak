use sea_orm::Set;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{enums::*, transaction::*};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct UpdateTransaction {
    pub id: i64,
    pub price: FieldChange<i64>,
    pub quantity: FieldChange<i64>,
    pub created_at: FieldChange<String>,
    pub user_name: FieldChange<String>,
    pub credits: FieldChange<i64>,
    pub item_unique_name: FieldChange<String>,
    pub properties: FieldChange<Value>,
}

impl UpdateTransaction {
    pub fn apply_to(self, mut item: transaction::ActiveModel) -> transaction::ActiveModel {
        use FieldChange::*;
        if let Value(v) = self.price {
            item.price = Set(v)
        }
        if let Value(v) = self.quantity {
            item.quantity = Set(v)
        }
        if let Value(v) = self.user_name {
            item.user_name = Set(v)
        }
        if let Value(v) = self.created_at {
            item.created_at = Set(v.parse().unwrap())
        }
        match self.properties {
            Value(v) => item.properties = Set(Some(v)),
            Null => item.properties = Set(None),
            _ => {}
        }
        if let Value(v) = self.credits {
            item.credits = Set(v)
        }
        if let Value(v) = self.item_unique_name {
            item.item_unique_name = Set(v)
        }

        item
    }
    pub fn new(id: i64) -> Self {
        UpdateTransaction {
            id,
            price: FieldChange::Ignore,
            quantity: FieldChange::Ignore,
            user_name: FieldChange::Ignore,
            created_at: FieldChange::Ignore,
            properties: FieldChange::Ignore,
            credits: FieldChange::Ignore,
            item_unique_name: FieldChange::Ignore,
        }
    }
}
