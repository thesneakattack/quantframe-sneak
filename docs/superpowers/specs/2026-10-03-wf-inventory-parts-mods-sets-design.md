# WF Inventory: Parts, Mods and Sets

Status: approved design, not yet implemented
Date: 2026-10-03

## Purpose

The WF Inventory page exists to get things into **Stock**, which the live
scraper then prices and lists on warframe.market. Today it surfaces only
rivens, so every other tradable thing you own has to be entered by hand from
the Live Scraper page.

This adds three tabs — **Parts**, **Mods** and **Sets** — so a part, mod,
arcane or whole set can be listed straight from what the game says you own.

Sets carry a second purpose beyond selling: showing which components you are
missing, so you can target what to buy next.

## Background: where the data comes from

The inventory source (AlecaFrame, File or Profile) yields a
`WarframeRootObject`. The relevant buckets, measured against a real account:

| Bucket | Holds | Entries | Tradable |
| --- | --- | --- | --- |
| `RawUpgrades` | unranked mod/arcane stacks, with `ItemCount` | 893 | 851 |
| `Upgrades` | individually ranked instances, each with `UpgradeFingerprint {"lvl":N}` | 811 | 786 non-riven |
| `Recipes` | blueprints owned | 751 | 254 |
| `MiscItems` | built components, resources, gems, fish | 1004 | 262 |

"Tradable" means present in `TradableItems.json`. That lookup *is* the filter:
resources, fish and gems are absent from the cache, so they drop out without
any path heuristics. `CacheTradableItem.category` is uniform
(`"TradableItems"`) and useless for classification; `tags` is the working
discriminator.

### Sets are derivable from cache we already ship

`TradableItems.json` contains 234 items tagged `set`. Each set's `uniqueName`
is exactly the `resultType` of its blueprint recipe in `Recipes.json`. That
join resolves **234 of 234** with no misses, and the recipe's `ingredients`
give the member list.

```jsonc
// Recipes.json — Ash Prime Blueprint
{
  "uniqueName": "/Lotus/Types/Recipes/WarframeRecipes/AshPrimeBlueprint",
  "resultType": "/Lotus/Powersuits/Ninja/AshPrime",   // == ash_prime_set.uniqueName
  "ingredients": [
    { "ItemCount": 1, "uniqueName": ".../AshPrimeHelmetComponent",
      "fromRecipe": ".../AshPrimeHelmetBlueprint", "isTradeable": true },
    { "ItemCount": 5, "uniqueName": ".../OrokinCell" }   // not tradable, ignored
  ]
}
```

### The counting rule

Warframe parts and weapon parts are owned differently, and this is the one
thing that must not be got wrong:

- **Warframe parts** — the ingredient has a non-empty `fromRecipe`. You own
  the *blueprint*, in `Recipes`. Count `fromRecipe`.
- **Weapon parts** — the ingredient has an **empty** `fromRecipe`. You own the
  *built component*, in `MiscItems`. Count `uniqueName`.

So the member key is `fromRecipe` if non-empty, else `uniqueName`, decided
per ingredient. Across the 234 sets: 76 are all-`fromRecipe`, 154 are
all-empty, and 4 are mixed.

57 tradable ingredients across those sets require more than one copy, so
completion is integer division, not a presence check.

### Shared members

Three blueprints belong to two sets each, because the akimbo prime sets are
built from the single-pistol blueprint:

- `VastoPrimeBlueprint` → Vasto Prime Set, Akvasto Prime Set
- `LexPrimeBlueprint` → Lex Prime Set, Aklex Prime Set
- `BroncoPrimeBlueprint` → Bronco Prime Set, Akbronco Prime Set

Counts are computed independently per set, so owning one Vasto Prime
Blueprint shows that member as held by *both* sets even though it can only
complete one. This is a known and accepted inaccuracy: it affects 3 of 864
members, and modelling contention between sets would complicate every count
to fix a case the user can see for themselves in the expanded member list.

## Inherited defect this fixes

`CacheRecipe::can_craft` (`src-tauri/src/cache/types/cache_recipe.rs`) picks
one key mode for a whole recipe via a `from_recipe_only` flag, and returns
`false` outright when that flag is set and an ingredient's `fromRecipe` is
empty. `log_parser/types/trade.rs` works around it by trying both modes in
sequence.

The two-pass workaround covers the uniform cases but **the 4 mixed sets can
never match either pass**, so trade-log set detection silently misses them.

Fix: drop the `from_recipe_only` parameter and resolve each ingredient with
the per-ingredient rule above. `trade.rs` then makes one call instead of two.
This changes trade-log set detection — an improvement, but it is a behaviour
change outside this page, and worth calling out in `NOTICE.md`.

## Design

### 1. Set index — cache layer

`Recipes.json × TradableItems.json` is a pure cache-layer join, so it belongs
with the other cache modules, built once at load:

`src-tauri/src/cache/modules/item_set.rs`

```rust
pub struct CacheItemSet {
    pub set: CacheTradableItem,          // ash_prime_set — itself tradable
    pub members: Vec<CacheItemSetMember>,
}

pub struct CacheItemSetMember {
    pub unique_name: String,   // fromRecipe if non-empty, else uniqueName
    pub required: i64,         // ItemCount from the recipe
    pub is_main_blueprint: bool,
}
```

The member-key rule lives in exactly one function here. The module also
exposes a reverse lookup, member `unique_name` → parent sets, used by the
Parts conflict warning.

Exactly one of the 234 sets resolves to more than one recipe; take the first,
consistent with how `RecipeModule` already handles its `MultiKeyMap`.

### 2. Inventory projection

`WarframeRootObject` gains `misc_items` for `MiscItems`, which is not parsed
today and is required for weapon parts.

Fill the `ItemModule` stub and add a sibling `SetsModule`:

- **`get_parts(query)`** — `recipes` + `misc_items`, keeping entries present
  in `TradableItems`. Each row carries `in_sets: Vec<String>` so the UI can
  narrow to the 452 that belong to a set, and `in_stock_sets: Vec<String>`
  for the warning below. Both are lists because a part can belong to more
  than one set — see "Shared members" below.
- **`get_mods(query)`** — `raw_upgrades` as rank-0 rows, plus `upgrades`
  minus rivens, grouped by `(unique_name, lvl)` into rows carrying
  `sub_type: SubType { rank }`. Arcanes need no special handling; they live
  in `RawUpgrades` (111 tradable) and come along with mods.
- **`get_sets(query)`** — count each member across `recipes` + `misc_items`,
  then `complete_copies = members.map(have / required).min()`. Drop sets with
  no members owned. Sort complete-first by copies descending, then partials
  by **missing-member count ascending**, so the nearly-complete sets are the
  ones you see first.

Rows reuse `WFInvItemBase`. Sets add:

```rust
pub struct WFInvSet {
    pub base: WFInvItemBase,
    pub members: Vec<WFInvSetMember>,   // unique_name, name, have, required
    pub complete_copies: i64,
    pub owned_members: i64,
    pub total_members: i64,
}
```

### 3. Commands

`wf_inventory_get_parts`, `wf_inventory_get_mods`, `wf_inventory_get_sets`.

Each loads `StockItemQuery::get_all` once and stamps `is_in_stock` onto
`properties`, mirroring how `wf_inventory_get_rivens` does it with riven
uuids. Matching is on `(wfm_url, sub_type)`, so an unranked copy and a maxed
copy of the same mod track independently.

`get_parts` additionally stamps `in_stock_sets` from the reverse lookup.

### 4. Listing to stock

No new backend. `CreateStockItem.raw` resolves through
`cache.tradable_item().get_by(raw)`, a multi-key lookup, so **everything here
is already a valid stock item** — parts, mods, arcanes and sets alike.

Each row gets the per-row cart action the Rivens tab already uses: green when
in stock, red when not, opening the generic `prompt` modal.

| Field | Default | Bounds | Shown for |
| --- | --- | --- | --- |
| Bought | `0` | `min: 0` | all |
| Quantity | owned count, or `complete_copies` for a set | `min: 1`, `max: owned` | all |
| Rank | the row's actual rank | `min: 0`, `max: item.maxRank` | mods and arcanes |

On confirm, through `createGenericMutation` (which already handles
notifications and refetch):

```ts
{ raw: wfm_url, bought, quantity, sub_type: maxRank != null ? { rank } : undefined }
```

**Rank prefills from inventory, not from `maxRank`.**
`SelectTradableItem.handleSelect` defaults to `maxRank` because it has no
better guess; here we know the real rank, so an unranked Serration prefills
`0` and a maxed one prefills `10`. Defaulting to max would silently misprice
stock.

Bought is `0` for anything acquired in-game; the field exists so a part you
bought to flip carries its cost into the financial report.

### 5. Component-in-stock-set warning

When a part belongs to a set that is currently in stock, listing the part
separately undercuts that set listing. The row shows a warning icon naming
the sets, and the add-to-stock modal renders an alert above the fields.

**Warning only — confirm still proceeds.** No guard, no reconciliation
handler. That is deliberate for this iteration.

This needs one small shared addition: an optional `message` on
`PromptModalProps`, rendered as a Mantine `Alert` above the form.
`DynamicForm` has no slot for it today, and a field `description` is the
wrong home for a row-level warning.

The condition is **"the set is in stock"**, not "the set is complete in
inventory". If `ash_prime_set` is listed, selling its Systems separately
undercuts that listing regardless of what else is held. Since only complete
sets can be listed from the Sets tab the two readings normally coincide; they
diverge only when a set reached stock by another route (Live Scraper's Item
tab, or trade-log auto-detection), and the in-stock reading is correct there
too.

### 6. Sets tab sells complete sets only

The cart action renders only when `complete_copies >= 1`, with quantity
bounded `min: 1, max: complete_copies`. A set lists as the set item itself
(`ash_prime_set`), so there is no rank.

Incomplete rows carry no action. They exist to show what is missing, which is
why partials sort by fewest-missing-first.

Listing a set does **not** consume or hide its member parts, so a set and its
components can both sit in stock at once. §5 warns about that; nothing
prevents it.

### 7. Frontend

Three entries added to the `tabs` array in `src/pages/wf_inventory/index.tsx`,
with `Tabs/Parts/`, `Tabs/Mods/` and `Tabs/Sets/` each following the Rivens
shape (`index.tsx`, `queries.ts`, `mutations.ts`).

- **Parts** — set-member-only toggle; warning icon per §5.
- **Mods** — rank column; filter between unranked stacks and ranked instances.
- **Sets** — expandable row listing each member as `have/required` with
  missing ones badged; cart only on complete rows.

Three methods on `WfInventoryModule`, i18n keys under
`wf_inventory.tabs.{parts,mods,sets}`.

The tab stays behind the existing dev-mode gate in
`src/components/Layouts/LogIn/index.tsx`; unhiding it is a separate decision.

## Error handling

- Items absent from `TradableItems` are skipped silently — that absence *is*
  the tradability filter, not an error.
- A set whose recipe fails to resolve logs a warning and is omitted, matching
  how `RivenModule` handles an unparseable riven.
- Stock creation errors surface through `createGenericMutation`'s existing
  error notification.
- An empty inventory root yields empty tabs, not an error; the user may
  simply not have run an inventory update yet.

## Testing

Rust unit tests over small fixtures, covering the logic that is genuinely
easy to get wrong:

- member-key fallback across all three recipe shapes (all-`fromRecipe`,
  all-empty, mixed)
- `complete_copies` where `required > 1`
- tradability filter drops items absent from the cache
- rank grouping collapses equal-`lvl` instances and keeps unequal ones apart
- `can_craft` regression for the mixed-ingredient case that fails today

Then the usual `ddev check` gates: eslint, `pnpm build`, `cargo fmt --check`,
`cargo clippy -D warnings`, `cargo test`.

The UI is not unit-testable here, so verify by running the app against the
real inventory in `local/warframe/`, as with the Rivens tab.

## Out of scope

- Any guard or reconciliation when a set and its parts are both in stock.
- Pricing or valuation in the inventory tabs; stock and the live scraper own
  that.
- Unhiding the WF Inventory nav entry outside dev mode.
- `WFInventorySource::Game` (issue #1), which changes where inventory comes
  from but not how it is projected here.

## Expected scale

Measured against the real account these tabs will show:

| Tab | Rows | Notes |
| --- | --- | --- |
| Parts | 516 | 452 belong to a set |
| Mods | 851 unranked + ~786 ranked | 15,847 copies; 111 arcanes |
| Sets | 207 | 25 complete, 28 total copies, 182 partial |
