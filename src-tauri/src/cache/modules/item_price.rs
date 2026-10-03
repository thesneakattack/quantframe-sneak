use std::collections::HashMap;
use std::{
    fs::File,
    io::Write,
    path::PathBuf,
    sync::{Arc, Mutex, Weak},
};

use crate::{
    cache::{client::CacheState, types::item_price_info::ItemPriceInfo},
    emit_startup,
    utils::ErrorFromExt,
};
use qf_api::Client as QFClient;
use utils::SubType;
use utils::{find_by, get_location, info, read_json_file_optional, Error, LoggerOptions};

#[derive(Debug)]
pub struct ItemPriceModule {
    path: PathBuf,
    items: Mutex<Vec<ItemPriceInfo>>,
    client: Weak<CacheState>,
}

impl ItemPriceModule {
    pub fn new(client: Arc<CacheState>) -> Arc<Self> {
        Arc::new(Self {
            path: client.base_path.join("items/ItemPrices.json"),
            items: Mutex::new(Vec::new()),
            client: Arc::downgrade(&client),
        })
    }
    pub async fn check_update(&self, qf_client: &QFClient) -> Result<(bool, String), Error> {
        let client = self.client.upgrade().expect("Client should not be dropped");
        let current_version = client.version.id_price.clone();
        let remote_version = match qf_client.cache().get_cache_id("item_price").await {
            Ok(id) => id,
            Err(e) => {
                let err = Error::from_qf(
                    "Cache:ItemPrice:CheckUpdate",
                    "Failed to get item price cache ID",
                    e,
                    get_location!(),
                );
                err.log("cache_version.json");
                return Err(err);
            }
        };

        if !self.path.exists() {
            Ok((true, remote_version))
        } else {
            Ok((current_version != remote_version, remote_version))
        }
    }

    pub async fn load(
        &self,
        qf_client: &QFClient,
        price_require_update: bool,
    ) -> Result<(), Error> {
        let _client = self.client.upgrade().expect("Client should not be dropped");
        if price_require_update {
            match self.extract(qf_client).await {
                Ok(()) => {
                    info(
                        "Cache:ItemPrice:Load",
                        "Item price cache extracted successfully.",
                        &LoggerOptions::default(),
                    );
                }
                Err(e) => {
                    e.log("cache_version.json");
                    return Err(e);
                }
            }
        }
        match read_json_file_optional::<Vec<ItemPriceInfo>>(&self.path) {
            Ok(items) => {
                let mut items_lock = self.items.lock().unwrap();
                *items_lock = items;
                info(
                    "Cache:ItemPrice:Load",
                    format!(
                        "Item price cache loaded successfully with {} items.",
                        items_lock.len()
                    ),
                    &LoggerOptions::default(),
                );
            }
            Err(e) => return Err(e.with_location(get_location!())),
        }
        Ok(())
    }
    async fn extract(&self, qf_client: &QFClient) -> Result<(), Error> {
        emit_startup!("cache.item_price_updating", json!({}));
        let content = qf_client
            .cache()
            .download_cache("item_price")
            .await
            .map_err(|e| {
                Error::from_qf(
                    "Cache:ItemPrice",
                    "Failed to download cache",
                    e,
                    get_location!(),
                )
            })?;

        // Create parent directory if it doesn't exist
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                Error::from_io(
                    "Cache:ItemPrice",
                    &parent.to_path_buf(),
                    "Failed to create parent directory",
                    e,
                    get_location!(),
                )
            })?;
        }

        let mut file = File::create(self.path.clone()).map_err(|e| {
            Error::from_io(
                "Cache:ItemPrice",
                &self.path,
                "Failed to create file",
                e,
                get_location!(),
            )
        })?;

        file.write_all(&content).map_err(|e| {
            Error::from_io(
                "Cache:ItemPrice",
                &self.path,
                "Failed to write file",
                e,
                get_location!(),
            )
        })?;
        Ok(())
    }

    pub fn get_items(&self) -> Result<Vec<ItemPriceInfo>, Error> {
        let items = self
            .items
            .lock()
            .expect("Failed to lock items mutex")
            .clone();
        Ok(items)
    }

    pub fn find_by(
        &self,
        url: impl Into<String>,
        sub_type: Option<SubType>,
    ) -> Result<Option<ItemPriceInfo>, Error> {
        let url = url.into();
        let items = self.get_items()?;
        let item = find_by(&items, |u| u.wfm_url == url && u.sub_type == sub_type);
        Ok(item.cloned())
    }
    pub fn find_by_id(
        &self,
        id: impl Into<String>,
        sub_type: Option<SubType>,
    ) -> Result<Option<ItemPriceInfo>, Error> {
        let id = id.into();
        let items = self.get_items()?;
        let item = find_by(&items, |u| u.wfm_id == id && u.sub_type == sub_type);
        Ok(item.cloned())
    }
    pub fn get_by_filter<F>(&self, predicate: F) -> Vec<ItemPriceInfo>
    where
        F: Fn(&ItemPriceInfo) -> bool,
    {
        let items = self.get_items().expect("Failed to get items");
        items
            .into_iter()
            .filter(|item| predicate(item))
            .collect::<Vec<ItemPriceInfo>>()
    }
}

/// The warframe.market moving average per (item, sub type).
///
/// Built once per request from a single `get_items()` call: `find_by` clones
/// all ~1390 entries and scans them linearly, which is fine for one lookup
/// and ruinous for the hundreds a table page needs.
pub type PriceIndex = HashMap<(String, Option<SubType>), f64>;

pub fn build_price_index(items: &[ItemPriceInfo]) -> PriceIndex {
    let mut index = PriceIndex::new();
    for item in items {
        // moving_avg is the figure the live scraper itself prices against
        // (live_scraper/modules/item.rs). An absent one is not a zero.
        if let Some(moving_avg) = item.moving_avg {
            index.insert((item.wfm_url.clone(), item.sub_type.clone()), moving_avg);
        }
    }
    index
}

/// The moving average for an item, preferring an exact sub-type match.
///
/// Most ranks are not priced individually, so a rank with no entry of its own
/// falls back to the plain entry: a number of the right order beats a blank
/// cell. An exact match always wins, which matters for maxed mods - those are
/// a different product at a very different price.
pub fn lookup_price(index: &PriceIndex, wfm_url: &str, sub_type: Option<SubType>) -> Option<f64> {
    if let Some(price) = index.get(&(wfm_url.to_string(), sub_type.clone())) {
        return Some(*price);
    }
    if sub_type.is_some() {
        return index.get(&(wfm_url.to_string(), None)).copied();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{build_price_index, lookup_price};
    use crate::cache::ItemPriceInfo;
    use utils::SubType;

    fn priced(url: &str, sub_type: Option<SubType>, moving_avg: f64) -> ItemPriceInfo {
        ItemPriceInfo {
            wfm_url: url.to_string(),
            sub_type,
            moving_avg: Some(moving_avg),
            ..Default::default()
        }
    }

    fn ranked(rank: i64) -> Option<SubType> {
        Some(SubType {
            rank: Some(rank),
            ..Default::default()
        })
    }

    #[test]
    fn finds_the_price_for_an_item_with_no_sub_type() {
        let index = build_price_index(&[priced("nova_prime_set", None, 72.0)]);
        assert_eq!(lookup_price(&index, "nova_prime_set", None), Some(72.0));
    }

    /// A maxed mod is a different product from an unranked one and is priced
    /// separately: 213 of the cached rows are rank 5, 68 are rank 10.
    #[test]
    fn prefers_an_exact_sub_type_match_over_the_plain_entry() {
        let index = build_price_index(&[
            priced("serration", None, 38.0),
            priced("serration", ranked(10), 120.0),
        ]);
        assert_eq!(lookup_price(&index, "serration", ranked(10)), Some(120.0));
        assert_eq!(lookup_price(&index, "serration", None), Some(38.0));
    }

    /// Most ranks are not priced individually. Falling back to the plain
    /// entry gives a number of the right order rather than a blank cell.
    #[test]
    fn falls_back_to_the_plain_entry_when_the_rank_is_not_priced() {
        let index = build_price_index(&[priced("serration", None, 38.0)]);
        assert_eq!(lookup_price(&index, "serration", ranked(7)), Some(38.0));
    }

    #[test]
    fn reports_nothing_for_an_item_with_no_price_at_all() {
        let index = build_price_index(&[priced("serration", None, 38.0)]);
        assert_eq!(lookup_price(&index, "ammo_drum", None), None);
    }

    /// moving_avg is Option on the type even though the shipped data always
    /// sets it; an absent one must not become a confident zero.
    #[test]
    fn skips_entries_with_no_moving_average() {
        let index = build_price_index(&[ItemPriceInfo {
            wfm_url: "obscure_item".to_string(),
            moving_avg: None,
            ..Default::default()
        }]);
        assert_eq!(lookup_price(&index, "obscure_item", None), None);
    }
}
