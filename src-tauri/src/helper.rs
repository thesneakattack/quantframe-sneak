use chrono::{DateTime, Utc};
use entity::{
    dto::{FinancialGraph, FinancialReport, PaginatedResult, PriceHistory},
    enums::RivenGrade,
    stock_riven::RivenAttribute,
    transaction::TransactionPaginationQueryDto,
};
use serde_json::{json, Value};
use service::TransactionQuery;
use std::{
    fs::{self},
    path::PathBuf,
};
use tauri::{Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use utils::SubType;
use utils::*;
use wf_market::{enums::OrderType, types::AuctionLike, Authenticated};

use crate::{
    cache::{
        derive_riven_summary_attributes, grade_riven, scale_attributes, CacheState, CacheWeaponBase,
    },
    utils::{auction_list_ext::AuctionWithOwnerListExt, ErrorFromExt, OrderListExt, SubTypeExt},
    APP, DATABASE,
};

pub static APP_PATH: &str = "dev.thesneakattack.quantframe";

pub fn get_device_id() -> String {
    let app = APP.get().unwrap();
    let home_dir = match app.path().home_dir() {
        Ok(val) => val,
        Err(_) => {
            panic!("Could not find home directory");
        }
    };
    let device_name = home_dir.file_name().unwrap().to_str().unwrap();
    device_name.to_string()
}
pub fn get_app_storage_path() -> PathBuf {
    let app = APP.get().unwrap();
    let local_path = match app.path().local_data_dir() {
        Ok(val) => val,
        Err(_) => {
            panic!("Could not find app path");
        }
    };

    let app_path = local_path.join(APP_PATH);
    if !app_path.exists() {
        fs::create_dir_all(&app_path).unwrap()
    }
    app_path
}

pub fn get_sounds_path() -> PathBuf {
    let sounds_path = get_app_storage_path().join("sounds");
    if !sounds_path.exists() {
        fs::create_dir_all(&sounds_path).unwrap()
    }
    sounds_path
}

pub fn get_desktop_path() -> PathBuf {
    let app = APP.get().unwrap();
    let desktop_path = match app.path().desktop_dir() {
        Ok(val) => val,
        Err(_) => {
            panic!("Could not find desktop path");
        }
    };
    desktop_path
}
pub fn generate_transaction_summary(
    transactions: &[entity::transaction::Model],
    date: DateTime<Utc>,
    group_by1: GroupByDate,
    group_by2: &[GroupByDate],
    _previous: bool,
) -> (FinancialReport, FinancialGraph<i64>) {
    let (start, end) = get_start_end_of(date, group_by1);
    let transactions = filters_by(transactions, |t| {
        t.created_at >= start && t.created_at <= end
    });

    let mut grouped = group_by_date(&transactions, |t| t.created_at, group_by2);

    fill_missing_date_keys(&mut grouped, start, end, group_by2);

    let graph = FinancialGraph::<i64>::from(&grouped, |group| {
        FinancialReport::from(&group.to_vec()).total_profit
    });
    (FinancialReport::from(&transactions), graph)
}

/// Paginate a vector of items
pub fn paginate<T: Clone>(items: &[T], page: i64, per_page: i64) -> PaginatedResult<T> {
    let total_items = items.len() as i64;

    let start = (page.saturating_sub(1)) * per_page;
    let end = (start + per_page).min(total_items);

    let start_usize = start as usize;
    let end_usize = end as usize;

    let page_items = if start < total_items && end > 0 {
        items[start_usize..end_usize].to_vec()
    } else if per_page == -1 {
        items.to_vec()
    } else {
        Vec::new()
    };
    let total_pages = if per_page == -1 {
        1
    } else {
        (total_items as f64 / per_page as f64).ceil() as i64
    };
    PaginatedResult {
        results: page_items,
        page,
        limit: per_page,
        total: total_items,
        total_pages,
    }
}

pub fn get_local_data_path() -> PathBuf {
    let app = APP.get().unwrap();
    let local_path = match app.path().local_data_dir() {
        Ok(val) => val,
        Err(_) => {
            panic!("Could not find local data path");
        }
    };
    local_path
}

pub fn get_or_create_window(
    label: &str,
    url: &str,
    title: &str,
    size: Option<(f64, f64)>,
    resizable: bool,
) -> Result<(bool, WebviewWindow), Error> {
    let t_app = match APP.get() {
        Some(app) => app,
        None => {
            return Err(Error::new(
                "Helper::GetOrCreateWindow",
                "App state not found.",
                get_location!(),
            ));
        }
    };

    let app_handle = t_app.app_handle();

    // Return existing window
    if let Some(window) = app_handle.get_webview_window(label) {
        return Ok((true, window));
    }

    // Build new window
    let mut builder = WebviewWindowBuilder::new(app_handle, label, WebviewUrl::App(url.into()))
        .title(title)
        .resizable(resizable);

    if let Some((w, h)) = size {
        builder = builder.inner_size(w, h);
    }

    let window = builder.build().map_err(|e| {
        Error::new(
            "Helper::GetOrCreateWindow",
            format!("Failed to build window: {}", e),
            get_location!(),
        )
    })?;
    Ok((false, window))
}

pub async fn populate_item_market_properties(
    properties: &mut Properties,
    raw: impl Into<String>,
    sub_type: Option<SubType>,
    bought: i64,
    list_price: Option<i64>,
    mut operations: OperationSet,
    order_type: OrderType,
    cache: &CacheState,
    wfm: &wf_market::client::Client<Authenticated>,
) -> Result<(), Error> {
    let conn = DATABASE.get().unwrap();
    let raw = raw.into();
    let wfm_sub_type: wf_market::types::SubType = SubTypeExt::from_entity(sub_type.clone());

    // ---------------- Item Info ----------------
    let item_info = cache
        .tradable_item()
        .get_by(&raw)
        .map_err(|e| e.with_location(get_location!()))?;

    properties.set_property_value("name", item_info.name.clone());
    properties.set_property_value("image", item_info.icon.clone());
    properties.set_property_value("t_type", item_info.sub_type.clone());
    properties.set_property_value("bulk_tradable", item_info.bulk_tradable);

    // ---------------- Order Info ----------------
    let order = wfm
        .order()
        .cache_orders()
        .find_order(&item_info.wfm_id, &wfm_sub_type, order_type);

    let (platinum, order_properties) = if let Some(order) = order {
        let order_operations = order
            .properties
            .get_property_value("operations", OperationSet::new());

        properties.update_property("price_history", |ph: &mut Vec<PriceHistory>| {
            if ph.is_empty() {
                ph.push(PriceHistory::new(
                    order.updated_at.clone(),
                    order.platinum as i64,
                ));
            }
        });
        operations.merge(&order_operations);
        (order.platinum as i64, order.properties.clone())
    } else {
        (
            list_price.unwrap_or(0),
            wf_market::types::Properties::default(),
        )
    };
    // ---------------- Profitability Info ----------------
    if operations.has("ProfitabilityInfo") {
        let potential_profit = platinum - bought;
        let roi = if bought > 0 {
            (potential_profit as f64 / bought as f64) * 100.0
        } else {
            0.0
        };
        properties.set_property_value("roi_percent", roi);
        properties.set_property_value("potential_profit", potential_profit);
    }
    // ---------------- Transaction Info ----------------
    if operations.has("TransactionInfo") {
        let transactions = TransactionQuery::get_all(
            conn,
            TransactionPaginationQueryDto::new(1, -1)
                .set_wfm_id(&item_info.wfm_id)
                .set_sub_type(sub_type.clone()),
        )
        .await
        .map_err(|e| e.with_location(get_location!()))?;

        properties.set_property_value("report", FinancialReport::from(&transactions.results));
        properties.set_property_value("last_transactions", transactions.take_top(5));
    }

    // ---------------- Market Info ----------------
    if operations.has("MarketInfo") && !operations.has("MarketPopulated") {
        let mut orders = wfm
            .order()
            .get_orders_by_item(&item_info.wfm_url)
            .await
            .map_err(|e| {
                Error::from_wfm(
                    "Command::StockItemGetById",
                    "Failed to fetch orders from WFM: {}",
                    e,
                    get_location!(),
                )
            })?;

        orders.filter_by_sub_type(wfm_sub_type.clone(), false);
        orders.filter_user_status(wf_market::enums::StatusType::InGame, false);
        orders.sort_by_platinum();
        orders.apply_item_info(cache)?;

        // Metrics for Highest, Lowest Sell and Buy Prices
        let sell_highest = orders.highest_price(OrderType::Sell);
        let sell_lowest = orders.lowest_price(OrderType::Sell);
        let buy_highest = orders.highest_price(OrderType::Buy);
        let buy_lowest = orders.lowest_price(OrderType::Buy);

        properties.set_property_value("sell_highest_price", sell_highest);
        properties.set_property_value("sell_lowest_price", sell_lowest);
        properties.set_property_value("buy_highest_price", buy_highest);
        properties.set_property_value("buy_lowest_price", buy_lowest);
        properties.set_property_value("supply", orders.sell_orders.len());
        properties.set_property_value("demand", orders.buy_orders.len());

        let spread = sell_lowest - buy_highest;
        properties.set_property_value("spread", spread);

        let spread_pct = if sell_lowest > 0 {
            spread as f64 / sell_lowest as f64 * 100.0
        } else {
            0.0
        };

        properties.set_property_value("spread_percent", spread_pct);
        properties.set_property_value("orders", orders.take_top(5, order_type));
    }
    // ----------------- Market Populated Info -----------------
    properties.merge_properties(order_properties.properties, true, true);

    // ----------------- Operations Info -----------------
    properties.set_property_value("ui_operations", operations.operations.clone());
    Ok(())
}
pub async fn populate_riven_market_properties(
    properties: &mut Properties,
    raw: impl Into<String>,
    mastery_rank: i64,
    rerolls: i64,
    rank: i32,
    raw_attributes: Vec<(String, f64, bool)>,
    uuid: String,
    bought: i64,
    list_price: Option<i64>,
    mut operations: OperationSet,
    cache: &CacheState,
    wfm: &wf_market::client::Client<Authenticated>,
) -> Result<Vec<RivenAttribute>, Error> {
    let conn = DATABASE.get().unwrap();
    let raw = raw.into();

    // ---------------- Item Info ----------------
    let weapon_info = cache
        .weapon()
        .get_by(&raw)
        .map_err(|e| e.with_location(get_location!()))?;

    properties.set_property_value("name", weapon_info.name.clone());
    properties.set_property_value("image", weapon_info.icon.clone());
    properties.set_property_value("disposition_rank", weapon_info.disposition_rank);

    // ---------------- Attributes Info ----------------
    let mut attributes =
        derive_riven_summary_attributes(cache, &weapon_info, &raw_attributes, rank)?;
    // ---------------- Auction Info ----------------
    let auction = wfm.auction().cache_auctions().get_by_uuid(&uuid);

    let (platinum, auction_properties) = if let Some(auction) = auction {
        let auction_operations = auction
            .properties
            .get_property_value("operations", OperationSet::new());
        operations.merge(&auction_operations);
        (auction.starting_price as i64, auction.properties.clone())
    } else {
        (
            list_price.unwrap_or(0),
            wf_market::types::Properties::default(),
        )
    };

    // ---------------- Profitability Info ----------------
    if operations.has("ProfitabilityInfo") {
        let potential_profit = platinum - bought;
        let roi = if bought > 0 {
            (potential_profit as f64 / bought as f64) * 100.0
        } else {
            0.0
        };
        properties.set_property_value("roi_percent", roi);
        properties.set_property_value("potential_profit", potential_profit);
    }

    // ---------------- Grade Info ----------------
    if operations.has("GradeInfo") {
        match cache.riven_good_roll().get_by(&weapon_info.unique_name) {
            Ok(god_roll) => {
                let (grade, grads) = grade_riven(&god_roll, &attributes, "tag");

                for i in 0..grads.len() {
                    attributes[i]
                        .properties
                        .set_property_value("grade", grads[i].1.clone());
                }

                properties.set_property_value("grade", grade);
            }
            Err(_) => {
                warning(
                    "GradeInfo",
                    format!(
                        "Could not find good roll info for weapon: {}",
                        weapon_info.unique_name
                    ),
                    &LoggerOptions::default(),
                );
                properties.set_property_value("grade", RivenGrade::Unknown);
            }
        }
    }

    // ---------------- Variant Info ----------------
    if operations.has("VariantInfo") {
        let mut weapons = Vec::new();

        let collect_weapon = |weapon: &CacheWeaponBase| {
            Properties::from(json!({
                "unique_name": weapon.unique_name,
                "name": weapon.name,
                "disposition": weapon.disposition,
                "disposition_rank": weapon.disposition_rank
            }))
        };
        let variants = cache
            .weapon()
            .get_weapons_by_family(&weapon_info.family)
            .unwrap_or_default();
        for variant in variants {
            weapons.push(collect_weapon(&variant));
        }

        for wea in &mut weapons {
            let disposition = wea.get_property_value("disposition", 0.0);
            let ratio = disposition / weapon_info.disposition;

            let ranks = (0..=8)
                .map(|i| scale_attributes(&attributes, ratio, i))
                .collect::<Vec<Vec<RivenAttribute>>>();

            wea.set_property_value("ranks", ranks);
        }

        properties.set_property_value(
            "variants",
            weapons
                .iter()
                .map(|w| w.get_properties(Value::Null))
                .collect::<Vec<Value>>(),
        );
    }

    // ---------------- Roll Evaluation Info ----------------
    if operations.has("RollEvaluation") {
        match cache.riven_good_roll().get_by(&weapon_info.unique_name) {
            Ok(god_roll) => {
                let roll_evaluation = god_roll.fill_roll_evaluation(
                    &weapon_info.upgrade_type,
                    raw_attributes.clone(),
                    cache,
                )?;
                properties.set_property_value("roll_evaluation", roll_evaluation);
            }
            Err(_) => {
                warning(
                    "RollEvaluation",
                    format!(
                        "Could not find good roll info for weapon: {}",
                        weapon_info.unique_name
                    ),
                    &LoggerOptions::default(),
                );
            }
        }
    }

    // ---------------- Transaction Info ----------------
    if operations.has("TransactionInfo") {
        let transactions = TransactionQuery::get_all(
            conn,
            TransactionPaginationQueryDto::new(1, -1).set_wfm_url(&weapon_info.wfm_riven_url),
        )
        .await
        .map_err(|e| e.with_location(get_location!()))?;

        properties.set_property_value("report", FinancialReport::from(&transactions.results));
        properties.set_property_value("last_transactions", transactions.take_top(5));
    }

    // ---------------- Market Info ----------------
    if operations.has("MarketInfo") && !operations.has("MarketPopulated") {
        let mut filter = wf_market::types::AuctionFilter::new(
            wf_market::enums::AuctionType::Riven,
            &weapon_info.wfm_riven_url,
        );
        filter.similarity_attributes = Some(
            attributes
                .iter()
                .map(|att| {
                    wf_market::types::ItemAttribute::new(
                        att.wfm_url.clone(),
                        att.positive,
                        att.value,
                    )
                })
                .collect(),
        );
        filter.similarity = Some(34);

        let mut auctions = wfm.auction().search_auctions(filter).await.map_err(|e| {
            Error::from_wfm(
                "Command::StockRivenGetById",
                "Failed to search auctions",
                e,
                get_location!(),
            )
        })?;
        auctions.sort_by_similarity(false);
        auctions.apply_item_info(cache)?;

        // Metrics for Lowest Sell
        let sell_highest = auctions.highest_price();
        let sell_lowest = auctions.lowest_price();

        properties.set_property_value("sell_highest_price", sell_highest);
        properties.set_property_value("sell_lowest_price", sell_lowest);
        properties.set_property_value("supply", auctions.total_auctions());

        let mut auctions = auctions.to_vec();
        for auction in auctions.iter_mut().map(|auction| auction.to_auction_mut()) {
            let similarity = auction.item.similarity.clone();
            if let Some(attrs) = &mut auction.item.attributes {
                for attr in attrs.iter_mut() {
                    attr.properties
                        .set_property_value("matched", similarity.has_attribute(&attr.url_name));
                }
            }
        }
        properties.set_property_value("auctions", auctions);
    }

    // ----------------- Endo Info -----------------
    if operations.has("EndoInfo") {
        let endo =
            100 * (mastery_rank - 8) + (22.5 * 2_f64.powi(rank)).floor() as i64 + 200 * rerolls - 7;
        properties.set_property_value("endo", endo);
    }

    // ----------------- Kuva Info -----------------
    if operations.has("KuvaInfo") {
        const COSTS: [i64; 9] = [900, 1000, 1200, 1400, 1700, 2000, 2350, 2750, 3150];

        let kuva = (0..rerolls as usize)
            .map(|i| COSTS.get(i).copied().unwrap_or(3500))
            .sum::<i64>();
        properties.set_property_value("kuva", kuva);
    }

    // ----------------- Market Populated Info -----------------
    if operations.has("MarketPopulated") {
        properties.merge_properties(auction_properties.properties, true, true);
    }
    properties.set_property_value("attributes", scale_attributes(&attributes, 1.0, rank));
    properties.set_property_value("ui_operations", operations.operations.clone());
    Ok(vec![])
}

#[cfg(test)]
mod tests {
    use super::paginate;

    /// Stand-in for a page of inventory rows: 1..=n, so the value of an item
    /// is also its 1-based position and a wrong slice is obvious on sight.
    fn rows(n: i64) -> Vec<i64> {
        (1..=n).collect()
    }

    /// The common case. Every paginated table in the app walks middle pages,
    /// and an off-by-one here shows the user the neighbouring page's rows
    /// without any visible sign that anything is wrong.
    #[test]
    fn a_middle_page_returns_exactly_its_own_slice() {
        let page = paginate(&rows(25), 2, 10);
        assert_eq!(page.results, vec![11, 12, 13, 14, 15, 16, 17, 18, 19, 20]);
        assert_eq!(page.page, 2);
        assert_eq!(page.limit, 10);
        assert_eq!(page.total, 25);
        assert_eq!(page.total_pages, 3);
    }

    /// 25 rows over 10 per page leaves a short final page. Reading past the
    /// end of the slice would panic and take the whole tab down, so the end
    /// of the range has to be clamped to the item count.
    #[test]
    fn the_last_page_is_short_when_the_total_does_not_divide_evenly() {
        let page = paginate(&rows(25), 3, 10);
        assert_eq!(page.results, vec![21, 22, 23, 24, 25]);
        assert_eq!(page.total, 25);
        assert_eq!(page.total_pages, 3);
    }

    /// An exact multiple must not round up to a phantom extra page. 1500 rows
    /// at 25 per page is 60 pages, not 61, and a 61st page would render empty.
    #[test]
    fn an_exactly_divisible_total_does_not_gain_a_trailing_empty_page() {
        let page = paginate(&rows(20), 2, 10);
        assert_eq!(page.results, vec![11, 12, 13, 14, 15, 16, 17, 18, 19, 20]);
        assert_eq!(page.total, 20);
        assert_eq!(page.total_pages, 2);
    }

    /// `per_page == -1` is the app's "no limit" sentinel, used by every caller
    /// that wants the full set in one shot. It must return every row, not a
    /// slice, and report a single page rather than a negative page count.
    #[test]
    fn a_per_page_of_minus_one_returns_every_item_as_one_page() {
        let page = paginate(&rows(25), 1, -1);
        assert_eq!(page.results, rows(25));
        assert_eq!(page.limit, -1);
        assert_eq!(page.total, 25);
        assert_eq!(page.total_pages, 1);
    }

    /// "No limit" ignores the page number entirely: asking for page 2 of an
    /// unlimited query still hands back everything rather than an empty
    /// second page. Pinned because a stale page number in the UI state would
    /// otherwise silently empty a tab.
    #[test]
    fn a_per_page_of_minus_one_ignores_the_page_number() {
        let page = paginate(&rows(25), 2, -1);
        assert_eq!(page.results, rows(25));
        assert_eq!(page.total, 25);
        assert_eq!(page.total_pages, 1);
    }

    /// A page limit wider than the collection is not an error: one page holds
    /// the lot. Guards against the end of the range running past the slice.
    #[test]
    fn a_page_size_larger_than_the_total_returns_everything_on_page_one() {
        let page = paginate(&rows(25), 1, 100);
        assert_eq!(page.results, rows(25));
        assert_eq!(page.total, 25);
        assert_eq!(page.total_pages, 1);
    }

    /// Asking for a page beyond the end yields no rows rather than panicking.
    /// This is reachable whenever a filter shrinks the result set while the
    /// user sits on a high page, so it has to degrade to an empty table with
    /// the real totals still attached for the pager to correct itself.
    #[test]
    fn a_page_past_the_end_returns_no_rows_but_keeps_the_real_totals() {
        let page = paginate(&rows(25), 10, 10);
        assert!(page.results.is_empty());
        assert_eq!(page.page, 10);
        assert_eq!(page.total, 25);
        assert_eq!(page.total_pages, 3);
    }

    /// The first page past a clean boundary. 20 rows at 10 per page has no
    /// third page, and the range would start exactly at the end of the slice;
    /// this must be empty rather than an out-of-range read.
    #[test]
    fn the_page_just_past_an_exact_boundary_is_empty() {
        let page = paginate(&rows(20), 3, 10);
        assert!(page.results.is_empty());
        assert_eq!(page.total_pages, 2);
    }

    /// Pages are 1-based, so page 0 is a caller mistake. It yields nothing
    /// rather than wrapping round to the last page or reading backwards off
    /// the front of the slice.
    #[test]
    fn page_zero_returns_no_rows_rather_than_wrapping_around() {
        let page = paginate(&rows(25), 0, 10);
        assert!(page.results.is_empty());
        assert_eq!(page.page, 0);
        assert_eq!(page.total, 25);
        assert_eq!(page.total_pages, 3);
    }

    /// A negative page computes a negative start offset. Casting that to a
    /// slice index would be a vast unsigned number and an instant panic, so
    /// the guard has to reject it before the cast is used.
    #[test]
    fn a_negative_page_returns_no_rows_rather_than_panicking() {
        let page = paginate(&rows(25), -1, 10);
        assert!(page.results.is_empty());
        assert_eq!(page.page, -1);
        assert_eq!(page.total, 25);
        assert_eq!(page.total_pages, 3);
    }

    /// An empty inventory is the first thing a new user sees. No rows, no
    /// pages, and no panic from slicing an empty collection.
    #[test]
    fn an_empty_input_reports_no_items_and_no_pages() {
        let page = paginate::<i64>(&[], 1, 10);
        assert!(page.results.is_empty());
        assert_eq!(page.total, 0);
        assert_eq!(page.total_pages, 0);
    }

    /// An unlimited query over an empty collection reports one (empty) page
    /// rather than zero, because the -1 sentinel short-circuits the page
    /// count. Pinned deliberately: it disagrees with the limited case above,
    /// so any UI that trusts `total_pages` has to cope with both.
    #[test]
    fn an_empty_input_with_no_limit_still_reports_a_single_page() {
        let page = paginate::<i64>(&[], 1, -1);
        assert!(page.results.is_empty());
        assert_eq!(page.total, 0);
        assert_eq!(page.total_pages, 1);
    }

    /// Walking every page of a collection must visit each row exactly once,
    /// in order. This is the property an off-by-one at either edge of the
    /// range breaks, and the one a user notices as a missing or duplicated
    /// item somewhere in a 60-page tab.
    #[test]
    fn walking_every_page_in_order_reproduces_the_whole_collection() {
        let all = rows(97);
        let per_page = 10;
        let total_pages = paginate(&all, 1, per_page).total_pages;
        assert_eq!(total_pages, 10);

        let mut seen = Vec::new();
        for page in 1..=total_pages {
            seen.extend(paginate(&all, page, per_page).results);
        }
        assert_eq!(seen, all);
    }
}
