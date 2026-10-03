# WF Inventory Parts, Mods and Sets Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add Parts, Mods and Sets tabs to the WF Inventory page so a part, mod, arcane or complete set can be listed to Stock directly from what the game says you own.

**Architecture:** A new cache-layer module joins `TradableItems.json` (items tagged `set`) to `Recipes.json` (by `resultType`) to produce a set index. Two `wf_inventory` modules project the live inventory root through that index and the tradable-items cache into paginated rows. Three Tauri commands expose them, stamping stock status. Three React tabs follow the existing Rivens shape and list to stock through the existing `stock_item_create`.

**Tech Stack:** Rust (Tauri 2, SeaORM, serde), React 19, Mantine 9, TanStack Query, react-i18next, pnpm, DDEV.

**Spec:** `docs/superpowers/specs/2026-10-03-wf-inventory-parts-mods-sets-design.md`

## Global Constraints

- All gates must pass before any commit is considered done: `ddev check` runs eslint, `pnpm build`, `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --all-targets`, `cargo test --workspace --doc`.
- Clippy warnings are errors. Do not add `#[allow(...)]` to silence a new lint without a comment explaining why the lint is wrong here.
- Run all Rust and pnpm commands inside the container: `ddev exec '<command>'`. Never run `cargo` or `pnpm` on the host.
- Every new `pub` item that is not yet called must be genuinely called by the end of its own task, or the task is not done. Do not leave dead code behind `#[allow(dead_code)]`.
- Tradability is determined **only** by presence in `TradableItems.json` via `TradableItemModule::get_by`. Never classify by `uniqueName` path prefix, and never use `CacheTradableItem.category` (it is uniformly `"TradableItems"`).
- The set member key is `fromRecipe` when non-empty, else `uniqueName`, decided **per ingredient**. This rule must exist in exactly one function.
- Translations: add new keys to `public/lang/en.json` only. The other 13 locale files fall back to English and are not hand-edited.
- The WF Inventory nav entry stays behind its existing dev-mode gate in `src/components/Layouts/LogIn/index.tsx`. Do not unhide it.
- Issue #3 (shared akimbo blueprints double-counting across two sets) is explicitly out of scope. Do not attempt to resolve set contention.

## Review Focus

These are the failure modes the spec implies that the happy-path tests would not catch. Each has a test assigned to the task that owns the code.

1. **An inventory root with no `MiscItems` key at all** — the Profile source and older AlecaFrame dumps may omit it; it must deserialize to an empty vec, not fail the whole inventory parse. (Task 3)
2. **A set recipe with zero tradable ingredients, or an ingredient with `ItemCount` of 0** — `min()` over an empty iterator and division by zero both panic, taking down the Sets tab. (Task 5)
3. **The same `ItemType` appearing twice in one inventory bucket** — counts must sum, not overwrite, or owned quantities silently read low. (Task 3)
4. **A stock item with `sub_type: None` versus `Some(SubType { rank: Some(0) })`** — these mean the same thing and must match, or an unranked mod already in stock shows a red "not in stock" badge. (Task 6)
5. **A ranked upgrade whose `UpgradeFingerprint` is absent or malformed JSON** — `UpgradeFingerprint::from` falls back to `default()` (rank 0); grouping must not panic and must not merge the malformed entry into the genuine unranked stack. (Task 4)

---

## File Structure

**Create:**
- `src-tauri/src/cache/types/cache_item_set.rs` — `CacheItemSet`, `CacheItemSetMember`, and the one member-key function
- `src-tauri/src/cache/modules/item_set.rs` — `ItemSetModule`: builds the set index and the reverse member lookup
- `src-tauri/src/wf_inventory/types/item_set.rs` — `WFInvSet`, `WFInvSetMember`
- `src-tauri/src/wf_inventory/modules/item_set.rs` — `SetsModule`: set completion against the live inventory
- `src/pages/wf_inventory/Tabs/Parts/{index.tsx,queries.ts,mutations.ts,modals.tsx}`
- `src/pages/wf_inventory/Tabs/Mods/{index.tsx,queries.ts,mutations.ts,modals.tsx}`
- `src/pages/wf_inventory/Tabs/Sets/{index.tsx,queries.ts,mutations.ts,modals.tsx}`

**Modify:**
- `src-tauri/src/cache/types/mod.rs`, `src-tauri/src/cache/modules/mod.rs` — exports
- `src-tauri/src/cache/client.rs` — `ItemSetModule` field, accessor, and load ordering
- `src-tauri/src/cache/types/cache_recipe.rs` — `can_craft` per-ingredient key
- `src-tauri/src/log_parser/types/trade.rs:143-150` — collapse the two-pass workaround
- `src-tauri/src/wf_inventory/types/wf_root_object.rs` — add `misc_items`
- `src-tauri/src/wf_inventory/types/mod.rs`, `src-tauri/src/wf_inventory/modules/mod.rs` — exports
- `src-tauri/src/wf_inventory/modules/item.rs` — fill the stub
- `src-tauri/src/wf_inventory/client.rs` — expose `item()` and `sets()`
- `src-tauri/src/commands/wf_inventory.rs` — three commands
- `src-tauri/src/lib.rs:384` — register the commands
- `src/components/Modals/Prompt/index.tsx` — optional `message`
- `src/api/wf_inventory/index.ts` — three methods
- `src/types/tauri.type.ts:950-987` — row and response types
- `src/pages/wf_inventory/index.tsx:14` — three tabs
- `public/lang/en.json` — new keys
- `NOTICE.md` — record the `can_craft` behaviour change

---

## Task 1: Set index in the cache layer

**Files:**

- Create: `src-tauri/src/cache/types/cache_item_set.rs`
- Create: `src-tauri/src/cache/modules/item_set.rs`
- Modify: `src-tauri/src/cache/types/mod.rs`
- Modify: `src-tauri/src/cache/modules/mod.rs`
- Modify: `src-tauri/src/cache/client.rs`

**Interfaces:**

- Consumes: `CacheTradableItem` (`cache/types/cache_tradable_item.rs`), `CacheRecipe` and `CacheIngredient` (`cache/types/cache_recipe.rs`, `cache_ingredient.rs`), `TradableItemModule::get_items`, `RecipeModule::get_all_items`.
- Produces:
  - `pub fn member_key(ingredient: &CacheIngredient) -> String`
  - `pub struct CacheItemSetMember { pub unique_name: String, pub name: String, pub required: i64, pub is_main_blueprint: bool }`
  - `pub struct CacheItemSet { pub set: CacheTradableItem, pub members: Vec<CacheItemSetMember> }`
  - `ItemSetModule::new() -> Arc<Self>`
  - `ItemSetModule::load(&self, client: &CacheState) -> Result<(), Error>`
  - `ItemSetModule::get_all_sets(&self) -> Result<Vec<CacheItemSet>, Error>`
  - `ItemSetModule::get_sets_for_member(&self, unique_name: &str) -> Vec<CacheItemSet>`
  - `CacheState::item_set(&self) -> Arc<ItemSetModule>`

- [ ] **Step 1: Write the failing tests for the member key rule**

Create `src-tauri/src/cache/types/cache_item_set.rs` with only the test module to start:

```rust
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
        let ing = ingredient("/Lotus/Types/Recipes/Weapons/WeaponParts/LatronPrimeBarrel", "");
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
        assert_eq!(keys, vec!["/Blueprint".to_string(), "/BarePart".to_string()]);
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `ddev exec 'cd /var/www/html/src-tauri && cargo test -p Quantframe cache_item_set 2>&1 | tail -20'`
Expected: FAIL — `cannot find function member_key`, and the module is not declared yet.

- [ ] **Step 3: Write the types and the member key function**

Prepend to `src-tauri/src/cache/types/cache_item_set.rs`, above the test module:

```rust
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
```

Add to `src-tauri/src/cache/types/mod.rs`:

```rust
pub mod cache_item_set;
pub use cache_item_set::*;
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `ddev exec 'cd /var/www/html/src-tauri && cargo test -p Quantframe cache_item_set 2>&1 | tail -20'`
Expected: PASS, 3 tests.

- [ ] **Step 5: Write the module**

Create `src-tauri/src/cache/modules/item_set.rs`:

```rust
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
            by_result.entry(recipe.result_type.clone()).or_insert(recipe);
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
                members.push(CacheItemSetMember {
                    unique_name: member_key(ingredient),
                    name: ingredient.base.name.clone(),
                    required: ingredient.base.quantity,
                    is_main_blueprint: false,
                });
            }

            sets.push(CacheItemSet {
                set: item,
                members,
            });
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
            .map(|indices| indices.iter().filter_map(|i| sets.get(*i).cloned()).collect())
            .unwrap_or_default()
    }
}
```

Add to `src-tauri/src/cache/modules/mod.rs`:

```rust
pub mod item_set;
pub use item_set::*;
```

- [ ] **Step 6: Wire the module into `CacheState`**

In `src-tauri/src/cache/client.rs`, make four edits.

Add the field to the `CacheState` struct, after `syndicate_module`:

```rust
    item_set_module: OnceLock<Arc<ItemSetModule>>,
```

Add to the `arc()` clone block, after `syndicate_module: self.syndicate_module.clone(),`:

```rust
                    item_set_module: self.item_set_module.clone(),
```

Add to the `CacheState` literal in `new()`, after `syndicate_module: OnceLock::new(),`:

```rust
            item_set_module: OnceLock::new(),
```

Add the accessor at the end of `impl CacheState`, after `syndicate()`:

```rust
    pub fn item_set(&self) -> Arc<ItemSetModule> {
        self.item_set_module.get_or_init(ItemSetModule::new).clone()
    }
```

In `load()`, the set index reads both the tradable items and the recipes, so it must come after both. Add it immediately after `self.syndicate().load(language)?;` and before `self.weapon().load(self)?;`:

```rust
        self.item_set().load(self)?;
```

- [ ] **Step 7: Verify it compiles and the whole suite passes**

Run: `ddev exec 'cd /var/www/html/src-tauri && cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -20 && cargo test --workspace --all-targets 2>&1 | tail -20'`
Expected: no clippy warnings; all tests pass.

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/cache/types/cache_item_set.rs src-tauri/src/cache/types/mod.rs \
        src-tauri/src/cache/modules/item_set.rs src-tauri/src/cache/modules/mod.rs \
        src-tauri/src/cache/client.rs
git commit -m "Add a cache-layer index of tradable item sets

Each item tagged 'set' in TradableItems.json has a uniqueName equal to the
resultType of its blueprint recipe, so the two files join cleanly into a
set definition with its member list.

The member key rule lives in one function: Warframe parts are owned as
blueprints and carry a fromRecipe, weapon parts are owned as built
components and do not."
```

---

## Task 2: Fix `can_craft` to resolve each ingredient individually

**Files:**

- Modify: `src-tauri/src/cache/types/cache_recipe.rs:23-75`
- Modify: `src-tauri/src/cache/modules/recipe.rs:49-62`
- Modify: `src-tauri/src/log_parser/types/trade.rs:143-150`
- Modify: `NOTICE.md`

**Interfaces:**

- Consumes: `member_key` from Task 1.
- Produces:
  - `CacheRecipe::can_craft(&self, tradeable_only: bool, items: &[CacheItemBase]) -> bool` — the `from_recipe_only` parameter is **removed**.
  - `RecipeModule::can_craft(&self, items: &[CacheItemBase], tradeable_only: bool) -> Result<Vec<CacheRecipe>, Error>` — likewise.

- [ ] **Step 1: Write the failing regression test**

Append to `src-tauri/src/cache/types/cache_recipe.rs`:

```rust
#[cfg(test)]
mod tests {
    use crate::cache::{CacheIngredient, CacheItemBase, CacheRecipe};

    fn ingredient(unique_name: &str, from_recipe: &str, quantity: i64) -> CacheIngredient {
        let mut base = CacheItemBase::new(unique_name, quantity);
        base.is_tradeable = true;
        CacheIngredient {
            base,
            from_recipe: from_recipe.to_string(),
        }
    }

    fn recipe(ingredients: Vec<CacheIngredient>) -> CacheRecipe {
        let mut base = CacheItemBase::new("/Set/MainBlueprint", 1);
        base.is_tradeable = true;
        CacheRecipe {
            base,
            result_type: "/Set/Result".to_string(),
            override_unique_name: String::new(),
            ingredients,
        }
    }

    fn owned(entries: &[(&str, i64)]) -> Vec<CacheItemBase> {
        entries
            .iter()
            .map(|(name, qty)| CacheItemBase::new(*name, *qty))
            .collect()
    }

    /// A set whose parts are all Warframe-style: owned as blueprints named by
    /// fromRecipe.
    #[test]
    fn matches_a_set_whose_ingredients_all_have_a_from_recipe() {
        let r = recipe(vec![
            ingredient("/Component/A", "/Blueprint/A", 1),
            ingredient("/Component/B", "/Blueprint/B", 1),
        ]);
        let items = owned(&[("/Set/MainBlueprint", 1), ("/Blueprint/A", 1), ("/Blueprint/B", 1)]);
        assert!(r.can_craft(true, &items));
    }

    /// A set whose parts are all weapon-style: owned as built components, no
    /// fromRecipe.
    #[test]
    fn matches_a_set_whose_ingredients_have_no_from_recipe() {
        let r = recipe(vec![
            ingredient("/Part/Barrel", "", 1),
            ingredient("/Part/Receiver", "", 1),
        ]);
        let items = owned(&[("/Set/MainBlueprint", 1), ("/Part/Barrel", 1), ("/Part/Receiver", 1)]);
        assert!(r.can_craft(true, &items));
    }

    /// The case the old all-or-nothing flag could never satisfy: with
    /// from_recipe_only the bare part produced an empty key and bailed out,
    /// without it the blueprint-backed component was looked up under a name
    /// the player never holds. Four real sets are shaped like this.
    #[test]
    fn matches_a_set_that_mixes_both_kinds_of_ingredient() {
        let r = recipe(vec![
            ingredient("/Component/A", "/Blueprint/A", 1),
            ingredient("/Part/Bare", "", 1),
        ]);
        let items = owned(&[("/Set/MainBlueprint", 1), ("/Blueprint/A", 1), ("/Part/Bare", 1)]);
        assert!(r.can_craft(true, &items));
    }

    /// Quantity still has to be satisfied: 57 tradable ingredients across the
    /// real set list require more than one copy.
    #[test]
    fn rejects_a_set_when_a_multi_copy_ingredient_is_short() {
        let r = recipe(vec![ingredient("/Part/Bare", "", 3)]);
        let short = owned(&[("/Set/MainBlueprint", 1), ("/Part/Bare", 2)]);
        let enough = owned(&[("/Set/MainBlueprint", 1), ("/Part/Bare", 3)]);
        assert!(!r.can_craft(true, &short));
        assert!(r.can_craft(true, &enough));
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `ddev exec 'cd /var/www/html/src-tauri && cargo test -p Quantframe cache_recipe 2>&1 | tail -30'`
Expected: FAIL — `can_craft` takes 3 arguments, not 2.

- [ ] **Step 3: Change `CacheRecipe::can_craft`**

In `src-tauri/src/cache/types/cache_recipe.rs`, replace the signature and the key selection. The function currently reads:

```rust
    pub fn can_craft(
        &self,
        tradeable_only: bool,
        from_recipe_only: bool,
        items: &[CacheItemBase],
    ) -> bool {
```

Change it to:

```rust
    pub fn can_craft(&self, tradeable_only: bool, items: &[CacheItemBase]) -> bool {
```

and replace this block:

```rust
            let ingredient_key = if from_recipe_only {
                ingredient.from_recipe.clone()
            } else {
                ingredient.base.unique_name.clone()
            };
            if ingredient_key.is_empty() {
                return false;
            }
```

with:

```rust
            // Per ingredient, never per recipe: four sets mix blueprint-backed
            // Warframe parts with bare weapon components.
            let ingredient_key = super::cache_item_set::member_key(ingredient);
            if ingredient_key.is_empty() {
                return false;
            }
```

- [ ] **Step 4: Update the two call sites**

In `src-tauri/src/cache/modules/recipe.rs`, change:

```rust
    pub fn can_craft(
        &self,
        items: &[CacheItemBase],
        tradeable_only: bool,
        from_recipe_only: bool,
    ) -> Result<Vec<CacheRecipe>, Error> {
        let recipes = self.get_all_items()?;
        let mut buildable_recipes: Vec<CacheRecipe> = Vec::new();
        for recipe in recipes {
            if recipe.can_craft(tradeable_only, from_recipe_only, items) {
```

to:

```rust
    pub fn can_craft(
        &self,
        items: &[CacheItemBase],
        tradeable_only: bool,
    ) -> Result<Vec<CacheRecipe>, Error> {
        let recipes = self.get_all_items()?;
        let mut buildable_recipes: Vec<CacheRecipe> = Vec::new();
        for recipe in recipes {
            if recipe.can_craft(tradeable_only, items) {
```

In `src-tauri/src/log_parser/types/trade.rs`, replace the two-pass workaround:

```rust
        let get_buildable_set = |recipe_only| cache.recipe().can_craft(&items, true, recipe_only);

        // strict pass first, then relaxed fallback
        let mut recipes = get_buildable_set(false).unwrap_or_default();
        if recipes.is_empty() {
            recipes = get_buildable_set(true).unwrap_or_default();
        }

        if recipes.is_empty() {
            return;
        }
```

with:

```rust
        // One pass: can_craft now resolves each ingredient's key individually,
        // so the strict/relaxed fallback that used to be needed here is gone.
        let recipes = cache.recipe().can_craft(&items, true).unwrap_or_default();

        if recipes.is_empty() {
            return;
        }
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `ddev exec 'cd /var/www/html/src-tauri && cargo test --workspace --all-targets 2>&1 | tail -20'`
Expected: PASS, including the 4 new `cache_recipe` tests.

- [ ] **Step 6: Record the behaviour change in NOTICE.md**

Add to the "Defect fixes" section of `NOTICE.md`:

```markdown
- `CacheRecipe::can_craft` chose a single ingredient-key mode for a whole
  recipe, and bailed out entirely when asked for `fromRecipe` on an
  ingredient that had none. The trade log parser worked around it by trying
  both modes in sequence, which covers recipes whose ingredients are
  uniformly one kind but never the four sets that mix both. The key is now
  resolved per ingredient and the fallback pass is gone, so trade log set
  detection recognises those sets.
```

- [ ] **Step 7: Verify all gates**

Run: `ddev exec 'cd /var/www/html && .ddev/commands/web/check 2>&1 | tail -20'`
Expected: all checks passed.

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/cache/types/cache_recipe.rs src-tauri/src/cache/modules/recipe.rs \
        src-tauri/src/log_parser/types/trade.rs NOTICE.md
git commit -m "Resolve each recipe ingredient's inventory key individually

can_craft picked one key mode for a whole recipe and returned false
outright when from_recipe_only met an empty fromRecipe. The trade parser
compensated by trying both modes, which cannot help a recipe that mixes
blueprint-backed Warframe parts with bare weapon components - four of the
234 sets are shaped that way and were invisible to set detection."
```

---

## Task 3: Parse `MiscItems` and project the Parts rows

**Files:**

- Modify: `src-tauri/src/wf_inventory/types/wf_root_object.rs`
- Modify: `src-tauri/src/wf_inventory/modules/item.rs`
- Modify: `src-tauri/src/wf_inventory/client.rs`

**Interfaces:**

- Consumes: `WFInvItemRaw`, `WFInvItemBase`, `CacheState::tradable_item`, `CacheState::item_set`, `paginate` (`crate::helper`), `states::cache_client` (`crate::utils::modules::states`).
- Produces:
  - `WarframeRootObject.misc_items: Vec<WFInvItemRaw>`
  - `pub fn owned_counts(buckets: &[&[WFInvItemRaw]]) -> HashMap<String, i64>` in `wf_inventory/modules/item.rs`
  - `ItemModule::get_parts(&self, query: WFItemPaginationDto) -> Result<PaginatedResult<WFInvItemBase>, Error>`
  - `WFInventoryState::item(&self) -> Arc<ItemModule>`
  - Each part row's `properties` carries `in_sets: Vec<String>`.

- [ ] **Step 1: Write the failing tests**

Append to `src-tauri/src/wf_inventory/modules/item.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::owned_counts;
    use crate::wf_inventory::WFInvItemRaw;

    fn raw(unique_name: &str, quantity: i64) -> WFInvItemRaw {
        WFInvItemRaw {
            id: Default::default(),
            quantity,
            unique_name: unique_name.to_string(),
            upgrade_fingerprint: None,
            last_added: Default::default(),
            xp: 0,
        }
    }

    #[test]
    fn sums_quantities_across_buckets() {
        let recipes = vec![raw("/A", 2)];
        let misc = vec![raw("/B", 5)];
        let counts = owned_counts(&[&recipes, &misc]);
        assert_eq!(counts.get("/A"), Some(&2));
        assert_eq!(counts.get("/B"), Some(&5));
    }

    /// A real inventory can list the same ItemType twice. Overwriting instead
    /// of summing would silently under-report what the player holds.
    #[test]
    fn sums_duplicate_entries_rather_than_overwriting() {
        let recipes = vec![raw("/A", 2), raw("/A", 3)];
        let counts = owned_counts(&[&recipes]);
        assert_eq!(counts.get("/A"), Some(&5));
    }

    #[test]
    fn ignores_entries_with_no_item_type() {
        let recipes = vec![raw("", 7), raw("/A", 1)];
        let counts = owned_counts(&[&recipes]);
        assert_eq!(counts.get(""), None);
        assert_eq!(counts.get("/A"), Some(&1));
    }
}
```

Append to `src-tauri/src/wf_inventory/types/wf_root_object.rs`:

```rust
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
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `ddev exec 'cd /var/www/html/src-tauri && cargo test -p Quantframe wf_inventory 2>&1 | tail -20'`
Expected: FAIL — no field `misc_items`, no function `owned_counts`.

- [ ] **Step 3: Add the `misc_items` field**

In `src-tauri/src/wf_inventory/types/wf_root_object.rs`, add after the `recipes` field and before `affiliations`:

```rust
    #[serde(rename = "MiscItems", default)]
    pub misc_items: Vec<WFInvItemRaw>,
```

- [ ] **Step 4: Implement `owned_counts` and `get_parts`**

Replace the whole body of `src-tauri/src/wf_inventory/modules/item.rs` above the test module with:

```rust
use std::collections::HashMap;
use std::sync::{Arc, Weak};

use entity::{dto::PaginatedResult, enums::FieldChange};
use utils::Error;

use crate::{
    helper::paginate,
    utils::modules::states,
    wf_inventory::{item_base::WFInvItemBase, *},
};

/// Total quantity held per unique name, summed across the given inventory
/// buckets. Duplicate entries for one unique name are added together, which a
/// real inventory does contain.
pub fn owned_counts(buckets: &[&[WFInvItemRaw]]) -> HashMap<String, i64> {
    let mut counts: HashMap<String, i64> = HashMap::new();
    for bucket in buckets {
        for entry in bucket.iter() {
            if entry.unique_name.is_empty() {
                continue;
            }
            *counts.entry(entry.unique_name.clone()).or_insert(0) += entry.quantity;
        }
    }
    counts
}

#[derive(Debug)]
pub struct ItemModule {
    client: Weak<WFInventoryState>,
}

impl ItemModule {
    pub fn new(client: Arc<WFInventoryState>) -> Arc<Self> {
        Arc::new(Self {
            client: Arc::downgrade(&client),
        })
    }

    /// Tradable parts: everything in the Recipes and MiscItems buckets that
    /// the tradable-items cache knows about. That lookup is the filter -
    /// resources, fish and gems are absent from the cache and drop out here
    /// without any path matching.
    pub fn get_parts(
        &self,
        query: WFItemPaginationDto,
    ) -> Result<PaginatedResult<WFInvItemBase>, Error> {
        let client = self.client.upgrade().unwrap();
        let root = client.get_root();
        let cache = states::cache_client()?;

        let counts = owned_counts(&[&root.recipes, &root.misc_items]);
        let item_set = cache.item_set();

        let mut items: Vec<WFInvItemBase> = Vec::new();
        for (unique_name, quantity) in counts {
            let Ok(tradable) = cache.tradable_item().get_by(&unique_name) else {
                continue;
            };
            let in_sets: Vec<String> = item_set
                .get_sets_for_member(&unique_name)
                .iter()
                .map(|set| set.set.name.clone())
                .collect();

            let mut item = WFInvItemBase {
                id: tradable.wfm_id.clone(),
                name: tradable.name.clone(),
                unique_name,
                wfm_url: tradable.wfm_url.clone(),
                quantity,
                sub_type: None,
                ..Default::default()
            };
            item.properties.set_property_value("in_sets", in_sets);
            item.properties
                .set_property_value("tags", tradable.tags.clone());
            items.push(item);
        }

        if let FieldChange::Value(text) = &query.query {
            let text = text.to_lowercase();
            items.retain(|item| item.name.to_lowercase().contains(&text));
        }
        if let FieldChange::Value(properties) = &query.properties {
            if properties.get_property_value("in_set_only", false) {
                items.retain(|item| {
                    !item
                        .properties
                        .get_property_value::<Vec<String>>("in_sets", vec![])
                        .is_empty()
                });
            }
        }

        items.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(paginate(
            &items,
            query.pagination.page,
            query.pagination.limit,
        ))
    }
}
```

- [ ] **Step 5: Expose `item()` on `WFInventoryState`**

In `src-tauri/src/wf_inventory/client.rs`, replace the commented-out accessor block:

```rust
    // pub fn item(&self) -> Arc<ItemModule> {
    //     self.item_module
    //         .get()
    //         .expect("ItemModule not initialized")
    //         .clone()
    // }
```

with:

```rust
    pub fn item(&self) -> Arc<ItemModule> {
        self.item_module
            .get()
            .expect("ItemModule not initialized")
            .clone()
    }
```

- [ ] **Step 6: Run the tests to verify they pass**

Run: `ddev exec 'cd /var/www/html/src-tauri && cargo test --workspace --all-targets 2>&1 | tail -20'`
Expected: PASS, including the 3 `owned_counts` tests and the 2 root-object tests.

Note `get_parts` and `item()` are not called yet — Task 6 calls them. Clippy will not complain, because they are `pub` on a `pub` type.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/wf_inventory/types/wf_root_object.rs \
        src-tauri/src/wf_inventory/modules/item.rs src-tauri/src/wf_inventory/client.rs
git commit -m "Project tradable parts out of the Recipes and MiscItems buckets

MiscItems was never parsed, which is where weapon parts live. Presence in
the tradable-items cache is the filter: resources, fish and gems are absent
from it and drop out without any path matching."
```

---

## Task 4: Project the Mods rows with rank grouping

**Files:**

- Modify: `src-tauri/src/wf_inventory/modules/item.rs`

**Interfaces:**

- Consumes: `owned_counts` and `ItemModule` from Task 3, `WFInvItemRaw::is_riven`, `WFInvItemRaw::get_upgrade_fingerprint`, `UpgradeFingerprint::mod_rank`, `utils::SubType`.
- Produces:
  - `pub fn rank_groups(upgrades: &[WFInvItemRaw]) -> HashMap<(String, i64), i64>`
  - `ItemModule::get_mods(&self, query: WFItemPaginationDto) -> Result<PaginatedResult<WFInvItemBase>, Error>`
  - Each mod row's `sub_type` is `Some(SubType { rank: Some(n), .. })` and its `properties` carries `tags: Vec<String>`.

- [ ] **Step 1: Write the failing tests**

Add to the existing `mod tests` in `src-tauri/src/wf_inventory/modules/item.rs`:

```rust
    use super::rank_groups;

    fn ranked(unique_name: &str, fingerprint: &str) -> WFInvItemRaw {
        WFInvItemRaw {
            id: Default::default(),
            quantity: 1,
            unique_name: unique_name.to_string(),
            upgrade_fingerprint: Some(fingerprint.to_string()),
            last_added: Default::default(),
            xp: 0,
        }
    }

    #[test]
    fn groups_instances_of_the_same_item_and_rank() {
        let upgrades = vec![
            ranked("/Mods/Serration", r#"{"lvl":10}"#),
            ranked("/Mods/Serration", r#"{"lvl":10}"#),
            ranked("/Mods/Serration", r#"{"lvl":3}"#),
        ];
        let groups = rank_groups(&upgrades);
        assert_eq!(groups.get(&("/Mods/Serration".to_string(), 10)), Some(&2));
        assert_eq!(groups.get(&("/Mods/Serration".to_string(), 3)), Some(&1));
    }

    /// Rivens have their own tab and their own stock type; they must not
    /// appear among the mods.
    #[test]
    fn excludes_rivens() {
        let upgrades = vec![
            ranked("/Lotus/Upgrades/Mods/Randomized/LotusRifleRandomMod", r#"{"lvl":8}"#),
            ranked("/Mods/Serration", r#"{"lvl":8}"#),
        ];
        let groups = rank_groups(&upgrades);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups.get(&("/Mods/Serration".to_string(), 8)), Some(&1));
    }

    /// A malformed or absent fingerprint falls back to rank 0. It must not
    /// panic, and it must stay distinguishable from the unranked RawUpgrades
    /// stack, which is counted separately and never passes through here.
    #[test]
    fn treats_an_unparseable_fingerprint_as_rank_zero() {
        let upgrades = vec![
            ranked("/Mods/Serration", "not json at all"),
            WFInvItemRaw {
                id: Default::default(),
                quantity: 1,
                unique_name: "/Mods/Serration".to_string(),
                upgrade_fingerprint: None,
                last_added: Default::default(),
                xp: 0,
            },
        ];
        let groups = rank_groups(&upgrades);
        assert_eq!(groups.get(&("/Mods/Serration".to_string(), 0)), Some(&2));
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `ddev exec 'cd /var/www/html/src-tauri && cargo test -p Quantframe wf_inventory::modules::item 2>&1 | tail -20'`
Expected: FAIL — `cannot find function rank_groups`.

- [ ] **Step 3: Implement `rank_groups` and `get_mods`**

Add `rank_groups` to `src-tauri/src/wf_inventory/modules/item.rs`, directly below `owned_counts`:

```rust
/// Ranked upgrade instances collapsed to (unique name, rank) -> count.
///
/// The Upgrades bucket lists each ranked copy individually with its own
/// ItemId, so two maxed Serrations are two entries. Rivens live here too and
/// are excluded: they have their own tab and their own stock type.
pub fn rank_groups(upgrades: &[WFInvItemRaw]) -> HashMap<(String, i64), i64> {
    let mut groups: HashMap<(String, i64), i64> = HashMap::new();
    for entry in upgrades.iter() {
        if entry.unique_name.is_empty() || entry.is_riven() {
            continue;
        }
        let rank = entry.get_upgrade_fingerprint().mod_rank;
        *groups
            .entry((entry.unique_name.clone(), rank))
            .or_insert(0) += 1;
    }
    groups
}
```

Add `get_mods` to `impl ItemModule`, after `get_parts`:

```rust
    /// Tradable mods and arcanes. The RawUpgrades bucket holds unranked
    /// stacks; the Upgrades bucket holds individually ranked instances, which
    /// are grouped by rank so a maxed copy lists separately from an unranked
    /// one.
    pub fn get_mods(
        &self,
        query: WFItemPaginationDto,
    ) -> Result<PaginatedResult<WFInvItemBase>, Error> {
        let client = self.client.upgrade().unwrap();
        let root = client.get_root();
        let cache = states::cache_client()?;

        let mut rows: Vec<(String, i64, i64)> = Vec::new(); // unique name, rank, quantity
        for (unique_name, quantity) in owned_counts(&[&root.raw_upgrades]) {
            rows.push((unique_name, 0, quantity));
        }
        for ((unique_name, rank), quantity) in rank_groups(&root.upgrades) {
            rows.push((unique_name, rank, quantity));
        }

        let mut items: Vec<WFInvItemBase> = Vec::new();
        for (unique_name, rank, quantity) in rows {
            let Ok(tradable) = cache.tradable_item().get_by(&unique_name) else {
                continue;
            };
            let mut item = WFInvItemBase {
                id: tradable.wfm_id.clone(),
                name: tradable.name.clone(),
                unique_name,
                wfm_url: tradable.wfm_url.clone(),
                quantity,
                sub_type: Some(utils::SubType {
                    rank: Some(rank),
                    ..Default::default()
                }),
                ..Default::default()
            };
            item.properties
                .set_property_value("tags", tradable.tags.clone());
            item.properties.set_property_value(
                "max_rank",
                tradable.sub_type.as_ref().and_then(|s| s.max_rank),
            );
            items.push(item);
        }

        if let FieldChange::Value(text) = &query.query {
            let text = text.to_lowercase();
            items.retain(|item| item.name.to_lowercase().contains(&text));
        }
        if let FieldChange::Value(properties) = &query.properties {
            let rank_filter = properties.get_property_value("rank_filter", String::new());
            match rank_filter.as_str() {
                "unranked" => items.retain(|item| {
                    item.sub_type.as_ref().and_then(|s| s.rank).unwrap_or(0) == 0
                }),
                "ranked" => items.retain(|item| {
                    item.sub_type.as_ref().and_then(|s| s.rank).unwrap_or(0) > 0
                }),
                _ => {}
            }
        }

        items.sort_by(|a, b| {
            a.name.cmp(&b.name).then(
                a.sub_type
                    .as_ref()
                    .and_then(|s| s.rank)
                    .cmp(&b.sub_type.as_ref().and_then(|s| s.rank)),
            )
        });
        Ok(paginate(
            &items,
            query.pagination.page,
            query.pagination.limit,
        ))
    }
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `ddev exec 'cd /var/www/html/src-tauri && cargo test --workspace --all-targets 2>&1 | tail -20'`
Expected: PASS, including the 3 new `rank_groups` tests.

- [ ] **Step 5: Verify clippy is clean**

Run: `ddev exec 'cd /var/www/html/src-tauri && cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -20'`
Expected: no warnings.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/wf_inventory/modules/item.rs
git commit -m "Project tradable mods and arcanes, grouped by rank

RawUpgrades holds unranked stacks and Upgrades holds individually ranked
instances, so a maxed Serration lists separately from an unranked one and
carries the rank that stock prices it by. Arcanes need no special handling:
they live in RawUpgrades alongside the mods."
```

---

## Task 5: Project the Sets rows with completion counts

**Files:**

- Create: `src-tauri/src/wf_inventory/types/item_set.rs`
- Create: `src-tauri/src/wf_inventory/modules/item_set.rs`
- Modify: `src-tauri/src/wf_inventory/types/mod.rs`
- Modify: `src-tauri/src/wf_inventory/modules/mod.rs`
- Modify: `src-tauri/src/wf_inventory/client.rs`

**Interfaces:**

- Consumes: `CacheItemSet`, `CacheItemSetMember` (Task 1), `owned_counts` (Task 3), `WFInvItemBase`.
- Produces:
  - `pub struct WFInvSetMember { pub unique_name: String, pub name: String, pub have: i64, pub required: i64, pub is_main_blueprint: bool }`
  - `pub struct WFInvSet { pub base: WFInvItemBase, pub members: Vec<WFInvSetMember>, pub complete_copies: i64, pub owned_members: i64, pub total_members: i64 }`
  - `pub fn complete_copies(members: &[WFInvSetMember]) -> i64`
  - `SetsModule::new(client: Arc<WFInventoryState>) -> Arc<Self>`
  - `SetsModule::get_sets(&self, query: WFItemPaginationDto) -> Result<PaginatedResult<WFInvSet>, Error>`
  - `WFInventoryState::sets(&self) -> Arc<SetsModule>`

- [ ] **Step 1: Write the failing tests**

Create `src-tauri/src/wf_inventory/types/item_set.rs` containing the two row types (the tests need `WFInvSetMember` to exist in order to compile) and the test module, but **not** `complete_copies`:

```rust
use serde::{Deserialize, Serialize};

use crate::wf_inventory::item_base::WFInvItemBase;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WFInvSetMember {
    pub unique_name: String,
    pub name: String,
    pub have: i64,
    pub required: i64,
    pub is_main_blueprint: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WFInvSet {
    #[serde(flatten)]
    pub base: WFInvItemBase,
    pub members: Vec<WFInvSetMember>,
    pub complete_copies: i64,
    pub owned_members: i64,
    pub total_members: i64,
}

#[cfg(test)]
mod tests {
    use super::{complete_copies, WFInvSetMember};

    fn member(have: i64, required: i64) -> WFInvSetMember {
        WFInvSetMember {
            unique_name: "/X".to_string(),
            name: "X".to_string(),
            have,
            required,
            is_main_blueprint: false,
        }
    }

    #[test]
    fn counts_whole_copies_only() {
        let members = vec![member(3, 1), member(2, 1), member(5, 1)];
        assert_eq!(complete_copies(&members), 2);
    }

    #[test]
    fn divides_by_the_required_quantity() {
        let members = vec![member(12, 6), member(2, 1)];
        assert_eq!(complete_copies(&members), 2);
    }

    #[test]
    fn reports_zero_when_a_member_is_missing() {
        let members = vec![member(3, 1), member(0, 1)];
        assert_eq!(complete_copies(&members), 0);
    }

    /// A recipe with no tradable ingredients leaves nothing to divide. min()
    /// over an empty iterator is None, and unwrapping it would panic and take
    /// the whole Sets tab down.
    #[test]
    fn reports_zero_for_a_set_with_no_members() {
        assert_eq!(complete_copies(&[]), 0);
    }

    /// A required quantity of zero would be a division by zero. Treat it as
    /// unsatisfiable rather than crashing.
    #[test]
    fn treats_a_zero_requirement_as_unsatisfiable_rather_than_panicking() {
        let members = vec![member(5, 0), member(5, 1)];
        assert_eq!(complete_copies(&members), 0);
    }
}
```

Add to `src-tauri/src/wf_inventory/types/mod.rs`:

```rust
pub mod item_set;
pub use item_set::*;
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `ddev exec 'cd /var/www/html/src-tauri && cargo test -p Quantframe item_set 2>&1 | tail -20'`
Expected: FAIL — `cannot find function complete_copies in this scope`.

- [ ] **Step 3: Write `complete_copies`**

Add to `src-tauri/src/wf_inventory/types/item_set.rs`, between the `WFInvSet` struct and the test module:

```rust
/// How many whole copies of the set the player holds.
///
/// Integer division, because 57 tradable ingredients across the real set list
/// require more than one copy of a part. A member whose `required` is zero or
/// negative is treated as unsatisfiable rather than dividing by zero, and a
/// set with no members yields zero rather than panicking on an empty `min()`.
pub fn complete_copies(members: &[WFInvSetMember]) -> i64 {
    members
        .iter()
        .map(|member| {
            if member.required <= 0 {
                0
            } else {
                member.have / member.required
            }
        })
        .min()
        .unwrap_or(0)
}
```

Run: `ddev exec 'cd /var/www/html/src-tauri && cargo test -p Quantframe item_set 2>&1 | tail -20'`
Expected: PASS, 5 tests.

- [ ] **Step 4: Write the module**

Create `src-tauri/src/wf_inventory/modules/item_set.rs`:

```rust
use std::sync::{Arc, Weak};

use entity::{dto::PaginatedResult, enums::FieldChange};
use utils::Error;

use crate::{
    helper::paginate,
    utils::modules::states,
    wf_inventory::{item_base::WFInvItemBase, modules::item::owned_counts, *},
};

#[derive(Debug)]
pub struct SetsModule {
    client: Weak<WFInventoryState>,
}

impl SetsModule {
    pub fn new(client: Arc<WFInventoryState>) -> Arc<Self> {
        Arc::new(Self {
            client: Arc::downgrade(&client),
        })
    }

    /// Every set the player holds at least one member of, complete ones first
    /// and the rest ordered by how few members are missing - the partials are
    /// there to show what to buy next.
    pub fn get_sets(
        &self,
        query: WFItemPaginationDto,
    ) -> Result<PaginatedResult<WFInvSet>, Error> {
        let client = self.client.upgrade().unwrap();
        let root = client.get_root();
        let cache = states::cache_client()?;

        let counts = owned_counts(&[&root.recipes, &root.misc_items]);

        let mut sets: Vec<WFInvSet> = Vec::new();
        for cache_set in cache.item_set().get_all_sets()? {
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

            let owned_members = members.iter().filter(|m| m.have > 0).count() as i64;
            if owned_members == 0 {
                continue;
            }

            let copies = complete_copies(&members);
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

            sets.push(WFInvSet {
                base,
                total_members: members.len() as i64,
                owned_members,
                complete_copies: copies,
                members,
            });
        }

        if let FieldChange::Value(text) = &query.query {
            let text = text.to_lowercase();
            sets.retain(|set| set.base.name.to_lowercase().contains(&text));
        }
        if let FieldChange::Value(properties) = &query.properties {
            if properties.get_property_value("complete_only", false) {
                sets.retain(|set| set.complete_copies > 0);
            }
        }

        // Complete sets first by copies descending, then partials by fewest
        // members missing, so the nearly-complete ones are what you see.
        sets.sort_by(|a, b| {
            b.complete_copies
                .cmp(&a.complete_copies)
                .then((a.total_members - a.owned_members).cmp(&(b.total_members - b.owned_members)))
                .then(a.base.name.cmp(&b.base.name))
        });

        Ok(paginate(
            &sets,
            query.pagination.page,
            query.pagination.limit,
        ))
    }
}
```

Add to `src-tauri/src/wf_inventory/modules/mod.rs`:

```rust
pub mod item_set;
pub use item_set::*;
```

- [ ] **Step 5: Wire `SetsModule` into `WFInventoryState`**

In `src-tauri/src/wf_inventory/client.rs`, add the field to the struct after `syndicate_module`:

```rust
    sets_module: OnceLock<Arc<SetsModule>>,
```

Add to the `Arc::new(Self { ... })` literal in `new()`, after `syndicate_module: OnceLock::new(),`:

```rust
            sets_module: OnceLock::new(),
```

Add to `init_modules`, after the syndicate line:

```rust
        self.sets_module
            .get_or_init(|| SetsModule::new(self.clone()));
```

Add the accessor after `syndicate()`:

```rust
    pub fn sets(&self) -> Arc<SetsModule> {
        self.sets_module
            .get()
            .expect("SetsModule not initialized")
            .clone()
    }
```

- [ ] **Step 6: Run the whole suite**

Run: `ddev exec 'cd /var/www/html/src-tauri && cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -20 && cargo test --workspace --all-targets 2>&1 | tail -20'`
Expected: no clippy warnings; all tests pass.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/wf_inventory/types/item_set.rs src-tauri/src/wf_inventory/types/mod.rs \
        src-tauri/src/wf_inventory/modules/item_set.rs src-tauri/src/wf_inventory/modules/mod.rs \
        src-tauri/src/wf_inventory/client.rs
git commit -m "Compute set completion against the live inventory

Counts each member across the Recipes and MiscItems buckets and takes the
minimum of have/required, so a set needing six of a part is not reported
complete on one. Sets with nothing owned are dropped; the rest sort
complete-first then by fewest members missing."
```

---

## Task 6: Expose the three commands

**Files:**

- Modify: `src-tauri/src/commands/wf_inventory.rs`
- Modify: `src-tauri/src/lib.rs:384`

**Interfaces:**

- Consumes: `ItemModule::get_parts`, `ItemModule::get_mods` (Tasks 3-4), `SetsModule::get_sets` (Task 5), `StockItemQuery::get_all`, `StockItemPaginationQueryDto::new`.
- Produces:
  - `wf_inventory_get_parts(query: WFItemPaginationDto, wf_inventory: State<...>) -> Result<Value, Error>`
  - `wf_inventory_get_mods(...)`, `wf_inventory_get_sets(...)` with the same shape
  - `pub fn stock_key(wfm_url: &str, rank: Option<i64>) -> String`
  - Rows gain `is_in_stock: bool`; part rows gain `in_stock_sets: Vec<String>`.

- [ ] **Step 1: Write the failing test for stock key matching**

Append to `src-tauri/src/commands/wf_inventory.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::stock_key;

    /// A stock item saved without a sub_type and one saved with rank 0 are the
    /// same unranked item. Treating them differently shows a red "not in
    /// stock" badge on something that is already listed.
    #[test]
    fn treats_no_sub_type_and_rank_zero_as_the_same_item() {
        assert_eq!(stock_key("serration", None), stock_key("serration", Some(0)));
    }

    #[test]
    fn keeps_different_ranks_apart() {
        assert_ne!(stock_key("serration", Some(0)), stock_key("serration", Some(10)));
    }

    #[test]
    fn keeps_different_items_apart() {
        assert_ne!(stock_key("serration", Some(10)), stock_key("vitality", Some(10)));
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `ddev exec 'cd /var/www/html/src-tauri && cargo test -p Quantframe commands::wf_inventory 2>&1 | tail -20'`
Expected: FAIL — `cannot find function stock_key`.

- [ ] **Step 3: Implement the helper and the three commands**

Add these imports at the top of `src-tauri/src/commands/wf_inventory.rs`, alongside the existing ones:

```rust
use entity::stock_item::StockItemPaginationQueryDto;
use service::StockItemQuery;
use std::collections::HashSet;
```

Add the helper above the first command:

```rust
/// Identity of a stock listing for matching against an inventory row.
///
/// An absent sub_type and an explicit rank 0 both mean "unranked", so they
/// must collapse to the same key.
pub fn stock_key(wfm_url: &str, rank: Option<i64>) -> String {
    format!("{}#{}", wfm_url, rank.unwrap_or(0))
}
```

Add the three commands after `wf_inventory_get_syndicates`:

```rust
#[tauri::command]
pub async fn wf_inventory_get_parts(
    query: WFItemPaginationDto,
    wf_inventory: tauri::State<'_, Mutex<Arc<WFInventoryState>>>,
) -> Result<Value, Error> {
    let wf_inventory = wf_inventory.lock()?.clone();
    let conn = DATABASE.get().unwrap();
    let stock = StockItemQuery::get_all(conn, StockItemPaginationQueryDto::new(1, -1)).await?;

    let listed: HashSet<String> = stock
        .results
        .iter()
        .map(|item| stock_key(&item.wfm_url, item.sub_type.as_ref().and_then(|s| s.rank)))
        .collect();
    let listed_sets: HashSet<String> = stock
        .results
        .iter()
        .map(|item| item.item_name.clone())
        .collect();

    let mut parts = wf_inventory.item().get_parts(query)?;
    for item in parts.results.iter_mut() {
        let in_stock = listed.contains(&stock_key(&item.wfm_url, None));
        item.properties.set_property_value("is_in_stock", in_stock);

        // Listing a part separately undercuts a set listing that contains it.
        let conflicting: Vec<String> = item
            .properties
            .get_property_value::<Vec<String>>("in_sets", vec![])
            .into_iter()
            .filter(|set_name| listed_sets.contains(set_name))
            .collect();
        item.properties
            .set_property_value("in_stock_sets", conflicting);
    }
    Ok(json!(parts))
}

#[tauri::command]
pub async fn wf_inventory_get_mods(
    query: WFItemPaginationDto,
    wf_inventory: tauri::State<'_, Mutex<Arc<WFInventoryState>>>,
) -> Result<Value, Error> {
    let wf_inventory = wf_inventory.lock()?.clone();
    let conn = DATABASE.get().unwrap();
    let stock = StockItemQuery::get_all(conn, StockItemPaginationQueryDto::new(1, -1)).await?;
    let listed: HashSet<String> = stock
        .results
        .iter()
        .map(|item| stock_key(&item.wfm_url, item.sub_type.as_ref().and_then(|s| s.rank)))
        .collect();

    let mut mods = wf_inventory.item().get_mods(query)?;
    for item in mods.results.iter_mut() {
        let rank = item.sub_type.as_ref().and_then(|s| s.rank);
        let in_stock = listed.contains(&stock_key(&item.wfm_url, rank));
        item.properties.set_property_value("is_in_stock", in_stock);
    }
    Ok(json!(mods))
}

#[tauri::command]
pub async fn wf_inventory_get_sets(
    query: WFItemPaginationDto,
    wf_inventory: tauri::State<'_, Mutex<Arc<WFInventoryState>>>,
) -> Result<Value, Error> {
    let wf_inventory = wf_inventory.lock()?.clone();
    let conn = DATABASE.get().unwrap();
    let stock = StockItemQuery::get_all(conn, StockItemPaginationQueryDto::new(1, -1)).await?;
    let listed: HashSet<String> = stock
        .results
        .iter()
        .map(|item| stock_key(&item.wfm_url, None))
        .collect();

    let mut sets = wf_inventory.sets().get_sets(query)?;
    for set in sets.results.iter_mut() {
        let in_stock = listed.contains(&stock_key(&set.base.wfm_url, None));
        set.base
            .properties
            .set_property_value("is_in_stock", in_stock);
    }
    Ok(json!(sets))
}
```

- [ ] **Step 4: Register the commands**

In `src-tauri/src/lib.rs`, add after the line `commands::wf_inventory::wf_inventory_get_rivens,`:

```rust
            commands::wf_inventory::wf_inventory_get_parts,
            commands::wf_inventory::wf_inventory_get_mods,
            commands::wf_inventory::wf_inventory_get_sets,
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `ddev exec 'cd /var/www/html/src-tauri && cargo test --workspace --all-targets 2>&1 | tail -20'`
Expected: PASS, including the 3 new `stock_key` tests.

- [ ] **Step 6: Verify all Rust gates**

Run: `ddev exec 'cd /var/www/html/src-tauri && cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -20'`
Expected: clean.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/commands/wf_inventory.rs src-tauri/src/lib.rs
git commit -m "Expose parts, mods and sets as inventory commands

Each stamps is_in_stock the way the rivens command does. Matching is by
wfm_url and rank, with an absent sub_type and an explicit rank 0 collapsing
to the same key so an unranked mod already in stock is not shown as missing.
Part rows also carry the names of any in-stock set they belong to."
```

---

## Task 7: Add an optional warning message to the prompt modal

**Files:**

- Modify: `src/components/Modals/Prompt/index.tsx`
- Modify: `public/lang/en.json`

**Interfaces:**

- Produces: `PromptModalProps.message?: string` — rendered as a Mantine `Alert` above the form when present.

- [ ] **Step 1: Add the prop and render it**

In `src/components/Modals/Prompt/index.tsx`, change the imports:

```tsx
import { Alert, Container, FocusTrap } from "@mantine/core";
```

Add to `PromptModalProps`, after `height?: string;`:

```tsx
  /** Shown as a warning above the fields. Advisory only; it does not block submit. */
  message?: string;
```

Destructure it in the component:

```tsx
  const { height, confirmLabel, cancelLabel, fields, message, onConfirm, onCancel } = innerProps;
```

Render it inside the `Container`, immediately before `<FocusTrap ...>`:

```tsx
      {message && (
        <Alert color="yellow.7" mb="md">
          {message}
        </Alert>
      )}
```

- [ ] **Step 2: Add the translation keys**

In `public/lang/en.json`, under `common.prompts.bought_manual.fields`, add a `rank` label beside the existing `bought` and `quantity`:

```json
          "rank": {
            "label": "Rank"
          }
```

Under `common.prompts`, add a new sibling entry after `bought_manual`:

```json
      "stock_set_conflict": {
        "message": "{{sets}} is already in stock. Listing this part on its own may undercut that set."
      },
```

- [ ] **Step 3: Verify the frontend builds and lints**

Run: `ddev exec 'cd /var/www/html && pnpm exec eslint src/components/Modals/Prompt/index.tsx && pnpm build 2>&1 | tail -10'`
Expected: no eslint errors; build succeeds.

- [ ] **Step 4: Commit**

```bash
git add src/components/Modals/Prompt/index.tsx public/lang/en.json
git commit -m "Let the prompt modal carry a warning above its fields

A row-level warning does not belong in a field description, and
DynamicForm has no slot for one."
```

---

## Task 8: Parts tab

**Files:**

- Modify: `src/types/tauri.type.ts` (after the `WFInvSyndicateControllerGetListData` block, around line 987)
- Modify: `src/api/wf_inventory/index.ts`
- Create: `src/pages/wf_inventory/Tabs/Parts/queries.ts`
- Create: `src/pages/wf_inventory/Tabs/Parts/mutations.ts`
- Create: `src/pages/wf_inventory/Tabs/Parts/modals.tsx`
- Create: `src/pages/wf_inventory/Tabs/Parts/index.tsx`
- Modify: `src/pages/wf_inventory/index.tsx`
- Modify: `public/lang/en.json`

**Interfaces:**

- Consumes: `wf_inventory_get_parts` (Task 6), `PromptModalProps.message` (Task 7), `api.stock_item.create`, `createGenericMutation`.
- Produces: `WfInventoryModule.getPartsPagination`, `TauriTypes.WFInvPartsControllerGetListData`, `PartsPanel`.

- [ ] **Step 1: Add the TypeScript types**

In `src/types/tauri.type.ts`, add after the `WFInvSyndicateControllerGetListData` block:

```ts
  export type WFInvItemRowProperties = {
    is_in_stock: boolean;
    tags: string[];
    in_sets?: string[];
    in_stock_sets?: string[];
    max_rank?: number | null;
  };
  export type WFInvPartsControllerGetListData = PaginatedDto & {
    results?: WFInvItemBase<WFInvItemRowProperties>[];
  };
  export type WFInvModsControllerGetListData = PaginatedDto & {
    results?: WFInvItemBase<WFInvItemRowProperties>[];
  };
  export interface WFInvSetMember {
    unique_name: string;
    name: string;
    have: number;
    required: number;
    is_main_blueprint: boolean;
  }
  export type WFInvSet = WFInvItemBase<WFInvItemRowProperties> & {
    members: WFInvSetMember[];
    complete_copies: number;
    owned_members: number;
    total_members: number;
  };
  export type WFInvSetsControllerGetListData = PaginatedDto & {
    results?: WFInvSet[];
  };
```

- [ ] **Step 2: Add the API methods**

In `src/api/wf_inventory/index.ts`, add three methods inside `WfInventoryModule`, after `getSyndicatesPagination`:

```ts
  async getPartsPagination(query: TauriTypes.WFItemControllerGetListParams): Promise<TauriTypes.WFInvPartsControllerGetListData> {
    return await this.client.sendInvoke<TauriTypes.WFInvPartsControllerGetListData>("wf_inventory_get_parts", {
      query: this.client.convertToTauriQuery(query),
    });
  }
  async getModsPagination(query: TauriTypes.WFItemControllerGetListParams): Promise<TauriTypes.WFInvModsControllerGetListData> {
    return await this.client.sendInvoke<TauriTypes.WFInvModsControllerGetListData>("wf_inventory_get_mods", {
      query: this.client.convertToTauriQuery(query),
    });
  }
  async getSetsPagination(query: TauriTypes.WFItemControllerGetListParams): Promise<TauriTypes.WFInvSetsControllerGetListData> {
    return await this.client.sendInvoke<TauriTypes.WFInvSetsControllerGetListData>("wf_inventory_get_sets", {
      query: this.client.convertToTauriQuery(query),
    });
  }
```

- [ ] **Step 3: Write the queries hook**

Create `src/pages/wf_inventory/Tabs/Parts/queries.ts`:

```ts
import { useQuery } from "@tanstack/react-query";
import api from "@api/index";
import { TauriTypes } from "$types";

interface QueriesHooks {
  queryData: TauriTypes.WFItemControllerGetListParams;
  isActive?: boolean;
}

export const useQueries = ({ queryData, isActive }: QueriesHooks) => {
  const getPartsQuery = useQuery({
    queryKey: ["wf_inventory_get_parts", queryData],
    queryFn: () => api.wf_inventory.getPartsPagination(queryData),
    retry: false,
    enabled: isActive,
  });
  const refetchQueries = () => {
    getPartsQuery.refetch();
  };
  return {
    partsQuery: getPartsQuery,
    refetchQueries,
  };
};
```

- [ ] **Step 4: Write the mutations hook**

Create `src/pages/wf_inventory/Tabs/Parts/mutations.ts`:

```ts
import { TauriTypes } from "$types";
import api from "@api/index";
import { createGenericMutation, MutationHooks } from "@utils/genericMutation.helper";

export const useMutations = ({ refetchQueries, setLoadingRows }: MutationHooks) => {
  const hooks = { refetchQueries, setLoadingRows };

  const createMutation = createGenericMutation(
    {
      mutationFn: (data: TauriTypes.CreateStockItem) => api.stock_item.create(data),
      successKey: "create_stock_item",
      errorKey: "create_stock_item",
      getSuccessMessage: (data: any) => ({ name: data.item_name }),
    },
    hooks,
  );
  return { createMutation };
};
```

- [ ] **Step 5: Write the modal hook**

Create `src/pages/wf_inventory/Tabs/Parts/modals.tsx`:

```tsx
import { modals } from "@mantine/modals";
import { TauriTypes } from "$types";
import { useTranslateCommon } from "@hooks/useTranslate.hook";

interface ModalHooks {
  createMutation: {
    mutateAsync: (data: TauriTypes.CreateStockItem) => Promise<any>;
  };
}

export const useModals = ({ createMutation }: ModalHooks) => {
  const OpenAddToStockModal = (item: TauriTypes.WFInvItemBase<TauriTypes.WFInvItemRowProperties>) => {
    const owned = item.quantity || 1;
    const conflicts = item.properties?.in_stock_sets || [];
    modals.openContextModal({
      modal: "prompt",
      title: item.name,
      innerProps: {
        message: conflicts.length ? useTranslateCommon("prompts.stock_set_conflict.message", { sets: conflicts.join(", ") }) : undefined,
        fields: [
          {
            name: "bought",
            label: useTranslateCommon("prompts.bought_manual.fields.bought.label"),
            attributes: { min: 0 },
            value: 0,
            type: "number",
          },
          {
            name: "quantity",
            label: useTranslateCommon("prompts.bought_manual.fields.quantity.label"),
            attributes: { min: 1, max: owned },
            value: owned,
            type: "number",
          },
        ],
        onConfirm: async (data: { bought: number; quantity: number }) => {
          await createMutation.mutateAsync({
            raw: item.wfm_url,
            bought: data.bought,
            quantity: data.quantity,
          });
        },
        onCancel: (id: string) => modals.close(id),
      },
    });
  };

  return { OpenAddToStockModal };
};
```

- [ ] **Step 6: Write the panel**

Create `src/pages/wf_inventory/Tabs/Parts/index.tsx`:

```tsx
import { Box, Group, ScrollArea, Switch, Table, Text, TextInput, Tooltip } from "@mantine/core";
import { useState } from "react";
import { TauriTypes } from "$types";
import { ActionWithTooltip } from "@components/Shared/ActionWithTooltip";
import { PaginationFooter } from "@components/Shared/PaginationFooter";
import { useTranslatePages } from "@hooks/useTranslate.hook";
import { useHasAlert } from "@hooks/useHasAlert.hook";
import { faAdd, faTriangleExclamation } from "@fortawesome/free-solid-svg-icons";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { useQueries } from "./queries";
import { useMutations } from "./mutations";
import { useModals } from "./modals";

interface PartsPanelProps {
  isActive: boolean;
}

export const PartsPanel = ({ isActive }: PartsPanelProps) => {
  const useTranslate = (key: string, context?: { [key: string]: any }, i18Key?: boolean) =>
    useTranslatePages(`wf_inventory.tabs.parts.${key}`, { ...context }, i18Key);

  const [queryData, setQueryData] = useState<TauriTypes.WFItemControllerGetListParams>({ page: 1, limit: 50 });
  const [, setLoadingRows] = useState<string[]>([]);

  const { partsQuery, refetchQueries } = useQueries({ queryData, isActive });
  const { createMutation } = useMutations({ refetchQueries, setLoadingRows });
  const { OpenAddToStockModal } = useModals({ createMutation });

  return (
    <Box>
      <Group gap="md">
        <TextInput
          w={250}
          placeholder={useTranslate("search")}
          value={queryData.query || ""}
          onChange={(event) => setQueryData((prev) => ({ ...prev, page: 1, query: event.currentTarget.value }))}
        />
        <Switch
          label={useTranslate("filters.in_set_only")}
          checked={Boolean((queryData.properties as any)?.in_set_only)}
          onChange={(event) =>
            setQueryData((prev) => ({ ...prev, page: 1, properties: { ...(prev.properties as any), in_set_only: event.currentTarget.checked } }))
          }
        />
      </Group>
      <ScrollArea mt="md" data-has-alert={useHasAlert()}>
        <Table highlightOnHover>
          <Table.Thead>
            <Table.Tr>
              <Table.Th>{useTranslate("columns.name")}</Table.Th>
              <Table.Th>{useTranslate("columns.owned")}</Table.Th>
              <Table.Th>{useTranslate("columns.set")}</Table.Th>
              <Table.Th />
            </Table.Tr>
          </Table.Thead>
          <Table.Tbody>
            {partsQuery.data?.results?.map((item) => {
              const conflicts = item.properties?.in_stock_sets || [];
              return (
                <Table.Tr key={`${item.unique_name}`}>
                  <Table.Td>
                    <Group gap={6}>
                      <Text>{item.name}</Text>
                      {conflicts.length > 0 && (
                        <Tooltip label={useTranslate("stock_set_conflict", { sets: conflicts.join(", ") })}>
                          <Text c="yellow.7" component="span">
                            <FontAwesomeIcon icon={faTriangleExclamation} />
                          </Text>
                        </Tooltip>
                      )}
                    </Group>
                  </Table.Td>
                  <Table.Td>{item.quantity}</Table.Td>
                  <Table.Td>
                    <Text c="dimmed">{(item.properties?.in_sets || []).join(", ")}</Text>
                  </Table.Td>
                  <Table.Td>
                    <ActionWithTooltip
                      icon={faAdd}
                      color={item.properties?.is_in_stock ? "var(--mantine-color-green-6)" : "var(--mantine-color-red-6)"}
                      actionProps={{ size: "sm" }}
                      iconProps={{ size: "xs" }}
                      tooltip={useTranslate(`stock_status.${item.properties?.is_in_stock ? "found" : "not_found"}`)}
                      onClick={() => OpenAddToStockModal(item)}
                    />
                  </Table.Td>
                </Table.Tr>
              );
            })}
          </Table.Tbody>
        </Table>
      </ScrollArea>
      <PaginationFooter
        page={queryData.page}
        limit={queryData.limit || 50}
        total={partsQuery.data?.total || 0}
        onPageChange={(page) => setQueryData((prev) => ({ ...prev, page }))}
        onLimitChange={(limit) => setQueryData((prev) => ({ ...prev, page: 1, limit }))}
      />
    </Box>
  );
};
```

- [ ] **Step 7: Register the tab**

In `src/pages/wf_inventory/index.tsx`, add the import:

```tsx
import { PartsPanel } from "./Tabs/Parts";
```

and extend the `tabs` array:

```tsx
  const tabs = [
    { label: useTranslateTabs("riven.title"), component: (isActive: boolean) => <RivenPanel isActive={isActive} />, id: "riven" },
    { label: useTranslateTabs("parts.title"), component: (isActive: boolean) => <PartsPanel isActive={isActive} />, id: "parts" },
  ];
```

- [ ] **Step 8: Add the translations**

In `public/lang/en.json`, under `pages.wf_inventory.tabs`, add a sibling to `riven`:

```json
      "parts": {
        "title": "Parts",
        "search": "Search parts",
        "stock_set_conflict": "Already in stock as part of: {{sets}}",
        "stock_status": {
          "found": "Already in stock",
          "not_found": "Not in stock"
        },
        "columns": {
          "name": "Name",
          "owned": "Owned",
          "set": "Set"
        },
        "filters": {
          "in_set_only": "Set components only"
        }
      }
```

- [ ] **Step 9: Verify the frontend gates**

Run: `ddev exec 'cd /var/www/html && pnpm exec eslint src/pages/wf_inventory src/api/wf_inventory src/types && pnpm build 2>&1 | tail -10'`
Expected: no eslint errors; build succeeds.

- [ ] **Step 10: Commit**

```bash
git add src/types/tauri.type.ts src/api/wf_inventory/index.ts \
        src/pages/wf_inventory/Tabs/Parts src/pages/wf_inventory/index.tsx public/lang/en.json
git commit -m "Add the Parts tab to WF Inventory

Lists every tradable entry in the Recipes and MiscItems buckets with its
owned count, and warns on rows whose set is already listed."
```

---

## Task 9: Mods tab

**Files:**

- Create: `src/pages/wf_inventory/Tabs/Mods/{queries.ts,mutations.ts,modals.tsx,index.tsx}`
- Modify: `src/pages/wf_inventory/index.tsx`
- Modify: `public/lang/en.json`

**Interfaces:**

- Consumes: `api.wf_inventory.getModsPagination` (Task 8), `createGenericMutation`.
- Produces: `ModsPanel`.

- [ ] **Step 1: Write the queries hook**

Create `src/pages/wf_inventory/Tabs/Mods/queries.ts`:

```ts
import { useQuery } from "@tanstack/react-query";
import api from "@api/index";
import { TauriTypes } from "$types";

interface QueriesHooks {
  queryData: TauriTypes.WFItemControllerGetListParams;
  isActive?: boolean;
}

export const useQueries = ({ queryData, isActive }: QueriesHooks) => {
  const getModsQuery = useQuery({
    queryKey: ["wf_inventory_get_mods", queryData],
    queryFn: () => api.wf_inventory.getModsPagination(queryData),
    retry: false,
    enabled: isActive,
  });
  const refetchQueries = () => {
    getModsQuery.refetch();
  };
  return {
    modsQuery: getModsQuery,
    refetchQueries,
  };
};
```

- [ ] **Step 2: Write the mutations hook**

Create `src/pages/wf_inventory/Tabs/Mods/mutations.ts` with exactly the same content as `src/pages/wf_inventory/Tabs/Parts/mutations.ts`:

```ts
import { TauriTypes } from "$types";
import api from "@api/index";
import { createGenericMutation, MutationHooks } from "@utils/genericMutation.helper";

export const useMutations = ({ refetchQueries, setLoadingRows }: MutationHooks) => {
  const hooks = { refetchQueries, setLoadingRows };

  const createMutation = createGenericMutation(
    {
      mutationFn: (data: TauriTypes.CreateStockItem) => api.stock_item.create(data),
      successKey: "create_stock_item",
      errorKey: "create_stock_item",
      getSuccessMessage: (data: any) => ({ name: data.item_name }),
    },
    hooks,
  );
  return { createMutation };
};
```

- [ ] **Step 3: Write the modal hook with the rank field**

Create `src/pages/wf_inventory/Tabs/Mods/modals.tsx`:

```tsx
import { modals } from "@mantine/modals";
import { TauriTypes } from "$types";
import { PromptField } from "@components/Modals/Prompt";
import { useTranslateCommon } from "@hooks/useTranslate.hook";

interface ModalHooks {
  createMutation: {
    mutateAsync: (data: TauriTypes.CreateStockItem) => Promise<any>;
  };
}

export const useModals = ({ createMutation }: ModalHooks) => {
  const OpenAddToStockModal = (item: TauriTypes.WFInvItemBase<TauriTypes.WFInvItemRowProperties>) => {
    const owned = item.quantity || 1;
    const maxRank = item.properties?.max_rank ?? null;
    // Prefilled from the inventory row, not from maxRank: the row already
    // knows the real rank, and defaulting to max would misprice the listing.
    const currentRank = item.sub_type?.rank ?? 0;

    const fields: PromptField[] = [
      {
        name: "bought",
        label: useTranslateCommon("prompts.bought_manual.fields.bought.label"),
        attributes: { min: 0 },
        value: 0,
        type: "number",
      },
      {
        name: "quantity",
        label: useTranslateCommon("prompts.bought_manual.fields.quantity.label"),
        attributes: { min: 1, max: owned },
        value: owned,
        type: "number",
      },
    ];
    if (maxRank != null) {
      fields.push({
        name: "rank",
        label: useTranslateCommon("prompts.bought_manual.fields.rank.label"),
        attributes: { min: 0, max: maxRank },
        value: currentRank,
        type: "number",
      });
    }

    modals.openContextModal({
      modal: "prompt",
      title: item.name,
      innerProps: {
        fields,
        onConfirm: async (data: { bought: number; quantity: number; rank?: number }) => {
          await createMutation.mutateAsync({
            raw: item.wfm_url,
            bought: data.bought,
            quantity: data.quantity,
            sub_type: maxRank != null ? { rank: data.rank ?? currentRank } : undefined,
          });
        },
        onCancel: (id: string) => modals.close(id),
      },
    });
  };

  return { OpenAddToStockModal };
};
```

- [ ] **Step 4: Write the panel**

Create `src/pages/wf_inventory/Tabs/Mods/index.tsx`:

```tsx
import { Box, Group, ScrollArea, SegmentedControl, Table, Text, TextInput } from "@mantine/core";
import { useState } from "react";
import { TauriTypes } from "$types";
import { ActionWithTooltip } from "@components/Shared/ActionWithTooltip";
import { PaginationFooter } from "@components/Shared/PaginationFooter";
import { useTranslatePages } from "@hooks/useTranslate.hook";
import { useHasAlert } from "@hooks/useHasAlert.hook";
import { faAdd } from "@fortawesome/free-solid-svg-icons";
import { useQueries } from "./queries";
import { useMutations } from "./mutations";
import { useModals } from "./modals";

interface ModsPanelProps {
  isActive: boolean;
}

export const ModsPanel = ({ isActive }: ModsPanelProps) => {
  const useTranslate = (key: string, context?: { [key: string]: any }, i18Key?: boolean) =>
    useTranslatePages(`wf_inventory.tabs.mods.${key}`, { ...context }, i18Key);

  const [queryData, setQueryData] = useState<TauriTypes.WFItemControllerGetListParams>({ page: 1, limit: 50 });
  const [, setLoadingRows] = useState<string[]>([]);

  const { modsQuery, refetchQueries } = useQueries({ queryData, isActive });
  const { createMutation } = useMutations({ refetchQueries, setLoadingRows });
  const { OpenAddToStockModal } = useModals({ createMutation });

  return (
    <Box>
      <Group gap="md">
        <TextInput
          w={250}
          placeholder={useTranslate("search")}
          value={queryData.query || ""}
          onChange={(event) => setQueryData((prev) => ({ ...prev, page: 1, query: event.currentTarget.value }))}
        />
        <SegmentedControl
          value={((queryData.properties as any)?.rank_filter as string) || "all"}
          onChange={(value) => setQueryData((prev) => ({ ...prev, page: 1, properties: { ...(prev.properties as any), rank_filter: value } }))}
          data={[
            { label: useTranslate("filters.all"), value: "all" },
            { label: useTranslate("filters.unranked"), value: "unranked" },
            { label: useTranslate("filters.ranked"), value: "ranked" },
          ]}
        />
      </Group>
      <ScrollArea mt="md" data-has-alert={useHasAlert()}>
        <Table highlightOnHover>
          <Table.Thead>
            <Table.Tr>
              <Table.Th>{useTranslate("columns.name")}</Table.Th>
              <Table.Th>{useTranslate("columns.rank")}</Table.Th>
              <Table.Th>{useTranslate("columns.owned")}</Table.Th>
              <Table.Th />
            </Table.Tr>
          </Table.Thead>
          <Table.Tbody>
            {modsQuery.data?.results?.map((item) => (
              <Table.Tr key={`${item.unique_name}#${item.sub_type?.rank ?? 0}`}>
                <Table.Td>{item.name}</Table.Td>
                <Table.Td>
                  <Text c="dimmed">
                    {item.sub_type?.rank ?? 0}
                    {item.properties?.max_rank != null ? ` / ${item.properties.max_rank}` : ""}
                  </Text>
                </Table.Td>
                <Table.Td>{item.quantity}</Table.Td>
                <Table.Td>
                  <ActionWithTooltip
                    icon={faAdd}
                    color={item.properties?.is_in_stock ? "var(--mantine-color-green-6)" : "var(--mantine-color-red-6)"}
                    actionProps={{ size: "sm" }}
                    iconProps={{ size: "xs" }}
                    tooltip={useTranslate(`stock_status.${item.properties?.is_in_stock ? "found" : "not_found"}`)}
                    onClick={() => OpenAddToStockModal(item)}
                  />
                </Table.Td>
              </Table.Tr>
            ))}
          </Table.Tbody>
        </Table>
      </ScrollArea>
      <PaginationFooter
        page={queryData.page}
        limit={queryData.limit || 50}
        total={modsQuery.data?.total || 0}
        onPageChange={(page) => setQueryData((prev) => ({ ...prev, page }))}
        onLimitChange={(limit) => setQueryData((prev) => ({ ...prev, page: 1, limit }))}
      />
    </Box>
  );
};
```

- [ ] **Step 5: Register the tab**

In `src/pages/wf_inventory/index.tsx`, add the import `import { ModsPanel } from "./Tabs/Mods";` and append to the `tabs` array:

```tsx
    { label: useTranslateTabs("mods.title"), component: (isActive: boolean) => <ModsPanel isActive={isActive} />, id: "mods" },
```

- [ ] **Step 6: Add the translations**

In `public/lang/en.json`, under `pages.wf_inventory.tabs`, add:

```json
      "mods": {
        "title": "Mods",
        "search": "Search mods and arcanes",
        "stock_status": {
          "found": "Already in stock",
          "not_found": "Not in stock"
        },
        "columns": {
          "name": "Name",
          "rank": "Rank",
          "owned": "Owned"
        },
        "filters": {
          "all": "All",
          "unranked": "Unranked",
          "ranked": "Ranked"
        }
      }
```

- [ ] **Step 7: Verify the frontend gates**

Run: `ddev exec 'cd /var/www/html && pnpm exec eslint src/pages/wf_inventory && pnpm build 2>&1 | tail -10'`
Expected: no eslint errors; build succeeds.

- [ ] **Step 8: Commit**

```bash
git add src/pages/wf_inventory/Tabs/Mods src/pages/wf_inventory/index.tsx public/lang/en.json
git commit -m "Add the Mods tab to WF Inventory

Unranked stacks and ranked instances list as separate rows, and the rank
field prefills from the row rather than from the item's max rank."
```

---

## Task 10: Sets tab

**Files:**

- Create: `src/pages/wf_inventory/Tabs/Sets/{queries.ts,mutations.ts,modals.tsx,index.tsx}`
- Modify: `src/pages/wf_inventory/index.tsx`
- Modify: `public/lang/en.json`
- Modify: `docs/FORK.md`

**Interfaces:**

- Consumes: `api.wf_inventory.getSetsPagination` (Task 8), `TauriTypes.WFInvSet`.
- Produces: `SetsPanel`.

- [ ] **Step 1: Write the queries hook**

Create `src/pages/wf_inventory/Tabs/Sets/queries.ts`:

```ts
import { useQuery } from "@tanstack/react-query";
import api from "@api/index";
import { TauriTypes } from "$types";

interface QueriesHooks {
  queryData: TauriTypes.WFItemControllerGetListParams;
  isActive?: boolean;
}

export const useQueries = ({ queryData, isActive }: QueriesHooks) => {
  const getSetsQuery = useQuery({
    queryKey: ["wf_inventory_get_sets", queryData],
    queryFn: () => api.wf_inventory.getSetsPagination(queryData),
    retry: false,
    enabled: isActive,
  });
  const refetchQueries = () => {
    getSetsQuery.refetch();
  };
  return {
    setsQuery: getSetsQuery,
    refetchQueries,
  };
};
```

- [ ] **Step 2: Write the mutations hook**

Create `src/pages/wf_inventory/Tabs/Sets/mutations.ts` with the same content as the Parts one:

```ts
import { TauriTypes } from "$types";
import api from "@api/index";
import { createGenericMutation, MutationHooks } from "@utils/genericMutation.helper";

export const useMutations = ({ refetchQueries, setLoadingRows }: MutationHooks) => {
  const hooks = { refetchQueries, setLoadingRows };

  const createMutation = createGenericMutation(
    {
      mutationFn: (data: TauriTypes.CreateStockItem) => api.stock_item.create(data),
      successKey: "create_stock_item",
      errorKey: "create_stock_item",
      getSuccessMessage: (data: any) => ({ name: data.item_name }),
    },
    hooks,
  );
  return { createMutation };
};
```

- [ ] **Step 3: Write the modal hook**

Create `src/pages/wf_inventory/Tabs/Sets/modals.tsx`:

```tsx
import { modals } from "@mantine/modals";
import { TauriTypes } from "$types";
import { useTranslateCommon } from "@hooks/useTranslate.hook";

interface ModalHooks {
  createMutation: {
    mutateAsync: (data: TauriTypes.CreateStockItem) => Promise<any>;
  };
}

export const useModals = ({ createMutation }: ModalHooks) => {
  const OpenAddToStockModal = (set: TauriTypes.WFInvSet) => {
    // Only complete sets reach here, so complete_copies is at least 1.
    const copies = set.complete_copies;
    modals.openContextModal({
      modal: "prompt",
      title: set.name,
      innerProps: {
        fields: [
          {
            name: "bought",
            label: useTranslateCommon("prompts.bought_manual.fields.bought.label"),
            attributes: { min: 0 },
            value: 0,
            type: "number",
          },
          {
            name: "quantity",
            label: useTranslateCommon("prompts.bought_manual.fields.quantity.label"),
            attributes: { min: 1, max: copies },
            value: copies,
            type: "number",
          },
        ],
        onConfirm: async (data: { bought: number; quantity: number }) => {
          await createMutation.mutateAsync({
            raw: set.wfm_url,
            bought: data.bought,
            quantity: data.quantity,
          });
        },
        onCancel: (id: string) => modals.close(id),
      },
    });
  };

  return { OpenAddToStockModal };
};
```

- [ ] **Step 4: Write the panel**

Create `src/pages/wf_inventory/Tabs/Sets/index.tsx`:

```tsx
import { Badge, Box, Collapse, Group, ScrollArea, Switch, Table, Text, TextInput } from "@mantine/core";
import { useState } from "react";
import { TauriTypes } from "$types";
import { ActionWithTooltip } from "@components/Shared/ActionWithTooltip";
import { PaginationFooter } from "@components/Shared/PaginationFooter";
import { useTranslatePages } from "@hooks/useTranslate.hook";
import { useHasAlert } from "@hooks/useHasAlert.hook";
import { faAdd, faChevronDown, faChevronRight } from "@fortawesome/free-solid-svg-icons";
import { useQueries } from "./queries";
import { useMutations } from "./mutations";
import { useModals } from "./modals";

interface SetsPanelProps {
  isActive: boolean;
}

export const SetsPanel = ({ isActive }: SetsPanelProps) => {
  const useTranslate = (key: string, context?: { [key: string]: any }, i18Key?: boolean) =>
    useTranslatePages(`wf_inventory.tabs.sets.${key}`, { ...context }, i18Key);

  const [queryData, setQueryData] = useState<TauriTypes.WFItemControllerGetListParams>({ page: 1, limit: 50 });
  const [, setLoadingRows] = useState<string[]>([]);
  const [expanded, setExpanded] = useState<string[]>([]);

  const { setsQuery, refetchQueries } = useQueries({ queryData, isActive });
  const { createMutation } = useMutations({ refetchQueries, setLoadingRows });
  const { OpenAddToStockModal } = useModals({ createMutation });

  const toggle = (uniqueName: string) =>
    setExpanded((prev) => (prev.includes(uniqueName) ? prev.filter((n) => n !== uniqueName) : [...prev, uniqueName]));

  return (
    <Box>
      <Group gap="md">
        <TextInput
          w={250}
          placeholder={useTranslate("search")}
          value={queryData.query || ""}
          onChange={(event) => setQueryData((prev) => ({ ...prev, page: 1, query: event.currentTarget.value }))}
        />
        <Switch
          label={useTranslate("filters.complete_only")}
          checked={Boolean((queryData.properties as any)?.complete_only)}
          onChange={(event) =>
            setQueryData((prev) => ({ ...prev, page: 1, properties: { ...(prev.properties as any), complete_only: event.currentTarget.checked } }))
          }
        />
      </Group>
      <ScrollArea mt="md" data-has-alert={useHasAlert()}>
        <Table highlightOnHover>
          <Table.Thead>
            <Table.Tr>
              <Table.Th w={40} />
              <Table.Th>{useTranslate("columns.name")}</Table.Th>
              <Table.Th>{useTranslate("columns.owned_members")}</Table.Th>
              <Table.Th>{useTranslate("columns.complete")}</Table.Th>
              <Table.Th />
            </Table.Tr>
          </Table.Thead>
          <Table.Tbody>
            {setsQuery.data?.results?.map((set) => {
              const isOpen = expanded.includes(set.unique_name);
              return (
                <>
                  <Table.Tr key={set.unique_name} onClick={() => toggle(set.unique_name)} style={{ cursor: "pointer" }}>
                    <Table.Td>
                      <ActionWithTooltip
                        icon={isOpen ? faChevronDown : faChevronRight}
                        actionProps={{ size: "sm", variant: "subtle" }}
                        iconProps={{ size: "xs" }}
                        tooltip={useTranslate("expand")}
                        onClick={() => toggle(set.unique_name)}
                      />
                    </Table.Td>
                    <Table.Td>{set.name}</Table.Td>
                    <Table.Td>
                      <Text c={set.owned_members === set.total_members ? undefined : "dimmed"}>
                        {set.owned_members} / {set.total_members}
                      </Text>
                    </Table.Td>
                    <Table.Td>
                      {set.complete_copies > 0 ? (
                        <Badge color="green.7">{useTranslate("copies", { count: set.complete_copies })}</Badge>
                      ) : (
                        <Badge color="gray.7" variant="light">
                          {useTranslate("missing", { count: set.total_members - set.owned_members })}
                        </Badge>
                      )}
                    </Table.Td>
                    <Table.Td>
                      {/* Only complete sets can be listed; partials are here to show what to hunt for. */}
                      {set.complete_copies > 0 && (
                        <ActionWithTooltip
                          icon={faAdd}
                          color={set.properties?.is_in_stock ? "var(--mantine-color-green-6)" : "var(--mantine-color-red-6)"}
                          actionProps={{ size: "sm" }}
                          iconProps={{ size: "xs" }}
                          tooltip={useTranslate(`stock_status.${set.properties?.is_in_stock ? "found" : "not_found"}`)}
                          onClick={() => OpenAddToStockModal(set)}
                        />
                      )}
                    </Table.Td>
                  </Table.Tr>
                  <Table.Tr key={`${set.unique_name}-members`}>
                    <Table.Td colSpan={5} p={0}>
                      <Collapse in={isOpen}>
                        <Box p="sm">
                          {set.members.map((member) => (
                            <Group key={member.unique_name} gap="xs" justify="space-between" px="md" py={2}>
                              <Text size="sm">{member.name}</Text>
                              <Text size="sm" c={member.have >= member.required ? "green.6" : "red.6"}>
                                {member.have} / {member.required}
                              </Text>
                            </Group>
                          ))}
                        </Box>
                      </Collapse>
                    </Table.Td>
                  </Table.Tr>
                </>
              );
            })}
          </Table.Tbody>
        </Table>
      </ScrollArea>
      <PaginationFooter
        page={queryData.page}
        limit={queryData.limit || 50}
        total={setsQuery.data?.total || 0}
        onPageChange={(page) => setQueryData((prev) => ({ ...prev, page }))}
        onLimitChange={(limit) => setQueryData((prev) => ({ ...prev, page: 1, limit }))}
      />
    </Box>
  );
};
```

- [ ] **Step 5: Register the tab**

In `src/pages/wf_inventory/index.tsx`, add `import { SetsPanel } from "./Tabs/Sets";` and append:

```tsx
    { label: useTranslateTabs("sets.title"), component: (isActive: boolean) => <SetsPanel isActive={isActive} />, id: "sets" },
```

- [ ] **Step 6: Add the translations**

In `public/lang/en.json`, under `pages.wf_inventory.tabs`:

```json
      "sets": {
        "title": "Sets",
        "search": "Search sets",
        "expand": "Show components",
        "copies": "{{count}} complete",
        "missing": "Missing {{count}}",
        "stock_status": {
          "found": "Already in stock",
          "not_found": "Not in stock"
        },
        "columns": {
          "name": "Name",
          "owned_members": "Components",
          "complete": "Status"
        },
        "filters": {
          "complete_only": "Complete sets only"
        }
      }
```

- [ ] **Step 7: Document the tabs**

In `docs/FORK.md`, under the WF Inventory section, add:

```markdown
### Inventory tabs

Rivens, Parts, Mods and Sets. Each row lists to Stock through the normal
`stock_item_create`, taking a bought price (0 for anything acquired
in-game), a quantity, and a rank for mods and arcanes.

Sets are derived by joining items tagged `set` in `TradableItems.json` to
their blueprint recipe in `Recipes.json` by `resultType`. Only complete sets
can be listed; incomplete ones are shown so you can see which component to
buy next, ordered by how few are missing.

Listing a part whose set is already in stock shows a warning but is not
blocked.
```

- [ ] **Step 8: Run the full gate**

Run: `ddev exec 'cd /var/www/html && .ddev/commands/web/check 2>&1 | tail -20'`
Expected: all checks passed.

- [ ] **Step 9: Verify against the real inventory**

Run the app and open the WF Inventory page:

Run: `ddev exec 'cd /var/www/html && QF_API_URL=https://api.quantframe.app pnpm tauri dev'`

Check each of these against the measured expectations from the spec:
- Parts tab shows roughly 516 rows; toggling "Set components only" drops it to roughly 452.
- Mods tab shows roughly 851 unranked rows plus ranked ones; switching to "Ranked" leaves only rows with rank above 0.
- Sets tab shows roughly 207 rows, with about 25 showing a green complete badge and the partials ordered missing-1 before missing-3.
- Expanding a complete weapon set (for example Latron Prime Set) shows its barrel, receiver and stock with `have / required`, proving `MiscItems` is being counted.
- Expanding a Warframe set (for example Nyx Prime Set) shows its blueprints, proving `Recipes` is being counted.
- The cart button is absent on incomplete sets and present on complete ones.
- Listing a part that belongs to an in-stock set shows the yellow warning and still allows confirm.

- [ ] **Step 10: Commit**

```bash
git add src/pages/wf_inventory/Tabs/Sets src/pages/wf_inventory/index.tsx \
        public/lang/en.json docs/FORK.md
git commit -m "Add the Sets tab to WF Inventory

Only complete sets can be listed to stock. Incomplete ones are shown so you
can see which component to buy next, ordered by how few are missing, with
each member's have/required visible on expand."
```
