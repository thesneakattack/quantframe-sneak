use serde::{Deserialize, Serialize};
use std::fmt::Display;

use crate::enums::*;
use crate::types::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemPricePaginationQueryDto {
    #[serde(flatten)]
    pub pagination: PaginationQueryDto,
    // Add any stock item specific filters or fields here
    #[serde(default)]
    pub query: FieldChange<String>,
    #[serde(default)]
    pub sort_by: FieldChange<String>,
    #[serde(default)]
    pub sort_direction: FieldChange<SortDirection>,

    from_date: String,
    to_date: String,
    #[serde(default)]
    tags: FieldChange<Vec<String>>,

    #[serde(default)]
    pub wfm_id: FieldChange<String>,
    #[serde(default)]
    pub lasted: FieldChange<bool>,

    #[serde(default)]
    pub volume_gt: FieldChange<i64>,
    #[serde(default)]
    pub volume_lt: FieldChange<i64>,
    #[serde(default)]
    pub supply_gt: FieldChange<i64>,
    #[serde(default)]
    pub supply_lt: FieldChange<i64>,
    #[serde(default)]
    pub demand_gt: FieldChange<i64>,
    #[serde(default)]
    pub demand_lt: FieldChange<i64>,
    #[serde(default)]
    pub min_price_gt: FieldChange<i64>,
    #[serde(default)]
    pub min_price_lt: FieldChange<i64>,
    #[serde(default)]
    pub max_price_gt: FieldChange<i64>,
    #[serde(default)]
    pub max_price_lt: FieldChange<i64>,
}
impl ItemPricePaginationQueryDto {
    pub fn new(page: i64, limit: i64, from_date: String, to_date: String) -> Self {
        Self {
            pagination: PaginationQueryDto::new(page, limit),
            query: FieldChange::Ignore,
            sort_by: FieldChange::Ignore,
            sort_direction: FieldChange::Value(SortDirection::Asc),
            from_date,
            to_date,
            tags: FieldChange::Ignore,
            wfm_id: FieldChange::Ignore,
            lasted: FieldChange::Ignore,
            volume_gt: FieldChange::Ignore,
            volume_lt: FieldChange::Ignore,
            supply_gt: FieldChange::Ignore,
            supply_lt: FieldChange::Ignore,
            demand_gt: FieldChange::Ignore,
            demand_lt: FieldChange::Ignore,
            min_price_gt: FieldChange::Ignore,
            min_price_lt: FieldChange::Ignore,
            max_price_gt: FieldChange::Ignore,
            max_price_lt: FieldChange::Ignore,
        }
    }
    pub fn get_query(&self) -> String {
        use FieldChange::*;
        let mut query: Vec<String> = Vec::new();
        query.push(format!("page={}", self.pagination.page));
        query.push(format!("limit={}", self.pagination.limit));
        if let Value(q) = &self.query {
            query.push(format!("query={}", q))
        }
        if let Value(s) = &self.sort_by {
            query.push(format!("sort_by={}", s))
        }
        if let Value(d) = &self.sort_direction {
            query.push(format!("sort_direction={}", d.to_string()))
        }
        if let Value(v) = &self.volume_gt {
            query.push(format!("volumeGt={}", v))
        }
        if let Value(v) = &self.volume_lt {
            query.push(format!("volumeLt={}", v))
        }
        if let Value(v) = &self.supply_gt {
            query.push(format!("supplyGt={}", v))
        }
        if let Value(v) = &self.supply_lt {
            query.push(format!("supplyLt={}", v))
        }
        if let Value(v) = &self.demand_gt {
            query.push(format!("demandGt={}", v))
        }
        if let Value(v) = &self.demand_lt {
            query.push(format!("demandLt={}", v))
        }
        if let Value(v) = &self.min_price_gt {
            query.push(format!("minPriceGt={}", v))
        }
        if let Value(v) = &self.min_price_lt {
            query.push(format!("minPriceLt={}", v))
        }
        if let Value(v) = &self.max_price_gt {
            query.push(format!("maxPriceGt={}", v))
        }
        if let Value(v) = &self.max_price_lt {
            query.push(format!("maxPriceLt={}", v))
        }
        if let Value(v) = &self.wfm_id {
            query.push(format!("wfm_id={}", v))
        }
        if let Value(v) = &self.lasted {
            query.push(format!("lasted={}", v))
        }
        query.push(format!("from_date={}", self.from_date));
        query.push(format!("to_date={}", self.to_date));

        if let Value(t) = &self.tags {
            for tag in t {
                query.push(format!("tags={}", tag));
            }
        }
        query.join("&")
    }
    pub fn set_pagination(mut self, pagination: PaginationQueryDto) -> Self {
        self.pagination = pagination;
        self
    }

    pub fn set_query(mut self, query: impl Into<String>) -> Self {
        self.query = FieldChange::Value(query.into());
        self
    }

    pub fn set_sort_by(mut self, sort_by: impl Into<String>) -> Self {
        self.sort_by = FieldChange::Value(sort_by.into());
        self
    }

    pub fn set_sort_direction(mut self, sort_direction: SortDirection) -> Self {
        self.sort_direction = FieldChange::Value(sort_direction);
        self
    }

    pub fn set_wfm_id(mut self, wfm_id: impl Into<String>) -> Self {
        self.wfm_id = FieldChange::Value(wfm_id.into());
        self
    }

    pub fn set_lasted(mut self, lasted: bool) -> Self {
        self.lasted = FieldChange::Value(lasted);
        self
    }
}

impl Display for ItemPricePaginationQueryDto {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let query_str = match &self.query {
            FieldChange::Value(q) => format!("Some(\"{}\")", q),
            FieldChange::Ignore => "Ignore".to_string(),
            FieldChange::Null => "Null".to_string(),
        };
        let sort_by_str = match &self.sort_by {
            FieldChange::Value(s) => format!("Some(\"{}\")", s),
            FieldChange::Ignore => "Ignore".to_string(),
            FieldChange::Null => "Null".to_string(),
        };
        let sort_direction_str = match &self.sort_direction {
            FieldChange::Value(d) => format!("Some({:?})", d),
            FieldChange::Ignore => "Ignore".to_string(),
            FieldChange::Null => "Null".to_string(),
        };
        let from_date_str = self.from_date.clone();
        let to_date_str = self.to_date.clone();

        let tags_str = match &self.tags {
            FieldChange::Value(t) => format!("Some({:?})", t),
            FieldChange::Ignore => "Ignore".to_string(),
            FieldChange::Null => "Null".to_string(),
        };
        write!(
            f,
            "Page: {}, Limit: {}, Query: {}, Sort By: {}, Sort Direction: {}, From Date: {}, To Date: {}, Tags: {}",
            self.pagination.page,
            self.pagination.limit,
            query_str,
            sort_by_str,
            sort_direction_str,
            from_date_str,
            to_date_str,
            tags_str
        )
    }
}
