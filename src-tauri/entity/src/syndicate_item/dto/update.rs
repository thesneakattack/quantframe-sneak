use sea_orm::{ActiveValue, Set};
use serde::{Deserialize, Serialize};
use utils::Properties;
use utils::SubType;

use crate::{dto::*, enums::*, syndicate_item::*};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct UpdateSyndicateItem {
    pub id: i64,

    #[serde(default)]
    pub list_price: FieldChange<i64>,

    #[serde(default)]
    pub status: FieldChange<StockStatus>,

    #[serde(default)]
    pub price_history: FieldChange<Vec<PriceHistory>>,

    #[serde(default)]
    pub sub_type: FieldChange<Option<SubType>>,

    #[serde(default)]
    pub syndicate_name: FieldChange<String>,

    #[serde(default)]
    pub syndicate_unique_name: FieldChange<String>,

    #[serde(default)]
    pub standing_cost: FieldChange<i64>,

    #[serde(default, flatten)]
    pub properties: FieldChange<Properties>,
}

impl UpdateSyndicateItem {
    pub fn apply_to(self, mut item: syndicate_item::ActiveModel) -> syndicate_item::ActiveModel {
        use FieldChange::*;

        match self.list_price {
            Value(v) => item.list_price = Set(Some(v)),
            Null => item.list_price = Set(None),
            _ => {}
        }
        match self.status {
            Value(v) => item.status = Set(v),
            _ => {}
        }
        match self.price_history {
            Value(v) => item.price_history = Set(PriceHistoryVec(v)),
            _ => {}
        }
        match self.sub_type {
            Value(v) => item.sub_type = Set(v),
            Null => item.sub_type = Set(None),
            _ => {}
        }
        match self.syndicate_name {
            Value(v) => item.syndicate_name = Set(v),
            _ => {}
        }
        match self.syndicate_unique_name {
            Value(v) => item.syndicate_unique_name = Set(v),
            _ => {}
        }
        match self.standing_cost {
            Value(v) => item.standing_cost = Set(v),
            _ => {}
        }
        match self.properties {
            Value(mut v) => {
                v.keep_property_values(ALLOWED_PROPERTIES_FIELDS);
                v.nullify_zeroed_properties(ALLOWED_PROPERTIES_FIELDS);

                let properties = match item.properties {
                    ActiveValue::Set(mut existing) | ActiveValue::Unchanged(mut existing) => {
                        existing.merge_properties(v.properties, true, true);
                        existing
                    }
                    _ => v,
                };
                item.properties = Set(properties);
            }
            _ => {}
        }
        item
    }
    pub fn new(id: i64) -> Self {
        UpdateSyndicateItem {
            id,
            list_price: FieldChange::Ignore,
            status: FieldChange::Ignore,
            price_history: FieldChange::Ignore,
            sub_type: FieldChange::Ignore,
            syndicate_name: FieldChange::Ignore,
            syndicate_unique_name: FieldChange::Ignore,
            standing_cost: FieldChange::Ignore,
            properties: FieldChange::Ignore,
        }
    }

    pub fn with_list_price(mut self, list_price: Option<i64>) -> Self {
        self.list_price = match list_price {
            Some(v) => FieldChange::Value(v),
            None => FieldChange::Null,
        };
        self
    }

    pub fn with_status(mut self, status: StockStatus) -> Self {
        self.status = FieldChange::Value(status);
        self
    }
    pub fn with_price_history(mut self, price_history: Option<Vec<PriceHistory>>) -> Self {
        self.price_history = match price_history {
            Some(v) => FieldChange::Value(v),
            None => FieldChange::Null,
        };
        self
    }
    pub fn with_syndicate_name(mut self, syndicate_name: impl Into<String>) -> Self {
        self.syndicate_name = FieldChange::Value(syndicate_name.into());
        self
    }
    pub fn with_syndicate_unique_name(mut self, syndicate_unique_name: impl Into<String>) -> Self {
        self.syndicate_unique_name = FieldChange::Value(syndicate_unique_name.into());
        self
    }
    pub fn with_standing_cost(mut self, standing_cost: i64) -> Self {
        self.standing_cost = FieldChange::Value(standing_cost);
        self
    }
    pub fn with_properties(mut self, properties: Properties) -> Self {
        self.properties = FieldChange::Value(properties);
        self
    }
}
