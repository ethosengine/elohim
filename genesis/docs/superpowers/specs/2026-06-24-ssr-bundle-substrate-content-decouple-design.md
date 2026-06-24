---
title: SSR Bundle as Substrate Content — Decoupling the Angular SSR Runtime from the Doorway Image
id: ssr-bundle-substrate-content-decouple-design
status: Draft
class: protocol-canonical
domain: D8
topic: [ssr, doorway, projection, content-addressing, app-bundle, build-decouple, render, elohim-render, blob]
informed-by:
  - genesis/docs/content/elohim-protocol/architecture/2026-06-02-doorway-ssr-runtime.md
cites:
  - doorway-ssr-runtime | SSR-as-compute-capability architecture seed (D8); this spec moves its bundle distribution from image-bake to substrate content, reusing its cold-start + pod-resource-floor budget | sha256:7f75b3027ae4f9d4 | path: genesis/docs/content/elohim-protocol/architecture/2026-06-02-doorway-ssr-runtime.md
  - elohim-seam-map-concern-routing | Seam placement (D8 projection); names the brittle sed that strips V8/SSR from the storage image that this spec retires | sha256:54b5809fb8e688d1 | path: genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md
requires_env: [household-nodes]
---

# SSR Bundle as Substrate Content — Decoupling the Angular SSR Runtime from the Doorway Image

- **Date:** 2026-06-24
- **Status:** Design (approved spine; pre-implementation)
- **Seam:** Doorway projection (atlas §3.9, Track 4) + SDK/app-build artifact; render runtime shared via `elohim-render`
- **Supersedes the brittle pair:** in-doorway SSR re-bake (`doorway/doorway-service/Dockerfile` `ssr-bundle` stage) **and** the storage-image `elohim-render`/`ssr` sed-strip (`elohim/elohim-storage/Dockerfile`).

## 1. Problem

The Angular **SSR server bundle** (`dist/elohim-app/server/main.server.mjs` + ~171 `.mjs` files, ~50 MB) that renders the `elohim-host-landing` surface is **baked into the doorway image** at build time (`doorway/doorway-service/Dockerfile` `ssr-bundle` stage → `COPY --from=ssr-bundle … /opt/elohim-render`). The doorway image is only rebuilt when the **doorway-Rust** changeset triggers it (`Build Doorway` is gated by `shouldRunStep('cargo-build-doorway')`, whose `STEPS` the orchestrator derives from the edge `build-manifest.json` `gate.projects` = `doorway/doorway-service` + `elohim/elohim-storage` only). **`app/` is not an edge build trigger.**

Consequence: an **app-only** change (e.g. the 2026-06-12 resilience-card dark-theme fix `05827e5ed`) **never re-bakes the doorway SSR bundle**. The bundle refreshes only as a side effect of an unrelated doorway-Rust rebuild. Observed 2026-06-24: `elohim.host` served the app-build-`8a2c65e` (pre-fix) SSR bundle while its doorway pod reported image tag `dev-80c959d8` — the EPR badge hash (Angular app version) and the pod image tag (Rust doorway version) had silently diverged. Cache and CDN were ruled out (origin pod served the stale bundle; cache-buster + 15 h age).

The same artifact is mismanaged on the storage side in the mirror-image way: `elohim/elohim-storage/Dockerfile` **sed-strips** the optional `elohim-render` path-dep and the `ssr` cargo feature, because "SSR belongs in the doorway image, not here." Both are symptoms of treating one **shared app-build artifact** as a per-runtime *build* concern.

The render **runtime is already shared**: `elohim-render` (`elohim/elohim-render`) is consumed by doorway (always) and elohim-storage (optional, `ssr = ["dep:elohim-render"]`). Both load a bundle identically from `SSR_BUNDLE_PATH` via the same `Renderer`/`AngularRenderer`/`DataFetcher` abstraction (doorway's `DataFetcher` is a V8 fetch-shim to storage; storage's is a `LocalFetcher`). Only the *distribution* of the bundle is wrong.

The browser **sibling** half already does it right: the app pipeline zips `dist/.../browser`, PUTs it as a **content-addressed blob** (`/blob/{hash}`), and PATCHes a content-row pointer (`/db/content/elohim-host-landing`) — p2p-native, replicated by the substrate dataplane, served through projection (`scripts/ci/stage-spa-blob.sh`, root `Jenkinsfile:224-272,380`).

## 2. Goals / Non-goals

**Goals**
- The SSR server bundle becomes a **content-addressed substrate artifact**, distributed exactly like its browser sibling — no Harbor/OCI/ORAS artifact, no initContainer, no orchestrator tag handshake.
- App-only changes reach every rendering runtime through the **app pipeline's** publish step (which already watches `app/**`), not the edge trigger.
- One artifact, **mounted uniformly** by every rendering runtime (doorway + SSR-enabled storage) via the existing `SSR_BUNDLE_PATH` + `elohim-render` contract.
- Retire **both** brittle mechanisms: the doorway `ssr-bundle` bake stage and the storage sed-strip.
- Preserve today's **graceful degradation**: no bundle ⇒ renderer `None` ⇒ fall through to storage-proxied SPA.

**Non-goals (this spec)**
- Background hot-swap of a running renderer (V8 isolate reload). Deferred to Phase 2 (§9).
- Changing the browser-bundle (`elohim-host-landing`) flow — untouched.
- Resuming the paused prod pipeline. Prod manifests inherit the same pattern when un-paused.

## 3. Architecture / data flow

```
app pipeline  (ng build, SSR mode — watches app/elohim-app/**, app/elohim-library/**, …)
  ├─ dist/elohim-app/browser → zip → PUT /blob → PATCH db/content/elohim-host-landing       (EXISTS)
  └─ dist/elohim-app/server  → zip → PUT /blob → PATCH db/content/elohim-host-landing-ssr    (NEW)
        both rows PATCHed in the SAME build  ⇒  same app sha  ⇒  server/browser co-versioned

runtime boot  (doorway via STORAGE_URL · storage via local blob store)
  if SSR_BUNDLE_SLUG set:
     resolve slug → blobHash   (GET /db/content/<slug>)
     GET /blob/<blobHash> → verify bytes hash == blobHash (CID integrity) → unzip → SSR_BUNDLE_PATH dir
     elohim-render builds renderer from SSR_BUNDLE_PATH
  else / on any failure:
     renderer = None  →  fall through (doorway: storage-proxied SPA; storage: 503 on SSR-gated handler)
  lazy-on-miss: if absent at boot, retry the resolve+fetch on the first SSR request
```

Versioning is the **content-row head** (`slug → current blobHash`), the same mutable-head→immutable-CID pointer the browser bundle uses ("current landing"). Rollback = re-PATCH the row to a prior blobHash; immutable CIDs remain fetchable indefinitely — reproducible *and* convenient, with no deploy-time resolve and no orchestrator-tracked app↔runtime pairing.

## 4. Components

### 4.1 Publish (app pipeline)
- Generalize the `{distDir, slug}` upload (`scripts/ci/stage-spa-blob.sh`, invoked from root `Jenkinsfile`) to also stage the **server** dir under slug `elohim-host-landing-ssr`, in the **same** `stageSpaBlobs` invocation as the browser bundle (atomic per app build ⇒ paired rows).
  - The server dir is **not** an `/apps/{slug}/*` browser surface; it needs only the blob + content-row pointer (no `index.html` materialization, no `/apps` mount/verify leg). The script branches on a `kind=server|browser` flag.
  - Reuse the existing admin-key PATCH + GET-readback seatbelt (`set -euo pipefail`, `curl -fSs`): a 4xx/5xx on the `-ssr` row fails the build.

### 4.2 Consume (runtime fetch-and-materialize helper)
- A small shared helper lives in **`elohim-render`** (a new `bootstrap` module — both runtimes already depend on the crate for SSR, so it avoids a new crate and keeps the materialize-then-render concern in one place). It is parameterized by a blob-fetch closure/trait each runtime supplies — doorway: HTTP `GET` to its `STORAGE_URL`; storage: a local blob-store read (distinct from `elohim-render`'s render-time `DataFetcher`, which threads SSR data — this fetches bundle bytes). Given `SSR_BUNDLE_SLUG` + that fetcher it:
  1. resolves `slug → blobHash` (`GET /db/content/<slug>`),
  2. fetches the blob (`GET /blob/<blobHash>`),
  3. **integrity-verifies** bytes against `blobHash` (content-addressed — the hash *is* the identity),
  4. unzips into the `SSR_BUNDLE_PATH` directory,
  5. returns the entry path (or `None` on any failure, logged `warn!`).
- **Doorway** supplies a fetcher over its upstream `STORAGE_URL` (it already self-registers + pulls `/manifest` at boot — this slots into that sequence). **Storage** supplies a `LocalFetcher` over its own blob store (the bytes are already local).
- `init_renderer` (`doorway/doorway-service/src/server/http.rs:364`) and `SsrState::from_env` (`elohim/elohim-storage/src/ssr.rs`) change from "read a baked path" to "materialize via the helper, then read the path." The downstream `elohim-render` usage is unchanged.

### 4.3 Dockerfiles
- **doorway** (`doorway/doorway-service/Dockerfile`): delete the `ssr-bundle` stage (≈`131-225`) and the `COPY --from=ssr-bundle … /opt/elohim-render` (≈`250`). `SSR_BUNDLE_PATH` now points at a writable, runtime-populated dir. Image loses ~50 MB + the pnpm/Node toolchain + ~5-10 min build.
- **storage** (`elohim/elohim-storage/Dockerfile`): remove the `elohim-render`/`ssr`-feature sed-strip (≈`113-119`). Introduce a **clean `--features ssr` build variant** (build arg, e.g. `STORAGE_FEATURES`) producing two images: a **lean default** (no V8, for low-tier/household nodes) and an **ssr-enabled** variant (for storage that serves the app). No brittle sed in either path.

### 4.4 Manifests
- **Doorway** (`genesis/orchestrator/manifests/doorway/{alpha,alpha-b,prod}.yaml`): `SSR_BUNDLE_PATH` → a writable `emptyDir` mounted at `/opt/elohim-render` (runtime-populated, not baked); add `SSR_BUNDLE_SLUG=elohim-host-landing-ssr`.
- **Storage** (`genesis/orchestrator/manifests/humans/*` + `_edgenode-consolidated.template.yaml`): SSR-serving nodes use the **ssr-enabled image variant** + `SSR_BUNDLE_SLUG` + the writable dir. Lean nodes use the default variant and leave `SSR_BUNDLE_SLUG` unset ⇒ no SSR (current behavior). This is the smartwatch→rack leanness lever.

### 4.5 Env / config
- `SSR_BUNDLE_SLUG` (new) — content row to resolve; unset ⇒ SSR disabled for that runtime.
- `SSR_BUNDLE_PATH` (existing) — local materialization target (now writable emptyDir).
- *(Phase 2)* `SSR_BUNDLE_REFRESH_SECS` — background re-resolve interval for hot-swap.

## 5. p2p-design-gate

- **Source-of-truth class:** operational content projection (Class C) — **no new DHT entry type.** Reuses the existing EPR content-blob + content-row mechanism that the `elohim-host-landing` browser bundle already uses.
- **Existing entry type?** Yes — content blob (`bafkrei…` raw/Sha2-256 over bytes) + the `db/content/{slug}` content row. New slug `elohim-host-landing-ssr`, same shapes.
- **Identity:** content-derived CID (the blob hash *is* the identity; verified on fetch). The slug is a human-readable pointer to the current CID — same justification as the browser sibling.
- **Coordinator/route:** none new. `PUT /blob`, `PATCH /db/content/{slug}`, `GET /db/content/{slug}`, `GET /blob/{hash}` all exist. The runtime consumes via existing client paths; no new HTTP route is authored.

## 6. Error handling / graceful degradation

| Condition | Behavior |
|---|---|
| `SSR_BUNDLE_SLUG` unset | No SSR; lean node. (storage default) |
| Row/blob missing (e.g. first boot before app deploy) | `warn!`, renderer `None`, fall through to storage-proxied SPA (doorway) / 503 on the SSR-gated handler (storage). No outage. |
| Blob hash mismatch / corrupt zip | Integrity check fails ⇒ `warn!`, renderer `None`, fall through. Never render from unverified bytes. |
| Storage unreachable at doorway boot | Same as missing; lazy-on-miss retry on first SSR request. |

The contract is byte-identical to today's `init_renderer`/`SsrState::from_env` "returns `None` when the file fails to load" — the pod stays healthy and routes fall through. This is what makes the phased rollout safe.

## 7. Testing + regression seatbelt

- **Unit:** the fetch/verify/unzip helper — happy path, hash mismatch (must reject), missing row, partial zip. Mock blob+content client.
- **Integration:** (a) doorway boots against a storage with a seeded `elohim-host-landing-ssr` row ⇒ SSR renders current; (b) storage-self-SSR (`--features ssr` variant) renders from local blob; (c) absent row ⇒ both fall through cleanly.
- **a2o regression (the class seatbelt):** *an app-only style/content change is visible on the doorway-projected surface after an app deploy, with no doorway image rebuild.* This is the dark-mode-card bug encoded as an executable scenario — the failure mode becomes structurally impossible because the app pipeline (which watches `app/**`) owns publication.
- **Deploy verify:** extend the `verify-epr-mount.sh` pattern to assert the `-ssr` row exists and the SSR surface renders the current app build.

## 8. Rollout (phased; each step reversible via fall-through)

1. **Publish only** — app pipeline writes the `-ssr` blob + row. Additive, no consumer. Verify the row populates.
2. **Doorway consumes** — fetch-at-boot + lazy-on-miss; switch `SSR_BUNDLE_PATH` to the writable emptyDir; delete the doorway `ssr-bundle` bake stage. Before the `-ssr` row exists, doorway falls through to the storage-proxied SPA (pre-SSR behavior) — no hard outage window.
3. **Storage consumes** — remove the sed-strip; ship the `--features ssr` variant; SSR-serving storage nodes get the slug + dir. Lean storage nodes unchanged.

**Rollback:** at any step, an unset slug or absent row degrades to fall-through. A bad bundle is rolled back by re-PATCHing the content row to a prior (immutable, still-fetchable) blobHash — no image rebuild.

## 9. Decisions made / deferred

- **Refresh model = fetch-at-boot + lazy-on-miss (v1).** A new bundle requires a pod rollout-restart (cheap — no image rebuild), not a redeploy. **Phase 2:** background re-resolve + renderer hot-swap (`SSR_BUNDLE_REFRESH_SECS`) so app deploys propagate to *running* runtimes with zero restart — the fullest cure, layered on without redesign. The trickiest piece is the V8 isolate reload under concurrency; out of scope here.
- **Storage build = clean `--features ssr` variant.** Two images (lean default / ssr-enabled), replacing the sed. Keeps the low end of the device spectrum V8-free.

## 10. Watch-outs

- **Cold-start.** Materializing ~50 MB from the substrate at boot adds startup latency; the `doorway-ssr-runtime` architecture doc already carries a cold-start + pod-resource-floor section — reuse its budget. The materialized dir may persist across restarts to amortize.
- **Pairing.** The server and browser rows MUST be PATCHed in the same app build; a half-published pair (server new, browser old or vice-versa) is the one way to reintroduce drift. The build-fails-on-PATCH-error seatbelt covers this.
- **Storage build cost.** The ssr variant compiles V8 (rusty_v8) — longer build, larger binary. Confined to the ssr variant, so lean nodes are unaffected.
- **`uhCok…`/`uhC0k…`-class confusion does not apply** (no DNA/wasm hashes here); the relevant identity is the content CID.

## 11. References

- Root cause + evidence: this session's analysis (EPR badge app-hash vs pod image-tag divergence; cache ruled out).
- `doorway/doorway-service/Dockerfile` — `ssr-bundle` stage + final `COPY --from=ssr-bundle`.
- `elohim/elohim-storage/Dockerfile` — `elohim-render`/`ssr` sed-strip.
- `elohim/elohim-render` — shared render crate; consumers `doorway/doorway-service/Cargo.toml`, `elohim/elohim-storage/Cargo.toml` (`ssr` feature).
- `doorway/doorway-service/src/server/http.rs` (`init_renderer`, `SSR_BUNDLE_PATH`), `elohim/elohim-storage/src/ssr.rs` (`SsrState::from_env`, `LocalFetcher`).
- `scripts/ci/stage-spa-blob.sh` + root `Jenkinsfile:224-272,380` — the browser-bundle content-blob upload this generalizes.
- `genesis/docs/content/elohim-protocol/architecture/2026-06-02-doorway-ssr-runtime.md` — SSR-as-compute-capability, cold-start, pod resource floor.
- `genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md` — seam placement; flags the "brittle sed strips V8/SSR from the storage image."
