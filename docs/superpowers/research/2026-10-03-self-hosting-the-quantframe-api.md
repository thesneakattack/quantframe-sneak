# Self-hosting the Quantframe API — research findings

**Date:** 3 October 2026
**Type:** Spike (feasibility investigation). No code was written.
**Question:** Can `api.quantframe.app` realistically be self-hosted, and what is the
minimum viable server that makes this client fully functional?
**Verdict:** Feasible, incrementally, with one optional blocker.

> Status: this is research, not a commitment. `api.quantframe.app` remains the
> configured backend and nothing in the client has been repointed. See
> [`NOTICE.md`](../../../NOTICE.md) for the couplings deliberately left in place.

## Summary

The Quantframe API is **not** the trading engine and does **not** hold proprietary
data. The desktop client talks to warframe.market directly for all actual trading.
The QF API is an account system plus a caching and aggregation convenience layer
over public data sources. That makes it reimplementable.

The single genuine blocker — AlecaFrame decryption keys — is optional, because WF
Inventory supports two other sources.

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
| `GET /alecaframe/decrypt-keys` | **blocked** | Returns AES `key` and `iv` as byte arrays, used to decrypt AlecaFrame's `lastData.dat`. Reverse-engineered from a third-party tool; must be sourced independently |

### The blocker is optional

WF Inventory supports three sources — Warframe profile, AlecaFrame, and a plain
JSON file. Only the AlecaFrame path needs those keys. Choosing Profile or File in
Settings → Advanced sidesteps it entirely.

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

Skip the AlecaFrame keys and use the Profile or File inventory source.

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
