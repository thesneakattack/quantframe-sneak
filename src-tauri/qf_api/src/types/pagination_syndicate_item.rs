use serde::{Deserialize, Serialize};

use crate::enums::*;
use crate::types::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyndicateItemPricePaginationQueryDto {
    #[serde(flatten)]
    pub pagination: PaginationQueryDto,
    // Add any stock item specific filters or fields here
    #[serde(default)]
    pub query: FieldChange<String>,
    #[serde(default)]
    pub sort_by: FieldChange<String>,
    #[serde(default)]
    pub sort_direction: FieldChange<SortDirection>,

    #[serde(default)]
    pub volume_gt: FieldChange<i64>,
    #[serde(default)]
    pub volume_lt: FieldChange<i64>,
    #[serde(default)]
    pub min_price_gt: FieldChange<i64>,
    #[serde(default)]
    pub min_price_lt: FieldChange<i64>,
    #[serde(default)]
    pub standing_cost_gt: FieldChange<i64>,
    #[serde(default)]
    pub standing_cost_lt: FieldChange<i64>,
    #[serde(default)]
    pub syndicates: FieldChange<Vec<String>>,
}
impl SyndicateItemPricePaginationQueryDto {
    pub fn new(page: i64, limit: i64) -> Self {
        Self {
            pagination: PaginationQueryDto::new(page, limit),
            query: FieldChange::Ignore,
            sort_by: FieldChange::Ignore,
            sort_direction: FieldChange::Value(SortDirection::Asc),
            volume_gt: FieldChange::Ignore,
            volume_lt: FieldChange::Ignore,
            standing_cost_gt: FieldChange::Ignore,
            standing_cost_lt: FieldChange::Ignore,
            min_price_gt: FieldChange::Ignore,
            min_price_lt: FieldChange::Ignore,
            syndicates: FieldChange::Ignore,
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
        if let Value(v) = &self.min_price_gt {
            query.push(format!("minPriceGt={}", v))
        }
        if let Value(v) = &self.min_price_lt {
            query.push(format!("minPriceLt={}", v))
        }
        if let Value(v) = &self.standing_cost_gt {
            query.push(format!("standingCostGt={}", v))
        }
        if let Value(v) = &self.standing_cost_lt {
            query.push(format!("standingCostLt={}", v))
        }
        if let Value(s) = &self.syndicates {
            for syndicate in s {
                query.push(format!("syndicates={}", syndicate));
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
}
