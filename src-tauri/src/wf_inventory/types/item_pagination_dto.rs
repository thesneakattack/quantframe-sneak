use entity::{dto::*, enums::*};

use serde::{Deserialize, Serialize};
use utils::{Properties, SortDirection};

/// One column of a sort, with its own direction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortField {
    pub by: String,
    #[serde(default = "default_direction")]
    pub direction: SortDirection,
}

fn default_direction() -> SortDirection {
    SortDirection::Asc
}

/// A column's value for the purpose of ordering.
pub enum SortValue {
    Num(f64),
    /// A number that may be absent, such as a price nobody has resolved yet.
    MaybeNum(Option<f64>),
    Text(String),
}

impl SortValue {
    /// Compares two values of the same column.
    ///
    /// An absent number always sorts last, whichever direction is asked for:
    /// "unknown" is not a small value, and burying unknowns at the bottom is
    /// what a reader expects of both "cheapest first" and "dearest first".
    fn cmp_with(&self, other: &SortValue, direction: &SortDirection) -> std::cmp::Ordering {
        use std::cmp::Ordering;
        let flip = |ordering: Ordering| match direction {
            SortDirection::Asc => ordering,
            SortDirection::Desc => ordering.reverse(),
        };
        match (self, other) {
            (SortValue::MaybeNum(None), SortValue::MaybeNum(None)) => Ordering::Equal,
            (SortValue::MaybeNum(None), _) => Ordering::Greater,
            (_, SortValue::MaybeNum(None)) => Ordering::Less,
            (SortValue::MaybeNum(Some(a)), SortValue::MaybeNum(Some(b)))
            | (SortValue::Num(a), SortValue::Num(b)) => {
                flip(a.partial_cmp(b).unwrap_or(Ordering::Equal))
            }
            (SortValue::Text(a), SortValue::Text(b)) => flip(a.cmp(b)),
            _ => Ordering::Equal,
        }
    }
}

/// Order rows by each field in turn, the first field deciding and later ones
/// breaking its ties.
///
/// Always finishes on the name, so rows that tie on every requested field
/// still come out in a defined order. Without that last step a sort on a
/// column where most rows share a value - "owned", where nearly everything
/// is 1 - returns them in whatever order the projection happened to build,
/// which reads as the sort not having worked at all.
pub fn sort_by_fields<T>(
    rows: &mut [T],
    sorts: &[SortField],
    value_of: impl Fn(&T, &str) -> SortValue,
) {
    rows.sort_by(|a, b| {
        for field in sorts {
            let ordering =
                value_of(a, &field.by).cmp_with(&value_of(b, &field.by), &field.direction);
            if ordering != std::cmp::Ordering::Equal {
                return ordering;
            }
        }
        value_of(a, "name").cmp_with(&value_of(b, "name"), &SortDirection::Asc)
    });
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WFItemPaginationDto {
    #[serde(flatten)]
    pub pagination: PaginationQueryDto,

    #[serde(default)]
    pub query: FieldChange<String>,

    #[serde(default)]
    pub item_types: FieldChange<Vec<String>>,

    #[serde(default, flatten)]
    pub properties: FieldChange<Properties>,

    #[serde(default)]
    pub sort_by: FieldChange<String>,

    #[serde(default)]
    pub sort_direction: FieldChange<SortDirection>,

    /// Several columns in priority order. Takes precedence over the single
    /// `sort_by`/`sort_direction` pair, which the riven and syndicate tabs
    /// still use.
    #[serde(default)]
    pub sorts: FieldChange<Vec<SortField>>,
}

impl WFItemPaginationDto {
    /// The sort to apply: the priority list when given, otherwise the single
    /// field, otherwise nothing.
    pub fn sort_fields(&self) -> Vec<SortField> {
        if let FieldChange::Value(sorts) = &self.sorts {
            if !sorts.is_empty() {
                return sorts.clone();
            }
        }
        match (&self.sort_by, &self.sort_direction) {
            (FieldChange::Value(by), direction) => vec![SortField {
                by: by.clone(),
                direction: direction.get_value(SortDirection::Asc),
            }],
            _ => vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::WFItemPaginationDto;
    use entity::enums::FieldChange;

    /// The priority list has to survive the same trip, and win over the
    /// single-field pair when both are present.
    #[test]
    fn a_priority_list_survives_the_wire_and_wins_over_the_single_field() {
        let json = r#"{
            "page": 1,
            "limit": 25,
            "sort_by": "name",
            "sort_direction": "asc",
            "sorts": [
                { "by": "price", "direction": "desc" },
                { "by": "quantity", "direction": "asc" }
            ]
        }"#;
        let dto: WFItemPaginationDto = serde_json::from_str(json).expect("should parse");
        let fields = dto.sort_fields();
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].by, "price");
        assert_eq!(fields[0].direction, utils::SortDirection::Desc);
        assert_eq!(fields[1].by, "quantity");
    }

    /// With no list, the single field the column headers set still applies,
    /// which is what the riven and syndicate tabs rely on.
    #[test]
    fn falls_back_to_the_single_sort_field() {
        let json = r#"{ "page": 1, "limit": 25, "sort_by": "quantity", "sort_direction": "desc" }"#;
        let dto: WFItemPaginationDto = serde_json::from_str(json).expect("should parse");
        let fields = dto.sort_fields();
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].by, "quantity");
        assert_eq!(fields[0].direction, utils::SortDirection::Desc);
    }

    #[test]
    fn no_sort_at_all_yields_no_fields() {
        let dto: WFItemPaginationDto =
            serde_json::from_str(r#"{ "page": 1, "limit": 25 }"#).expect("should parse");
        assert!(dto.sort_fields().is_empty());
    }

    /// Exactly what `convertToTauriQuery` sends: page and limit at the top
    /// level, a redundant nested `pagination` object, the sort pair, and the
    /// properties bag.
    #[test]
    fn parses_the_query_the_frontend_actually_sends() {
        let json = r#"{
            "page": 1,
            "limit": 25,
            "sort_by": "price",
            "sort_direction": "desc",
            "properties": { "min_price": 10 },
            "pagination": { "page": 1, "limit": 25 }
        }"#;
        let dto: WFItemPaginationDto = serde_json::from_str(json).expect("should parse");

        assert_eq!(dto.pagination.page, 1);
        assert_eq!(dto.pagination.limit, 25);
        match &dto.sort_by {
            FieldChange::Value(by) => assert_eq!(by, "price"),
            other => panic!("sort_by did not survive: {other:?}"),
        }
        match &dto.sort_direction {
            FieldChange::Value(dir) => assert_eq!(*dir, utils::SortDirection::Desc),
            other => panic!("sort_direction did not survive: {other:?}"),
        }
        match &dto.properties {
            FieldChange::Value(props) => {
                assert_eq!(props.get_property_value("min_price", 0i64), 10)
            }
            other => panic!("properties did not survive: {other:?}"),
        }
    }
}

#[cfg(test)]
mod sort_field_tests {
    use super::{sort_by_fields, SortField, SortValue};
    use utils::SortDirection;

    #[derive(Debug, Clone, PartialEq)]
    struct Row {
        name: &'static str,
        owned: i64,
        price: Option<f64>,
    }

    fn value_of(row: &Row, column: &str) -> SortValue {
        match column {
            "quantity" => SortValue::Num(row.owned as f64),
            "price" => SortValue::MaybeNum(row.price),
            _ => SortValue::Text(row.name.to_string()),
        }
    }

    fn rows() -> Vec<Row> {
        vec![
            Row {
                name: "Zephyr",
                owned: 1,
                price: Some(5.0),
            },
            Row {
                name: "Ash",
                owned: 1,
                price: Some(50.0),
            },
            Row {
                name: "Mag",
                owned: 9,
                price: None,
            },
        ]
    }

    fn names(rows: &[Row]) -> Vec<&str> {
        rows.iter().map(|r| r.name).collect()
    }

    fn sort(sorts: &[(&str, SortDirection)]) -> Vec<Row> {
        let fields: Vec<SortField> = sorts
            .iter()
            .map(|(by, dir)| SortField {
                by: by.to_string(),
                direction: dir.clone(),
            })
            .collect();
        let mut rows = rows();
        sort_by_fields(&mut rows, &fields, value_of);
        rows
    }

    /// Equal values used to come out in whatever order the HashMap happened
    /// to yield, which reads as "sorting is broken" even though it ran.
    #[test]
    fn breaks_ties_by_name_so_the_order_is_never_arbitrary() {
        let sorted = sort(&[("quantity", SortDirection::Asc)]);
        assert_eq!(names(&sorted), vec!["Ash", "Zephyr", "Mag"]);
    }

    /// The point of multi-sort: the second field decides where the first ties.
    #[test]
    fn applies_each_field_in_priority_order() {
        let sorted = sort(&[
            ("quantity", SortDirection::Asc),
            ("price", SortDirection::Desc),
        ]);
        assert_eq!(names(&sorted), vec!["Ash", "Zephyr", "Mag"]);

        let flipped = sort(&[
            ("quantity", SortDirection::Asc),
            ("price", SortDirection::Asc),
        ]);
        assert_eq!(names(&flipped), vec!["Zephyr", "Ash", "Mag"]);
    }

    #[test]
    fn each_field_carries_its_own_direction() {
        let sorted = sort(&[("quantity", SortDirection::Desc)]);
        assert_eq!(names(&sorted), vec!["Mag", "Ash", "Zephyr"]);
    }

    /// An unknown price is not a cheap one, so it sorts to the end either
    /// way rather than pretending to be zero.
    #[test]
    fn rows_with_no_value_sort_last_in_both_directions() {
        let ascending = sort(&[("price", SortDirection::Asc)]);
        assert_eq!(names(&ascending), vec!["Zephyr", "Ash", "Mag"]);

        let descending = sort(&[("price", SortDirection::Desc)]);
        assert_eq!(names(&descending), vec!["Ash", "Zephyr", "Mag"]);
    }

    /// No sort requested still yields a defined order rather than whatever
    /// the projection happened to build.
    #[test]
    fn falls_back_to_name_when_nothing_is_requested() {
        let sorted = sort(&[]);
        assert_eq!(names(&sorted), vec!["Ash", "Mag", "Zephyr"]);
    }
}
