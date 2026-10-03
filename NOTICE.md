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
| Build environment | Added `.ddev/` — a containerised Rust + Node toolchain for reproducible builds and checks | New; does not alter program behaviour. |

## Unchanged

The following remain pointed at upstream-operated infrastructure, because the
application is a client of that service and does not function without it:

- `src-tauri/qf_api/src/client.rs` — `https://api.quantframe.app` (authentication, analytics)
- `src/components/Modals/PatreonModal` — Patreon OAuth link endpoint
- `src-tauri/src/app/client.rs` — the `Quantframe/<version>` User-Agent, kept so the
  upstream API continues to recognise the client
- `src/contexts/static/theme.ts` — default theme still credited to Kenya-DK, which is correct

If this project ever needs to stand alone, those are the couplings to replace.
