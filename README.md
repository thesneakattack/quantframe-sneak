# Quantframe Sneak

A modified build of **[Quantframe](https://github.com/Kenya-DK/quantframe-react)** by
[Kenya-DK](https://github.com/Kenya-DK) — a Warframe Market trading assistant built with
Tauri, React and Rust.

> **This is not official Quantframe.** It is an independent, modified version.
> Do not report problems with this build to the upstream project. See
> [`NOTICE.md`](./NOTICE.md) for exactly what was changed and why.
>
> It installs under its own identifier (`dev.thesneakattack.quantframe`), so it keeps a
> separate database and settings directory and will not disturb an official Quantframe
> installation on the same machine.

If you want the real thing, maintained by its author, go to
[Kenya-DK/quantframe-react](https://github.com/Kenya-DK/quantframe-react) — and consider
supporting their work via [Buy Me a Coffee](https://www.buymeacoffee.com/kenyadk) or
[Patreon](https://patreon.com/kenya_dk).

## What it does

- Warframe Market API client — orders, auctions, rivens, syndicate items
- Live scraper: an automated buy/sell pipeline with profit thresholds and order cooldowns
- Log parser that watches Warframe's `EE.log` to detect in-game trades
- Inventory sync from a Warframe profile, AlecaFrame, or a plain JSON file
- Trading analytics with transaction history and profit charting
- Local SQLite storage, inspectable with tools like [Beekeeper Studio](https://beekeeperstudio.io)

Data and logs live in `%LOCALAPPDATA%\dev.thesneakattack.quantframe\` on Windows.

## Stack

| Layer | Technology |
| --- | --- |
| Shell | [Tauri 2](https://tauri.app) — Rust backend, system webview, no Chromium |
| Frontend | [React 19](https://react.dev) + [Mantine 9](https://mantine.dev), TanStack Query, i18next |
| Backend | Rust workspace: app + `qf_api`, `service`, `entity`, `migration`, `utils` |
| Storage | SQLite via [SeaORM](https://www.sea-ql.org/SeaORM/), with migrations |

## Building

The repository ships a [DDEV](https://ddev.com) environment so you do not need Rust or
pnpm on your host. It is a **build-and-check** environment — it compiles everything but
does not open the desktop window; run the app itself on Windows.

```bash
ddev start          # first run builds the image: Rust toolchain + webkit2gtk
ddev pnpm install
ddev check          # eslint, tsc, vite build, cargo fmt, clippy, cargo test
```

See [`docs/FORK.md`](./docs/FORK.md) for the full command list, how upstream changes get
merged in, and how to activate the (currently stubbed) auto-updater.

### Building without DDEV

Install the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) and Node,
then:

```bash
pnpm install
pnpm tauri build
```

On Windows, build on Windows — not in WSL.

## Licence

GPL-3.0-only, inherited from the upstream project. See [`LICENSE`](./LICENSE) and
[`NOTICE.md`](./NOTICE.md).

Quantframe was itself inspired by
[Akmayer's Warframe-Algo-Trader](https://github.com/akmayer/Warframe-Algo-Trader);
[`docs/readme.md`](./docs/readme.md) maps the two implementations against each other.
