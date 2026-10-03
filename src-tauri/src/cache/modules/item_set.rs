use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use utils::{info, warning, Error, LoggerOptions};

use crate::cache::*;

static COMPONENT: &str = "Cache:ItemSet";

#[derive(Debug)]
pub struct ItemSetModule {
    sets: Mutex<Vec<CacheItemSet>>,
    /// member unique name -> indices into `sets`
    by_member: Mutex<HashMap<String, Vec<usize>>>,
}

impl ItemSetModule {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            sets: Mutex::new(Vec::new()),
            by_member: Mutex::new(HashMap::new()),
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

            let mut members = vec![CacheItemSetMember {
                unique_name: recipe.base.unique_name.clone(),
                name: recipe.base.name.clone(),
                required: 1,
                is_main_blueprint: true,
            }];
            for ingredient in &recipe.ingredients {
                if !ingredient.base.is_tradeable {
                    continue;
                }
                let key = member_key(ingredient);
                // Recipe ingredients carry no display name of their own, only
                // a uniqueName, so resolve it the way the Parts tab does.
                let name = client
                    .tradable_item()
                    .get_by(&key)
                    .map(|item| item.name)
                    .unwrap_or_else(|_| {
                        if ingredient.base.name.is_empty() {
                            key.rsplit('/').next().unwrap_or(&key).to_string()
                        } else {
                            ingredient.base.name.clone()
                        }
                    });
                members.push(CacheItemSetMember {
                    unique_name: key,
                    name,
                    required: ingredient.base.quantity,
                    is_main_blueprint: false,
                });
            }

            sets.push(CacheItemSet { set: item, members });
        }

        let mut by_member: HashMap<String, Vec<usize>> = HashMap::new();
        for (index, set) in sets.iter().enumerate() {
            for member in &set.members {
                by_member
                    .entry(member.unique_name.clone())
                    .or_default()
                    .push(index);
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

        *self.sets.lock().unwrap() = sets;
        *self.by_member.lock().unwrap() = by_member;
        Ok(())
    }

    pub fn get_all_sets(&self) -> Result<Vec<CacheItemSet>, Error> {
        Ok(self.sets.lock().unwrap().clone())
    }

    /// Every set this unique name is a member of. Normally zero or one, but
    /// the akimbo prime sets share the single-pistol blueprint, so three
    /// members belong to two sets each (issue #3).
    pub fn get_sets_for_member(&self, unique_name: &str) -> Vec<CacheItemSet> {
        let by_member = self.by_member.lock().unwrap();
        let sets = self.sets.lock().unwrap();
        by_member
            .get(unique_name)
            .map(|indices| {
                indices
                    .iter()
                    .filter_map(|i| sets.get(*i).cloned())
                    .collect()
            })
            .unwrap_or_default()
    }
}
