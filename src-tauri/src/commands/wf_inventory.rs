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
/// must collapse to the same key.
pub fn stock_key(wfm_url: &str, rank: Option<i64>) -> String {
    format!("{}#{}", wfm_url, rank.unwrap_or(0))
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

    let listed: HashSet<String> = stock
        .results
        .iter()
        .map(|item| stock_key(&item.wfm_url, item.sub_type.as_ref().and_then(|s| s.rank)))
        .collect();
    let listed_sets: HashSet<String> = stock
        .results
        .iter()
        .map(|item| item.item_name.clone())
        .collect();

    let mut parts = wf_inventory.item().get_parts(query)?;
    for item in parts.results.iter_mut() {
        let in_stock = listed.contains(&stock_key(&item.wfm_url, None));
        item.properties.set_property_value("is_in_stock", in_stock);

        // Listing a part separately undercuts a set listing that contains it.
        let conflicting: Vec<String> = item
            .properties
            .get_property_value::<Vec<String>>("in_sets", vec![])
            .into_iter()
            .filter(|set_name| listed_sets.contains(set_name))
            .collect();
        item.properties
            .set_property_value("in_stock_sets", conflicting);
    }
    Ok(json!(parts))
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
            stock_key("serration", None),
            stock_key("serration", Some(0))
        );
    }

    #[test]
    fn keeps_different_ranks_apart() {
        assert_ne!(
            stock_key("serration", Some(0)),
            stock_key("serration", Some(10))
        );
    }

    #[test]
    fn keeps_different_items_apart() {
        assert_ne!(
            stock_key("serration", Some(10)),
            stock_key("vitality", Some(10))
        );
    }
}
