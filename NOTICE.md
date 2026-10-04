# Notice of modification

This program is a modified version of **Quantframe**, originally created by
**Kenya-DK** and distributed under the GNU General Public License v3.0.

- Upstream project: https://github.com/Kenya-DK/quantframe-react
- Upstream branch used as the baseline: `development`
- Upstream version at the point of divergence: `1.6.29`
- Baseline imported: 3 October 2026

This repository is **not** a fork in the GitHub sense and carries none of the
upstream commit history. Commit `78c87d1` is a verbatim, unmodified snapshot of
the upstream tree, so every change made here is visible as a diff against it.

In accordance with GPL-3.0 §5(a), the following changes have been made to the
original work. This list is maintained as changes are made.

## 3 October 2026 — initial divergence

| Area | Change | Reason |
| --- | --- | --- |
| Application identity | `productName` → `Quantframe Sneak`; `mainBinaryName` → `QuantframeSneak`; window title → `Quantframe Sneak vX.Y.Z - based on Quantframe by Kenya-DK` | GPL-3.0 §5(a) requires modified versions be marked as changed. Also prevents confusion with official builds. |
| Bundle identifier | `dev.kenya.quantframe` → `dev.thesneakattack.quantframe` (`tauri.conf.json`, `src-tauri/src/helper.rs`) | Gives this build its own SQLite database, settings and log directory, so it can be installed and run side by side with official Quantframe without corrupting its data. |
| Auto-updater | Upstream minisign public key and the `api.quantframe.app` release endpoint removed. The plugin remains registered with an empty `pubkey` and an endpoint pointing at this repository's own release feed. Updater artifact generation disabled. | This build must never silently replace itself with an official Quantframe release, and it cannot be signed with a key we do not hold. Left as a working stub so it can be activated later — see `docs/FORK.md`. |
| Update check | `checkForUpdates` in `src/contexts/app.context.tsx` now tolerates updater failure and reports "no update" instead of surfacing an error | The stub endpoint returns 404 until real releases exist. |
| Links | Release and Discord-avatar URLs repointed at this repository | Upstream URLs would serve official artifacts. |
| Licensing metadata | `src-tauri/Cargo.toml`: `license` set to `GPL-3.0-only` (upstream left it empty), `authors` and `repository` updated | The upstream manifest did not declare the license its `LICENSE` file grants. |
| API endpoint | The Quantframe API base URL is resolved at runtime — `QF_API_URL`, then `qf_api_url` in the project-root `config.json`, then the compiled default — instead of being fixed at compile time by `cfg!(dev)` | Upstream required editing `qf_api/src/client.rs` and rebuilding to change endpoint, which blocked both running a dev build against the real API and pointing at a self-hosted server. Defaults are unchanged, so behaviour without configuration matches upstream. |
| AlecaFrame decryption | The AES key and IV may be supplied locally, as sixteen byte values under `wf_decrypt_key`/`wf_decrypt_iv` in the project-root `config.json`, or as hex in `QF_WF_DECRYPT_KEY`/`QF_WF_DECRYPT_IV`, instead of only from `/alecaframe/decrypt-keys` | That endpoint returns 403 on accounts without the entitlement, making the AlecaFrame inventory source unusable for them. The keys are static rather than per-account, and are not shipped here. The API remains the default when nothing is configured. |
| Fork configuration | The fork's own settings live in a gitignored `config.json` in the project root, not in the app-data `settings.json` | `settings.json` is seeded from upstream Quantframe's own settings, so anything stored there is liable to be wiped by a re-seed and blurs the line between the two apps' state. |
| WF Inventory: Parts, Mods and Sets | Three new tabs listing owned tradable parts, mods and item sets, each with a warframe.market value, an owned count and a one-click add to stock. Sets show component completion and only complete sets can be listed. Filters for minimum value (tied to the live scraper's `min_profit`), spare count, vault status and mastery; sort by several columns in priority order | Upstream's WF Inventory had only Rivens and Syndicates, so there was no way to see which owned parts and mods are worth listing for the scraper to sell. |
| Inventory valuations | Values come from the bundled item-price cache, with anything it does not cover filled in from warframe.market by a background trickle (10 every 30s) that refreshes the most valuable first and remembers what it learns across restarts. A resolved value stands for a day before being asked again. The figure is the 48 hour volume-weighted average where the item has traded that recently, falling back to the 90 day average where it has not | Pricing on demand made sorting hang and would have hammered the endpoint. Nothing in the tabs needs an up-to-the-second figure. Refreshing hourly was self-defeating: re-asking every known price every hour consumed the entire request budget, so rows with no price at all stayed queued behind it indefinitely and never resolved. A daily cadence is still well inside the 48 hour window the figure describes, and leaves the budget free to finish the dataset. The fallback exists because most of the inventory does not trade every two days: over a sample of 25 rows carrying no price, 48 hours could price 3 where 90 days could price 21, so without it most mods and relics stay permanently unpriced. |
| Build environment | Added `.ddev/` — a containerised Rust + Node toolchain for reproducible builds and checks | New; does not alter program behaviour. |

## Defect fixes

Bugs found and fixed here that are present upstream. Listed separately from the
table above because they restore intended behaviour rather than diverge from it.

| Defect | Effect |
| --- | --- |
| `rename_all = "snake_case"` split the acronyms in four `ApplicationEvent` variants, emitting `w_f_inventory_update` and `w_f_m_auction_*` where the API accepts `wf_inventory_update` and `wfm_auction_*` | Those events were rejected with HTTP 400. Because a failed flush re-queues its batch, a single one became a poison pill that stalled the whole analytics queue and retried every 10 seconds indefinitely. Now covered by regression tests |
| `utils::file_watcher` tested `*pos > size \|\| size < *pos` — the same comparison transposed | Dead operand; separately, the truncation trace logged the position after zeroing it, so every rotation reported `Current Position: 0` |
| `qf_api` `Display for ApiError` called `println!` | Debug output leaked to stdout whenever a parsing error was formatted, including from logging paths |
| A stray `debugger;` in the item-details save handler | Halted the app for anyone with devtools open |
| `WFInvItemRaw::is_riven` had a branch chain that returned `true` from every path | The trailing `false` was unreachable; the function only ever tested its path prefix. Collapsed without changing behaviour, but the dead branches suggest unimplemented intent |
| `CacheRecipe::can_craft` chose one ingredient-key mode for a whole recipe via a `from_recipe_only` flag, and returned `false` outright when that flag met an ingredient with no `fromRecipe` | The trade log parser compensated by trying both modes in sequence, which covers recipes whose ingredients are uniformly one kind but never the four sets that mix blueprint-backed Warframe parts with bare weapon components. Those sets were invisible to trade set detection. The key is now resolved per ingredient and the fallback pass is gone. Now covered by regression tests |

## Unchanged

The following remain pointed at upstream-operated infrastructure, because the
application is a client of that service and does not function without it:

- `src-tauri/qf_api/src/client.rs` — `https://api.quantframe.app` (authentication, analytics)
- `src/components/Modals/PatreonModal` — Patreon OAuth link endpoint
- `src-tauri/src/app/client.rs` — the `Quantframe/<version>` User-Agent, kept so the
  upstream API continues to recognise the client
- `src/contexts/static/theme.ts` — default theme still credited to Kenya-DK, which is correct

If this project ever needs to stand alone, those are the couplings to replace.
