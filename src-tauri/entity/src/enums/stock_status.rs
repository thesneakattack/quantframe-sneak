use sea_orm::entity::prelude::*;
use sea_orm::sea_query::{ColumnType, SeaRc};
use sea_orm::{ActiveEnum, ColumnDef, DbErr};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, EnumIter)]
pub enum StockStatus {
    #[serde(rename = "error")]
    Error,

    #[serde(rename = "unknown")]
    Unknown,

    #[serde(rename = "pending")]
    Pending,

    #[serde(rename = "live")]
    Live,

    #[serde(rename = "to_low_profit")]
    ToLowProfit,

    #[serde(rename = "no_sellers")]
    NoSellers,

    #[serde(rename = "no_buyers")]
    NoBuyers,

    #[serde(rename = "inactive")]
    InActive,

    #[serde(rename = "sma_limit")]
    SMALimit,

    #[serde(rename = "order_limit")]
    OrderLimit,

    #[serde(rename = "overpriced")]
    Overpriced,

    #[serde(rename = "underpriced")]
    Underpriced,

    #[serde(rename = "max_price_drop")]
    MaxPriceDrop,

    #[serde(rename = "insufficient_standing")]
    InsufficientStanding,
}

#[derive(Debug, DeriveIden)]
pub struct StockStatusEnum;

impl ActiveEnum for StockStatus {
    type Value = String;

    type ValueVec = Vec<String>;

    fn name() -> DynIden {
        SeaRc::new(StockStatusEnum)
    }

    fn to_value(&self) -> Self::Value {
        self.as_str().to_owned()
    }

    fn try_from_value(v: &Self::Value) -> Result<Self, DbErr> {
        Ok(Self::from_str(v))
    }

    fn db_type() -> ColumnDef {
        ColumnType::String(Some(50)).def()
    }
}

impl Into<sea_orm::sea_query::Value> for StockStatus {
    fn into(self) -> sea_orm::sea_query::Value {
        <Self as sea_orm::ActiveEnum>::to_value(&self).into()
    }
}

impl sea_orm::TryGetable for StockStatus {
    fn try_get_by<I: sea_orm::ColIdx>(
        res: &sea_orm::QueryResult,
        idx: I,
    ) -> std::result::Result<Self, sea_orm::TryGetError> {
        let value =
            <<Self as sea_orm::ActiveEnum>::Value as sea_orm::TryGetable>::try_get_by(res, idx)?;
        <Self as sea_orm::ActiveEnum>::try_from_value(&value).map_err(sea_orm::TryGetError::DbErr)
    }
}

impl sea_orm::sea_query::ValueType for StockStatus {
    fn try_from(
        v: sea_orm::sea_query::Value,
    ) -> std::result::Result<Self, sea_orm::sea_query::ValueTypeErr> {
        let value =
            <<Self as sea_orm::ActiveEnum>::Value as sea_orm::sea_query::ValueType>::try_from(v)?;
        <Self as sea_orm::ActiveEnum>::try_from_value(&value)
            .map_err(|_| sea_orm::sea_query::ValueTypeErr)
    }

    fn type_name() -> String {
        <<Self as sea_orm::ActiveEnum>::Value as sea_orm::sea_query::ValueType>::type_name()
    }

    fn array_type() -> sea_orm::sea_query::ArrayType {
        <<Self as sea_orm::ActiveEnum>::Value as sea_orm::sea_query::ValueType>::array_type()
    }

    fn column_type() -> sea_orm::sea_query::ColumnType {
        <Self as sea_orm::ActiveEnum>::db_type()
            .get_column_type()
            .to_owned()
            .into()
    }
}

impl sea_orm::sea_query::Nullable for StockStatus {
    fn null() -> sea_orm::sea_query::Value {
        <<Self as sea_orm::ActiveEnum>::Value as sea_orm::sea_query::Nullable>::null()
    }
}

impl StockStatus {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Unknown => "unknown",
            Self::Pending => "pending",
            Self::Live => "live",
            Self::ToLowProfit => "to_low_profit",
            Self::NoSellers => "no_sellers",
            Self::NoBuyers => "no_buyers",
            Self::InActive => "inactive",
            Self::SMALimit => "sma_limit",
            Self::OrderLimit => "order_limit",
            Self::Overpriced => "overpriced",
            Self::Underpriced => "underpriced",
            Self::MaxPriceDrop => "max_price_drop",
            Self::InsufficientStanding => "insufficient_standing",
        }
    }

    pub fn from_str(value: &str) -> Self {
        match value {
            "error" => Self::Error,
            "unknown" => Self::Unknown,
            "pending" => Self::Pending,
            "live" => Self::Live,
            "to_low_profit" => Self::ToLowProfit,
            "no_sellers" => Self::NoSellers,
            "no_buyers" => Self::NoBuyers,
            "inactive" => Self::InActive,
            "sma_limit" => Self::SMALimit,
            "order_limit" => Self::OrderLimit,
            "overpriced" => Self::Overpriced,
            "underpriced" => Self::Underpriced,
            "max_price_drop" => Self::MaxPriceDrop,
            "insufficient_standing" => Self::InsufficientStanding,
            _ => Self::Unknown,
        }
    }
}

impl std::fmt::Display for StockStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for StockStatus {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;

        Ok(Self::from_str(&value))
    }
}
