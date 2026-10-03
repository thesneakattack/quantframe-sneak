use sea_orm::sea_query::Func;
use sea_orm::*;
use sea_query::Expr;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utils::sorting::SortDirection;

use crate::{dto::*, enums::*, syndicate_item::*};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyndicateItemPaginationQueryDto {
    #[serde(flatten)]
    pub pagination: PaginationQueryDto,
    #[serde(default)]
    pub query: FieldChange<String>,
    #[serde(default)]
    pub sort_by: FieldChange<String>,
    #[serde(default)]
    pub sort_direction: FieldChange<SortDirection>,
    #[serde(default)]
    pub status: FieldChange<String>,
    #[serde(default)]
    pub syndicate: FieldChange<String>,
}
impl SyndicateItemPaginationQueryDto {
    pub fn new(page: i64, limit: i64) -> Self {
        Self {
            pagination: PaginationQueryDto::new(page, limit),
            query: FieldChange::Ignore,
            sort_by: FieldChange::Ignore,
            sort_direction: FieldChange::Value(SortDirection::Asc),
            status: FieldChange::Ignore,
            syndicate: FieldChange::Ignore,
        }
    }
    pub fn get_query(&self) -> Select<syndicate_item::Entity> {
        use FieldChange::*;
        let mut stmt = syndicate_item::Entity::find();
        match &self.query {
            Value(q) => {
                stmt = stmt.filter(
                    Condition::any()
                        .add(
                            Expr::expr(Func::lower(Expr::col(syndicate_item::Column::WfmUrl)))
                                .like(&format!("%{}%", q.to_lowercase())),
                        )
                        .add(
                            Expr::expr(Func::lower(Expr::col(syndicate_item::Column::ItemName)))
                                .like(&format!("%{}%", q.to_lowercase())),
                        )
                        .add(
                            Expr::expr(Func::lower(Expr::col(
                                syndicate_item::Column::ItemUniqueName,
                            )))
                            .like(&format!("%{}%", q.to_lowercase())),
                        ),
                )
            }
            _ => {}
        }
        match self.status {
            Value(ref q) => stmt = stmt.filter(syndicate_item::Column::Status.eq(q)),
            _ => {}
        }
        match self.syndicate {
            Value(ref q) => stmt = stmt.filter(syndicate_item::Column::SyndicateName.eq(q)),
            _ => {}
        }
        match &self.sort_by {
            Value(sort_by) => {
                let dir = match &self.sort_direction {
                    Value(dir) => dir,
                    _ => &SortDirection::Asc,
                };
                let order = match dir {
                    SortDirection::Asc => Order::Asc,
                    SortDirection::Desc => Order::Desc,
                };
                // Only allow sorting by known columns for safety
                match sort_by.as_str() {
                    "item_name" => stmt = stmt.order_by(syndicate_item::Column::ItemName, order),
                    "status" => stmt = stmt.order_by(syndicate_item::Column::Status, order),
                    "list_price" => stmt = stmt.order_by(syndicate_item::Column::ListPrice, order),
                    "syndicate_name" => {
                        stmt = stmt.order_by(syndicate_item::Column::SyndicateName, order)
                    }
                    "standing_cost" => {
                        stmt = stmt.order_by(syndicate_item::Column::StandingCost, order)
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        stmt
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

    pub fn set_status(mut self, status: impl Into<String>) -> Self {
        self.status = FieldChange::Value(status.into());
        self
    }

    pub fn set_syndicate(mut self, syndicate: impl Into<String>) -> Self {
        self.syndicate = FieldChange::Value(syndicate.into());
        self
    }
}

impl Display for SyndicateItemPaginationQueryDto {
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
        let status_str = match &self.status {
            FieldChange::Value(s) => format!("Some(\"{}\")", s),
            FieldChange::Ignore => "Ignore".to_string(),
            FieldChange::Null => "Null".to_string(),
        };
        let syndicate_str = match &self.syndicate {
            FieldChange::Value(s) => format!("Some(\"{}\")", s),
            FieldChange::Ignore => "Ignore".to_string(),
            FieldChange::Null => "Null".to_string(),
        };
        write!(
            f,
            "Page: {}, Limit: {}, Query: {}, Sort By: {}, Sort Direction: {}, Status: {}, Syndicate: {}",
            self.pagination.page,
            self.pagination.limit,
            query_str,
            sort_by_str,
            sort_direction_str,
            status_str,
            syndicate_str
        )
    }
}
impl Default for SyndicateItemPaginationQueryDto {
    fn default() -> Self {
        Self {
            pagination: PaginationQueryDto::default(),
            query: FieldChange::Ignore,
            sort_by: FieldChange::Ignore,
            sort_direction: FieldChange::Value(SortDirection::Asc),
            status: FieldChange::Ignore,
            syndicate: FieldChange::Ignore,
        }
    }
}
