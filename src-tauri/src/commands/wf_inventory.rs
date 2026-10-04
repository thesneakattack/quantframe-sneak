use entity::stock_item::StockItemPaginationQueryDto;
use entity::stock_riven::StockRivenPaginationQueryDto;
use qf_api::enums::app_events::ApplicationEvent as EventType;
use serde_json::{json, Value};
use service::{StockItemQuery, StockRivenQuery};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use utils::Error;

use crate::wf_inventory::WFInventoryState;
use crate::wf_inventory::WFItemPaginationDto;
use crate::{track_event, DATABASE};

/// Identity of a stock listing for matching against an inventory row.
///
/// An absent sub_type and an explicit rank 0 both mean "unranked", so they
/// must collapse to the same key. The variant is part of the identity too: a
/// Radiant relic and an Intact one share a wfm_url but are different products.
pub fn stock_key(wfm_url: &str, rank: Option<i64>, variant: Option<&str>) -> String {
    format!(
        "{}#{}#{}",
        wfm_url,
        rank.unwrap_or(0),
        variant.unwrap_or("")
    )
}

/// What the stock section currently lists, in the two shapes the inventory
/// tabs ask about.
///
/// This is the only place the inventory section reads stock. The tabs work on
/// the answers rather than on stock's tables, so how a listing is stored and
/// queried stays stock's business, and a change there lands in one function
/// instead of in every tab's command.
pub struct ListedStock {
    /// Exact listings: url, rank and variant together.
    keys: HashSet<String>,
    /// Urls listed in any form at all.
    urls: HashSet<String>,
}

impl ListedStock {
    fn from_listings<'a>(
        listings: impl IntoIterator<Item = (&'a str, Option<i64>, Option<&'a str>)>,
    ) -> Self {
        let mut keys = HashSet::new();
        let mut urls = HashSet::new();
        for (wfm_url, rank, variant) in listings {
            keys.insert(stock_key(wfm_url, rank, variant));
            urls.insert(wfm_url.to_string());
        }
        Self { keys, urls }
    }

    /// Whether this exact listing is in stock. A Radiant relic and an Intact
    /// one share a url but are different products, so both parts matter.
    pub fn has(&self, wfm_url: &str, rank: Option<i64>, variant: Option<&str>) -> bool {
        self.keys.contains(&stock_key(wfm_url, rank, variant))
    }

    /// Whether anything with this url is in stock, whatever its rank or
    /// variant. A set is listed as itself, so the url is the whole question.
    pub fn has_url(&self, wfm_url: &str) -> bool {
        self.urls.contains(wfm_url)
    }
}

/// Read what stock currently lists.
async fn listed_stock() -> Result<ListedStock, Error> {
    let conn = DATABASE.get().unwrap();
    let stock = StockItemQuery::get_all(conn, StockItemPaginationQueryDto::new(1, -1)).await?;
    Ok(ListedStock::from_listings(stock.results.iter().map(
        |item| {
            let sub_type = item.sub_type.as_ref();
            (
                item.wfm_url.as_str(),
                sub_type.and_then(|s| s.rank),
                sub_type.and_then(|s| s.variant.as_deref()),
            )
        },
    )))
}

#[tauri::command]
pub async fn wf_inventory_get_rivens(
    query: WFItemPaginationDto,
    wf_inventory: tauri::State<'_, Mutex<Arc<WFInventoryState>>>,
) -> Result<Value, Error> {
    let wf_inventory = wf_inventory.lock()?.clone();
    let conn = DATABASE.get().unwrap();
    let rivens = StockRivenQuery::get_all(conn, StockRivenPaginationQueryDto::new(1, -1)).await?;
    let uuids: Vec<String> = rivens.results.iter().map(|r| r.uuid.clone()).collect();
    let mut veiled = wf_inventory.riven().get_rivens(query)?;
    for item in veiled.results.iter_mut() {
        item.base
            .properties
            .set_property_value("is_in_stock", uuids.contains(&item.base.uuid));
    }

    Ok(json!(veiled))
}
#[tauri::command]
pub async fn wf_inventory_get_syndicates(
    query: WFItemPaginationDto,
    wf_inventory: tauri::State<'_, Mutex<Arc<WFInventoryState>>>,
) -> Result<Value, Error> {
    let wf_inventory = wf_inventory.lock()?.clone();
    Ok(json!(&wf_inventory.syndicate().get_syndicates(query)?))
}

#[tauri::command]
pub async fn wf_inventory_update(
    wf_inventory: tauri::State<'_, Mutex<Arc<WFInventoryState>>>,
) -> Result<(), Error> {
    let wf_inventory = wf_inventory.lock()?.clone();
    wf_inventory.update().inspect_err(|_e| {
        track_event!(
            EventType::WFInventoryUpdate,
            [
                ("success", "false".to_string()),
                ("error_type", "update_failed".to_string()),
            ]
        );
    })?;
    track_event!(
        EventType::WFInventoryUpdate,
        [("success", "true".to_string())]
    );
    Ok(())
}

#[tauri::command]
pub async fn wf_inventory_get_parts(
    query: WFItemPaginationDto,
    wf_inventory: tauri::State<'_, Mutex<Arc<WFInventoryState>>>,
) -> Result<Value, Error> {
    let wf_inventory = wf_inventory.lock()?.clone();
    let listed = listed_stock().await?;

    let mut parts = wf_inventory.item().get_parts(query)?;
    for item in parts.results.iter_mut() {
        let variant = item.sub_type.as_ref().and_then(|s| s.variant.clone());
        let in_stock = listed.has(&item.wfm_url, None, variant.as_deref());
        item.properties.set_property_value("is_in_stock", in_stock);

        // Listing a part separately undercuts a set listing that contains it.
        let names = item
            .properties
            .get_property_value::<Vec<String>>("in_sets", vec![]);
        let urls = item
            .properties
            .get_property_value::<Vec<String>>("in_set_urls", vec![]);
        // Matched by url, not by item_name: that column stores whatever
        // language was active when the row was created, so a language change
        // would silently stop the warning from ever appearing.
        let conflicting: Vec<String> = urls
            .iter()
            .enumerate()
            .filter(|(_, url)| listed.has_url(url))
            .filter_map(|(i, _)| names.get(i).cloned())
            .collect();
        item.properties
            .set_property_value("in_stock_sets", conflicting);
    }
    Ok(json!(parts))
}

#[tauri::command]
pub async fn wf_inventory_get_arcanes(
    query: WFItemPaginationDto,
    wf_inventory: tauri::State<'_, Mutex<Arc<WFInventoryState>>>,
) -> Result<Value, Error> {
    let wf_inventory = wf_inventory.lock()?.clone();
    let listed = listed_stock().await?;

    let mut arcanes = wf_inventory.item().get_arcanes(query)?;
    for item in arcanes.results.iter_mut() {
        let sub_type = item.sub_type.as_ref();
        let in_stock = listed.has(
            &item.wfm_url,
            sub_type.and_then(|s| s.rank),
            sub_type.and_then(|s| s.variant.as_deref()),
        );
        item.properties.set_property_value("is_in_stock", in_stock);
    }
    Ok(json!(arcanes))
}

#[tauri::command]
pub async fn wf_inventory_get_mods(
    query: WFItemPaginationDto,
    wf_inventory: tauri::State<'_, Mutex<Arc<WFInventoryState>>>,
) -> Result<Value, Error> {
    let wf_inventory = wf_inventory.lock()?.clone();
    let listed = listed_stock().await?;

    let mut mods = wf_inventory.item().get_mods(query)?;
    for item in mods.results.iter_mut() {
        let sub_type = item.sub_type.as_ref();
        let in_stock = listed.has(
            &item.wfm_url,
            sub_type.and_then(|s| s.rank),
            sub_type.and_then(|s| s.variant.as_deref()),
        );
        item.properties.set_property_value("is_in_stock", in_stock);
    }
    Ok(json!(mods))
}

#[tauri::command]
pub async fn wf_inventory_get_relics(
    query: WFItemPaginationDto,
    wf_inventory: tauri::State<'_, Mutex<Arc<WFInventoryState>>>,
) -> Result<Value, Error> {
    let wf_inventory = wf_inventory.lock()?.clone();
    let listed = listed_stock().await?;

    let mut relics = wf_inventory.relics().get_relics(query)?;
    for relic in relics.results.iter_mut() {
        let in_stock = listed.has_url(&relic.base.wfm_url);
        relic
            .base
            .properties
            .set_property_value("is_in_stock", in_stock);
    }
    Ok(json!(relics))
}

#[tauri::command]
pub async fn wf_inventory_get_sets(
    query: WFItemPaginationDto,
    wf_inventory: tauri::State<'_, Mutex<Arc<WFInventoryState>>>,
) -> Result<Value, Error> {
    let wf_inventory = wf_inventory.lock()?.clone();
    let listed = listed_stock().await?;

    let mut sets = wf_inventory.sets().get_sets(query)?;
    for set in sets.results.iter_mut() {
        let in_stock = listed.has_url(&set.base.wfm_url);
        set.base
            .properties
            .set_property_value("is_in_stock", in_stock);
    }
    Ok(json!(sets))
}

#[cfg(test)]
mod tests {
    use super::{stock_key, ListedStock};

    /// The one seam between the inventory tabs and stock has to answer both
    /// questions the tabs ask: "is this exact listing in stock" for a row,
    /// and "is anything with this url in stock" for a set.
    #[test]
    fn answers_exact_and_url_wide_questions() {
        let listed = ListedStock::from_listings([
            ("serration", Some(10), None),
            ("axi_a17_relic", None, Some("intact")),
        ]);

        assert!(listed.has("serration", Some(10), None));
        assert!(!listed.has("serration", Some(9), None));
        assert!(listed.has_url("serration"));

        // A Radiant relic is a different product from the Intact one that is
        // listed, but the set-level question is only about the url.
        assert!(!listed.has("axi_a17_relic", None, Some("radiant")));
        assert!(listed.has("axi_a17_relic", None, Some("intact")));
        assert!(listed.has_url("axi_a17_relic"));

        assert!(!listed.has_url("mag_prime_set"));
    }

    /// Unranked is unranked however it was saved, through the seam too.
    #[test]
    fn an_unranked_listing_matches_an_unranked_row() {
        let listed = ListedStock::from_listings([("serration", Some(0), None)]);
        assert!(listed.has("serration", None, None));
    }

    /// A stock item saved without a sub_type and one saved with rank 0 are the
    /// same unranked item. Treating them differently shows a red "not in
    /// stock" badge on something that is already listed.
    #[test]
    fn treats_no_sub_type_and_rank_zero_as_the_same_item() {
        assert_eq!(
            stock_key("serration", None, None),
            stock_key("serration", Some(0), None)
        );
    }

    #[test]
    fn keeps_different_ranks_apart() {
        assert_ne!(
            stock_key("serration", Some(0), None),
            stock_key("serration", Some(10), None)
        );
    }

    #[test]
    fn keeps_different_items_apart() {
        assert_ne!(
            stock_key("serration", Some(10), None),
            stock_key("vitality", Some(10), None)
        );
    }

    /// A Radiant relic and an Intact one share a wfm_url. Keying on the url
    /// alone lights the "already in stock" badge on the wrong row.
    #[test]
    fn keeps_different_variants_apart() {
        assert_ne!(
            stock_key("lith_b6_relic", None, Some("radiant")),
            stock_key("lith_b6_relic", None, Some("intact"))
        );
    }
}
