use serde::{Deserialize, Serialize};

use crate::wf_inventory::*;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct WarframeRootObject {
    #[serde(rename = "PlayerLevel", default)]
    pub mastery_rank: i64,

    #[serde(rename = "PremiumCredits", default)]
    pub platinum: i64,

    #[serde(rename = "RegularCredits", default)]
    pub credits: i64,

    #[serde(rename = "DailyAffiliation", default)]
    pub daily_affiliation_syndicate: i64,

    #[serde(rename = "DailyAffiliationPvp", default)]
    pub daily_affiliation_pvp: i64,

    #[serde(rename = "DailyAffiliationLibrary", default)]
    pub daily_affiliation_library: i64,

    #[serde(rename = "DailyAffiliationCetus", default)]
    pub daily_affiliation_cetus: i64,

    #[serde(rename = "DailyAffiliationQuills", default)]
    pub daily_affiliation_quills: i64,

    #[serde(rename = "DailyAffiliationVentkids", default)]
    pub daily_affiliation_ventkids: i64,

    #[serde(rename = "DailyAffiliationVox", default)]
    pub daily_affiliation_vox: i64,

    #[serde(rename = "DailyAffiliationEntrati", default)]
    pub daily_affiliation_entrati: i64,

    #[serde(rename = "DailyAffiliationZariman", default)]
    pub daily_affiliation_zariman: i64,

    #[serde(rename = "DailyAffiliationNecraloid", default)]
    pub daily_affiliation_necraloid: i64,

    #[serde(rename = "DailyAffiliationKahl", default)]
    pub daily_affiliation_kahl: i64,

    #[serde(rename = "DailyAffiliationCavia", default)]
    pub daily_affiliation_cavia: i64,

    #[serde(rename = "DailyAffiliationHex", default)]
    pub daily_affiliation_hex: i64,

    #[serde(rename = "TradesRemaining", default)]
    pub trades_remaining: i64,

    #[serde(rename = "RawUpgrades", default)]
    pub raw_upgrades: Vec<WFInvItemRaw>,

    #[serde(rename = "Upgrades", default)]
    pub upgrades: Vec<WFInvItemRaw>,

    #[serde(rename = "Recipes", default)]
    pub recipes: Vec<WFInvItemRaw>,

    #[serde(rename = "MiscItems", default)]
    pub misc_items: Vec<WFInvItemRaw>,

    #[serde(rename = "Affiliations", default)]
    pub affiliations: Vec<WFInvAffiliation>,
}

#[cfg(test)]
mod tests {
    use super::WarframeRootObject;

    #[test]
    fn parses_misc_items() {
        let json = r#"{"MiscItems":[{"ItemCount":3,"ItemType":"/Part/Barrel"}]}"#;
        let root: WarframeRootObject = serde_json::from_str(json).expect("should parse");
        assert_eq!(root.misc_items.len(), 1);
        assert_eq!(root.misc_items[0].unique_name, "/Part/Barrel");
        assert_eq!(root.misc_items[0].quantity, 3);
    }

    /// The Profile source and older AlecaFrame dumps may omit the key
    /// entirely. It must default to empty rather than failing the whole
    /// inventory parse and leaving every tab blank.
    #[test]
    fn treats_a_missing_misc_items_key_as_empty() {
        let root: WarframeRootObject =
            serde_json::from_str(r#"{"PlayerLevel":30}"#).expect("should parse");
        assert!(root.misc_items.is_empty());
        assert_eq!(root.mastery_rank, 30);
    }
}
