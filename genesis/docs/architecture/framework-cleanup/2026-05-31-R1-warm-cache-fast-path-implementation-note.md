# R1 — Warm-cache fast path on the EPR-app routes: ready-to-land implementation note

> Produced during the `doorway-routing-projection-shakeout` shift (2026-05-31). The shift landed
> the `/auth/portal` routing un-shadow + **R20** (pooled `ssr_http_client` on the EPR proxy path).
> **R1 is deliberately NOT landed in that shift** — it carries one unverified integration assumption
> that wants an operator eyeball against alpha. This note makes the landing a <30-min, low-risk task.

## The defect (recap)

`dispatch_to_projected_epr` (`doorway/doorway-service/src/server/http.rs:1306`) serves `/`, `/lamad`,
and (now) `/auth/portal` by proxying straight to `{storage}/apps/{epr_id}/{sub_path}` via
`state.ssr_http_client`. It **never consults the doorway's MongoDB `AppFileCacheService`**, so the SPA
entry points + their assets round-trip to elohim-storage on every request. The direct `/apps/...`
handler (`routes/apps.rs::handle_app_request:53`) already has the cache-first logic; the EPR path
does not reuse it.

## The fix — extract `serve_app_file`

Add to `routes/apps.rs`:

```rust
pub(crate) async fn serve_app_file(state: &AppState, epr_id: &str, sub_path: &str) -> Response<Full<Bytes>>
```

It is the body of `handle_app_request` from line 105 onward, with `slug = epr_id`,
`file_path = sub_path`, `full_path = format!("/apps/{}/{}", epr_id, sub_path)`. Then in
`dispatch_to_projected_epr`, after the reach-gate + mode-gate pass, replace the GET/build block
(`http.rs:1346–1433`) with a call to `serve_app_file(...)` and re-inject the `x-epr-router: dispatched`
header on the returned response.

**Non-regression guarantee:** when `app_file_cache` is `None` (no-Mongo boot) or `resolve_blob_hash`
returns `None` (slug not indexed), `serve_app_file` falls through to
`forward_app_request_with_header(..., "BYPASS")` — functionally the same GET the EPR path issues today
(plus 2× 502 retry, a strict improvement). Cache-cold == current behavior; cache-warm == faster.

## THREE invariants the implementation MUST preserve (the reason it didn't auto-land)

1. **`cache_enabled` admin bypass.** `handle_app_request` checks `state.cache_enabled` (`apps.rs:71`)
   and short-circuits to `BYPASS-ADMIN` when `POST /admin/cache/disable` was called. `serve_app_file`
   MUST do the same at its top, or the admin cache-disable tool silently stops working for EPR routes.
   No existing test catches this omission.
2. **`x-epr-router: dispatched` re-injection.** The cache-hit path returns from `build_app_response`,
   which does not set this diagnostic header. Re-add it via `response.headers_mut().insert(...)` after
   `serve_app_file` returns. (No app/SW consumer reads it — operator tooling only — but keep parity.)
3. **Slug-index parity (the integration unknown — VALIDATE ON ALPHA).** `load_slug_index` indexes
   `projected_entries` where `contentFormat ∈ {html5-app, spa-bundle}`. The warm path only fires if the
   EPR projection's `epr_id` equals the slug stored in MongoDB. If they diverge, `resolve_blob_hash`
   returns `None` → BYPASS forever → cache never warms (no error, no regression, but no win). This can
   only be confirmed against a live Mongo-backed doorway.

## Testable in Che (add a shakeout-style oracle)
- `serve_app_file` with `app_file_cache = None` → falls through to BYPASS (mock the HTTP layer).
- `cache_enabled = false` → returns `BYPASS-ADMIN` immediately.
- Pure seams already covered: `cache_key`, `in_flight_key`, `is_content_address`, `parse_app_path`,
  `derive_app_subpath`, `build_app_response` (all have `#[cfg(test)]` tests).

## Needs alpha (operator, <5 min)
After deploy: hit `/lamad` once (cold), then again (warm); compare the `X-Cache` response header
(expect `MISS`/`BYPASS` then `HIT`). Check `GET /admin/cache/stats` → `appFileCache.cachedFiles` > 0.
If it stays `BYPASS`/`0`, invariant #3 (slug parity) is the culprit — fix the seed/index mapping, not
the proxy.

## Companion defects in the same file/area (tee-up)
- **R7** — `warm_stream` writes `projected_entries` but never calls `load_slug_index()` → first
  `/apps/{slug}` on a cold pod is `BYPASS` until a content event arrives. (`projection/warm_stream.rs:269`.)
- **R9** — EprRouter boot race: boot-fetch (10s) races storage DB readiness; probe
  `/db/rea_commitments?action=project-epr` (not `/health`) before serving; tighten refresh while empty.
- **R20** — DONE this shift (pooled `ssr_http_client` on the EPR proxy path).

File anchors: `routes/apps.rs` (`handle_app_request:53`, `fetch_and_cache:215`,
`forward_app_request_with_header:364`, `build_app_response:331`); `cache/app_file_cache.rs`
(`AppFileCacheService:60`, `resolve_blob_hash:278`, `get:113`); `server/http.rs`
(`dispatch_to_projected_epr:1306`, `derive_app_subpath:1195`, `cache_enabled:200`).
