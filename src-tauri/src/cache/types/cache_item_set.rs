use serde::{Deserialize, Serialize};

use crate::cache::{CacheIngredient, CacheTradableItem};

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
    use super::member_key;
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
}
