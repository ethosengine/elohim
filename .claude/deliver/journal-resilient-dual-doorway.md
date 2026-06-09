# /deliver journal — resilient-dual-doorway-delivery

Promise: `.claude/deliver/feature-promise-resilient-dual-doorway.json`
Operator direction (2026-06-09): "Two doorways, facilitating/projection and hosted user auth routing
to p2p-hosted elohim-protocol landing and lamad-epr apps, actually delivering that resiliency feature,
and being able to see that resiliency on the UI (elohim.host, alpha.elohim.host)."

## iter-0 — initial render + diagnosis (2026-06-09 ~21:30-21:45Z)

**Render (live curl, both doorways):**

| Surface | First probe | After heal |
|---|---|---|
| alpha.elohim.host/ | 404 `App ZIP blob not found: sha256-38c23ba…` | **200** (healed) |
| alpha.elohim.host/lamad | 404 `App ZIP blob not found: sha256-34f85…` | **200** (healed) |
| elohim.host/ | 302 → /threshold (empty EPR router) | unchanged |
| elohim.host/lamad | 404 (falls past router to service-404) | unchanged |
| /admin/federation/peers (both) | ✅ bidirectional, reachable:true | — |
| /api/v1/commitments?action=operate-doorway | ✅ rows exist | — |
| /api/v1/resilience/elohim-host-landing | 200 — `stewardingCollectives:0, protectionStatus:"at-risk"` | — |
| p2p /health peerCount | 4 on both doorways | — |

**Tier-3 verdict: `partial`.** Deliverable `alpha-serves-landing-and-lamad` was REGRESSED at iter-0
open (was delivered 2026-05-30) and healed during diagnosis; `apex-serves-landing-and-lamad` never
delivered (the 05-30 leftover, still live); `doorway-degrades-through-pool`,
`blob-heal-on-demand-for-apps` (apps path), `resilience-visible-on-ui` missing. No manifest mint
(nothing conferred; single-render heal has no stability evidence).

**Root causes (evidence-pinned):**

1. **alpha blob regression** — matthew's storage lacked the ZIP bytes for both content rows
   (`db/content` rows pointed at hashes the local blob_store missed). Manual
   `GET /blob/<hash>` healed both URLs: the /blob route has the T17 peer race-fetch
   (`http.rs:2093-2160` — peer_blob_inventory → race_fetch → finalize_fetch_success + serve-blob
   REA event). The **apps-resolver** (`http.rs:4741`) calls bare `blob_store.get` → 404, no heal.
   Suspected regression vector: `stageSpaBlobs` blob PUT is `|| echo WARNING` (non-fatal) — rows
   update, bytes don't land; needs verify-after-upload.
2. **apex empty router** — doorway-B's EPR refresh loop (`main.rs:631-676`) is hardwired to
   `args.storage_url` = adam, which returns **0 rows** for `doorwayId=apex-elohim-host`
   (Loki: `EPR router periodic refresh: replaced projections, count=0` every 30s, DEBUG level —
   invisible). The same pod's `/db` route-registry proxy forwards to **matthew**
   (Loki: `Forwarding request to elohim-storage, url=http://elohim-matthew-alpha…`) which has the
   3 rows (landing@/, lamad-spa@/lamad, imagodei-portal@/auth/portal). adam missed the
   edge-triggered ReaProjectionSignals (stale binary at signal time); reseeds collapse to 409 on
   matthew so no signal ever re-fires → **edge-triggered projection with no reconciliation**
   (P1 gap). Current source (rea_projection.rs:390) threads in_scope_of correctly.
3. **resilience invisible** — `/api/v1/resilience/elohim-host-landing` knew the content was
   `at-risk` the whole time; no UI surface shows it (felt-resilience spec gap #1).

**iter-1 dispatch (parallel):**
- Fix A (rust-architect): doorway EPR router boot+refresh consults the storage pool when primary
  is unreachable/empty; degraded-primary logs WARN. → unblocks apex via matthew while adam heals.
- Fix B (rust-architect): storage apps-resolver shares the /blob T17 heal-on-miss path.
  → alpha-class blob regressions self-heal on first page visit; serve-blob REA event books the
  mutual-aid evidence the UI can show.
- Fix C (angular-architect): felt-resilience connective tissue — nav reachability for
  /shefa/cluster + /shefa/peers per spec gap #1 (bounded slice).
- Fix D (self): stageSpaBlobs verify-after-upload in root Jenkinsfile.

Deploy + fresh-trigger render are operator-gated (commit-only; sprint-branch not orchestrator-indexed).
