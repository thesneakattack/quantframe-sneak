use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use std::collections::{HashMap, HashSet};

use utils::{Error, SubType};

use crate::utils::modules::states;
use crate::wf_inventory::{
    is_arcane, is_relic,
    item_base::WFInvItemBase,
    modules::item::{merge_rank_rows, owned_counts, rank_groups, variant_of},
    WFInvItemRaw, WFInvSet, WFInvSetMember, WarframeRootObject,
};

/// Every row of every inventory tab, derived once per inventory.
///
/// Building a row is expensive: each one needs a lookup in the tradable-items
/// cache, which takes a mutex and clones the item, and the Parts rows also
/// walk the set index. Doing that per request meant a sort click paid for the
/// whole inventory - roughly three thousand mutex-guarded clones to render
/// twenty-five rows - while contending with the live scraper for the same
/// lock.
///
/// None of it depends on the request. It depends on the inventory, which
/// changes only when the source refreshes it, so it is derived there instead
/// and queries just filter, sort and page over these vectors.
#[derive(Debug, Default)]
pub struct InventorySnapshot {
    pub parts: Vec<WFInvItemBase>,
    /// One row per relic *and refinement*: that is how the inventory stores
    /// them and how warframe.market prices them. The Relics tab groups them
    /// for reading, at query time, once prices are known.
    pub relics: Vec<WFInvItemBase>,
    pub mods: Vec<WFInvItemBase>,
    /// Arcanes, kept out of Mods: they rank to 5 rather than 10, are priced
    /// per rank, and are the things actually worth listing in that bucket.
    pub arcanes: Vec<WFInvItemBase>,
    pub sets: Vec<WFInvSet>,
}

impl InventorySnapshot {
    /// Derive every tab's rows from one inventory.
    pub fn build(root: &WarframeRootObject) -> Result<Self, Error> {
        let cache = states::cache_client()?;
        let tradable = cache.tradable_item();
        let item_set = cache.item_set();
        let unvaulted = unvaulted_items(&cache);
        let mastered = mastered_items(root);

        // Parts: the Recipes and MiscItems buckets, kept where the
        // tradable-items cache knows the entry.
        let mut parts: Vec<WFInvItemBase> = Vec::new();
        let mut relics: Vec<WFInvItemBase> = Vec::new();
        for (unique_name, quantity) in owned_counts(&[&root.recipes, &root.misc_items]) {
            let Ok(item) = tradable.get_by(&unique_name) else {
                continue;
            };
            let parent_sets = item_set.get_sets_for_member(&unique_name);
            let variant = variant_of(&item, &unique_name);
            let mut row = WFInvItemBase {
                id: item.wfm_id.clone(),
                name: item.name.clone(),
                wfm_url: item.wfm_url.clone(),
                quantity,
                sub_type: variant.map(|variant| SubType {
                    variant: Some(variant),
                    ..Default::default()
                }),
                ..Default::default()
            };
            row.properties.set_property_value(
                "in_sets",
                parent_sets
                    .iter()
                    .map(|s| s.set.name.clone())
                    .collect::<Vec<_>>(),
            );
            // Names are translated and stock rows store whatever language was
            // active when they were created, so conflicts match on wfm_url.
            row.properties.set_property_value(
                "in_set_urls",
                parent_sets
                    .iter()
                    .map(|s| s.set.wfm_url.clone())
                    .collect::<Vec<_>>(),
            );
            row.properties.set_property_value("tags", item.tags.clone());
            row.properties
                .set_property_value("is_unvaulted", unvaulted.contains(&unique_name));
            row.properties.set_property_value(
                "is_mastered",
                builds_something_mastered(
                    parent_sets.iter().map(|s| s.set.unique_name.as_str()),
                    &mastered,
                ),
            );
            row.unique_name = unique_name;
            // Relics share the Recipes/MiscItems buckets with parts but are a
            // different thing to trade: they carry a refinement rather than a
            // parent set, so they get their own tab rather than diluting Parts.
            if is_relic(&item.tags) {
                relics.push(row);
            } else {
                parts.push(row);
            }
        }

        // Mods: unranked stacks plus ranked instances grouped by rank.
        let mut rows: Vec<(String, i64, i64)> = Vec::new();
        for (unique_name, quantity) in owned_counts_excluding_rivens(&root.raw_upgrades) {
            rows.push((unique_name, 0, quantity));
        }
        for ((unique_name, rank), quantity) in rank_groups(&root.upgrades) {
            rows.push((unique_name, rank, quantity));
        }
        let mut mods: Vec<WFInvItemBase> = Vec::new();
        let mut arcanes: Vec<WFInvItemBase> = Vec::new();
        for (unique_name, rank, quantity) in merge_rank_rows(rows) {
            let Ok(item) = tradable.get_by(&unique_name) else {
                continue;
            };
            let variant = variant_of(&item, &unique_name);
            let mut row = WFInvItemBase {
                id: item.wfm_id.clone(),
                name: item.name.clone(),
                wfm_url: item.wfm_url.clone(),
                quantity,
                // Unranked rows carry no rank at all, matching how stock
                // represents an unranked item.
                sub_type: if rank > 0 || variant.is_some() {
                    Some(SubType {
                        rank: (rank > 0).then_some(rank),
                        variant,
                        ..Default::default()
                    })
                } else {
                    None
                },
                ..Default::default()
            };
            row.properties.set_property_value("tags", item.tags.clone());
            row.properties
                .set_property_value("max_rank", item.sub_type.as_ref().and_then(|s| s.max_rank));
            row.properties
                .set_property_value("is_unvaulted", unvaulted.contains(&unique_name));
            row.properties
                .set_property_value("is_mastered", mastered.contains(&unique_name));
            row.unique_name = unique_name;
            if is_arcane(&item.tags) {
                arcanes.push(row);
            } else {
                mods.push(row);
            }
        }

        // Sets: completion counted across the same two buckets.
        let counts = owned_counts(&[&root.recipes, &root.misc_items]);
        let mut sets: Vec<WFInvSet> = Vec::new();
        for cache_set in item_set.get_all_sets()? {
            let members: Vec<WFInvSetMember> = cache_set
                .members
                .iter()
                .map(|member| WFInvSetMember {
                    unique_name: member.unique_name.clone(),
                    name: member.name.clone(),
                    have: counts.get(&member.unique_name).copied().unwrap_or(0),
                    required: member.required,
                    is_main_blueprint: member.is_main_blueprint,
                })
                .collect();
            if members.iter().all(|m| m.have == 0) {
                continue;
            }
            let copies = crate::wf_inventory::complete_copies(&members);
            let mut base = WFInvItemBase {
                id: cache_set.set.wfm_id.clone(),
                name: cache_set.set.name.clone(),
                unique_name: cache_set.set.unique_name.clone(),
                wfm_url: cache_set.set.wfm_url.clone(),
                quantity: copies,
                sub_type: None,
                ..Default::default()
            };
            base.properties
                .set_property_value("tags", cache_set.set.tags.clone());
            // A set counts as unvaulted when any member still drops.
            base.properties.set_property_value(
                "is_unvaulted",
                members.iter().any(|m| unvaulted.contains(&m.unique_name)),
            );
            base.properties
                .set_property_value("is_mastered", mastered.contains(&cache_set.set.unique_name));
            sets.push(WFInvSet {
                base,
                total_members: members.len() as i64,
                owned_members: crate::wf_inventory::satisfied_members(&members),
                complete_copies: copies,
                members,
            });
        }

        Ok(Self {
            parts,
            relics,
            mods,
            arcanes,
            sets,
        })
    }
}

/// `owned_counts` over the unranked bucket, minus rivens.
///
/// Rivens have their own tab and their own stock type, so letting them
/// through would list one inventory as two incompatible things.
fn owned_counts_excluding_rivens(raw_upgrades: &[WFInvItemRaw]) -> HashMap<String, i64> {
    crate::wf_inventory::modules::item::owned_counts_where(&[raw_upgrades], |e| !e.is_riven())
}

/// Every item still dropping from at least one unvaulted relic.
///
/// Relics carry `vaultedAt` and list what they drop, so vault status is
/// already in the shipped cache - no lookup needed. Rewards are named with a
/// `/Lotus/StoreItems/` prefix that inventory entries do not use.
fn unvaulted_items(cache: &crate::cache::CacheState) -> HashSet<String> {
    let mut open = HashSet::new();
    for relic in cache.relics().get_all_items().unwrap_or_default() {
        if relic.vaulted_at.is_some() {
            continue;
        }
        for reward in &relic.relic_rewards {
            open.insert(reward.reward_name.replace("/Lotus/StoreItems/", "/Lotus/"));
            open.insert(reward.reward_name.clone());
        }
    }
    open
}

/// Every item the player has already levelled to rank 30.
///
/// Mastery is earned once per item type, so an item already mastered is one
/// whose only remaining value is its sale price.
fn mastered_items(root: &WarframeRootObject) -> HashSet<String> {
    root.xp_info
        .iter()
        .filter(|entry| entry.xp >= mastery_cap(&entry.item_type))
        .map(|entry| entry.item_type.clone())
        .collect()
}

/// Whether the item a part builds has already been mastered.
///
/// A part's own unique name is a recipe path - `/Lotus/Types/Recipes/...` -
/// and mastery is recorded against the equippable item, so the two never
/// meet: matching a part against the mastery table directly leaves every part
/// unmastered. A set is keyed by exactly that equippable item, so the sets a
/// part belongs to are the bridge between the two.
fn builds_something_mastered<'a>(
    parent_set_keys: impl IntoIterator<Item = &'a str>,
    mastered: &HashSet<String>,
) -> bool {
    parent_set_keys
        .into_iter()
        .any(|key| mastered.contains(key))
}

/// The experience a type needs for rank 30. Warframes, companions and
/// archwings level half as fast as weapons, so they need twice the total.
fn mastery_cap(unique_name: &str) -> i64 {
    const WEAPON_CAP: i64 = 450_000;
    const FRAME_CAP: i64 = 900_000;
    if unique_name.contains("/Powersuits/")
        || unique_name.contains("/Sentinels/")
        || unique_name.contains("/Archwing/")
    {
        FRAME_CAP
    } else {
        WEAPON_CAP
    }
}

/// Identifies the contents of an inventory.
///
/// The three inventory sources all publish by replacing the root wholesale, so
/// there is no change signal to hook. Hashing what the rows are actually built
/// from is cheap next to building them - a few thousand string hashes against
/// a few thousand cache lookups and clones - and it cannot go stale, which a
/// manually bumped counter on three separate code paths could.
pub fn root_fingerprint(root: &WarframeRootObject) -> u64 {
    fn hash_bucket(entries: &[WFInvItemRaw], state: &mut DefaultHasher) {
        entries.len().hash(state);
        for entry in entries {
            entry.unique_name.hash(state);
            entry.quantity.hash(state);
            // Rank lives in the fingerprint, not just the count: ranking a mod
            // changes the rows without changing any quantity.
            entry.upgrade_fingerprint.hash(state);
        }
    }

    let mut state = DefaultHasher::new();
    hash_bucket(&root.raw_upgrades, &mut state);
    hash_bucket(&root.upgrades, &mut state);
    hash_bucket(&root.recipes, &mut state);
    hash_bucket(&root.misc_items, &mut state);
    state.finish()
}

#[cfg(test)]
mod tests {
    use super::{builds_something_mastered, root_fingerprint, WarframeRootObject};
    use crate::wf_inventory::WFInvItemRaw;
    use std::collections::HashSet;

    fn raw(unique_name: &str, quantity: i64, fingerprint: Option<&str>) -> WFInvItemRaw {
        WFInvItemRaw {
            id: Default::default(),
            quantity,
            unique_name: unique_name.to_string(),
            upgrade_fingerprint: fingerprint.map(str::to_string),
            last_added: Default::default(),
            xp: 0,
        }
    }

    fn root(recipes: Vec<WFInvItemRaw>, upgrades: Vec<WFInvItemRaw>) -> WarframeRootObject {
        WarframeRootObject {
            recipes,
            upgrades,
            ..Default::default()
        }
    }

    #[test]
    fn the_same_inventory_fingerprints_the_same() {
        let a = root(vec![raw("/A", 2, None)], vec![]);
        let b = root(vec![raw("/A", 2, None)], vec![]);
        assert_eq!(root_fingerprint(&a), root_fingerprint(&b));
    }

    /// Acquiring or spending an item has to invalidate the snapshot, or the
    /// tabs keep showing counts the player no longer has.
    #[test]
    fn a_changed_quantity_changes_the_fingerprint() {
        let before = root(vec![raw("/A", 2, None)], vec![]);
        let after = root(vec![raw("/A", 3, None)], vec![]);
        assert_ne!(root_fingerprint(&before), root_fingerprint(&after));
    }

    #[test]
    fn a_new_item_changes_the_fingerprint() {
        let before = root(vec![raw("/A", 1, None)], vec![]);
        let after = root(vec![raw("/A", 1, None), raw("/B", 1, None)], vec![]);
        assert_ne!(root_fingerprint(&before), root_fingerprint(&after));
    }

    /// Ranking a mod moves it between rows without changing any quantity, so
    /// the fingerprint has to notice the rank too.
    #[test]
    fn ranking_a_mod_changes_the_fingerprint() {
        let before = root(vec![], vec![raw("/Mod", 1, Some(r#"{"lvl":0}"#))]);
        let after = root(vec![], vec![raw("/Mod", 1, Some(r#"{"lvl":10}"#))]);
        assert_ne!(root_fingerprint(&before), root_fingerprint(&after));
    }

    /// Two buckets holding the same entry must not cancel out.
    #[test]
    fn the_bucket_an_item_sits_in_is_part_of_the_identity() {
        let in_recipes = root(vec![raw("/A", 1, None)], vec![]);
        let in_upgrades = root(vec![], vec![raw("/A", 1, None)]);
        assert_ne!(
            root_fingerprint(&in_recipes),
            root_fingerprint(&in_upgrades)
        );
    }

    /// The defect this guards: a part was matched against the mastery table
    /// by its own unique name, which is a recipe path and never appears
    /// there, so "already mastered" matched nothing at all.
    #[test]
    fn a_part_counts_as_mastered_when_the_item_it_builds_is() {
        let mastered: HashSet<String> = ["/Lotus/Powersuits/Jade/NyxPrime".to_string()]
            .into_iter()
            .collect();
        assert!(builds_something_mastered(
            ["/Lotus/Powersuits/Jade/NyxPrime"],
            &mastered
        ));
    }

    #[test]
    fn a_part_of_nothing_mastered_does_not_count() {
        let mastered: HashSet<String> = ["/Lotus/Powersuits/Jade/NyxPrime".to_string()]
            .into_iter()
            .collect();
        assert!(!builds_something_mastered(
            ["/Lotus/Powersuits/Mag/MagPrime"],
            &mastered
        ));
        assert!(!builds_something_mastered([], &mastered));
    }

    /// A part can belong to more than one set; mastering any of them means
    /// the part's only remaining value is its sale price.
    #[test]
    fn one_mastered_parent_set_is_enough() {
        let mastered: HashSet<String> = ["/Lotus/Powersuits/Jade/NyxPrime".to_string()]
            .into_iter()
            .collect();
        assert!(builds_something_mastered(
            [
                "/Lotus/Weapons/Tenno/Rifle/SomethingElse",
                "/Lotus/Powersuits/Jade/NyxPrime"
            ],
            &mastered
        ));
    }
}
