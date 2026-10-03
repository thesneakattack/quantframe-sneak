//! SeaORM Entity. Mirrors the stock_item entity with syndicate-specific columns.

use crate::{dto::*, enums::*, syndicate_item::dto::UpdateSyndicateItem};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use utils::Properties;
use utils::SubType;

pub static ALLOWED_PROPERTIES_FIELDS: &[&str] = &["min_price", "max_rank", "cooldown"];

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "syndicate_item")]
pub struct Model {
    #[sea_orm(primary_key)]
    #[serde(skip_deserializing)]
    pub id: i64,

    pub wfm_id: String,

    pub wfm_url: String,

    pub item_name: String,

    pub item_unique_name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub_type: Option<SubType>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_price: Option<i64>,

    pub syndicate_name: String,

    pub syndicate_unique_name: String,

    pub standing_cost: i64,

    pub status: StockStatus,

    #[sea_orm(column_type = "Text")]
    pub price_history: PriceHistoryVec,

    #[sea_orm(updated_at)]
    pub updated_at: DateTimeUtc,

    #[sea_orm(created_at)]
    pub created_at: DateTimeUtc,

    #[sea_orm(ignore)]
    #[serde(rename = "is_dirty", default)]
    pub is_dirty: bool,

    #[sea_orm(ignore)]
    #[serde(rename = "locked", default)]
    pub locked: bool,

    #[sea_orm(ignore)]
    #[serde(rename = "changes")]
    pub changes: Vec<String>,

    // Extra properties
    #[sea_orm(column_type = "Json")]
    #[serde(default, flatten)]
    pub properties: Properties,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    pub fn new(
        wfm_id: String,
        wfm_url: String,
        item_name: String,
        item_unique_name: String,
        sub_type: Option<SubType>,
        syndicate_name: String,
        syndicate_unique_name: String,
        standing_cost: i64,
        properties: Properties,
    ) -> Self {
        Self {
            id: Default::default(),
            wfm_id,
            wfm_url,
            item_name,
            item_unique_name,
            sub_type,
            list_price: None,
            syndicate_name,
            syndicate_unique_name,
            standing_cost,
            status: StockStatus::Pending,
            price_history: PriceHistoryVec(vec![]),
            updated_at: Default::default(),
            created_at: Default::default(),
            is_dirty: true,
            locked: false,
            changes: vec![],
            properties,
        }
    }
    fn set_if_changed<T: PartialEq>(current: &mut T, new_value: T, is_dirty: &mut bool) -> bool {
        if *current != new_value {
            *current = new_value;
            *is_dirty = true;
            return true;
        }
        false
    }
    pub fn add_change(&mut self, field: &str) {
        if !self.changes.contains(&field.to_string()) {
            self.changes.push(field.to_string());
        }
    }
    pub fn has_change(&self, field: impl Into<String>) -> bool {
        self.changes.contains(&field.into())
    }

    pub fn set_list_price(&mut self, list_price: Option<i64>) {
        if self.locked {
            return;
        }
        if Self::set_if_changed(&mut self.list_price, list_price, &mut self.is_dirty) {
            println!("Updated list_price for item: {}", self.item_name);
            self.add_change("list_price");
        }
    }

    pub fn set_status(&mut self, status: StockStatus) {
        if self.locked {
            return;
        }
        if Self::set_if_changed(&mut self.status, status, &mut self.is_dirty) {
            self.add_change("status");
        }
    }

    pub fn add_price_history(&mut self, price_history: PriceHistory) {
        let mut items = self.price_history.0.clone();
        add_price_history(&mut items, price_history);
        self.price_history = PriceHistoryVec(items);
    }

    pub fn uuid(&self) -> String {
        let mut uuid = self.wfm_url.clone();
        if let Some(sub_type) = self.sub_type.clone() {
            uuid.push_str(&format!("-{}", sub_type.shot_display()));
        }
        uuid
    }
    pub fn set_properties(&mut self, properties: Properties) {
        if self.properties != properties {
            self.properties = properties;
            self.is_dirty = true;
            self.add_change("properties");
        }
    }
    pub fn to_update(&self) -> UpdateSyndicateItem {
        UpdateSyndicateItem {
            id: self.id,
            list_price: self
                .list_price
                .map_or(FieldChange::Null, |v| FieldChange::Value(v)),
            status: FieldChange::Value(self.status.clone()),
            price_history: FieldChange::Value(self.price_history.0.clone()),
            sub_type: FieldChange::Value(self.sub_type.clone()),
            syndicate_name: FieldChange::Value(self.syndicate_name.clone()),
            syndicate_unique_name: FieldChange::Value(self.syndicate_unique_name.clone()),
            standing_cost: FieldChange::Value(self.standing_cost),
            properties: FieldChange::Value(self.properties.clone()),
        }
    }
    pub fn update_gui(&self) -> bool {
        self.has_change("list_price") || self.has_change("status")
    }
}
