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
