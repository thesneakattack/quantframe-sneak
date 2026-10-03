use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ApplicationEvent {
    // --------------------------------------------------
    // Appearance
    // --------------------------------------------------
    ThemeCreate,
    SwitchTheme,
    ThemeOpenFolder,

    // --------------------------------------------------
    // Authentication
    // --------------------------------------------------
    AuthLogin,
    AuthLogout,

    // --------------------------------------------------
    // General
    // --------------------------------------------------
    PageView,
    AppStart,
    AppExit,

    // --------------------------------------------------
    // Stock Item
    // --------------------------------------------------
    StockItemCreate,
    StockItemSell,
    StockItemDelete,
    StockItemUpdate,
    StockItemExport,

    // --------------------------------------------------
    // Stock Riven
    // --------------------------------------------------
    StockRivenCreate,
    StockRivenSell,
    StockRivenDelete,
    StockRivenUpdate,
    StockRivenExport,

    // --------------------------------------------------
    // Syndicate Item
    // --------------------------------------------------
    SyndicateItemImport,
    SyndicateItemCreate,
    SyndicateItemSell,
    SyndicateItemDelete,
    SyndicateItemUpdate,
    SyndicateItemExport,

    // --------------------------------------------------
    // Syndicate Price
    // --------------------------------------------------
    SyndicateItemPricesLookup,
    SyndicateItemPricesExport,

    // --------------------------------------------------
    // Trade Entry
    // --------------------------------------------------
    TradeEntryCreate,
    TradeEntryDelete,
    TradeEntryUpdate,
    TradeEntryExport,

    // --------------------------------------------------
    // Trade Detection
    // --------------------------------------------------
    TradeAccepted,
    TradeAcceptedFailed,
    TradeCancelled,
    TradeFailed,
    TradeUnknown,

    // --------------------------------------------------
    // Warframe GDPR
    // --------------------------------------------------
    WarframeGdprLoad,

    // --------------------------------------------------
    // Wish List
    // --------------------------------------------------
    WishListCreate,
    WishListBought,
    WishListDelete,
    WishListUpdate,
    WishListExport,

    // --------------------------------------------------
    // Transaction
    // --------------------------------------------------
    TransactionDelete,
    TransactionUpdate,
    TransactionExport,
    TransactionCalculateTax,

    // --------------------------------------------------
    // Warframe Market Auction's
    // --------------------------------------------------
    // These four carry consecutive capitals, which `rename_all = "snake_case"`
    // splits: WFMAuctionRefresh becomes "w_f_m_auction_refresh" and
    // WFInventoryUpdate becomes "w_f_inventory_update". The API accepts
    // "wfm_auction_refresh" and "wf_inventory_update", so every one of these
    // events was rejected with 400 - and because a failed flush re-queues its
    // batch, a single one of them stalled the whole analytics queue and retried
    // forever. Spelled out explicitly rather than relying on the derive.
    #[serde(rename = "wfm_auction_refresh")]
    WFMAuctionRefresh,
    #[serde(rename = "wfm_auction_import")]
    WFMAuctionImport,
    #[serde(rename = "wfm_auction_delete")]
    WFMAuctionDelete,

    // --------------------------------------------------
    // Warframe Market Chat's
    // --------------------------------------------------
    ChatRefresh,
    ChatDelete,
    ChatGetMessages,
    ChatSendMessage,
    ChatConversationDetected,

    // --------------------------------------------------
    // Debug
    // --------------------------------------------------
    DebugExportEeLogs,
    DebugGetWfmState,
    DebugTest,

    // --------------------------------------------------
    // Item Prices
    // --------------------------------------------------
    ItemPriceLookup,
    ItemPriceExport,
    RivenPriceLookup,
    RivenPriceExport,

    // --------------------------------------------------
    // Orders
    // --------------------------------------------------
    OrderRefresh,
    OrderDeleteAll,
    OrderDeleteById,

    // --------------------------------------------------
    // Sound
    // --------------------------------------------------
    SoundAddCustomSound,
    SoundDeleteCustomSound,

    // --------------------------------------------------
    // WF Inventory
    // --------------------------------------------------
    // See the note on WFMAuctionRefresh above: snake_case would emit
    // "w_f_inventory_update", which the API rejects.
    #[serde(rename = "wf_inventory_update")]
    WFInventoryUpdate,

    // --------------------------------------------------
    // Logs
    // --------------------------------------------------
    LogExport,

    // --------------------------------------------------
    // Handlers
    // --------------------------------------------------
    HandledItems,

    // --------------------------------------------------
    // Live Scraper
    // --------------------------------------------------
    LiveScraperStop,
    LiveScraperError,
}

#[cfg(test)]
mod tests {
    use super::ApplicationEvent;

    fn wire_name(event: ApplicationEvent) -> String {
        serde_json::to_value(event)
            .expect("event should serialize")
            .as_str()
            .expect("event should serialize to a string")
            .to_string()
    }

    /// `rename_all = "snake_case"` inserts a separator between consecutive
    /// capitals, so WFMAuctionRefresh would go out as "w_f_m_auction_refresh".
    /// The API rejects those with 400, and a rejected batch is re-queued, so one
    /// bad variant stalls the analytics queue indefinitely. These are the names
    /// the API actually accepts.
    #[test]
    fn acronym_events_use_the_names_the_api_accepts() {
        for (event, expected) in [
            (ApplicationEvent::WFMAuctionRefresh, "wfm_auction_refresh"),
            (ApplicationEvent::WFMAuctionImport, "wfm_auction_import"),
            (ApplicationEvent::WFMAuctionDelete, "wfm_auction_delete"),
            (ApplicationEvent::WFInventoryUpdate, "wf_inventory_update"),
        ] {
            assert_eq!(wire_name(event), expected);
        }
    }

    /// A serialized name should never contain a one-letter segment: that is the
    /// signature of an acronym having been split apart.
    #[test]
    fn no_event_name_has_a_split_acronym() {
        for event in [
            ApplicationEvent::WFMAuctionRefresh,
            ApplicationEvent::WFMAuctionImport,
            ApplicationEvent::WFMAuctionDelete,
            ApplicationEvent::WFInventoryUpdate,
        ] {
            let name = wire_name(event);
            let split = name.split('_').find(|segment| segment.len() == 1);
            assert!(
                split.is_none(),
                "{name:?} contains the one-letter segment {split:?}, which means an \
                 acronym was split and the API will reject it"
            );
        }
    }

    /// Ordinary variants are unaffected and should keep deriving their name.
    #[test]
    fn plain_events_still_derive_snake_case() {
        assert_eq!(wire_name(ApplicationEvent::AppStart), "app_start");
        assert_eq!(
            wire_name(ApplicationEvent::StockItemCreate),
            "stock_item_create"
        );
    }
}
