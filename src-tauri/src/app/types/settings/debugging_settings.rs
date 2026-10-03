use serde::{Deserialize, Serialize};

use crate::live_scraper::ItemEntry;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[derive(Default)]
pub struct DebuggingSettings {
    pub live_scraper: DebuggingLiveScraperSettings,
}


#[derive(Clone, Debug, Serialize, Deserialize)]
#[derive(Default)]
pub struct DebuggingLiveScraperSettings {
    pub entries: Vec<ItemEntry>,
    pub fake_orders: bool,
}

