//! Fills the gaps the bundled price cache leaves.
//!
//! `api.quantframe.app` ships warframe.market statistics for about 1390 items,
//! which covers 86% of sets but only 26% of mods. Sampling the rows it misses
//! found roughly 45% worth 10p or more, so a missing price is not the same as
//! a worthless item - and the inventory tabs exist to pick out what is worth
//! listing.
//!
//! This resolves those gaps against warframe.market directly, but only for the
//! rows actually on screen, and remembers the answers. The cost is therefore
//! bounded by what you look at rather than by how big your inventory is, and
//! it never fires a bulk backfill at the endpoint.

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Mutex, OnceLock},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use utils::{info, warning, Error, LoggerOptions};

use crate::helper;

static COMPONENT: &str = "MarketPrices";

/// How long a resolved price stands before being asked again.
pub const FOUND_TTL_SECS: i64 = 60 * 60;

/// How long "warframe.market has no trades for this" stands. Silence does not
/// turn into a price quickly, and re-asking hourly would spend most of the
/// request budget re-confirming it.
pub const MISSING_TTL_SECS: i64 = 24 * 60 * 60;

/// Most a single call will fetch, so one page view cannot turn into a flood.
pub const MAX_PER_CALL: usize = 40;

/// Spacing between requests. warframe.market tolerates a few per second; this
/// stays well inside that while a page's worth still resolves in a couple of
/// seconds.
const REQUEST_SPACING: Duration = Duration::from_millis(150);

/// An item as the market prices it: the same thing can trade at several ranks
/// or refinements, each its own product.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PriceKey {
    pub wfm_url: String,
    pub rank: Option<i64>,
    pub variant: Option<String>,
}

impl PriceKey {
    fn id(&self) -> String {
        format!(
            "{}#{}#{}",
            self.wfm_url,
            self.rank.unwrap_or(0),
            self.variant.as_deref().unwrap_or("")
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct CachedPrice {
    /// None records that warframe.market was asked and had nothing.
    price: Option<f64>,
    fetched_at: i64,
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Whether a cached answer still stands. An entry stamped in the future is
/// treated as stale, so a clock that jumps backwards cannot pin one forever.
pub fn is_fresh(fetched_at: i64, now: i64, found: bool) -> bool {
    if fetched_at > now {
        return false;
    }
    let ttl = if found {
        FOUND_TTL_SECS
    } else {
        MISSING_TTL_SECS
    };
    now - fetched_at < ttl
}

/// The volume-weighted average price across the buckets that describe the item
/// we actually hold.
///
/// warframe.market reports mods per rank and relics per refinement, so a
/// bucket for another rank or refinement is a different product and is not
/// evidence about this one. When nothing matches, the answer is "unknown"
/// rather than another product's price.
pub fn select_price(statistics: &Value, key: &PriceKey) -> Option<f64> {
    let buckets = statistics.as_array()?;
    let wanted_rank = key.rank.unwrap_or(0);

    let mut total_value = 0.0;
    let mut total_volume = 0.0;
    for bucket in buckets {
        match bucket.get("mod_rank").and_then(Value::as_i64) {
            Some(rank) if rank != wanted_rank => continue,
            _ => {}
        }
        match (bucket.get("subtype").and_then(Value::as_str), &key.variant) {
            (Some(subtype), Some(wanted)) if subtype != wanted => continue,
            (Some(_), None) => continue,
            _ => {}
        }
        // A bucket without a usable price is skipped, not fatal: one odd
        // bucket must not discard the rest of the window.
        let Some(price) = bucket.get("avg_price").and_then(Value::as_f64) else {
            continue;
        };
        let volume = bucket.get("volume").and_then(Value::as_f64).unwrap_or(0.0);
        if volume <= 0.0 {
            continue;
        }
        total_value += price * volume;
        total_volume += volume;
    }

    (total_volume > 0.0).then(|| total_value / total_volume)
}

#[derive(Debug, Default)]
pub struct MarketPriceStore {
    entries: Mutex<HashMap<String, CachedPrice>>,
}

impl MarketPriceStore {
    fn path() -> PathBuf {
        // Deliberately beside the app data rather than inside cache/, which is
        // wiped wholesale whenever the shipped cache updates.
        helper::get_app_storage_path().join("market_prices.json")
    }

    fn load() -> Self {
        let path = Self::path();
        let entries = std::fs::read_to_string(&path)
            .ok()
            .and_then(|raw| serde_json::from_str::<HashMap<String, CachedPrice>>(&raw).ok())
            .unwrap_or_default();
        info(
            format!("{}:Load", COMPONENT),
            format!(
                "{} remembered prices from {}",
                entries.len(),
                path.display()
            ),
            &LoggerOptions::default(),
        );
        Self {
            entries: Mutex::new(entries),
        }
    }

    pub fn get() -> &'static MarketPriceStore {
        static STORE: OnceLock<MarketPriceStore> = OnceLock::new();
        STORE.get_or_init(MarketPriceStore::load)
    }

    fn remembered(&self, key: &PriceKey) -> Option<Option<f64>> {
        let entries = self.entries.lock().unwrap();
        let entry = entries.get(&key.id())?;
        is_fresh(entry.fetched_at, now_secs(), entry.price.is_some()).then_some(entry.price)
    }

    /// A price already resolved for this key, without asking the network.
    ///
    /// Lets the row builders filter on prices the bundled cache never had,
    /// once a previous page view has resolved them. Always cache-only: a
    /// query that builds a table must never make HTTP calls.
    pub fn remembered_price(&self, key: &PriceKey) -> Option<f64> {
        self.remembered(key).flatten()
    }

    fn remember(&self, key: &PriceKey, price: Option<f64>) {
        let mut entries = self.entries.lock().unwrap();
        entries.insert(
            key.id(),
            CachedPrice {
                price,
                fetched_at: now_secs(),
            },
        );
    }

    fn save(&self) {
        let path = Self::path();
        let entries = self.entries.lock().unwrap().clone();
        let Ok(raw) = serde_json::to_string(&entries) else {
            return;
        };
        if let Err(e) = std::fs::write(&path, raw) {
            // Losing the file only costs re-fetching later, so this is a
            // warning rather than a failure of the request that triggered it.
            warning(
                format!("{}:Save", COMPONENT),
                format!("Could not write {}: {}", path.display(), e),
                &LoggerOptions::default(),
            );
        }
    }
}

async fn fetch_statistics(wfm_url: &str) -> Result<Value, Error> {
    let client = crate::HTTP_CLIENT.get_or_init(reqwest::Client::new);
    let url = format!("https://api.warframe.market/v1/items/{wfm_url}/statistics");
    let response = client
        .get(&url)
        .header("accept", "application/json")
        .send()
        .await
        .map_err(|e| {
            Error::new(
                format!("{}:Fetch", COMPONENT),
                format!("Request for {wfm_url} failed: {e}"),
                utils::get_location!(),
            )
        })?;
    let body: Value = response.json().await.map_err(|e| {
        Error::new(
            format!("{}:Fetch", COMPONENT),
            format!("Response for {wfm_url} was not JSON: {e}"),
            utils::get_location!(),
        )
    })?;
    Ok(body
        .get("payload")
        .and_then(|p| p.get("statistics_closed"))
        .and_then(|c| c.get("48hours"))
        .cloned()
        .unwrap_or(Value::Array(vec![])))
}

/// Resolve prices for the given keys, using what is remembered and asking
/// warframe.market only for the rest.
///
/// A key that cannot be resolved maps to None, which the caller shows as
/// "unknown" - never as zero, and never as a reason to hide the row.
pub async fn resolve(keys: Vec<PriceKey>) -> HashMap<String, Option<f64>> {
    let store = MarketPriceStore::get();
    let mut resolved: HashMap<String, Option<f64>> = HashMap::new();
    let mut to_fetch: Vec<PriceKey> = Vec::new();

    for key in keys {
        match store.remembered(&key) {
            Some(price) => {
                resolved.insert(key.id(), price);
            }
            None if to_fetch.len() < MAX_PER_CALL => to_fetch.push(key),
            None => {}
        }
    }

    if to_fetch.is_empty() {
        return resolved;
    }

    let wanted = to_fetch.len();
    for key in to_fetch {
        let price = match fetch_statistics(&key.wfm_url).await {
            Ok(statistics) => select_price(&statistics, &key),
            Err(e) => {
                // A failed lookup is not remembered, so it is retried next
                // time rather than being cached as "no price".
                warning(
                    format!("{}:Resolve", COMPONENT),
                    format!("{e}"),
                    &LoggerOptions::default(),
                );
                resolved.insert(key.id(), None);
                continue;
            }
        };
        store.remember(&key, price);
        resolved.insert(key.id(), price);
        tokio::time::sleep(REQUEST_SPACING).await;
    }
    store.save();

    info(
        format!("{}:Resolve", COMPONENT),
        format!("Asked warframe.market for {wanted} prices"),
        &LoggerOptions::default(),
    );
    resolved
}

#[cfg(test)]
mod tests {
    use super::{is_fresh, select_price, PriceKey, FOUND_TTL_SECS};
    use serde_json::json;

    fn key(rank: Option<i64>, variant: Option<&str>) -> PriceKey {
        PriceKey {
            wfm_url: "an_item".to_string(),
            rank,
            variant: variant.map(str::to_string),
        }
    }

    /// Parts and sets carry no discriminator at all, so every bucket counts.
    #[test]
    fn averages_every_bucket_when_there_is_no_discriminator() {
        let stats = json!([
            {"avg_price": 8.0, "volume": 1},
            {"avg_price": 10.0, "volume": 1},
        ]);
        assert_eq!(select_price(&stats, &key(None, None)), Some(9.0));
    }

    /// Volume weighting, so a single outlier trade cannot drag the figure.
    #[test]
    fn weights_each_bucket_by_its_volume() {
        let stats = json!([
            {"avg_price": 10.0, "volume": 9},
            {"avg_price": 100.0, "volume": 1},
        ]);
        assert_eq!(select_price(&stats, &key(None, None)), Some(19.0));
    }

    /// warframe.market reports mod statistics per rank. Metal Fiber's last 48
    /// hours are all rank 10 at 35p; showing that against an unranked copy
    /// would overstate it by an order of magnitude.
    #[test]
    fn only_counts_buckets_matching_the_requested_rank() {
        let stats = json!([
            {"avg_price": 3.0, "volume": 1, "mod_rank": 0},
            {"avg_price": 35.0, "volume": 1, "mod_rank": 10},
        ]);
        assert_eq!(select_price(&stats, &key(None, None)), Some(3.0));
        assert_eq!(select_price(&stats, &key(Some(0), None)), Some(3.0));
        assert_eq!(select_price(&stats, &key(Some(10), None)), Some(35.0));
    }

    /// No data for the rank we hold is not the same as a cheap item. Reporting
    /// nothing keeps the row honestly unknown rather than quoting another
    /// rank's price.
    #[test]
    fn reports_nothing_when_no_bucket_matches_the_rank() {
        let stats = json!([{"avg_price": 35.0, "volume": 1, "mod_rank": 10}]);
        assert_eq!(select_price(&stats, &key(Some(3), None)), None);
    }

    /// Relics are reported per refinement, and an Intact is not a Radiant.
    #[test]
    fn only_counts_buckets_matching_the_requested_variant() {
        let stats = json!([
            {"avg_price": 3.0, "volume": 1, "subtype": "intact"},
            {"avg_price": 45.0, "volume": 1, "subtype": "radiant"},
        ]);
        assert_eq!(select_price(&stats, &key(None, Some("intact"))), Some(3.0));
        assert_eq!(
            select_price(&stats, &key(None, Some("radiant"))),
            Some(45.0)
        );
        assert_eq!(select_price(&stats, &key(None, None)), None);
    }

    #[test]
    fn reports_nothing_for_an_empty_window() {
        assert_eq!(select_price(&json!([]), &key(None, None)), None);
    }

    /// A window whose buckets all report no volume carries no price signal.
    #[test]
    fn reports_nothing_when_every_bucket_has_zero_volume() {
        let stats = json!([{"avg_price": 10.0, "volume": 0}]);
        assert_eq!(select_price(&stats, &key(None, None)), None);
    }

    #[test]
    fn a_recent_entry_is_fresh_and_an_old_one_is_not() {
        assert!(is_fresh(1_000, 1_000 + FOUND_TTL_SECS - 1, true));
        assert!(!is_fresh(1_000, 1_000 + FOUND_TTL_SECS + 1, true));
    }

    /// "Nothing traded" does not become true again quickly, so a known-absent
    /// price is held much longer than a found one. Re-asking every hour would
    /// spend most of the request budget re-confirming silence.
    #[test]
    fn a_known_absent_price_is_held_longer_than_a_found_one() {
        let hours_later = 1_000 + FOUND_TTL_SECS + 1;
        assert!(!is_fresh(1_000, hours_later, true));
        assert!(is_fresh(1_000, hours_later, false));
    }

    /// A clock that jumps backwards must not pin an entry as fresh forever.
    #[test]
    fn treats_an_entry_stamped_in_the_future_as_stale() {
        assert!(!is_fresh(10_000, 1_000, true));
    }
}
