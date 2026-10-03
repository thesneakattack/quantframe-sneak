use std::collections::HashMap;
use std::sync::{Arc, Weak};

use entity::{dto::PaginatedResult, enums::FieldChange};
use utils::{Error, SortDirection, SubType};

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

/// Ranked upgrade instances collapsed to (unique name, rank) -> count.
///
/// The Upgrades bucket lists each ranked copy individually with its own
/// ItemId, so two maxed Serrations are two entries. Rivens live here too and
/// are excluded: they have their own tab and their own stock type.
pub fn rank_groups(upgrades: &[WFInvItemRaw]) -> HashMap<(String, i64), i64> {
    let mut groups: HashMap<(String, i64), i64> = HashMap::new();
    for entry in upgrades.iter() {
        if entry.unique_name.is_empty() || entry.is_riven() {
            continue;
        }
        let rank = entry.get_upgrade_fingerprint().mod_rank;
        *groups.entry((entry.unique_name.clone(), rank)).or_insert(0) += 1;
    }
    groups
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

    /// Tradable mods and arcanes. The RawUpgrades bucket holds unranked
    /// stacks; the Upgrades bucket holds individually ranked instances, which
    /// are grouped by rank so a maxed copy lists separately from an unranked
    /// one.
    pub fn get_mods(
        &self,
        query: WFItemPaginationDto,
    ) -> Result<PaginatedResult<WFInvItemBase>, Error> {
        let client = self.client.upgrade().unwrap();
        let root = client.get_root();
        let cache = states::cache_client()?;

        // unique name, rank, quantity
        let mut rows: Vec<(String, i64, i64)> = Vec::new();
        for (unique_name, quantity) in owned_counts(&[&root.raw_upgrades]) {
            rows.push((unique_name, 0, quantity));
        }
        for ((unique_name, rank), quantity) in rank_groups(&root.upgrades) {
            rows.push((unique_name, rank, quantity));
        }

        let mut items: Vec<WFInvItemBase> = Vec::new();
        for (unique_name, rank, quantity) in rows {
            let Ok(tradable) = cache.tradable_item().get_by(&unique_name) else {
                continue;
            };
            let mut item = WFInvItemBase {
                id: tradable.wfm_id.clone(),
                name: tradable.name.clone(),
                unique_name,
                wfm_url: tradable.wfm_url.clone(),
                quantity,
                // Unranked rows carry no sub_type at all, matching how stock
                // represents an unranked item. Some(rank: 0) would make
                // ItemName render a bare "Rank " with no number after it.
                sub_type: if rank > 0 {
                    Some(SubType {
                        rank: Some(rank),
                        ..Default::default()
                    })
                } else {
                    None
                },
                ..Default::default()
            };
            item.properties
                .set_property_value("tags", tradable.tags.clone());
            item.properties.set_property_value(
                "max_rank",
                tradable.sub_type.as_ref().and_then(|s| s.max_rank),
            );
            items.push(item);
        }

        if let FieldChange::Value(text) = &query.query {
            let text = text.to_lowercase();
            items.retain(|item| item.name.to_lowercase().contains(&text));
        }
        let rank_filter = match &query.properties {
            FieldChange::Value(properties) => {
                properties.get_property_value("rank_filter", String::new())
            }
            _ => String::new(),
        };
        let rank_of =
            |item: &WFInvItemBase| item.sub_type.as_ref().and_then(|s| s.rank).unwrap_or(0);
        match rank_filter.as_str() {
            "unranked" => items.retain(|item| rank_of(item) == 0),
            "ranked" => items.retain(|item| rank_of(item) > 0),
            _ => {}
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
    use super::{owned_counts, rank_groups};
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

    fn ranked(unique_name: &str, fingerprint: &str) -> WFInvItemRaw {
        WFInvItemRaw {
            id: Default::default(),
            quantity: 1,
            unique_name: unique_name.to_string(),
            upgrade_fingerprint: Some(fingerprint.to_string()),
            last_added: Default::default(),
            xp: 0,
        }
    }

    #[test]
    fn groups_instances_of_the_same_item_and_rank() {
        let upgrades = vec![
            ranked("/Mods/Serration", r#"{"lvl":10}"#),
            ranked("/Mods/Serration", r#"{"lvl":10}"#),
            ranked("/Mods/Serration", r#"{"lvl":3}"#),
        ];
        let groups = rank_groups(&upgrades);
        assert_eq!(groups.get(&("/Mods/Serration".to_string(), 10)), Some(&2));
        assert_eq!(groups.get(&("/Mods/Serration".to_string(), 3)), Some(&1));
    }

    /// Rivens have their own tab and their own stock type; they must not
    /// appear among the mods.
    #[test]
    fn excludes_rivens() {
        let upgrades = vec![
            ranked(
                "/Lotus/Upgrades/Mods/Randomized/LotusRifleRandomMod",
                r#"{"lvl":8}"#,
            ),
            ranked("/Mods/Serration", r#"{"lvl":8}"#),
        ];
        let groups = rank_groups(&upgrades);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups.get(&("/Mods/Serration".to_string(), 8)), Some(&1));
    }

    /// A malformed or absent fingerprint falls back to rank 0. It must not
    /// panic, and it must stay distinguishable from the unranked RawUpgrades
    /// stack, which is counted separately and never passes through here.
    #[test]
    fn treats_an_unparseable_fingerprint_as_rank_zero() {
        let upgrades = vec![
            ranked("/Mods/Serration", "not json at all"),
            WFInvItemRaw {
                id: Default::default(),
                quantity: 1,
                unique_name: "/Mods/Serration".to_string(),
                upgrade_fingerprint: None,
                last_added: Default::default(),
                xp: 0,
            },
        ];
        let groups = rank_groups(&upgrades);
        assert_eq!(groups.get(&("/Mods/Serration".to_string(), 0)), Some(&2));
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
