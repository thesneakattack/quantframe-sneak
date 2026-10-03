# Working on this fork

## What this is

A modified build of [Quantframe](https://github.com/Kenya-DK/quantframe-react) by
Kenya-DK, in an independent repository rather than a GitHub fork. See
[`NOTICE.md`](../NOTICE.md) for the full list of changes and the GPL-3.0 position.

`78c87d1` is the pristine upstream snapshot. To see everything this fork changes:

```bash
git diff 78c87d1..HEAD
```

## Pulling in upstream changes

**This is a hard fork. Merging upstream is no longer a realistic workflow.**

That was a deliberate choice, and it has a cost worth understanding. The cleanup
commits reformatted every Rust file with rustfmt, applied ~490 clippy fixes and
repaired 27 doctests. Upstream did none of that, so a merge would now conflict
across essentially the whole of `src-tauri/`, not just the handful of files in
`NOTICE.md`.

If you need a specific upstream fix, cherry-pick the change by hand rather than
merging:

```bash
git remote add upstream https://github.com/Kenya-DK/quantframe-react.git
git fetch upstream development
git log upstream/development --oneline        # find the change
git show <sha>                                # read it, then apply by hand
```

Diffing a single file against upstream is still useful for orientation:

```bash
git diff upstream/development -- src-tauri/src/live_scraper/
```

If you later decide you *do* want to track upstream again, reverting the
formatting commit (see `.git-blame-ignore-revs`) is the first step, and the
clippy commit is the second. Both are isolated and contain nothing else.

## Development environment

Everything builds inside DDEV, so neither Rust nor pnpm needs to be installed on
the host. The container compiles the Rust backend and the frontend, and — on a
WSL2 host — can also open the desktop window through WSLg.

Note that **Windows is the real target**: that is where Warframe writes `EE.log`
and where AlecaFrame stores its data. A Linux run is a smoke test for the UI and
boot path, not a functional test of the trading features.

```bash
ddev start          # build/start the container (first run is slow: Rust + webkit2gtk)
ddev pnpm install   # install frontend dependencies
ddev check          # eslint, tsc, vite build, cargo fmt, clippy, cargo test
```

Individual tools:

```bash
ddev cargo check              # cargo, run from src-tauri/
ddev cargo clippy --all-targets
ddev tauri build --debug --no-bundle    # full backend compile, no bundling
ddev pnpm dev                 # Vite dev server -> https://quantframe-sneak.ddev.site:1421
ddev ssh                      # shell inside the container
```

### Running the GUI under WSLg (Linux smoke test)

`.ddev/docker-compose.wslg.yaml` mounts the WSLg X11 socket into the web container
and pins `GDK_BACKEND=x11`, plus the WebKitGTK flags that stop it rendering a blank
window without a GPU. With that in place:

```bash
ddev tauri dev                # opens a real window on your WSLg desktop
ddev exec import -window root /var/www/html/screenshot.png   # capture it
```

**That file is WSL2-only.** `/mnt/wslg` and `/tmp/.X11-unix` do not exist on macOS
or plain Linux, so delete `.ddev/docker-compose.wslg.yaml` there or `ddev start`
will fail trying to bind-mount them.

### Starting from your existing Quantframe data

The rebrand gave this build its own bundle identifier, which is what lets it sit
alongside official Quantframe without touching its database. The cost is that it
starts empty — no login, no settings, no history.

`scripts/seed-from-upstream.sh` copies that state across on Windows (run it from
WSL):

```bash
scripts/seed-from-upstream.sh --dry-run   # show what would be copied
scripts/seed-from-upstream.sh             # apply
```

It brings over `quantframeV2.sqlite`, `settings.json`, `auth.json` and the item
cache, and deliberately leaves behind the WebView2 profile and the other install's
logs. It refuses to overwrite a non-empty target unless given `--force`, so it
cannot silently clobber state once this build has its own.

> **Do not run both builds with the live scraper enabled at the same time.** They
> manage the same warframe.market orders and will fight — each independently
> creating, repricing and deleting the other's listings.

### Choosing which API the app talks to

Upstream compiled the endpoint in: `qf_api` picked `DEVELOPMENT_URL`
(`http://localhost:6969`) or `PRODUCTION_URL` (`https://api.quantframe.app`) purely
from `cfg!(dev)`, and its README told you to edit `client.rs` and rebuild to change
it. A dev build therefore could not reach the real API, and a self-hosted server
could not be pointed at without recompiling.

This fork resolves the base URL at runtime. Highest precedence first:

| Source | Example |
| --- | --- |
| `QF_API_URL` environment variable | `QF_API_URL=https://api.quantframe.app ddev tauri dev` |
| `advanced_settings.qf_api_url` in `settings.json` | `"qf_api_url": "http://localhost:6969"` |
| Compiled default | dev build → localhost:6969, release build → production |

Nothing changes if you set neither: the compiled defaults are the fallback, so
behaviour matches upstream out of the box.

This makes two things possible that were not before:

- **Run a dev build against the real API.** Useful because some UI is gated on
  `import.meta.env.DEV` — the WF Inventory panel, for instance, only appears in a
  dev build. Previously that meant choosing between the panel and a working
  backend.
- **Point at a self-hosted API** without touching source. See the
  [self-hosting research](superpowers/research/2026-10-03-self-hosting-the-quantframe-api.md);
  the `#[ignore]`d `qf_api` tests already target `localhost:6969`.

If no server answers, the app still boots and renders but shows
`Error in QFClient:AlertGetAlerts component`. That is the endpoint being
unreachable, not a crash.

### Real game data: `local/`

Testing against real data needs AlecaFrame's `lastData.dat`, Warframe's `EE.log`
and the app's own database and login. All of it lives in `local/`, which is
**gitignored**:

```
local/
  warframe/
    lastData.dat      copied from %LOCALAPPDATA%\AlecaFrame
    inventory.json    decrypted from the above
    EE.log            copied from %LOCALAPPDATA%\Warframe (a snapshot)
  appdata/            the app's data directory: settings.json, auth.json,
                      quantframeV2.sqlite, cache, logs
```

Two reasons it is here and not read from `C:\` through a bind mount. The project
directory is already mounted into the container, so these are visible from both
host and container with no machine-specific mount; and the container's own home
is ephemeral, so an app data directory there loses the database and login on
every rebuild. A `post-start` hook symlinks
`~/.local/share/dev.thesneakattack.quantframe` to `local/appdata` so that
persists.

Populate it from the Windows side, running on the host rather than in the
container:

```bash
scripts/sync-local-data.sh
```

`EE.log` is a snapshot — the game appends to it continuously — so re-run the
script to refresh. Everything in `local/` is account data, including the in-game
names of everyone you have traded or chatted with, so it must never be committed.

### Inventory tabs

Rivens, Parts, Mods and Sets. Each row lists to Stock through the normal
`stock_item_create`, taking a bought price (0 for anything acquired in-game), a
quantity, and a rank for mods and arcanes. The rank prefills from the inventory
row, not from the item's maximum, so an unranked copy does not list as a maxed
one.

Parts come from the `Recipes` and `MiscItems` buckets. Presence in
`TradableItems.json` is the filter: resources, fish and gems are absent from
that cache and drop out without any path matching. Mods come from
`RawUpgrades` (unranked stacks) and `Upgrades` (individually ranked instances,
grouped by rank); arcanes live in `RawUpgrades` alongside them.

Sets are derived by joining items tagged `set` in `TradableItems.json` to their
blueprint recipe in `Recipes.json` by `resultType`, which resolves for all 234
of them. Which unique name counts as a member depends on the ingredient:
Warframe parts are owned as blueprints and carry a `fromRecipe`, weapon parts
are owned as built components and do not. Only complete sets can be listed;
incomplete ones are shown so you can see which component to buy next, ordered
by how few are missing.

Listing a part whose set is already in stock shows a warning but is not
blocked. A blueprint shared between two sets — the three akimbo primes — counts
toward both (issue #3).

### Getting WF Inventory to work

The WF Inventory panel reads the `Upgrades` field of your
Warframe inventory. None of the three sources supplies it out of the box on an
account without the AlecaFrame entitlement:

- **Profile** returns Warframe's public profile, which has no inventory fields at
  all. It is for syndicate standings and mastery rank.
- **AlecaFrame** reads `lastData.dat`, but asks `api.quantframe.app` for the AES
  key, and that endpoint answers **403** without the entitlement.
- **File** wants plain JSON, and `lastData.dat` is encrypted.

The key and IV are static AES-128-CBC values, not per-account secrets. They are
**not stored in this repository** — it is public, and the values originate in a
Commons-Clause project, so republishing them here would add nothing and muddy the
licensing. Obtain them yourself; they are compile-time constants in AlecaFrame and
appear in [Sainan/warframe-api-helper](https://github.com/Sainan/warframe-api-helper).

Two ways to use them.

**Let the app decrypt (recommended).** Set both values, 32 hex characters each,
and the AlecaFrame source stops calling the API entirely:

```jsonc
// settings.json
"advanced_settings": {
  "wf_decrypt_key": "...32 hex chars...",
  "wf_decrypt_iv":  "...32 hex chars..."
}
```

or, taking precedence over those, `QF_WF_DECRYPT_KEY` and `QF_WF_DECRYPT_IV`.

Both must be set and valid; one alone is ignored with a warning, because a
half-configured pair is almost always a typo and silently falling back to the API
would surface as the same 403 it was meant to avoid. This keeps the file watcher,
so the inventory follows AlecaFrame's updates with no further work.

**Or decrypt externally** into the File source:

```bash
WF_DECRYPT_KEY=... WF_DECRYPT_IV=... scripts/decrypt-alecaframe.sh
```

That writes `inventory.json` beside `lastData.dat` and validates the result
rather than leaving a wrong key to produce a file the app silently ignores. Point
WF Inventory at it with the **File** source. The trade-off is that the JSON is a
snapshot: re-run the script whenever AlecaFrame refreshes.

The script reads `.env.local` (gitignored) if present, so the keys need not be on
the command line.

#### A note on warframe-api-helper

That tool pulls a fresh inventory straight from `mobile.warframe.com` and writes
both `inventory.json` and an AlecaFrame-compatible `lastData.dat`, which makes it
an alternative to running AlecaFrame at all. It works by scraping
`?accountId=...&nonce=...` out of the running game's memory, and requires finding
three identical copies of that string. The nonce advances as the game makes its
own API calls, so a long-running session accumulates stale nonces and the scan
starts failing with "Failed to gruzzle the crumbs". Restarting Warframe and
running the tool promptly is the usual remedy.

Its licence is MIT **plus Commons Clause**, which is not an open-source licence
and is incompatible with GPL-3.0, so none of its code can be vendored here.

### Producing a Windows build

Cross-compiling Tauri from Linux to Windows is not a supported path, so Windows
builds go through CI on a real `windows-latest` runner:

```bash
gh workflow run windows-build.yml
gh run watch                                  # wait for it
gh run download <run-id> -D ./dist-windows    # NSIS installer, MSI, bare exe
```

`.github/workflows/windows-build.yml` exists specifically for this. The inherited
`build.yml` also builds Windows, but it fans out across four platforms and opens a
draft release, which is more than you want just to try a build.

The result needs the **WebView2 runtime** on the target machine. Windows 11 and
up-to-date Windows 10 ship it; older installs may need the Evergreen bootstrapper.

Nothing is code-signed, so SmartScreen will warn on first run.

### Why builds don't pollute your working tree

`CARGO_HOME` and `CARGO_TARGET_DIR` point into DDEV's named cache volume
(`/mnt/ddev-global-cache/...`), not into the bind-mounted project directory. The
container also runs as your host UID/GID (1000:1000). Between the two, nothing the
container writes ever appears in your checkout as a root-owned file, and there is
no `src-tauri/target/` on the host to clean up.

The cost: the Rust build cache is tied to the DDEV volume. `ddev delete` or a
Docker volume prune discards it and the next build is a cold one.

### Disk usage

A full debug build of this workspace is large -- roughly **14 GB** of cargo target
output, plus ~450 MB of registry and ~240 MB of pnpm store. On Docker Desktop that
lives inside the VM's virtual disk, not on your WSL filesystem, so it does not show
up in `du` on the host but it does consume real space.

To see what is being used, and to reclaim it:

```bash
ddev exec 'du -sh /mnt/ddev-global-cache/*'
ddev exec 'cargo clean --manifest-path /var/www/html/src-tauri/Cargo.toml'   # target only
ddev exec 'rm -rf "$CARGO_TARGET_DIR"'                                       # same, bluntly
```

The cache volume is shared across all your DDEV projects, so do not prune it
wholesale unless you mean to cold-build everything.

## Activating the updater

The updater is deliberately a stub: registered as a plugin, granted its
capabilities, but with an empty `pubkey` and an endpoint that 404s. `check()`
fails and the app treats that as "no update available". To turn it on:

1. Generate a signing keypair — the private key must never be committed:

   ```bash
   ddev tauri signer generate -w ~/.quantframe-sneak-updater.key
   ```

2. Put the printed **public** key in `src-tauri/tauri.conf.json` under
   `plugins.updater.pubkey`.

3. Set `bundle.createUpdaterArtifacts` to `true` in the same file.

4. Add the private key and its password to this repository's GitHub Actions
   secrets as `TAURI_PRIVATE_KEY` and `TAURI_KEY_PASSWORD`, and restore the
   `TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` env block in
   `.github/workflows/build.yml`.

The endpoint in `tauri.conf.json` already points at this repository's
`releases/latest/download/latest.json`, which `tauri-action` produces once
updater artifacts are enabled. No code changes are needed.

## Research

- [Self-hosting the Quantframe API](superpowers/research/2026-10-03-self-hosting-the-quantframe-api.md)
  — feasibility study of replacing `api.quantframe.app`. Concludes it is doable
  incrementally; explains why the two "broken" `qf_api` tests should be kept.

## Inherited problems, and what was done about them

Everything below was pre-existing in the upstream tree, not introduced by this
fork. Each was verified against the baseline snapshot before being touched.

| Problem | Resolution |
| --- | --- |
| `pnpm lint` exited 2: no eslint config anywhere, and `eslint` was not a declared dependency | Added the project's first eslint config (flat, `eslint.config.js`) and the toolchain. `pnpm lint` now exits 0 |
| The tree was not rustfmt-clean | Formatted in one commit, listed in `.git-blame-ignore-revs` so `git blame` skips it |
| ~1200 clippy warnings, including a deny-level error that made clippy fail outright | `cargo clippy --workspace --all-targets -- -D warnings` now exits 0 |
| 27 doctests did not compile (26 in `utils`, 1 in `qf_api`) | All repaired. Most were missing imports; four had drifted out of sync with the signatures they document |
| Two `qf_api` tests could not pass unattended | Marked `#[ignore]` with reasons rather than deleted - they are the start of an integration suite for a self-hosted API |

Three real defects surfaced while doing this, none of them style:

- **`utils/src/file_watcher.rs`** tested `*pos > size || size < *pos` - the same
  comparison twice, transposed. That was the deny-level clippy error. Behaviour
  was unaffected, but in the same lines the truncation trace logged the position
  *after* zeroing it, so every rotation event reported `Current Position: 0`.
- **`qf_api` `Display for ApiError`** ran `println!` on every parsing error it
  formatted, leaking debug output to stdout from logging paths.
- **A stray `debugger;`** sat in the save handler of the item-details edit form,
  which would halt the app for anyone with devtools open.

One more is worth knowing about rather than fixing blind:
`WFInvItemRaw::is_riven` had a three-branch chain covering all four combinations
of its two conditions, every one returning `true`, making the trailing `false`
unreachable. The function was always equivalent to its leading path-prefix check.
It was collapsed without changing behaviour, but the dead branches suggest one
was meant to return `false` and never did. If riven detection is supposed to be
stricter than a path match, that was never actually implemented.

## Known debt

Deliberately deferred, recorded so it stays visible:

| Item | Size | Why not now |
| --- | --- | --- |
| `clippy::result_large_err` | ~490 | `utils::Error` is 136+ bytes and returned by value from nearly every fallible function. Satisfying it means boxing the error type across every signature in the workspace - a real refactor. Allowed in `src-tauri/Cargo.toml` with this reasoning |
| `@typescript-eslint/no-explicit-any` | ~600 | `any` is used pervasively. Typing it properly is its own project and wants test coverage first |
| `react-hooks/rules-of-hooks` | ~455 | One pattern, not 455 defects: every module under `src/api/` is a class whose methods call `useQuery`/`useMutation`. Since the methods are not named `use*`, nothing stops a caller invoking one from an event handler or a conditional - it works only because callers happen to use them during render |
| Test coverage | - | There are no meaningful tests. Every suite reports 0 passed. `ddev check` verifies that the code compiles, lints and formats cleanly; it does **not** verify that it behaves correctly |

Eight further clippy lints covering design and API shape (`too_many_arguments`,
`module_inception`, `type_complexity` and similar) are allowed at workspace level
with reasons in `src-tauri/Cargo.toml`, listed individually so new instances are
visible in review rather than vanishing into a warning count.

## What `ddev check` enforces

Every gate blocks: eslint, `tsc` + `vite build`, `cargo fmt --check`,
`cargo clippy -- -D warnings`, `cargo test --workspace --all-targets`, and
`cargo test --doc`. eslint fails on errors only - its ~1150 warnings are the
tracked debt above.

`ddev check --integration` additionally runs the `#[ignore]`d `qf_api` tests,
which need a Quantframe API server on `http://localhost:6969`.

CI (`.github/workflows/pr-build-check.yml`) only builds; it runs no tests, no
linting and no formatting check. `ddev check` is substantially stricter.

## Activating the updater

The updater is deliberately a stub: registered as a plugin, granted its
capabilities, but with an empty `pubkey` and an endpoint that 404s. `check()`
fails and the app treats that as "no update available". To turn it on:

1. Generate a signing keypair — the private key must never be committed:

   ```bash
   ddev tauri signer generate -w ~/.quantframe-sneak-updater.key
   ```

2. Put the printed **public** key in `src-tauri/tauri.conf.json` under
   `plugins.updater.pubkey`.

3. Set `bundle.createUpdaterArtifacts` to `true` in the same file.

4. Add the private key and its password to this repository's GitHub Actions
   secrets as `TAURI_PRIVATE_KEY` and `TAURI_KEY_PASSWORD`, and restore the
   `TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` env block in
   `.github/workflows/build.yml`.

The endpoint in `tauri.conf.json` already points at this repository's
`releases/latest/download/latest.json`, which `tauri-action` produces once
updater artifacts are enabled. No code changes are needed.

## Research

- [Self-hosting the Quantframe API](superpowers/research/2026-10-03-self-hosting-the-quantframe-api.md)
  — feasibility study of replacing `api.quantframe.app`. Concludes it is doable
  incrementally; explains why the two "broken" `qf_api` tests should be kept.

## Inherited problems

These are all pre-existing in the upstream tree, not caused by the fork. They are
recorded here so nobody re-diagnoses them, and so it is clear why `ddev check`
treats some things as advisory rather than fatal.

| Problem | Detail |
| --- | --- |
| `pnpm lint` does not run | `package.json` defines a `lint` script, but the repo contains no eslint config and does not declare `eslint` as a dependency (8.57.0 only resolves transitively via `@typescript-eslint/*`). Exits 2 on a clean checkout. |
| The tree is not rustfmt-clean | `cargo fmt --all -- --check` reports diffs across many files. Running `cargo fmt` would produce an enormous, review-hostile commit, so it has been left alone. |
| The tree is not clippy-clean | `cargo check` alone emits 58 warnings, mostly dead code and unused variables. |
| Two `qf_api` tests cannot pass | `tests::client::print_token` asserts a real user token is present in the environment; `tests::client::test_cache_extract` expects upstream's development backend on `http://localhost:6969`. Both are environment tests mislabelled as unit tests and fail on any clean checkout. |
| 26 doctests fail to compile | Doc examples in `src-tauri/utils` (`helper.rs`, `options.rs`, `zip_folder.rs`) reference functions and types without importing them, so `cargo test --doc` fails. The gate uses `--all-targets`, which excludes doctests. |
| No test coverage to speak of | Every other crate reports `0 tests`. `src-tauri/service/tests/mock.rs` exists but defines no test cases. |

`ddev check` runs the hard gates (frontend build, `cargo check --workspace`,
`cargo test --workspace --exclude qf_api --all-targets`) as blocking, and the rest as advisory.
`ddev check --strict` makes everything blocking — useful once a cleanup actually
happens.

Note that CI (`.github/workflows/pr-build-check.yml`) does not run tests at all; it
only builds. So `ddev check` is already a stricter gate than CI.
