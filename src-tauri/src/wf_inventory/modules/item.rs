use std::collections::HashMap;
use std::sync::{Arc, Weak};

use entity::{dto::PaginatedResult, enums::FieldChange};
use utils::Error;

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

/// Whether a row clears the minimum-price filter.
///
/// The price column exists to pick out what is worth listing, so the filter
/// hides the cheap. An **unknown** price is never treated as cheap: the
/// bundled price data covers only part of the inventory, and roughly 45% of
/// the rows it misses turn out to be worth 10p or more. Hiding those would
/// bury exactly the items the filter is meant to surface.
pub fn meets_min_price(price: Option<f64>, min_price: Option<f64>) -> bool {
    match (price, min_price) {
        (_, None) => true,
        (_, Some(min)) if min <= 0.0 => true,
        (None, Some(_)) => true,
        (Some(price), Some(min)) => price >= min,
    }
}

/// Whether a row's name matches the free-text search box.
///
/// The box matches case-insensitively, anywhere in the name: item names are
/// title case and translated, so a user typing "prime" would otherwise miss
/// every Prime row, and typing a word from the middle of a name is the normal
/// way to narrow a tab. An empty query matches everything, which is what makes
/// clearing the box restore the full table instead of emptying it.
pub fn matches_query(name: &str, query: &str) -> bool {
    name.to_lowercase().contains(&query.to_lowercase())
}

/// Whether a mod row passes the rank toggle.
///
/// An unranked row carries no rank at all rather than `Some(0)` - that is how
/// stock represents an unranked item - so "unranked" has to accept both or
/// half of them disappear from the tab. Anything else, including the UI's
/// default "all" and the empty string a query with no properties leaves
/// behind, is not a filter: an unrecognised value must never empty the table.
pub fn passes_rank_filter(rank: Option<i64>, filter: &str) -> bool {
    match filter {
        "unranked" => rank.unwrap_or(0) == 0,
        "ranked" => rank.unwrap_or(0) > 0,
        _ => true,
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

    /// Tradable parts: the Recipes and MiscItems buckets.
    pub fn get_parts(
        &self,
        query: WFItemPaginationDto,
    ) -> Result<PaginatedResult<WFInvItemBase>, Error> {
        let rows = self.client.upgrade().unwrap().rows()?;
        let mut items = priced_rows(&rows.parts)?;

        let in_set_only = flag(&query, "in_set_only");
        items.retain(|item| {
            !in_set_only
                || !item
                    .properties
                    .get_property_value::<Vec<String>>("in_sets", vec![])
                    .is_empty()
        });
        apply_common_filters(&mut items, &query);
        sort_by_fields(&mut items, &query.sort_fields(), row_value);
        Ok(paginate(
            &items,
            query.pagination.page,
            query.pagination.limit,
        ))
    }

    /// Tradable mods and arcanes, unranked stacks and ranked instances alike.
    pub fn get_mods(
        &self,
        query: WFItemPaginationDto,
    ) -> Result<PaginatedResult<WFInvItemBase>, Error> {
        let rows = self.client.upgrade().unwrap().rows()?;
        let mut items = priced_rows(&rows.mods)?;

        let rank_filter = match &query.properties {
            FieldChange::Value(properties) => {
                properties.get_property_value("rank_filter", String::new())
            }
            _ => String::new(),
        };
        items.retain(|item| {
            passes_rank_filter(item.sub_type.as_ref().and_then(|s| s.rank), &rank_filter)
        });
        apply_common_filters(&mut items, &query);
        sort_by_fields(&mut items, &query.sort_fields(), row_value);
        Ok(paginate(
            &items,
            query.pagination.page,
            query.pagination.limit,
        ))
    }
}

/// A boolean toggle from the query's property bag.
pub fn flag(query: &WFItemPaginationDto, name: &str) -> bool {
    match &query.properties {
        FieldChange::Value(properties) => properties.get_property_value(name, false),
        _ => false,
    }
}

/// A numeric threshold from the query's property bag.
pub fn threshold(query: &WFItemPaginationDto, name: &str) -> i64 {
    match &query.properties {
        FieldChange::Value(properties) => properties.get_property_value(name, 0i64),
        _ => 0,
    }
}

/// Whether a row holds at least as many as the threshold asks for.
///
/// The point of the owned column is finding duplicates worth selling, so the
/// useful question is "do I have a spare", not "how many exactly". A
/// threshold of zero or less is no filter.
pub fn meets_min_owned(quantity: i64, min_owned: i64) -> bool {
    min_owned <= 0 || quantity >= min_owned
}

/// The filters every tab shares: free text, minimum price, minimum owned,
/// and the two "worth my attention" flags.
pub fn apply_common_filters(items: &mut Vec<WFInvItemBase>, query: &WFItemPaginationDto) {
    if let FieldChange::Value(text) = &query.query {
        items.retain(|item| matches_query(&item.name, text));
    }
    let min_price = threshold(query, "min_price");
    items.retain(|item| {
        meets_min_price(
            item.properties
                .get_property_value::<Option<f64>>("price", None),
            Some(min_price as f64),
        )
    });
    let min_owned = threshold(query, "min_owned");
    items.retain(|item| meets_min_owned(item.quantity, min_owned));
    if flag(query, "unvaulted_only") {
        items.retain(|item| item.properties.get_property_value("is_unvaulted", false));
    }
    if flag(query, "mastered_only") {
        items.retain(|item| item.properties.get_property_value("is_mastered", false));
    }
}

/// Copy the snapshot rows and stamp each with the price known right now.
///
/// Prices are deliberately not baked into the snapshot: the live scraper
/// records new ones as it trades, and they must show without rebuilding
/// every row. Both lookups are prepared once here rather than per row.
pub fn priced_rows(rows: &[WFInvItemBase]) -> Result<Vec<WFInvItemBase>, Error> {
    let cache = states::cache_client()?;
    let statistics = build_price_index(&cache.item_price().get_items()?);
    let live = crate::market_prices::MarketPriceStore::get().all_prices();

    Ok(rows
        .iter()
        .map(|row| {
            let mut row = row.clone();
            let key = crate::market_prices::PriceKey {
                wfm_url: row.wfm_url.clone(),
                rank: row.sub_type.as_ref().and_then(|s| s.rank),
                variant: row.sub_type.as_ref().and_then(|s| s.variant.clone()),
            };
            let price = lookup_price(&statistics, &row.wfm_url, row.sub_type.clone())
                .or_else(|| live.get(&key.id()).copied());
            row.properties.set_property_value("price", price);
            row
        })
        .collect())
}

/// A Parts or Mods row's value for a sortable column.
fn row_value(item: &WFInvItemBase, column: &str) -> SortValue {
    match column {
        "quantity" => SortValue::Num(item.quantity as f64),
        "rank" => SortValue::Num(item.sub_type.as_ref().and_then(|s| s.rank).unwrap_or(0) as f64),
        "price" => SortValue::MaybeNum(item.properties.get_property_value("price", None)),
        _ => SortValue::Text(item.name.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        matches_query, meets_min_owned, meets_min_price, merge_rank_rows, owned_counts,
        owned_counts_where, passes_rank_filter, rank_groups, variant_of,
    };
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

    /// The owned column exists to find spares worth selling, so the filter
    /// asks "at least this many", not "exactly this many".
    #[test]
    fn the_owned_threshold_keeps_anything_at_or_above_it() {
        assert!(meets_min_owned(2, 2));
        assert!(meets_min_owned(9, 2));
        assert!(!meets_min_owned(1, 2));
    }

    /// Zero or less means the filter is off, not that worthless rows are
    /// wanted.
    #[test]
    fn an_owned_threshold_of_zero_or_less_is_no_filter() {
        assert!(meets_min_owned(0, 0));
        assert!(meets_min_owned(1, -5));
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

    /// The price column exists to pick out what is worth listing, so the
    /// filter keeps anything at or above the threshold.
    #[test]
    fn keeps_rows_at_or_above_the_threshold() {
        assert!(meets_min_price(Some(10.0), Some(10.0)));
        assert!(meets_min_price(Some(50.0), Some(10.0)));
        assert!(!meets_min_price(Some(9.0), Some(10.0)));
    }

    /// An unknown price is not a cheap one. Roughly 45% of the rows with no
    /// bundled price turn out to be worth 10p or more, so hiding them would
    /// bury exactly the items the filter is meant to surface.
    #[test]
    fn never_hides_a_row_whose_price_is_unknown() {
        assert!(meets_min_price(None, Some(10.0)));
        assert!(meets_min_price(None, Some(1000.0)));
    }

    #[test]
    fn keeps_everything_when_no_threshold_is_set() {
        assert!(meets_min_price(Some(1.0), None));
        assert!(meets_min_price(None, None));
    }

    /// A zero or negative threshold means "no filter" rather than an
    /// expression of interest in worthless items.
    #[test]
    fn treats_a_zero_threshold_as_no_filter() {
        assert!(meets_min_price(Some(0.0), Some(0.0)));
        assert!(meets_min_price(Some(1.0), Some(0.0)));
    }

    /// Item names are title case while nobody types them that way, so a
    /// case-sensitive search box would find nothing for most of what is typed.
    #[test]
    fn matches_a_name_whatever_the_case_of_the_query() {
        assert!(matches_query("Bo Prime Blueprint", "prime"));
        assert!(matches_query("Bo Prime Blueprint", "PRIME"));
        assert!(matches_query("bo prime blueprint", "Prime"));
    }

    /// Searching for "prime" has to find every Prime, not just the rows whose
    /// name starts with it, so the match runs anywhere in the name.
    #[test]
    fn matches_a_substring_from_anywhere_in_the_name() {
        assert!(matches_query("Bo Prime Blueprint", "Blueprint"));
        assert!(matches_query("Bo Prime Blueprint", "o Pri"));
    }

    /// Clearing the search box must restore the whole table. An empty query
    /// that filtered everything out would leave the tab looking broken.
    #[test]
    fn keeps_every_row_for_an_empty_query() {
        assert!(matches_query("Bo Prime Blueprint", ""));
        assert!(matches_query("", ""));
    }

    /// A typo should empty the table rather than quietly fall back to showing
    /// everything; that is how the user sees there is nothing to find.
    #[test]
    fn rejects_a_query_that_appears_nowhere_in_the_name() {
        assert!(!matches_query("Bo Prime Blueprint", "zephyr"));
        assert!(!matches_query("", "prime"));
    }

    /// Unranked rows carry no rank at all rather than Some(0) - that is how
    /// stock represents an unranked item - so the toggle has to accept both
    /// shapes or half the unranked mods vanish from the tab.
    #[test]
    fn the_unranked_filter_keeps_rows_with_no_rank_and_rank_zero() {
        assert!(passes_rank_filter(None, "unranked"));
        assert!(passes_rank_filter(Some(0), "unranked"));
        assert!(!passes_rank_filter(Some(10), "unranked"));
    }

    /// The ranked view exists to find maxed copies worth listing separately,
    /// so a rank-0 row - however it is spelled - is not one of them.
    #[test]
    fn the_ranked_filter_keeps_only_rows_above_rank_zero() {
        assert!(passes_rank_filter(Some(10), "ranked"));
        assert!(passes_rank_filter(Some(1), "ranked"));
        assert!(!passes_rank_filter(Some(0), "ranked"));
        assert!(!passes_rank_filter(None, "ranked"));
    }

    /// A query that sets no properties leaves the filter empty, and the UI's
    /// default sends "all". Neither is a filter, and an unrecognised value
    /// must never empty the table either.
    #[test]
    fn an_unset_or_unrecognised_rank_filter_keeps_everything() {
        for filter in ["", "all", "Unranked", "something_else"] {
            assert!(passes_rank_filter(None, filter), "filter {filter:?}");
            assert!(passes_rank_filter(Some(0), filter), "filter {filter:?}");
            assert!(passes_rank_filter(Some(10), filter), "filter {filter:?}");
        }
    }
}

#[cfg(test)]
mod sort_tests {
    use super::row_value;
    use crate::wf_inventory::{item_base::WFInvItemBase, sort_by_fields, SortField};
    use utils::{SortDirection, SubType};

    fn row(name: &str, quantity: i64, rank: i64, price: Option<f64>) -> WFInvItemBase {
        let mut item = WFInvItemBase {
            name: name.to_string(),
            quantity,
            sub_type: Some(SubType {
                rank: Some(rank),
                ..Default::default()
            }),
            ..Default::default()
        };
        item.properties.set_property_value("price", price);
        item
    }

    fn names(rows: &[WFInvItemBase]) -> Vec<&str> {
        rows.iter().map(|r| r.name.as_str()).collect()
    }

    fn field(by: &str, direction: SortDirection) -> SortField {
        SortField {
            by: by.to_string(),
            direction,
        }
    }

    /// Each column name the table offers has to reach the right field of the
    /// row, or the header sorts by something else entirely.
    #[test]
    fn every_sortable_column_maps_to_its_own_field() {
        let mut rows = vec![
            row("Bravo", 1, 10, Some(5.0)),
            row("Alpha", 9, 0, Some(50.0)),
        ];

        sort_by_fields(
            &mut rows,
            &[field("quantity", SortDirection::Desc)],
            row_value,
        );
        assert_eq!(names(&rows), vec!["Alpha", "Bravo"]);

        sort_by_fields(&mut rows, &[field("rank", SortDirection::Desc)], row_value);
        assert_eq!(names(&rows), vec!["Bravo", "Alpha"]);

        sort_by_fields(&mut rows, &[field("price", SortDirection::Desc)], row_value);
        assert_eq!(names(&rows), vec!["Alpha", "Bravo"]);

        sort_by_fields(&mut rows, &[field("name", SortDirection::Asc)], row_value);
        assert_eq!(names(&rows), vec!["Alpha", "Bravo"]);
    }

    /// Nearly every mod is owned once, so a sort on "owned" ties constantly.
    /// Those ties must settle by name rather than by projection order.
    #[test]
    fn rows_tied_on_the_sorted_column_fall_back_to_the_name() {
        let mut rows = vec![
            row("Zephyr", 1, 0, None),
            row("Ash", 1, 0, None),
            row("Mag", 1, 0, None),
        ];
        sort_by_fields(
            &mut rows,
            &[field("quantity", SortDirection::Asc)],
            row_value,
        );
        assert_eq!(names(&rows), vec!["Ash", "Mag", "Zephyr"]);
    }

    /// An unresolved price sorts last either way, so "dearest first" does not
    /// open with a screen of unknowns.
    #[test]
    fn an_unresolved_price_sorts_last_in_both_directions() {
        let mut rows = vec![
            row("Unknown", 1, 0, None),
            row("Cheap", 1, 0, Some(2.0)),
            row("Dear", 1, 0, Some(90.0)),
        ];
        sort_by_fields(&mut rows, &[field("price", SortDirection::Desc)], row_value);
        assert_eq!(names(&rows), vec!["Dear", "Cheap", "Unknown"]);

        sort_by_fields(&mut rows, &[field("price", SortDirection::Asc)], row_value);
        assert_eq!(names(&rows), vec!["Cheap", "Dear", "Unknown"]);
    }
}
