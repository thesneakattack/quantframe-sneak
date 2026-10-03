use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::cache::{CacheIngredient, CacheRecipe, CacheTradableItem};

/// Which unique name the player actually holds for this ingredient.
///
/// Warframe parts are received as blueprints and live in the inventory's
/// `Recipes` bucket, so the ingredient carries a `fromRecipe` naming that
/// blueprint while its own `uniqueName` is the built component. Weapon parts
/// are received as built components, live in `MiscItems`, and carry no
/// `fromRecipe` at all.
///
/// The choice is per ingredient: four sets mix both kinds.
pub fn member_key(ingredient: &CacheIngredient) -> String {
    if ingredient.from_recipe.is_empty() {
        ingredient.base.unique_name.clone()
    } else {
        ingredient.from_recipe.clone()
    }
}

/// Which unique name the player holds for a recipe's own blueprint.
///
/// Three set recipes carry an `overrideUniqueName`, and in each the base
/// `uniqueName` is a synthetic `/WFSpecial/` path that appears in no inventory
/// bucket. `CacheRecipe::can_craft` has always honoured the override; the set
/// index must too, or an owned set reports as missing a part that cannot exist.
pub fn main_blueprint_key(recipe: &CacheRecipe) -> String {
    if recipe.override_unique_name.is_empty() {
        recipe.base.unique_name.clone()
    } else {
        recipe.override_unique_name.clone()
    }
}

/// A recipe's tradable ingredients as (member key, total required), with
/// repeats merged.
///
/// The akimbo prime recipes ask for two of the single-pistol blueprint by
/// listing the ingredient twice rather than setting `ItemCount`. Kept as two
/// members, one owned copy would satisfy both and the set would report as
/// complete when it cannot be built. Order follows first appearance.
pub fn aggregate_ingredients(recipe: &CacheRecipe) -> Vec<(String, i64)> {
    let mut order: Vec<String> = Vec::new();
    let mut required: HashMap<String, i64> = HashMap::new();
    for ingredient in &recipe.ingredients {
        if !ingredient.base.is_tradeable {
            continue;
        }
        let key = member_key(ingredient);
        match required.get_mut(&key) {
            Some(total) => *total += ingredient.base.quantity,
            None => {
                order.push(key.clone());
                required.insert(key, ingredient.base.quantity);
            }
        }
    }
    order
        .into_iter()
        .map(|key| {
            let total = required[&key];
            (key, total)
        })
        .collect()
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct CacheItemSetMember {
    pub unique_name: String,
    pub name: String,
    pub required: i64,
    pub is_main_blueprint: bool,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct CacheItemSet {
    pub set: CacheTradableItem,
    pub members: Vec<CacheItemSetMember>,
}

#[cfg(test)]
mod tests {
    use super::{aggregate_ingredients, main_blueprint_key, member_key};
    use crate::cache::CacheRecipe;
    use crate::cache::{CacheIngredient, CacheItemBase};

    fn ingredient(unique_name: &str, from_recipe: &str) -> CacheIngredient {
        CacheIngredient {
            base: CacheItemBase::new(unique_name, 1),
            from_recipe: from_recipe.to_string(),
        }
    }

    /// Warframe parts are owned as blueprints in the Recipes bucket, and the
    /// ingredient names the built component. Counting uniqueName would look for
    /// a component the player never holds.
    #[test]
    fn prefers_from_recipe_when_present() {
        let ing = ingredient(
            "/Lotus/Types/Recipes/WarframeRecipes/AshPrimeHelmetComponent",
            "/Lotus/Types/Recipes/WarframeRecipes/AshPrimeHelmetBlueprint",
        );
        assert_eq!(
            member_key(&ing),
            "/Lotus/Types/Recipes/WarframeRecipes/AshPrimeHelmetBlueprint"
        );
    }

    /// Weapon parts are owned as built components in MiscItems and carry no
    /// fromRecipe at all. Returning an empty key here is what makes the
    /// existing can_craft miss every weapon set.
    #[test]
    fn falls_back_to_unique_name_when_from_recipe_is_empty() {
        let ing = ingredient(
            "/Lotus/Types/Recipes/Weapons/WeaponParts/LatronPrimeBarrel",
            "",
        );
        assert_eq!(
            member_key(&ing),
            "/Lotus/Types/Recipes/Weapons/WeaponParts/LatronPrimeBarrel"
        );
    }

    /// Four of the 234 sets mix both kinds, so the choice is per ingredient and
    /// never per recipe.
    #[test]
    fn decides_per_ingredient_not_per_recipe() {
        let mixed = [
            ingredient("/Component", "/Blueprint"),
            ingredient("/BarePart", ""),
        ];
        let keys: Vec<String> = mixed.iter().map(member_key).collect();
        assert_eq!(
            keys,
            vec!["/Blueprint".to_string(), "/BarePart".to_string()]
        );
    }

    fn recipe(override_name: &str, ingredients: Vec<CacheIngredient>) -> CacheRecipe {
        let mut base = CacheItemBase::new("/Set/MainBlueprint", 1);
        base.is_tradeable = true;
        CacheRecipe {
            base,
            result_type: "/Set/Result".to_string(),
            override_unique_name: override_name.to_string(),
            ingredients,
        }
    }

    fn tradable(unique_name: &str, from_recipe: &str, quantity: i64) -> CacheIngredient {
        let mut base = CacheItemBase::new(unique_name, quantity);
        base.is_tradeable = true;
        CacheIngredient {
            base,
            from_recipe: from_recipe.to_string(),
        }
    }

    /// `can_craft` has always honoured overrideUniqueName for the main
    /// blueprint. Three sets have one, and in each the base uniqueName is a
    /// synthetic /WFSpecial/ path that appears in no inventory bucket, so
    /// ignoring it reports an owned set as permanently missing a part.
    #[test]
    fn main_blueprint_key_prefers_the_override() {
        let r = recipe("/Real/WeaponPod", vec![]);
        assert_eq!(main_blueprint_key(&r), "/Real/WeaponPod");
    }

    #[test]
    fn main_blueprint_key_falls_back_to_the_base_unique_name() {
        let r = recipe("", vec![]);
        assert_eq!(main_blueprint_key(&r), "/Set/MainBlueprint");
    }

    /// The akimbo prime recipes ask for two of the single-pistol blueprint by
    /// listing the ingredient twice rather than setting ItemCount. Treating
    /// those as two independent members lets one copy satisfy both, so a set
    /// the player cannot build reports as complete.
    #[test]
    fn aggregate_ingredients_sums_a_repeated_ingredient() {
        let r = recipe(
            "",
            vec![
                tradable("/PrimeLex", "/LexPrimeBlueprint", 1),
                tradable("/PrimeLex", "/LexPrimeBlueprint", 1),
                tradable("/AklexPrimeLink", "", 1),
            ],
        );
        let got = aggregate_ingredients(&r);
        assert_eq!(
            got,
            vec![
                ("/LexPrimeBlueprint".to_string(), 2),
                ("/AklexPrimeLink".to_string(), 1),
            ]
        );
    }

    #[test]
    fn aggregate_ingredients_keeps_item_count_and_skips_untradable() {
        let mut untradable = CacheItemBase::new("/OrokinCell", 15);
        untradable.is_tradeable = false;
        let r = recipe(
            "",
            vec![
                tradable("/Part/Handle", "", 2),
                CacheIngredient {
                    base: untradable,
                    from_recipe: String::new(),
                },
            ],
        );
        assert_eq!(
            aggregate_ingredients(&r),
            vec![("/Part/Handle".to_string(), 2)]
        );
    }
}
