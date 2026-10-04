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
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use utils::{info, warning, Error, LoggerOptions};

use crate::helper;

static COMPONENT: &str = "MarketPrices";

/// How long a resolved price stands before being asked again.
///
/// Daily, not hourly. The figure describes a 48 hour window, so re-reading it
/// every hour re-confirms what it already said, and that cadence was
/// self-defeating: refreshing ~900 known prices every hour consumed the entire
/// request budget, leaving rows with no price at all queued behind it
/// indefinitely. A day keeps the figure well inside the window it describes
/// and frees the budget to finish the dataset.
pub const FOUND_TTL_SECS: i64 = 24 * 60 * 60;

/// How long "warframe.market has no trades for this" stands. Silence does not
/// turn into a price quickly, and re-asking would spend the request budget
/// re-confirming it. Deliberately longer than `FOUND_TTL_SECS`: an item with
/// no trades in a month is less likely to have moved than one that trades
/// daily.
pub const MISSING_TTL_SECS: i64 = 30 * 24 * 60 * 60;

/// An item as the market prices it: the same thing can trade at several ranks
/// or refinements, each its own product.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PriceKey {
    pub wfm_url: String,
    pub rank: Option<i64>,
    pub variant: Option<String>,
}

impl PriceKey {
    /// The storage key for this product.
    ///
    /// Built only from what the market uses to identify the thing being
    /// traded. Nothing about owning it takes part: no inventory id, no
    /// `uniqueName`, no owned count. A price is a fact about the item, so it
    /// has to outlive the inventory that happened to ask for it - selling your
    /// last copy, re-importing a profile or wiping the inventory entirely must
    /// leave the figure intact and reusable.
    /// This product expressed as a `SubType`, for the lookups that key on one.
    ///
    /// Faithful for inventory rows because they only ever carry a rank and a
    /// refinement; a `SubType` built elsewhere with star counts would not round
    /// trip through a `PriceKey`.
    pub fn as_sub_type(&self) -> Option<utils::SubType> {
        (self.rank.is_some() || self.variant.is_some()).then(|| utils::SubType {
            rank: self.rank,
            variant: self.variant.clone(),
            ..Default::default()
        })
    }

    pub fn id(&self) -> String {
        format!(
            "{}#{}#{}",
            self.wfm_url,
            self.rank.unwrap_or(0),
            self.variant.as_deref().unwrap_or("")
        )
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CachedPrice {
    /// The 48 hour volume-weighted average. None records that
    /// warframe.market was asked and had nothing.
    #[serde(default)]
    pub price: Option<f64>,
    #[serde(default)]
    pub fetched_at: i64,
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

/// One named window of buckets from a statistics response.
///
/// The response carries both `48hours` and `90days`. A window it does not
/// carry yields an empty one rather than failing the tick that asked for it.
pub fn window_from_payload(body: &Value, window: &str) -> Value {
    body.get("payload")
        .and_then(|p| p.get("statistics_closed"))
        .and_then(|c| c.get(window))
        .cloned()
        .unwrap_or(Value::Array(vec![]))
}

/// The price for a key, taken from the freshest window that actually has
/// trades.
///
/// A 48 hour average says what an item is going for now, so it wins whenever
/// it exists. But most of the inventory does not trade every two days: over a
/// sample of 25 rows carrying no price, the 48 hour window could price 3 where
/// the 90 day window could price 21. Falling through is the difference between
/// a figure and a permanent "unknown" for mods and relics. When neither window
/// has trades for the rank or refinement we hold, the answer stays unknown
/// rather than borrowing another product's price.
pub fn price_from_payload(body: &Value, key: &PriceKey) -> Option<f64> {
    select_price(&window_from_payload(body, "48hours"), key)
        .or_else(|| select_price(&window_from_payload(body, "90days"), key))
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

    let mut samples: Vec<(f64, f64)> = Vec::new();
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
        samples.push((price, volume));
    }

    let ceiling = outlier_ceiling(&samples)?;
    let (total_value, total_volume) = samples
        .iter()
        .filter(|(price, _)| *price <= ceiling)
        .fold((0.0, 0.0), |(value, volume), (p, v)| {
            (value + p * v, volume + v)
        });

    (total_volume > 0.0).then(|| total_value / total_volume)
}

/// Above this multiple of the window's median, a price is noise rather than
/// market. Deliberately generous: an item that genuinely trades over a wide
/// range must not be clipped, and only figures orders of magnitude away are
/// being excluded.
const OUTLIER_FACTOR: f64 = 20.0;

/// The highest price still treated as a real trade.
///
/// warframe.market carries joke listings, and volume weighting is no defence
/// against them in a thin market: a 90 day window for a 5p mod may hold ten
/// trades, so one "69420" sale among them is a tenth of the weight and prices
/// the mod at 4962p. The median is unmoved by a single absurd entry, so it is
/// what the ceiling is measured from.
///
/// With only two samples the median sits between them and nothing can be
/// identified as the outlier - which is correct, because with two points there
/// is no way to tell a joke from a real move.
fn outlier_ceiling(samples: &[(f64, f64)]) -> Option<f64> {
    if samples.is_empty() {
        return None;
    }
    let mut prices: Vec<f64> = samples.iter().map(|(price, _)| *price).collect();
    prices.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let middle = prices.len() / 2;
    let median = if prices.len().is_multiple_of(2) {
        (prices[middle - 1] + prices[middle]) / 2.0
    } else {
        prices[middle]
    };
    // A median of zero would make the ceiling zero and discard everything.
    if median <= 0.0 {
        return Some(f64::INFINITY);
    }
    Some(median * OUTLIER_FACTOR)
}

/// Choose what to refresh next, most valuable first.
///
/// A hundred-platinum set drifting costs a bad decision; a two-platinum mod
/// drifting costs nothing, so value leads and staleness breaks ties. Items
/// with no price at all cannot be ranked by value and queue behind the rest.
/// They are still queued, and that is what completes the dataset: a refreshed
/// item goes fresh and leaves the queue, so the backlog drains and the
/// unknowns are reached rather than starved.
///
/// Fresh entries are skipped entirely - the point is to spend the request
/// budget only where the figure has actually aged.
pub fn plan_backfill(
    candidates: Vec<(PriceKey, Option<f64>)>,
    known: &HashMap<String, CachedPrice>,
    now: i64,
    limit: usize,
) -> Vec<PriceKey> {
    let mut due: Vec<(bool, i64, i64, PriceKey)> = candidates
        .into_iter()
        .filter_map(|(key, value)| {
            let fetched_at = match known.get(&key.id()) {
                Some(entry) => {
                    if is_fresh(entry.fetched_at, now, entry.price.is_some()) {
                        return None;
                    }
                    entry.fetched_at
                }
                // Never asked: as overdue as anything can be.
                None => 0,
            };
            // Negated and scaled so a plain ascending sort puts the dearest
            // first; platinum prices never need sub-unit precision here.
            let rank = value.map(|v| -(v * 1000.0) as i64).unwrap_or(0);
            Some((value.is_none(), rank, fetched_at, key))
        })
        .collect();

    due.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
    due.into_iter()
        .take(limit)
        .map(|(_, _, _, key)| key)
        .collect()
}

/// Fold a bundled seed into what this install already knows.
///
/// A fresh install otherwise spends its first quarter of an hour rediscovering
/// prices that were already known when the build was made. The seed covers
/// every tradable item rather than any one player's inventory, so what ships
/// says nothing about who built it.
///
/// Whichever entry was fetched more recently wins. That is the only rule worth
/// having: a price this install fetched itself is better than one baked into
/// the build months ago, and a store left untouched for months is worse than a
/// seed from last week. Nothing here decides freshness - `is_fresh` still does
/// that, so a seeded entry past its TTL is simply due and gets refreshed like
/// any other.
pub fn merge_seed(known: &mut HashMap<String, CachedPrice>, seed: HashMap<String, CachedPrice>) {
    for (id, entry) in seed {
        match known.get(&id) {
            Some(existing) if existing.fetched_at >= entry.fetched_at => continue,
            _ => {
                known.insert(id, entry);
            }
        }
    }
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

    /// The seed shipped with the build.
    ///
    /// Bundled rather than fetched so a first run is useful offline and
    /// immediately, and resolved through the app handle because a packaged
    /// build keeps its resources beside the executable rather than in app data.
    fn seed_path() -> Option<PathBuf> {
        use tauri::{path::BaseDirectory, Manager};
        crate::APP
            .get()?
            .path()
            .resolve("resources/seed_prices.json", BaseDirectory::Resource)
            .ok()
    }

    fn load() -> Self {
        let path = Self::path();
        let mut entries = std::fs::read_to_string(&path)
            .ok()
            .and_then(|raw| serde_json::from_str::<HashMap<String, CachedPrice>>(&raw).ok())
            .unwrap_or_default();

        // A missing or unreadable seed is not a failure: it only means this
        // install discovers prices the slow way, which is what it did before
        // the seed existed.
        let seeded = Self::seed_path()
            .and_then(|seed| std::fs::read_to_string(seed).ok())
            .and_then(|raw| serde_json::from_str::<HashMap<String, CachedPrice>>(&raw).ok());
        if let Some(seed) = seeded {
            let before = entries.len();
            merge_seed(&mut entries, seed);
            info(
                format!("{}:Seed", COMPONENT),
                format!("Seed added {} prices", entries.len() - before),
                &LoggerOptions::default(),
            );
        }
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

    /// Every remembered price, keyed by `PriceKey::id`.
    ///
    /// Taken once per request so stamping a few thousand rows costs one lock
    /// rather than one per row.
    pub fn all_prices(&self) -> HashMap<String, f64> {
        self.entries
            .lock()
            .unwrap()
            .iter()
            .filter_map(|(id, entry)| entry.price.map(|price| (id.clone(), price)))
            .collect()
    }

    /// What to refresh next, most valuable first.
    pub fn plan_next_batch(
        &self,
        candidates: Vec<(PriceKey, Option<f64>)>,
        limit: usize,
    ) -> Vec<PriceKey> {
        let entries = self.entries.lock().unwrap();
        plan_backfill(candidates, &entries, now_secs(), limit)
    }

    /// Ask warframe.market about one key and remember the answer.
    pub async fn refresh(&self, key: &PriceKey) -> Option<f64> {
        fetch_and_remember(self, key).await
    }

    /// Write the batch just fetched to disk, so prices survive a restart.
    pub fn flush(&self) {
        self.save();
    }

    fn remember(&self, key: &PriceKey, price: Option<f64>) {
        let mut entries = self.entries.lock().unwrap();
        let entry = entries.entry(key.id()).or_default();
        entry.price = price;
        entry.fetched_at = now_secs();
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
    Ok(body)
}

/// Ask warframe.market about one key and remember the answer.
///
/// A request that fails is not remembered, so it is retried next time rather
/// than being cached as "no price".
async fn fetch_and_remember(store: &MarketPriceStore, key: &PriceKey) -> Option<f64> {
    match fetch_statistics(&key.wfm_url).await {
        Ok(statistics) => {
            let price = price_from_payload(&statistics, key);
            store.remember(key, price);
            price
        }
        Err(e) => {
            warning(
                format!("{}:Resolve", COMPONENT),
                format!("{e}"),
                &LoggerOptions::default(),
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        is_fresh, merge_seed, plan_backfill, price_from_payload, select_price, window_from_payload,
        CachedPrice, PriceKey, FOUND_TTL_SECS,
    };
    use serde_json::json;
    use std::collections::HashMap;

    fn key(rank: Option<i64>, variant: Option<&str>) -> PriceKey {
        PriceKey {
            wfm_url: "an_item".to_string(),
            rank,
            variant: variant.map(str::to_string),
        }
    }

    /// The key is the market's identity for the product and nothing else.
    ///
    /// Keying on anything the inventory owns - an id, a `uniqueName`, an owned
    /// count - would tie a price to a particular sighting of the item, so
    /// updating or clearing the inventory would orphan every figure already
    /// paid for. Rank and refinement are included because the market prices
    /// those separately; they describe the product, not the owning.
    #[test]
    fn keys_a_price_on_market_identity_alone() {
        assert_eq!(
            PriceKey {
                wfm_url: "lith_n15_relic".to_string(),
                rank: None,
                variant: Some("intact".to_string()),
            }
            .id(),
            "lith_n15_relic#0#intact"
        );
        assert_eq!(
            PriceKey {
                wfm_url: "arcane_grace".to_string(),
                rank: Some(5),
                variant: None,
            }
            .id(),
            "arcane_grace#5#"
        );
    }

    /// An unranked item and one explicitly at rank 0 are the same product, and
    /// must not end up with two entries that each pay for their own request.
    #[test]
    fn treats_an_absent_rank_and_rank_zero_as_one_product() {
        let absent = PriceKey {
            wfm_url: "an_item".to_string(),
            rank: None,
            variant: None,
        };
        let zero = PriceKey {
            wfm_url: "an_item".to_string(),
            rank: Some(0),
            variant: None,
        };
        assert_eq!(absent.id(), zero.id());
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

    /// warframe.market carries joke listings - a real 90 day window for
    /// Magazine Warp, a mod worth about 5p, contains a single "69420" sale.
    /// Volume weighting is no defence in a thin market where every bucket
    /// holds one trade, so that one entry priced the mod at 4962p and sent it
    /// to the top of the "most valuable" sort.
    #[test]
    fn discards_a_bucket_priced_absurdly_above_the_rest() {
        let mut buckets: Vec<serde_json::Value> = (0..9)
            .map(|_| json!({"avg_price": 5.0, "volume": 1}))
            .collect();
        buckets.push(json!({"avg_price": 69420.0, "volume": 1}));
        let stats = serde_json::Value::Array(buckets);
        assert_eq!(select_price(&stats, &key(None, None)), Some(5.0));
    }

    /// An item that genuinely trades over a wide range is not an outlier, and
    /// clipping it would understate what it is worth. Only prices orders of
    /// magnitude away from the rest are treated as noise.
    #[test]
    fn keeps_a_wide_but_believable_spread() {
        let stats = json!([
            {"avg_price": 10.0, "volume": 9},
            {"avg_price": 100.0, "volume": 1},
        ]);
        assert_eq!(select_price(&stats, &key(None, None)), Some(19.0));
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

    const DAY: i64 = 24 * 60 * 60;

    /// An item that trades daily has a 48 hour figure, and that is the one
    /// worth showing: it says what the thing is going for now.
    #[test]
    fn prefers_the_forty_eight_hour_price_when_that_window_has_trades() {
        let body = json!({"payload": {"statistics_closed": {
            "48hours": [{"avg_price": 99.0, "volume": 1}],
            "90days": [{"avg_price": 11.0, "volume": 1}],
        }}});
        assert_eq!(price_from_payload(&body, &key(None, None)), Some(99.0));
    }

    /// Most mods and relics do not trade every two days, so their 48 hour
    /// window is empty. Measured over a sample of the inventory rows with no
    /// price, 48 hours could price 3 in 25 where 90 days could price 21, so
    /// widening is the difference between a figure and a permanent "unknown".
    #[test]
    fn falls_back_to_the_ninety_day_price_when_the_short_window_is_empty() {
        let body = json!({"payload": {"statistics_closed": {
            "48hours": [],
            "90days": [{"avg_price": 11.0, "volume": 1}],
        }}});
        assert_eq!(price_from_payload(&body, &key(None, None)), Some(11.0));
    }

    /// A short window carrying only another rank is no evidence about the rank
    /// we hold, so it must fall through rather than report unknown.
    #[test]
    fn falls_back_when_the_short_window_has_no_bucket_for_our_rank() {
        let body = json!({"payload": {"statistics_closed": {
            "48hours": [{"avg_price": 99.0, "volume": 1, "mod_rank": 10}],
            "90days": [{"avg_price": 11.0, "volume": 1, "mod_rank": 0}],
        }}});
        assert_eq!(price_from_payload(&body, &key(Some(0), None)), Some(11.0));
    }

    /// Nothing in either window is an honest unknown, not a zero.
    #[test]
    fn reports_nothing_when_neither_window_has_trades() {
        let body = json!({"payload": {"statistics_closed": {"48hours": [], "90days": []}}});
        assert_eq!(price_from_payload(&body, &key(None, None)), None);
    }

    /// The response carries both a 48 hour series and a 90 day one. The short
    /// window is the one that describes what an item trades for right now,
    /// which is what the inventory tabs are read for.
    #[test]
    fn reads_the_forty_eight_hour_series_from_the_payload() {
        let body = json!({
            "payload": {
                "statistics_closed": {
                    "48hours": [{"avg_price": 99.0, "volume": 1}],
                    "90days": [{"avg_price": 11.0, "volume": 1}],
                }
            }
        });
        assert_eq!(prices(&window_from_payload(&body, "48hours")), vec![99.0]);
    }

    /// A response shaped differently than expected carries no prices, and must
    /// not fail the tick that asked for it.
    #[test]
    fn reports_an_empty_window_when_the_payload_has_no_series() {
        let window = window_from_payload(&json!({"payload": {}}), "48hours");
        assert_eq!(window.as_array().unwrap().len(), 0);
    }

    fn prices(window: &serde_json::Value) -> Vec<f64> {
        window
            .as_array()
            .unwrap()
            .iter()
            .map(|b| b.get("avg_price").unwrap().as_f64().unwrap())
            .collect()
    }

    fn seeded(price: f64, fetched_at: i64) -> CachedPrice {
        CachedPrice {
            price: Some(price),
            fetched_at,
        }
    }

    /// A fresh install knows nothing, and the whole point of shipping a seed is
    /// that it does not spend its first quarter of an hour discovering prices
    /// that were already known when the build was made.
    #[test]
    fn adopts_a_seeded_price_the_install_has_never_seen() {
        let mut known = HashMap::new();
        merge_seed(
            &mut known,
            HashMap::from([("an_item#0#".to_string(), seeded(10.0, 500))]),
        );
        assert_eq!(known["an_item#0#"].price, Some(10.0));
    }

    /// A price this install fetched itself is worth more than one baked into
    /// the build months earlier, so the seed must never overwrite it.
    #[test]
    fn keeps_a_locally_fetched_price_over_an_older_seeded_one() {
        let mut known = HashMap::from([("an_item#0#".to_string(), seeded(42.0, 9_000))]);
        merge_seed(
            &mut known,
            HashMap::from([("an_item#0#".to_string(), seeded(10.0, 500))]),
        );
        assert_eq!(known["an_item#0#"].price, Some(42.0));
    }

    /// The comparison is on when each was fetched, not on which side it came
    /// from: a store left untouched for months is the stale one.
    #[test]
    fn takes_a_seeded_price_that_is_newer_than_the_stored_one() {
        let mut known = HashMap::from([("an_item#0#".to_string(), seeded(42.0, 500))]);
        merge_seed(
            &mut known,
            HashMap::from([("an_item#0#".to_string(), seeded(10.0, 9_000))]),
        );
        assert_eq!(known["an_item#0#"].price, Some(10.0));
    }

    /// Equally old is not newer, so nothing churns on every start.
    #[test]
    fn leaves_an_equally_old_entry_alone() {
        let mut known = HashMap::from([("an_item#0#".to_string(), seeded(42.0, 500))]);
        merge_seed(
            &mut known,
            HashMap::from([("an_item#0#".to_string(), seeded(10.0, 500))]),
        );
        assert_eq!(known["an_item#0#"].price, Some(42.0));
    }

    fn candidate(name: &str, value: Option<f64>) -> (PriceKey, Option<f64>) {
        (
            PriceKey {
                wfm_url: name.to_string(),
                rank: None,
                variant: None,
            },
            value,
        )
    }

    /// A hundred-platinum set drifting is worth a request long before a
    /// two-platinum mod is.
    #[test]
    fn the_most_valuable_stale_item_is_refreshed_first() {
        let cheap = candidate("cheap", Some(2.0));
        let dear = candidate("dear", Some(120.0));
        let middling = candidate("middling", Some(40.0));
        let kn = HashMap::from([
            (cheap.0.id(), known(Some(2.0), 0)),
            (dear.0.id(), known(Some(120.0), 0)),
            (middling.0.id(), known(Some(40.0), 0)),
        ]);
        let plan = plan_backfill(vec![cheap, dear, middling], &kn, 10_000_000, 10);
        let order: Vec<&str> = plan.iter().map(|k| k.wfm_url.as_str()).collect();
        assert_eq!(order, vec!["dear", "middling", "cheap"]);
    }

    /// Among equally valuable items the one left longest goes first.
    #[test]
    fn equal_value_is_broken_by_staleness_oldest_first() {
        let newer = candidate("newer", Some(50.0));
        let older = candidate("older", Some(50.0));
        let kn = HashMap::from([
            (newer.0.id(), known(Some(50.0), 900_000)),
            (older.0.id(), known(Some(50.0), 1)),
        ]);
        let plan = plan_backfill(vec![newer, older], &kn, 10_000_000, 10);
        assert_eq!(plan[0].wfm_url, "older");
    }

    /// An item with no price at all cannot be ranked by value, so it waits
    /// behind the ones that can. It is still queued, which is what completes
    /// the dataset: refreshed items go fresh and leave the queue, so the
    /// unknowns are reached rather than starved.
    #[test]
    fn items_with_no_known_value_queue_after_the_ranked_ones() {
        let valued = candidate("valued", Some(5.0));
        let unknown = candidate("unknown", None);
        let kn = HashMap::from([(valued.0.id(), known(Some(5.0), 0))]);
        let plan = plan_backfill(vec![unknown, valued], &kn, 10_000_000, 10);
        let order: Vec<&str> = plan.iter().map(|k| k.wfm_url.as_str()).collect();
        assert_eq!(order, vec!["valued", "unknown"]);
    }

    /// A price fetched minutes ago is not worth a request.
    #[test]
    fn fresh_items_are_left_alone() {
        let c = candidate("fresh", Some(5.0));
        let kn = HashMap::from([(c.0.id(), known(Some(5.0), 1_000))]);
        assert!(plan_backfill(vec![c], &kn, 1_000 + 60, 10).is_empty());
    }

    /// The batch is capped so a tick cannot turn into a flood.
    #[test]
    fn the_batch_is_capped() {
        let candidates: Vec<(PriceKey, Option<f64>)> = (0..50)
            .map(|i| candidate(&format!("item_{i}"), Some(i as f64)))
            .collect();
        assert_eq!(
            plan_backfill(candidates, &HashMap::new(), 1_000, 5).len(),
            5
        );
    }

    fn known(price: Option<f64>, fetched_at: i64) -> CachedPrice {
        CachedPrice { price, fetched_at }
    }

    /// A 48 hour average is worth re-reading daily, not hourly. The hourly
    /// cadence was self-defeating: refreshing every known price every hour
    /// consumed the entire request budget, so rows with no price at all stayed
    /// queued behind it indefinitely. A day is still well inside the window
    /// the figure describes.
    #[test]
    fn a_found_price_stands_for_a_day_not_an_hour() {
        assert!(is_fresh(1_000, 1_000 + 12 * 3_600, true));
        assert!(!is_fresh(1_000, 1_000 + 2 * DAY, true));
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

/// Builds the seed shipped in `resources/seed_prices.json`.
///
/// Ignored because it makes 3891 requests and takes about twenty minutes. Run
/// it when the seed should be refreshed:
///
/// ```text
/// ddev cargo test --lib generate_seed_prices -- --ignored --nocapture
/// ```
///
/// `SEED_ITEMS` points at a `TradableItems.json`. `SEED_REUSE` optionally
/// points at an existing `market_prices.json`, whose prices are kept rather
/// than fetched again - which is most of the run time.
///
/// Scoped to every tradable item rather than any one inventory, so the file
/// says nothing about whoever generated it. One statistics response carries
/// every rank and refinement of an item, so 3891 requests yield about 7900
/// keys.
#[cfg(test)]
mod seed_generator {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::Semaphore;

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct SeedItem {
        wfm_url: Option<String>,
        #[serde(default)]
        sub_types: Option<SeedSubTypes>,
        #[serde(default)]
        variant_to_unique_name: Option<HashMap<String, String>>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct SeedSubTypes {
        max_rank: Option<i64>,
    }

    /// Every product the market prices separately for one item.
    fn keys_for(item: &SeedItem) -> Vec<PriceKey> {
        let Some(url) = item.wfm_url.clone() else {
            return vec![];
        };
        let variants: Vec<Option<String>> = match &item.variant_to_unique_name {
            Some(map) if !map.is_empty() => map.keys().cloned().map(Some).collect(),
            _ => vec![None],
        };
        let max_rank = item.sub_types.as_ref().and_then(|s| s.max_rank);
        let mut keys = Vec::new();
        for variant in variants {
            keys.push(PriceKey {
                wfm_url: url.clone(),
                rank: None,
                variant: variant.clone(),
            });
            if let Some(rank) = max_rank.filter(|r| *r > 0) {
                keys.push(PriceKey {
                    wfm_url: url.clone(),
                    rank: Some(rank),
                    variant,
                });
            }
        }
        keys
    }

    #[tokio::test]
    #[ignore = "makes 3891 requests to warframe.market; regenerates resources/seed_prices.json"]
    async fn generate_seed_prices() {
        let items_path = std::env::var("SEED_ITEMS")
            .unwrap_or_else(|_| "../local/appdata/cache/items/TradableItems.json".to_string());
        let raw = std::fs::read_to_string(&items_path)
            .unwrap_or_else(|e| panic!("could not read {items_path}: {e}"));
        let items: Vec<SeedItem> =
            serde_json::from_str(&raw).expect("tradable items did not parse");

        let mut by_url: HashMap<String, Vec<PriceKey>> = HashMap::new();
        for item in &items {
            for key in keys_for(item) {
                by_url.entry(key.wfm_url.clone()).or_default().push(key);
            }
        }
        let wanted = by_url.values().map(Vec::len).sum::<usize>();

        // Prices already fetched are prices worth keeping: re-asking for them
        // spends twenty minutes learning what is already known. Reused entries
        // keep their original timestamp, so an install still judges them by age
        // rather than by where they came from.
        let reused: HashMap<String, CachedPrice> = std::env::var("SEED_REUSE")
            .ok()
            .and_then(|path| std::fs::read_to_string(path).ok())
            .and_then(|raw| serde_json::from_str::<HashMap<String, CachedPrice>>(&raw).ok())
            .map(|known| {
                known
                    .into_iter()
                    .filter(|(_, entry)| entry.price.is_some())
                    .collect()
            })
            .unwrap_or_default();

        // Only an item whose every product is already known can be skipped:
        // one response carries them all, so a partial hit still costs the same
        // single request.
        by_url.retain(|_, keys| !keys.iter().all(|key| reused.contains_key(&key.id())));
        println!(
            "{} keys wanted, {} reused, {} items still to fetch",
            wanted,
            reused.len(),
            by_url.len()
        );

        let client = reqwest::Client::new();
        let out: Arc<Mutex<HashMap<String, CachedPrice>>> = Arc::new(Mutex::new(reused));
        let mut cadence = tokio::time::interval(std::time::Duration::from_micros(333_333));
        cadence.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let in_flight = Arc::new(Semaphore::new(8));
        let mut tasks = Vec::new();

        for (url, keys) in by_url {
            cadence.tick().await;
            let permit = in_flight.clone().acquire_owned().await.unwrap();
            let client = client.clone();
            let out = out.clone();
            tasks.push(tokio::spawn(async move {
                let body = client
                    .get(format!(
                        "https://api.warframe.market/v1/items/{url}/statistics"
                    ))
                    .header("accept", "application/json")
                    .send()
                    .await
                    .ok();
                if let Some(body) = body {
                    if let Ok(json) = body.json::<Value>().await {
                        let now = now_secs();
                        let mut out = out.lock().unwrap();
                        for key in keys {
                            if let Some(price) = price_from_payload(&json, &key) {
                                out.insert(
                                    key.id(),
                                    CachedPrice {
                                        price: Some(price),
                                        fetched_at: now,
                                    },
                                );
                            }
                        }
                    }
                }
                drop(permit);
            }));
        }
        for task in tasks {
            let _ = task.await;
        }

        let out = out.lock().unwrap();
        // Only priced keys are written: shipping "we asked and found nothing"
        // would suppress a real lookup for a day on every install.
        std::fs::write(
            "resources/seed_prices.json",
            serde_json::to_string(&*out).expect("seed did not serialise"),
        )
        .expect("could not write resources/seed_prices.json");
        println!("wrote {} priced keys", out.len());
        assert!(!out.is_empty(), "seed came back empty");
    }
}
