# Self-hosting the Quantframe API — research findings

**Date:** 3 October 2026
**Type:** Spike (feasibility investigation). No code was written.
**Question:** Can `api.quantframe.app` realistically be self-hosted, and what is the
minimum viable server that makes this client fully functional?
**Verdict:** Feasible, incrementally. No hard blockers.

> **Corrected 3 October 2026.** This document originally called
> `/alecaframe/decrypt-keys` the one hard blocker to self-hosting. That was
> wrong. The endpoint returns a *static* AES key and IV, not per-account
> secrets, so a self-hosted server can simply serve them. See
> [Correction](#correction-the-decrypt-keys-are-not-a-blocker).

> Status: this is research, not a commitment. `api.quantframe.app` remains the
> configured backend and nothing in the client has been repointed. See
> [`NOTICE.md`](../../../NOTICE.md) for the couplings deliberately left in place.

## Summary

The Quantframe API is **not** the trading engine and does **not** hold proprietary
data. The desktop client talks to warframe.market directly for all actual trading.
The QF API is an account system plus a caching and aggregation convenience layer
over public data sources. That makes it reimplementable.

There is no blocker. The one endpoint that looked like one,
`/alecaframe/decrypt-keys`, returns static values that a replacement can serve
directly.

## What the API actually is

Two findings reframed this investigation.

**The client never pushes data upward.** The only non-GET requests in the entire
`qf_api` crate are `/auth/login`, `/auth/logout`, `/users` and `/events/track`. No
client contributes price data, so the market endpoints cannot be a crowd-sourced
dataset. The server derives them independently.

**The price schema is warframe.market's own.** Comparing `qf_api`'s `ItemPrice`
against a live call to `https://api.warframe.market/v1/items/{slug}/statistics`:

```
WFM statistics_closed:  avg_price closed_price datetime donch_bot donch_top id
                        max_price median min_price moving_avg open_price volume
                        wa_price

QF ItemPrice:           avg_price closed_price          donch_bot donch_top
                        max_price median min_price moving_avg open_price volume
                        wa_price
                        + order_type supply demand
                        + name wfm_url wfm_id uuid sub_type tags trading_tax
```

An exact superset. `/market/items` re-serves warframe.market's public statistics,
enriched with item metadata from the cache bundle.

**Where the real trading happens.** The app uses the `wf-market` crate against
warframe.market directly in 74 places across `helper.rs`, `handlers/`,
`live_scraper/`, `commands/` and `log_parser/`. Orders, auctions, chat and
inventory never transit the QF API. Its value is doing the ~3000-item statistics
sweep once, centrally, instead of in every client.

## Endpoint-by-endpoint assessment

| Endpoint | Difficulty | How to serve it yourself |
| --- | --- | --- |
| `GET /health` | trivial | `{"status":"ok"}` |
| `POST /auth/login` | trivial | Returns `UserPrivate`: `id`, `name`, `banned`, `banned_reason`, `banned_until`, `permissions`, `token`, `patreon_tier` |
| `GET /auth/me` | trivial | Same shape, from the bearer token |
| `POST /auth/logout` | trivial | Returns a string |
| `POST /users` | trivial | Registration; returns `UserPrivate` |
| `POST /events/track` | trivial | Body is `{"events":[{event, properties}]}`, batched 50 at a time every 10s. Accept and discard |
| `GET /alert?…` | trivial | Return an empty `Paginated<Alert>` |
| `GET /market/users/activity?…` | trivial | Returns *Quantframe's own* userbase charts (`labels`, `registered_users_chart`, `total_users_chart`). Return your own figures or zeros |
| `GET /cache/md5?type=cache` | moderate | md5 of the bundle; the client skips download when unchanged |
| `GET /cache/download?type=cache` | moderate | A zip extracted over the cache dir. Must satisfy 32 module loaders (`warframe`, `weapon`, `mods`, `relics`, `arcane`, `riven_good_roll`, `tradable_items`, …). Upstream is almost certainly Kenya-DK's **public** `warframe-public-export-plus` repo |
| `GET /market/items?…` | moderate | WFM `/v1/items/{slug}/statistics`, 1:1 field mapping. `supply`/`demand` from live orders |
| `GET /market/items/{id}` | moderate | Same, single item, returns `ItemPriceDetails` |
| `GET /market/syndicate?…` | moderate | Syndicate metadata (`syndicate`, `standingCost`, `syndicateUniqueName`) joined with WFM prices |
| `GET /market/rivens?…` | heavy | Aggregate WFM riven auctions. Only 6 fields (`volume`, `min_price`, `max_price`, `avg_price`, `median_price`, `datetime`) but the auction aggregation is real work |
| `GET /market/rivens/{id}` | heavy | Same, single riven |
| `GET /alecaframe/decrypt-keys` | trivial | Returns a fixed AES-128-CBC `key` and `iv` as byte arrays, used to decrypt AlecaFrame's `lastData.dat`. The values are static, not per-account, so a replacement returns two constants. Obtaining them is a one-off; see the correction below |

### Confirmed empirically (3 October 2026)

The decrypt-keys blocker was predicted from reading the client. It has since been
observed directly, running the app against the production API with a valid
session:

```
[CRITICAL] DecryptLastData:GetKeys  Failed to get decrypt keys:
  403 Forbidden  GET https://api.quantframe.app/alecaframe/decrypt-keys
```

The 403 is an **account entitlement, not a client problem**. Tested with the same
token and headers, varying only the `IsDevelopment` header:

| Request | Result |
| --- | --- |
| `/alecaframe/decrypt-keys`, `IsDevelopment: true` | 403 |
| `/alecaframe/decrypt-keys`, `IsDevelopment: false` | 403 |
| `/auth/me` (control, same token) | 200 |

The account in question has `permissions: ""` and no `patreon_tier`, so the
endpoint appears gated behind a supporter tier or an explicit grant. A
self-hosted server would have to source the AES key and IV independently.

### What the other two sources actually give you

Worth recording, because it is not obvious and it determines whether the blocker
matters:

- **Profile** (`api.warframe.com/cdn/getProfileViewingData.php`) returns a public
  *profile*, not an inventory. Of the fields `WarframeRootObject` wants it
  supplies `PlayerLevel`, all fifteen `DailyAffiliation*` values and
  `Affiliations`, and omits `PremiumCredits`, `RegularCredits`, `TradesRemaining`,
  `RawUpgrades`, `Upgrades` and `Recipes`. `Upgrades` is where rivens live, so
  this source can never populate the WF Inventory panel — it exists to import
  **syndicate standings and mastery rank**, which is what the syndicate trading
  pipeline consumes.
- **File** expects plain JSON in `lastData`/`InventoryJson` format. AlecaFrame's
  own `lastData.dat` is AES-encrypted on disk, so it cannot be fed to this source
  without the keys above.

The practical consequence: on an account without the entitlement, the WF Inventory
panel (which has exactly one tab, Rivens) cannot show data under any of the three
sources. That is likely why upstream hides the panel outside `vite dev`.

### Correction: the decrypt keys are not a blocker

The original assessment inferred from `DecryptKeys { key: Vec<u8>, iv: Vec<u8> }`
that the endpoint dispensed something secret. It does not. The key and IV are
**fixed AES-128-CBC constants** compiled into AlecaFrame, identical for every
user, and they appear in public third-party code.

Verified by decrypting a real `lastData.dat` with them outside the application:
1,138,768 encrypted bytes produced 194 top-level JSON fields including
`Upgrades` (811 entries), `RawUpgrades`, `Recipes`, `PremiumCredits` and
`TradesRemaining` — every field `WarframeRootObject` reads.

Two consequences:

- **For self-hosting**, this endpoint is among the easiest to replace: return two
  constants. It should have been in the trivial tier from the start.
- **For this fork**, no server is needed at all. The keys can be configured
  locally — `advanced_settings.wf_decrypt_key`/`wf_decrypt_iv`, or
  `QF_WF_DECRYPT_KEY`/`QF_WF_DECRYPT_IV` — and the app decrypts in-process. That
  is implemented, and the WF Inventory panel renders rivens from it with no call
  to the gated endpoint. See `docs/FORK.md`.

The 403 remains real, and the account-entitlement finding above still stands. It
is simply no longer load-bearing: the entitlement gates an endpoint whose output
is a known constant.

### Avoiding the keys entirely

A better long-term option sidesteps AlecaFrame altogether. Warframe's own
`mobile.warframe.com/api/inventory.php?accountId=…&nonce=…` returns the full
inventory as plain JSON, with the authorisation pair readable from the running
game's memory. That removes AlecaFrame, `lastData.dat` and the keys from the
picture in one move. Tracked as a separate piece of work.

## Operational notes

- **The server is closed-source.** It is not among Kenya-DK's public repositories.
  It is NestJS: the 404 body is NestJS's exact default
  (`{"message":"Cannot GET /x","error":"Not Found","statusCode":404}`), and their
  public repos include `nestjs-swagger` and `NestjsEndpointGenerator`. It sits
  behind Cloudflare.
- **No published API spec.** `/docs`, `/api`, `/swagger`, `/api-docs`, `/api-json`
  and `/openapi.json` all return 404. `/health` returns 200. The `qf_api` crate is
  therefore the only available contract — treat it as the specification.
- **`AI-Ocko/quantframe-server` is not an API server**, despite the name and how
  search engines summarise it. It is a fork of the *client* that still references
  `quantframe.app` in 19 places.
- **The on-ramp already exists.** `qf_api/src/client.rs` defines
  `DEVELOPMENT_URL = "http://localhost:6969"` alongside `PRODUCTION_URL`, switched
  by an `is_development` flag. A partial server can be pointed at without
  modifying client logic.
- **`app_id` is a shared constant**, not a per-user secret
  (`"rqf6ahg*RFY3wkn4neq"`, `src-tauri/src/app/client.rs:41`). Your own server can
  accept anything.
- **Rate limits are a non-issue at single-user scale.** The client self-limits to
  3 requests/second; roughly 3000 tradable items is about 17 minutes for a full
  statistics sweep.

## Recommended sequence

Incremental. Each tier leaves a working application.

1. **Identity and no-ops** — `/health`, `/auth/*`, `/users`, `/events/track`,
   `/alert`, `/market/users/activity`. The app boots and logs in against your
   server. Roughly a day.
2. **Cache** — build the bundle from `warframe-public-export-plus`. The 32 module
   schemas must match exactly; that is the fiddly part.
3. **Market items and syndicate** — a warframe.market statistics scraper on a
   schedule.
4. **Rivens** — only if riven trading matters to you.

The AlecaFrame keys are not part of this sequence: they are two constants, and
the fork already resolves them locally without a server.

## Consequence for the test suite

`qf_api`'s two "broken" tests should **not** be deleted.

- `tests::client::test_cache_extract` already targets `http://localhost:6969` and
  asserts a cache download succeeds and extracts. That is a ready-made conformance
  test for tier 2.
- `tests::client::print_token` becomes meaningful as soon as you control
  authentication.

They are not junk; they are a stub integration suite for precisely this project.
See [`FORK.md`](../../FORK.md) for how the check gates treat them today.

## Sources

- `src-tauri/qf_api/` — the client crate, read in full; the de facto API contract
- `https://api.warframe.market/v1/items/{slug}/statistics` — schema comparison
- https://github.com/Kenya-DK — public repository listing
- https://github.com/AI-Ocko/quantframe-server — assessed and ruled out
