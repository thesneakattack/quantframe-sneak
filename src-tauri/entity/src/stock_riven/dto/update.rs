use sea_orm::{ActiveValue, Set};
use serde::{Deserialize, Serialize};
use utils::Properties;

use crate::{dto::*, enums::*, stock_riven::*};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct UpdateStockRiven {
    pub id: i64,

    #[serde(default)]
    pub bought: FieldChange<i64>,

    #[serde(default)]
    pub list_price: FieldChange<i64>,

    #[serde(default)]
    pub re_rolls: FieldChange<i64>,

    #[serde(default)]
    pub mastery_rank: FieldChange<i64>,

    #[serde(default)]
    pub status: FieldChange<StockStatus>,

    #[serde(default)]
    pub is_hidden: FieldChange<bool>,

    #[serde(default)]
    pub filter: FieldChange<MatchRivenStruct>,

    #[serde(default)]
    pub price_history: FieldChange<Vec<PriceHistory>>,

    #[serde(default)]
    pub grade: FieldChange<RivenGrade>,

    #[serde(default, flatten)]
    pub properties: FieldChange<Properties>,
}

impl UpdateStockRiven {
    pub fn apply_to(self, mut item: stock_riven::ActiveModel) -> stock_riven::ActiveModel {
        use FieldChange::*;
        if let Value(v) = self.bought { item.bought = Set(v) }
        match self.list_price {
            Value(v) => item.list_price = Set(Some(v)),
            Null => item.list_price = Set(None),
            _ => {}
        }
        if let Value(v) = self.is_hidden { item.is_hidden = Set(v) }
        if let Value(v) = self.status { item.status = Set(v) }
        if let Value(v) = self.filter { item.filter = Set(v) }
        if let Value(v) = self.mastery_rank { item.mastery_rank = Set(v) }
        if let Value(v) = self.re_rolls { item.re_rolls = Set(v) }
        if let Value(v) = self.price_history { item.price_history = Set(PriceHistoryVec(v)) }
        if let Value(mut v) = self.properties {
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

        item
    }
    pub fn new(id: i64) -> Self {
        UpdateStockRiven {
            id,
            bought: FieldChange::Ignore,
            list_price: FieldChange::Ignore,
            is_hidden: FieldChange::Ignore,
            filter: FieldChange::Ignore,
            status: FieldChange::Ignore,
            mastery_rank: FieldChange::Ignore,
            re_rolls: FieldChange::Ignore,
            price_history: FieldChange::Ignore,
            grade: FieldChange::Ignore,
            properties: FieldChange::Ignore,
        }
    }

    pub fn with_bought(mut self, bought: i64) -> Self {
        self.bought = FieldChange::Value(bought);
        self
    }

    pub fn with_list_price(mut self, list_price: Option<i64>) -> Self {
        self.list_price = match list_price {
            Some(v) => FieldChange::Value(v),
            None => FieldChange::Null,
        };
        self
    }

    pub fn with_is_hidden(mut self, is_hidden: bool) -> Self {
        self.is_hidden = FieldChange::Value(is_hidden);
        self
    }

    pub fn with_status(mut self, status: StockStatus) -> Self {
        self.status = FieldChange::Value(status);
        self
    }
    pub fn with_filter(mut self, filter: Option<MatchRivenStruct>) -> Self {
        self.filter = match filter {
            Some(v) => FieldChange::Value(v),
            None => FieldChange::Null,
        };
        self
    }
    pub fn with_price_history(mut self, price_history: Option<Vec<PriceHistory>>) -> Self {
        self.price_history = match price_history {
            Some(v) => FieldChange::Value(v),
            None => FieldChange::Null,
        };
        self
    }
    pub fn with_grade(mut self, grade: RivenGrade) -> Self {
        self.grade = FieldChange::Value(grade);
        self
    }
    pub fn with_properties(mut self, properties: Properties) -> Self {
        self.properties = FieldChange::Value(properties);
        self
    }
}
