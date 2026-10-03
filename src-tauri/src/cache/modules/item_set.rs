use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use utils::{info, warning, Error, LoggerOptions};

use crate::cache::*;

static COMPONENT: &str = "Cache:ItemSet";

/// The sets and the member lookup into them.
///
/// One lock, not two: `by_member` holds indices into `sets`, so a reader that
/// caught the new sets against the old index would resolve a member to the
/// wrong set. Today `load` runs once during startup, before any command can
/// read, so that cannot happen - but the two must be published together for
/// it to stay true if a reload is ever added.
#[derive(Debug, Default)]
struct SetIndex {
    sets: Vec<CacheItemSet>,
    /// member unique name -> indices into `sets`
    by_member: HashMap<String, Vec<usize>>,
}

#[derive(Debug)]
pub struct ItemSetModule {
    index: Mutex<SetIndex>,
}

impl ItemSetModule {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            index: Mutex::new(SetIndex::default()),
        })
    }

    /// Joins tradable items tagged `set` to their blueprint recipe by
    /// `resultType`. Must run after `tradable_item()` and `recipe()` have
    /// loaded, since it reads both.
    pub fn load(&self, client: &CacheState) -> Result<(), Error> {
        let recipes = client.recipe().get_all_items()?;
        let mut by_result: HashMap<String, CacheRecipe> = HashMap::new();
        for recipe in recipes {
            if recipe.result_type.is_empty() {
                continue;
            }
            // One set resolves to more than one recipe; keep the first.
            by_result
                .entry(recipe.result_type.clone())
                .or_insert(recipe);
        }

        let mut sets: Vec<CacheItemSet> = Vec::new();
        for item in client.tradable_item().get_items()? {
            if !item.tags.iter().any(|tag| tag == "set") {
                continue;
            }
            let Some(recipe) = by_result.get(&item.unique_name) else {
                warning(
                    format!("{}:Load", COMPONENT),
                    format!("No recipe found for set '{}'", item.unique_name),
                    &LoggerOptions::default(),
                );
                continue;
            };

            // Recipe ingredients carry no display name of their own, only a
            // uniqueName. The tradable-items cache names most of them; the
            // rest are not tradable in their own right but are recipes, which
            // do carry a name.
            let display_name = |key: &str| -> String {
                let tradable = client.tradable_item().get_by(key).ok().map(|i| i.name);
                let recipe = client.recipe().get_by(key).ok().map(|r| r.base.name);
                member_display_name(key, tradable.as_deref(), recipe.as_deref())
            };

            let blueprint_key = main_blueprint_key(recipe);
            let mut members = vec![CacheItemSetMember {
                name: display_name(&blueprint_key),
                unique_name: blueprint_key,
                required: 1,
                is_main_blueprint: true,
            }];
            for (key, required) in aggregate_ingredients(recipe) {
                members.push(CacheItemSetMember {
                    name: display_name(&key),
                    unique_name: key,
                    required,
                    is_main_blueprint: false,
                });
            }

            sets.push(CacheItemSet { set: item, members });
        }

        let mut by_member: HashMap<String, Vec<usize>> = HashMap::new();
        for (index, set) in sets.iter().enumerate() {
            for member in &set.members {
                let entry: &mut Vec<usize> =
                    by_member.entry(member.unique_name.clone()).or_default();
                // A member can appear once per set even if the recipe listed
                // it twice; never report the same set twice for one member.
                if !entry.contains(&index) {
                    entry.push(index);
                }
            }
        }

        info(
            format!("{}:Load", COMPONENT),
            format!(
                "Indexed {} sets covering {} distinct members",
                sets.len(),
                by_member.len()
            ),
            &LoggerOptions::default(),
        );

        *self.index.lock().unwrap() = SetIndex { sets, by_member };
        Ok(())
    }

    pub fn get_all_sets(&self) -> Result<Vec<CacheItemSet>, Error> {
        Ok(self.index.lock().unwrap().sets.clone())
    }

    /// Every set this unique name is a member of. Normally zero or one, but
    /// the akimbo prime sets share the single-pistol blueprint, so three
    /// members belong to two sets each (issue #3).
    pub fn get_sets_for_member(&self, unique_name: &str) -> Vec<CacheItemSet> {
        let index = self.index.lock().unwrap();
        index
            .by_member
            .get(unique_name)
            .map(|positions| {
                positions
                    .iter()
                    .filter_map(|i| index.sets.get(*i).cloned())
                    .collect()
            })
            .unwrap_or_default()
    }
}
