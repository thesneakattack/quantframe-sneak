use serde::{Deserialize, Serialize};

use crate::wf_inventory::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WFInvItemRaw {
    #[serde(rename = "ItemId", default)]
    pub id: ItemId,

    #[serde(rename = "ItemCount", default)]
    pub quantity: i64,

    #[serde(rename = "ItemType", default)]
    pub unique_name: String,

    #[serde(rename = "UpgradeFingerprint", skip_serializing_if = "Option::is_none")]
    pub upgrade_fingerprint: Option<String>,

    #[serde(rename = "LastAdded", default)]
    pub last_added: ItemId,

    #[serde(rename = "XP", default)]
    pub xp: i64,
}

impl WFInvItemRaw {
    pub fn is_riven(&self) -> bool {
        if !self
            .unique_name
            .starts_with("/Lotus/Upgrades/Mods/Randomized/")
        {
            return false;
        }
        // NOTE: the branch chain that used to live here was unconditionally true.
        // It tested (!unveiled && id.is_some()), then (id.is_none()), then (unveiled),
        // which between them cover all four combinations of those two booleans, so
        // every path returned true and the trailing `false` was unreachable. Removing
        // it is behaviour-preserving, but the dead branches suggest one of them was
        // meant to return false - if is_riven() should be stricter than the prefix
        // check alone, that intent was never actually implemented.
        true
    }
    // Unused today but part of this type's intended surface; kept rather than
    // deleted so the capability is not silently lost.
    #[allow(dead_code)]
    pub fn is_arcane(&self) -> bool {
        !self.unique_name.contains("/CosmeticEnhancers/Peculiars/")
            && self
                .unique_name
                .contains("/Lotus/Upgrades/CosmeticEnhancers")
    }
    // Unused today but part of this type's intended surface; kept rather than
    // deleted so the capability is not silently lost.
    #[allow(dead_code)]
    pub fn is_mod(&self) -> bool {
        !self.unique_name.contains("/Beginner/")
            && (self.unique_name.contains("/CosmeticEnhancers/Peculiars/")
                || self.unique_name.contains("/Lotus/Upgrades/Mods/Railjack/")
                || !self.is_arcane())
    }
    pub fn get_upgrade_fingerprint(&self) -> UpgradeFingerprint {
        if self.upgrade_fingerprint.is_none() {
            return UpgradeFingerprint::default();
        }
        UpgradeFingerprint::from(self.upgrade_fingerprint.as_ref().unwrap())
    }
}
