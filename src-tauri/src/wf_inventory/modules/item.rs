use std::collections::HashMap;
use std::sync::{Arc, Weak};

use entity::{dto::PaginatedResult, enums::FieldChange};
use utils::{Error, SortDirection};

use crate::{
    helper::paginate,
    utils::modules::states,
    wf_inventory::{item_base::WFInvItemBase, *},
};

/// Total quantity held per unique name, summed across the given inventory
/// buckets. Duplicate entries for one unique name are added together, which a
/// real inventory does contain.
pub fn owned_counts(buckets: &[&[WFInvItemRaw]]) -> HashMap<String, i64> {
    let mut counts: HashMap<String, i64> = HashMap::new();
    for bucket in buckets {
        for entry in bucket.iter() {
            if entry.unique_name.is_empty() {
                continue;
            }
            *counts.entry(entry.unique_name.clone()).or_insert(0) += entry.quantity;
        }
    }
    counts
}

/// Order rows by a requested column, following the allow-list shape the rest
/// of the app uses for in-memory pagination (see `commands/order.rs`,
/// `commands/chat.rs`, `modules/riven.rs`).
///
/// `utils::sort_data` is not usable here: its `get_sort_key` ignores the `by`
/// argument and clones the whole item, so it can only sort by `Ord`.
///
/// Anything unrecognised falls back to name ascending, so a row set is never
/// handed to the UI in arbitrary HashMap order.
pub fn sort_rows(
    rows: &mut [WFInvItemBase],
    sort_by: &FieldChange<String>,
    sort_direction: &FieldChange<SortDirection>,
) {
    let dir = match sort_direction {
        FieldChange::Value(dir) => dir,
        _ => &SortDirection::Asc,
    };
    let column = match sort_by {
        FieldChange::Value(column) => column.as_str(),
        _ => "name",
    };
    let rank = |item: &WFInvItemBase| item.sub_type.as_ref().and_then(|s| s.rank).unwrap_or(0);
    match column {
        "quantity" => rows.sort_by(|a, b| match dir {
            SortDirection::Asc => a.quantity.cmp(&b.quantity),
            SortDirection::Desc => b.quantity.cmp(&a.quantity),
        }),
        "rank" => rows.sort_by(|a, b| match dir {
            SortDirection::Asc => rank(a).cmp(&rank(b)),
            SortDirection::Desc => rank(b).cmp(&rank(a)),
        }),
        _ => rows.sort_by(|a, b| match dir {
            SortDirection::Desc => b.name.cmp(&a.name),
            _ => a.name.cmp(&b.name),
        }),
    }
}

#[derive(Debug)]
pub struct ItemModule {
    client: Weak<WFInventoryState>,
}

impl ItemModule {
    pub fn new(client: Arc<WFInventoryState>) -> Arc<Self> {
        Arc::new(Self {
            client: Arc::downgrade(&client),
        })
    }

    /// Tradable parts: everything in the Recipes and MiscItems buckets that
    /// the tradable-items cache knows about. That lookup is the filter -
    /// resources, fish and gems are absent from the cache and drop out here
    /// without any path matching.
    pub fn get_parts(
        &self,
        query: WFItemPaginationDto,
    ) -> Result<PaginatedResult<WFInvItemBase>, Error> {
        let client = self.client.upgrade().unwrap();
        let root = client.get_root();
        let cache = states::cache_client()?;

        let counts = owned_counts(&[&root.recipes, &root.misc_items]);
        let item_set = cache.item_set();

        let mut items: Vec<WFInvItemBase> = Vec::new();
        for (unique_name, quantity) in counts {
            let Ok(tradable) = cache.tradable_item().get_by(&unique_name) else {
                continue;
            };
            let in_sets: Vec<String> = item_set
                .get_sets_for_member(&unique_name)
                .iter()
                .map(|set| set.set.name.clone())
                .collect();

            let mut item = WFInvItemBase {
                id: tradable.wfm_id.clone(),
                name: tradable.name.clone(),
                unique_name,
                wfm_url: tradable.wfm_url.clone(),
                quantity,
                sub_type: None,
                ..Default::default()
            };
            item.properties.set_property_value("in_sets", in_sets);
            item.properties
                .set_property_value("tags", tradable.tags.clone());
            items.push(item);
        }

        if let FieldChange::Value(text) = &query.query {
            let text = text.to_lowercase();
            items.retain(|item| item.name.to_lowercase().contains(&text));
        }
        let in_set_only = match &query.properties {
            FieldChange::Value(properties) => properties.get_property_value("in_set_only", false),
            _ => false,
        };
        if in_set_only {
            items.retain(|item| {
                !item
                    .properties
                    .get_property_value::<Vec<String>>("in_sets", vec![])
                    .is_empty()
            });
        }

        sort_rows(&mut items, &query.sort_by, &query.sort_direction);
        Ok(paginate(
            &items,
            query.pagination.page,
            query.pagination.limit,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::owned_counts;
    use crate::wf_inventory::WFInvItemRaw;

    fn raw(unique_name: &str, quantity: i64) -> WFInvItemRaw {
        WFInvItemRaw {
            id: Default::default(),
            quantity,
            unique_name: unique_name.to_string(),
            upgrade_fingerprint: None,
            last_added: Default::default(),
            xp: 0,
        }
    }

    #[test]
    fn sums_quantities_across_buckets() {
        let recipes = vec![raw("/A", 2)];
        let misc = vec![raw("/B", 5)];
        let counts = owned_counts(&[&recipes, &misc]);
        assert_eq!(counts.get("/A"), Some(&2));
        assert_eq!(counts.get("/B"), Some(&5));
    }

    /// A real inventory can list the same ItemType twice. Overwriting instead
    /// of summing would silently under-report what the player holds.
    #[test]
    fn sums_duplicate_entries_rather_than_overwriting() {
        let recipes = vec![raw("/A", 2), raw("/A", 3)];
        let counts = owned_counts(&[&recipes]);
        assert_eq!(counts.get("/A"), Some(&5));
    }

    #[test]
    fn ignores_entries_with_no_item_type() {
        let recipes = vec![raw("", 7), raw("/A", 1)];
        let counts = owned_counts(&[&recipes]);
        assert_eq!(counts.get(""), None);
        assert_eq!(counts.get("/A"), Some(&1));
    }
}

#[cfg(test)]
mod sort_tests {
    use super::sort_rows;
    use crate::wf_inventory::item_base::WFInvItemBase;
    use entity::enums::FieldChange;
    use utils::{SortDirection, SubType};

    fn row(name: &str, quantity: i64, rank: i64) -> WFInvItemBase {
        WFInvItemBase {
            name: name.to_string(),
            quantity,
            sub_type: Some(SubType {
                rank: Some(rank),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    fn names(rows: &[WFInvItemBase]) -> Vec<&str> {
        rows.iter().map(|r| r.name.as_str()).collect()
    }

    /// With no sort requested the rows come back alphabetical, so the tab is
    /// never in arbitrary HashMap order.
    #[test]
    fn defaults_to_name_ascending() {
        let mut rows = vec![row("Vitality", 1, 0), row("Ammo Drum", 1, 0)];
        sort_rows(&mut rows, &FieldChange::Ignore, &FieldChange::Ignore);
        assert_eq!(names(&rows), vec!["Ammo Drum", "Vitality"]);
    }

    #[test]
    fn sorts_by_quantity_descending_when_asked() {
        let mut rows = vec![row("A", 2, 0), row("B", 9, 0), row("C", 5, 0)];
        sort_rows(
            &mut rows,
            &FieldChange::Value("quantity".to_string()),
            &FieldChange::Value(SortDirection::Desc),
        );
        assert_eq!(names(&rows), vec!["B", "C", "A"]);
    }

    #[test]
    fn sorts_by_rank() {
        let mut rows = vec![row("A", 1, 10), row("B", 1, 0), row("C", 1, 5)];
        sort_rows(
            &mut rows,
            &FieldChange::Value("rank".to_string()),
            &FieldChange::Value(SortDirection::Asc),
        );
        assert_eq!(names(&rows), vec!["B", "C", "A"]);
    }

    /// An unknown column must not silently leave HashMap order behind; it
    /// falls back to the name sort the user would expect.
    #[test]
    fn falls_back_to_name_for_an_unknown_column() {
        let mut rows = vec![row("Vitality", 1, 0), row("Ammo Drum", 1, 0)];
        sort_rows(
            &mut rows,
            &FieldChange::Value("nonsense".to_string()),
            &FieldChange::Value(SortDirection::Asc),
        );
        assert_eq!(names(&rows), vec!["Ammo Drum", "Vitality"]);
    }
}
