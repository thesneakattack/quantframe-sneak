//! Keeps the inventory's prices complete and current, slowly.
//!
//! Two jobs that are really one: items the shipped cache never priced need a
//! figure at all, and items that have one need it refreshed before it drifts.
//! Both are served by the same trickle, ordered most-valuable-first, so the
//! prices that matter to a trade are the ones kept current.
//!
//! Deliberately nothing to do with rendering. The tabs read whatever is
//! known at the moment they are opened; this fills in behind them. Doing it
//! on demand is what made sorting feel like a hang.

use std::time::Duration;

use utils::{info, LoggerOptions};

use crate::market_prices::{MarketPriceStore, PriceKey};
use crate::wf_inventory::modules::item::priced_rows;

static COMPONENT: &str = "MarketBackfill";

/// Gap between batches. warframe.market tolerates a few requests a second;
/// this is far below that, because nothing here is urgent and the budget is
/// shared with the live scraper.
const TICK: Duration = Duration::from_secs(30);

/// How many prices one tick may fetch. With the tick above this is roughly
/// twenty an hour on the slowest path and still clears a thousand-item
/// backlog overnight.
const PER_TICK: usize = 10;

/// Pause between requests inside a batch.
const SPACING: Duration = Duration::from_millis(400);

/// Start the trickle. Runs for the life of the app.
pub fn spawn() {
    tokio::spawn(async move {
        info(
            format!("{}:Start", COMPONENT),
            format!(
                "Filling and refreshing inventory prices, up to {PER_TICK} every {}s",
                TICK.as_secs()
            ),
            &LoggerOptions::default(),
        );
        loop {
            tokio::time::sleep(TICK).await;
            if let Err(e) = tick().await {
                // A bad tick is not worth stopping over; the next one retries.
                utils::warning(
                    format!("{}:Tick", COMPONENT),
                    format!("{e}"),
                    &LoggerOptions::default(),
                );
            }
        }
    });
}

async fn tick() -> Result<(), utils::Error> {
    // The live scraper is doing the same kind of work for trades in flight,
    // which matters more than topping up a table. Stay out of its way.
    if crate::utils::modules::states::live_scraper_is_running() {
        return Ok(());
    }

    let Some(candidates) = inventory_candidates()? else {
        return Ok(());
    };
    let store = MarketPriceStore::get();
    let due = store.plan_next_batch(candidates, PER_TICK);
    if due.is_empty() {
        return Ok(());
    }

    let wanted = due.len();
    for key in due {
        store.refresh(&key).await;
        tokio::time::sleep(SPACING).await;
    }
    store.flush();
    info(
        format!("{}:Tick", COMPONENT),
        format!("Updated {wanted} inventory prices"),
        &LoggerOptions::default(),
    );
    Ok(())
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
    let mods = priced_rows(&rows.mods)?;
    let sets = priced_rows(&rows.sets.iter().map(|s| s.base.clone()).collect::<Vec<_>>())?;

    let mut candidates = Vec::new();
    for row in parts.iter().chain(mods.iter()).chain(sets.iter()) {
        candidates.push((
            PriceKey {
                wfm_url: row.wfm_url.clone(),
                rank: row.sub_type.as_ref().and_then(|s| s.rank),
                variant: row.sub_type.as_ref().and_then(|s| s.variant.clone()),
            },
            row.properties
                .get_property_value::<Option<f64>>("price", None),
        ));
    }
    Ok(Some(candidates))
}
