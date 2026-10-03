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

/// `stock_key` for a stock row.
fn stock_key_of(item: &entity::stock_item::Model) -> String {
    let sub_type = item.sub_type.as_ref();
    stock_key(
        &item.wfm_url,
        sub_type.and_then(|s| s.rank),
        sub_type.and_then(|s| s.variant.as_deref()),
    )
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
    let conn = DATABASE.get().unwrap();
    let stock = StockItemQuery::get_all(conn, StockItemPaginationQueryDto::new(1, -1)).await?;

    let listed: HashSet<String> = stock.results.iter().map(stock_key_of).collect();
    // Matched by url, not by item_name: that column stores whatever language
    // was active when the row was created, so a language change would
    // silently stop the warning from ever appearing.
    let listed_urls: HashSet<&str> = stock
        .results
        .iter()
        .map(|item| item.wfm_url.as_str())
        .collect();

    let mut parts = wf_inventory.item().get_parts(query)?;
    for item in parts.results.iter_mut() {
        let variant = item.sub_type.as_ref().and_then(|s| s.variant.clone());
        let in_stock = listed.contains(&stock_key(&item.wfm_url, None, variant.as_deref()));
        item.properties.set_property_value("is_in_stock", in_stock);

        // Listing a part separately undercuts a set listing that contains it.
        let names = item
            .properties
            .get_property_value::<Vec<String>>("in_sets", vec![]);
        let urls = item
            .properties
            .get_property_value::<Vec<String>>("in_set_urls", vec![]);
        let conflicting: Vec<String> = urls
            .iter()
            .enumerate()
            .filter(|(_, url)| listed_urls.contains(url.as_str()))
            .filter_map(|(i, _)| names.get(i).cloned())
            .collect();
        item.properties
            .set_property_value("in_stock_sets", conflicting);
    }
    Ok(json!(parts))
}

#[tauri::command]
pub async fn wf_inventory_get_mods(
    query: WFItemPaginationDto,
    wf_inventory: tauri::State<'_, Mutex<Arc<WFInventoryState>>>,
) -> Result<Value, Error> {
    let wf_inventory = wf_inventory.lock()?.clone();
    let conn = DATABASE.get().unwrap();
    let stock = StockItemQuery::get_all(conn, StockItemPaginationQueryDto::new(1, -1)).await?;
    let listed: HashSet<String> = stock.results.iter().map(stock_key_of).collect();

    let mut mods = wf_inventory.item().get_mods(query)?;
    for item in mods.results.iter_mut() {
        let sub_type = item.sub_type.as_ref();
        let in_stock = listed.contains(&stock_key(
            &item.wfm_url,
            sub_type.and_then(|s| s.rank),
            sub_type.and_then(|s| s.variant.as_deref()),
        ));
        item.properties.set_property_value("is_in_stock", in_stock);
    }
    Ok(json!(mods))
}

#[tauri::command]
pub async fn wf_inventory_get_sets(
    query: WFItemPaginationDto,
    wf_inventory: tauri::State<'_, Mutex<Arc<WFInventoryState>>>,
) -> Result<Value, Error> {
    let wf_inventory = wf_inventory.lock()?.clone();
    let conn = DATABASE.get().unwrap();
    let stock = StockItemQuery::get_all(conn, StockItemPaginationQueryDto::new(1, -1)).await?;
    let listed: HashSet<String> = stock
        .results
        .iter()
        .map(|item| stock_key(&item.wfm_url, None, None))
        .collect();

    let mut sets = wf_inventory.sets().get_sets(query)?;
    for set in sets.results.iter_mut() {
        let in_stock = listed.contains(&stock_key(&set.base.wfm_url, None, None));
        set.base
            .properties
            .set_property_value("is_in_stock", in_stock);
    }
    Ok(json!(sets))
}

/// Resolve prices warframe.market knows but the bundled cache does not.
///
/// Called with the rows currently on screen that have no price, so the cost
/// is bounded by what the user looks at. Answers are remembered, including
/// "nothing traded", so revisiting a page is free.
#[tauri::command]
pub async fn wf_inventory_resolve_prices(
    keys: Vec<crate::market_prices::PriceKey>,
) -> Result<Value, Error> {
    Ok(json!(crate::market_prices::resolve(keys).await))
}

#[cfg(test)]
mod tests {
    use super::stock_key;

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
