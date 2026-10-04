use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct CacheRelics {
    #[serde(flatten)]
    pub base: super::cache_item_base::CacheItemBase,

    /// When this relic left circulation, absent while it still drops.
    ///
    /// Vault status is already in the shipped cache, so "can I still farm
    /// this?" needs no lookup against anything.
    #[serde(rename = "vaultedAt", default)]
    pub vaulted_at: Option<i64>,

    #[serde(rename = "relicRewards", default)]
    pub relic_rewards: Vec<CacheRelicReward>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct CacheRelicReward {
    /// Named with a `/Lotus/StoreItems/` prefix that inventory entries and
    /// the tradable-items cache do not use.
    #[serde(rename = "rewardName", default)]
    pub reward_name: String,

    #[serde(rename = "rarity", default)]
    pub rarity: String,

    #[serde(rename = "itemCount", default)]
    pub item_count: i64,
}
