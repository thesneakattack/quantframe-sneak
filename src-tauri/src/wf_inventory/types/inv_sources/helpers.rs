use crate::wf_inventory::WarframeRootObject;
use serde_json::Value;
use utils::*;

pub fn block_on_async<F: std::future::Future>(future: F) -> F::Output {
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        tokio::task::block_in_place(|| handle.block_on(future))
    } else {
        tokio::runtime::Runtime::new().unwrap().block_on(future)
    }
}

pub fn parse_lastdata(raw: &str) -> Result<WarframeRootObject, Error> {
    let mut json = raw.trim_end_matches(|c| c != '}').to_string();

    let json_value = serde_json::from_str::<Value>(&json).map_err(|e| Error::from(e))?;
    if json_value.get("InventoryJson").is_some() {
        info(
            "WFInventory:ParseLastData",
            "Detected InventoryJson field, extracting inventory data",
            &LoggerOptions::default(),
        );
        json = json_value["InventoryJson"]
            .to_string()
            .replace("\\\"", "\"")
            .replace("\\\"", "\"");
        json = json[1..json.len() - 1].to_string();
    }
    let mut value: Value = serde_json::from_str(&json).map_err(|e| Error::from(e))?;
    if value.get("Results").is_some() && value.get("Stats").is_some() {
        info(
            "WFInventory:ParseLastData",
            "Detected Results and Stats fields, merging them into a single object",
            &LoggerOptions::default(),
        );
        let result = value["Results"].get(0).cloned().unwrap_or_default();
        let stats = value["Stats"].clone();
        if result.is_object() && stats.is_object() {
            let mut merged = result.as_object().unwrap().clone();
            for (key, val) in stats.as_object().unwrap() {
                merged.insert(key.clone(), val.clone());
            }
            value = Value::Object(merged);
        }
    }
    serde_json::from_value(value).map_err(|e| Error::from(e))
}
