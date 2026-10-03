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

There is no `upstream` remote by default, because this is not a fork. To merge
upstream work in when you want it:

```bash
git remote add upstream https://github.com/Kenya-DK/quantframe-react.git
git fetch upstream development
# Upstream history is unrelated to ours, so the first merge needs:
git merge upstream/development --allow-unrelated-histories
```

Expect conflicts in the files listed in `NOTICE.md` — those are exactly the files
this fork diverges in. Resolve in favour of our identifiers, not upstream's.

## Development environment

Everything builds inside DDEV, so neither Rust nor pnpm needs to be installed on
the host. The container is **build-and-check only** — it compiles the Rust backend
and the frontend, but does not open the desktop window. Run the actual app on
Windows, which is where Warframe's `EE.log` and AlecaFrame data live anyway.

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
