use std::collections::HashMap;
use std::sync::{Arc, Weak};

use entity::{dto::PaginatedResult, enums::FieldChange};
use utils::{Error, SortDirection, SubType};

use crate::{
    cache::{
        modules::{build_price_index, lookup_price},
        CacheTradableItem,
    },
    helper::paginate,
    utils::modules::states,
    wf_inventory::{item_base::WFInvItemBase, *},
};

/// Total quantity held per unique name, summed across the given inventory
/// buckets. Duplicate entries for one unique name are added together, which a
/// real inventory does contain.
pub fn owned_counts(buckets: &[&[WFInvItemRaw]]) -> HashMap<String, i64> {
    owned_counts_where(buckets, |_| true)
}

/// `owned_counts`, keeping only entries the predicate accepts.
pub fn owned_counts_where(
    buckets: &[&[WFInvItemRaw]],
    keep: impl Fn(&WFInvItemRaw) -> bool,
) -> HashMap<String, i64> {
    let mut counts: HashMap<String, i64> = HashMap::new();
    for bucket in buckets {
        for entry in bucket.iter() {
            if entry.unique_name.is_empty() || !keep(entry) {
                continue;
            }
            *counts.entry(entry.unique_name.clone()).or_insert(0) += entry.quantity;
        }
    }
    counts
}

/// Which variant of a tradable item a unique name matched through, if any.
///
/// `TradableItemModule::load` registers every value of `variantToUniqueName`
/// as a lookup key, so all four relic refinements resolve to one
/// `CacheTradableItem`. Without recovering the variant, an Intact and a
/// Radiant relic are indistinguishable rows that both list as a variant-less
/// stock item, and the scraper prices the wrong product.
pub fn variant_of(item: &CacheTradableItem, unique_name: &str) -> Option<String> {
    if item.unique_name == unique_name {
        return None;
    }
    item.variant_to_unique_name
        .iter()
        .find(|(_, mapped)| mapped.as_str() == unique_name)
        .map(|(variant, _)| variant.clone())
}

/// Collapse (unique name, rank, quantity) rows that share an item and rank.
///
/// A rank-0 instance in `Upgrades` and the unranked `RawUpgrades` stack are
/// the same listing; emitted separately they render as two rows with the same
/// name and url and no way to tell them apart.
pub fn merge_rank_rows(rows: Vec<(String, i64, i64)>) -> Vec<(String, i64, i64)> {
    let mut merged: HashMap<(String, i64), i64> = HashMap::new();
    for (unique_name, rank, quantity) in rows {
        *merged.entry((unique_name, rank)).or_insert(0) += quantity;
    }
    merged
        .into_iter()
        .map(|((unique_name, rank), quantity)| (unique_name, rank, quantity))
        .collect()
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
    // Unpriced rows sort as 0 so they gather at one end rather than
    // interleaving unpredictably.
    let price = |item: &WFInvItemBase| item.properties.get_property_value("price", 0.0f64);
    match column {
        "price" => rows.sort_by(|a, b| {
            let (left, right) = match dir {
                SortDirection::Asc => (price(a), price(b)),
                SortDirection::Desc => (price(b), price(a)),
            };
            left.partial_cmp(&right)
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
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
        let prices = build_price_index(&cache.item_price().get_items()?);

        let mut items: Vec<WFInvItemBase> = Vec::new();
        for (unique_name, quantity) in counts {
            let Ok(tradable) = cache.tradable_item().get_by(&unique_name) else {
                continue;
            };
            let parent_sets = item_set.get_sets_for_member(&unique_name);
            let in_sets: Vec<String> = parent_sets.iter().map(|s| s.set.name.clone()).collect();
            // Names are translated and stock rows store whatever language was
            // active when they were created, so conflicts match on wfm_url.
            let in_set_urls: Vec<String> =
                parent_sets.iter().map(|s| s.set.wfm_url.clone()).collect();

            // A relic's four refinements all resolve to one cache item, so
            // the variant has to be recovered or Intact and Radiant become
            // indistinguishable rows that list as the same product.
            let variant = variant_of(&tradable, &unique_name);
            let mut item = WFInvItemBase {
                id: tradable.wfm_id.clone(),
                name: tradable.name.clone(),
                unique_name,
                wfm_url: tradable.wfm_url.clone(),
                quantity,
                sub_type: variant.map(|variant| SubType {
                    variant: Some(variant),
                    ..Default::default()
                }),
                ..Default::default()
            };
            item.properties.set_property_value("in_sets", in_sets);
            item.properties
                .set_property_value("in_set_urls", in_set_urls);
            item.properties.set_property_value(
                "price",
                lookup_price(&prices, &item.wfm_url, item.sub_type.clone()),
            );
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

        // unique name, rank, quantity. Rivens are excluded from both buckets:
        // the Rivens tab owns them and lists them as a StockRiven, so letting
        // them through here would split one inventory across two incompatible
        // stock representations.
        let mut rows: Vec<(String, i64, i64)> = Vec::new();
        for (unique_name, quantity) in owned_counts_where(&[&root.raw_upgrades], |e| !e.is_riven())
        {
            rows.push((unique_name, 0, quantity));
        }
        for ((unique_name, rank), quantity) in rank_groups(&root.upgrades) {
            rows.push((unique_name, rank, quantity));
        }
        let rows = merge_rank_rows(rows);

        let prices = build_price_index(&cache.item_price().get_items()?);
        let mut items: Vec<WFInvItemBase> = Vec::new();
        for (unique_name, rank, quantity) in rows {
            let Ok(tradable) = cache.tradable_item().get_by(&unique_name) else {
                continue;
            };
            let variant = variant_of(&tradable, &unique_name);
            let mut item = WFInvItemBase {
                id: tradable.wfm_id.clone(),
                name: tradable.name.clone(),
                unique_name,
                wfm_url: tradable.wfm_url.clone(),
                quantity,
                // Unranked rows carry no rank at all, matching how stock
                // represents an unranked item. Some(rank: 0) would make
                // ItemName render a bare "Rank " with no number after it.
                sub_type: if rank > 0 || variant.is_some() {
                    Some(SubType {
                        rank: (rank > 0).then_some(rank),
                        variant,
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
            item.properties.set_property_value(
                "price",
                lookup_price(&prices, &item.wfm_url, item.sub_type.clone()),
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
    use super::{merge_rank_rows, owned_counts, owned_counts_where, rank_groups, variant_of};
    use crate::cache::{CacheTradableItem, SubType as CacheSubType};
    use crate::wf_inventory::WFInvItemRaw;
    use std::collections::HashMap;

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

    fn relic(unique_name: &str, variants: &[(&str, &str)]) -> CacheTradableItem {
        CacheTradableItem {
            name: "Lith B6 Relic".to_string(),
            unique_name: unique_name.to_string(),
            wfm_id: "id".to_string(),
            wfm_url: "lith_b6_relic".to_string(),
            trade_tax: 0,
            mr_requirement: 0,
            tags: vec!["relic".to_string()],
            icon: String::new(),
            bulk_tradable: false,
            sub_type: Some(CacheSubType::default()),
            variant_to_unique_name: variants
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        }
    }

    /// The tradable-items cache registers every variant unique name as a
    /// lookup key, so all four relic refinements resolve to one item. Without
    /// recovering which variant was matched, a Radiant relic lists as a
    /// variant-less item and the scraper prices the wrong product.
    #[test]
    fn recovers_the_variant_a_unique_name_matched_through() {
        let item = relic(
            "/Relic/LithB6Intact",
            &[
                ("intact", "/Relic/LithB6Intact"),
                ("radiant", "/Relic/LithB6Radiant"),
            ],
        );
        assert_eq!(
            variant_of(&item, "/Relic/LithB6Radiant").as_deref(),
            Some("radiant")
        );
    }

    /// Matching the item's own unique name is not a variant match.
    #[test]
    fn reports_no_variant_for_a_direct_unique_name_match() {
        let item = relic("/Relic/LithB6Intact", &[("intact", "/Relic/LithB6Intact")]);
        assert_eq!(variant_of(&item, "/Relic/LithB6Intact"), None);
    }

    #[test]
    fn reports_no_variant_when_the_item_has_none() {
        let mut item = relic("/Part/Barrel", &[]);
        item.variant_to_unique_name = Default::default();
        assert_eq!(variant_of(&item, "/Part/Barrel"), None);
    }

    /// Veiled rivens live in RawUpgrades as well as Upgrades. The Rivens tab
    /// already owns them and lists them as StockRiven; letting them through
    /// here would list the same inventory as a StockItem.
    #[test]
    fn owned_counts_where_can_exclude_rivens() {
        let bucket = vec![
            raw("/Lotus/Upgrades/Mods/Randomized/RawMeleeRandomMod", 37),
            raw("/Mods/Serration", 2),
        ];
        let counts = owned_counts_where(&[&bucket], |e| !e.is_riven());
        assert_eq!(counts.len(), 1);
        assert_eq!(counts.get("/Mods/Serration"), Some(&2));
    }

    /// A rank-0 instance in Upgrades and the unranked RawUpgrades stack are
    /// the same listing. Emitting both gives two rows with the same name, the
    /// same url and no way to tell them apart.
    #[test]
    fn merges_rows_that_share_a_unique_name_and_rank() {
        let rows = vec![
            ("/Mods/Serration".to_string(), 0, 3),
            ("/Mods/Serration".to_string(), 0, 1),
            ("/Mods/Serration".to_string(), 10, 1),
        ];
        let mut merged = merge_rank_rows(rows);
        merged.sort();
        assert_eq!(
            merged,
            vec![
                ("/Mods/Serration".to_string(), 0, 4),
                ("/Mods/Serration".to_string(), 10, 1),
            ]
        );
    }

    #[test]
    fn merge_rank_rows_keeps_distinct_items_apart() {
        let rows = vec![("/Mods/A".to_string(), 0, 1), ("/Mods/B".to_string(), 0, 2)];
        let mut merged = merge_rank_rows(rows);
        merged.sort();
        assert_eq!(merged.len(), 2);
        let _ = HashMap::<String, i64>::new();
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
