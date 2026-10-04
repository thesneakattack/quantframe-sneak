//! Keeps the inventory's prices complete and current.
//!
//! Two jobs that are really one: items the shipped cache never priced need a
//! figure at all, and items that have one need it refreshed before it drifts.
//! Both are served by the same pass, ordered most-valuable-first, so the
//! prices that matter to a trade are the ones kept current.
//!
//! The rate follows the backlog rather than a fixed cadence. While rows are
//! still due it runs at the request ceiling; when nothing is due it idles.
//! The fixed trickle this replaced was asleep 85% of the time and left
//! hundreds of rows unpriced for days, which is the opposite of what a
//! rate limit is for.
//!
//! Deliberately nothing to do with rendering. The tabs read whatever is
//! known at the moment they are opened; this fills in behind them. Doing it
//! on demand is what made sorting feel like a hang.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Semaphore;

use utils::{info, LoggerOptions};

use crate::market_prices::{MarketPriceStore, PriceKey};
use crate::wf_inventory::modules::item::{max_rank_key, priced_rows, with_max_rank_prices};

static COMPONENT: &str = "MarketBackfill";

/// Gap between batches. warframe.market tolerates a few requests a second;
/// this is far below that, because nothing here is urgent and the budget is
/// shared with the live scraper.
/// How long to wait after a pass that found nothing due.
const IDLE: Duration = Duration::from_secs(30);

/// How many prices one pass may fetch before rebuilding its candidate list.
///
/// Rebuilding means projecting and pricing every inventory row, which is the
/// expensive part; a pass this size amortises it over about half a minute of
/// fetching while still noticing a changed inventory promptly.
const PER_PASS: usize = 100;

/// Requests a second. The ceiling `qf_api` already applies to the project's
/// other API, applied here by hand because these calls go straight to
/// `HTTP_CLIENT` rather than through that client and so bypass its limiter.
const REQUESTS_PER_SECOND: u32 = 3;

/// How many requests may be waiting on a reply at once.
///
/// The cadence sets the rate; this only bounds how far replies may fall
/// behind it. At three a second against a half-second round trip roughly two
/// are in flight in the steady state, so this is headroom for a latency
/// spike rather than a target.
const MAX_IN_FLIGHT: usize = 8;

/// The gap between requests that holds the loop at `per_second`.
///
/// Derived rather than written alongside the ceiling, because two constants
/// meaning the same thing drift apart, and the direction that drifts is the
/// one that hammers the endpoint. A ceiling of zero is a configuration
/// mistake, not licence to go as fast as the machine allows.
pub fn request_spacing(per_second: u32) -> Duration {
    if per_second == 0 {
        return Duration::from_secs(1);
    }
    Duration::from_micros(1_000_000 / per_second as u64)
}

/// How long to wait before looking for work again, if at all.
///
/// While rows are still coming back there is known work left, and pausing
/// only makes the backlog last longer: the old fixed cadence spent 85% of its
/// time asleep with hundreds of rows outstanding. The pause is for the
/// opposite case, where polling hard would spend the budget discovering there
/// is nothing to do.
pub fn pause_before_next_pass(fetched: usize) -> Option<Duration> {
    (fetched == 0).then_some(IDLE)
}

/// Start the trickle. Runs for the life of the app.
pub fn spawn() {
    tokio::spawn(async move {
        info(
            format!("{}:Start", COMPONENT),
            format!(
                "Filling and refreshing inventory prices at up to {REQUESTS_PER_SECOND}/s, idling {}s when nothing is due",
                IDLE.as_secs()
            ),
            &LoggerOptions::default(),
        );
        loop {
            let fetched = match tick().await {
                Ok(fetched) => fetched,
                Err(e) => {
                    // A bad pass is not worth stopping over; back off and
                    // retry rather than spinning on whatever failed.
                    utils::warning(
                        format!("{}:Tick", COMPONENT),
                        format!("{e}"),
                        &LoggerOptions::default(),
                    );
                    0
                }
            };
            if let Some(pause) = pause_before_next_pass(fetched) {
                tokio::time::sleep(pause).await;
            }
        }
    });
}

async fn tick() -> Result<usize, utils::Error> {
    // The live scraper is doing the same kind of work for trades in flight,
    // which matters more than topping up a table. Stay out of its way.
    if crate::utils::modules::states::live_scraper_is_running() {
        return Ok(0);
    }

    let Some(candidates) = inventory_candidates()? else {
        return Ok(0);
    };
    let store = MarketPriceStore::get();
    let due = store.plan_next_batch(candidates, PER_PASS);
    if due.is_empty() {
        return Ok(0);
    }

    let wanted = due.len();

    // Issued on a fixed cadence rather than one after another. Sequentially a
    // request costs its round trip *plus* the gap, so warframe.market's ~500ms
    // latency capped the loop near 1.2/s however small the gap was set. Firing
    // on the clock and letting replies overlap makes the ceiling the thing
    // that actually limits the rate.
    let mut cadence = tokio::time::interval(request_spacing(REQUESTS_PER_SECOND));
    // A slow reply must not earn back the time it cost by firing a burst of
    // catch-up requests, which is exactly when the endpoint is least able to
    // take them.
    cadence.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    let in_flight = Arc::new(Semaphore::new(MAX_IN_FLIGHT));
    let mut issued = Vec::with_capacity(wanted);
    for key in due {
        cadence.tick().await;
        // Back-pressure: if replies stop coming back the permits run out and
        // the cadence stalls with them, rather than piling requests onto an
        // endpoint already struggling.
        let Ok(permit) = in_flight.clone().acquire_owned().await else {
            break;
        };
        issued.push(tokio::spawn(async move {
            MarketPriceStore::get().refresh(&key).await;
            drop(permit);
        }));
    }
    for task in issued {
        // One panicked fetch must not abandon the rest of the pass, nor the
        // flush that makes the batch durable.
        let _ = task.await;
    }
    store.flush();
    info(
        format!("{}:Tick", COMPONENT),
        format!("Updated {wanted} inventory prices"),
        &LoggerOptions::default(),
    );
    Ok(wanted)
}

/// Every row the player actually owns, paired with what it is currently
/// thought to be worth.
///
/// Scoped to the inventory rather than all 3891 tradable items: a price is
/// only worth keeping current for something you could actually sell.
fn inventory_candidates() -> Result<Option<Vec<(PriceKey, Option<f64>)>>, utils::Error> {
    let Ok(inventory) = crate::utils::modules::states::wf_inventory() else {
        // Still starting up.
        return Ok(None);
    };
    let rows = inventory.rows()?;
    // Priced copies, not the bare snapshot rows: the snapshot deliberately
    // holds no prices, and value is what the ordering turns on.
    let parts = priced_rows(&rows.parts)?;
    let relics = priced_rows(&rows.relics)?;
    // The rankable tabs also show what the item is worth maxed, so these carry
    // that second figure for the ordering below.
    let mods = with_max_rank_prices(priced_rows(&rows.mods)?)?;
    let arcanes = with_max_rank_prices(priced_rows(&rows.arcanes)?)?;
    let sets = priced_rows(&rows.sets.iter().map(|s| s.base.clone()).collect::<Vec<_>>())?;

    // Every tab, or a tab's rows silently stop being refreshed - which is what
    // happened when relics and arcanes were split out of parts and mods.
    let owned = parts
        .iter()
        .chain(relics.iter())
        .chain(mods.iter())
        .chain(arcanes.iter())
        .chain(sets.iter());

    // A row already at its maximum rank yields the same key twice, and two
    // requests for one product in a batch is budget spent for nothing.
    let mut seen = HashSet::new();
    let mut candidates = Vec::new();
    let mut push = |key: PriceKey, value: Option<f64>| {
        if seen.insert(key.id()) {
            candidates.push((key, value));
        }
    };

    for row in owned {
        push(
            PriceKey {
                wfm_url: row.wfm_url.clone(),
                rank: row.sub_type.as_ref().and_then(|s| s.rank),
                variant: row.sub_type.as_ref().and_then(|s| s.variant.clone()),
            },
            row.properties
                .get_property_value::<Option<f64>>("price", None),
        );
    }

    // What a mod sells for maxed is a different product from what the copy you
    // hold sells for, so it has its own price and must be fetched to show.
    for row in mods.iter().chain(arcanes.iter()) {
        if let Some(key) = max_rank_key(row) {
            let value = row
                .properties
                .get_property_value::<Option<f64>>("max_rank_price", None);
            push(key, value);
        }
    }
    Ok(Some(candidates))
}

#[cfg(test)]
mod tests {
    use super::{pause_before_next_pass, request_spacing, IDLE};
    use std::time::Duration;

    /// The ceiling is written once, as requests a second, and the gap is
    /// derived. Writing both invites them to drift apart, and the direction
    /// that drifts is the one that hammers the endpoint.
    #[test]
    fn derives_the_gap_between_requests_from_the_ceiling() {
        assert_eq!(request_spacing(3), Duration::from_micros(333_333));
        assert_eq!(request_spacing(1), Duration::from_secs(1));
    }

    /// A ceiling of zero is a configuration mistake, not an instruction to
    /// issue requests as fast as the machine can. It must not divide by zero
    /// either.
    #[test]
    fn treats_a_ceiling_of_zero_as_one_request_a_second() {
        assert_eq!(request_spacing(0), Duration::from_secs(1));
    }

    /// Draining is the point. While rows are still coming back there is known
    /// work left, and pausing only makes the backlog last longer - which is
    /// what made the old fixed trickle idle 85% of the time.
    #[test]
    fn does_not_pause_while_there_is_still_work_coming_back() {
        assert_eq!(pause_before_next_pass(10), None);
        assert_eq!(pause_before_next_pass(1), None);
    }

    /// Nothing due is the case the pause exists for: polling hard would spend
    /// the budget discovering there is no work.
    #[test]
    fn pauses_when_a_pass_found_nothing_to_do() {
        assert_eq!(pause_before_next_pass(0), Some(IDLE));
    }
}
