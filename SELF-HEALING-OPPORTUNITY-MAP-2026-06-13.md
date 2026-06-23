# Self-Healing & User-Agency Opportunity Map

## 1. Reframe

The operator layer is already rich with self-healing: `projection_reconcile::run_sweep` heals 10-day projection divergence from the own conductor's DHT view, `custody::reconcile_pass` re-races missing blobs, `get_blob_or_heal` heals-on-read on every blob miss (the EprRouter-empties cure), `GapTracker` is a single shared local controller bounding retries, and the `AcquisitionState`/`PullStatusInfo` tri-state already encodes "keep waiting, not caught up" as a wire contract. All of this runs (V6 confirmed both sweeps fire in prod) and most of it works. **The gap is not the engine — it is the user axis and the closed loop.** Every reconcile stream is timer/event-driven with no force-trigger; every status counter (`caughtUp`, `divergentAnchor`, `degenerateRate`, `selfCidPresent`) is server-internal and unreachable through doorway (`/p2p/status`, `/api/v1/pins`, `/api/v1/status/projector` are all confirmed absent from `build_manifest()` → hard 404, not even MATTHEW-only); the one rich user health view (`HealthIndicatorComponent`) is mounted nowhere (V1); the entire social-recovery system is built and 98.7%-covered but unreachable behind two different dead routes (V4); and no runtime failure has any deterministic path to a finding/bug (L4: zero `.claude/data` writes from runtime code). The frame — (A) see, (B) reset, (C) pause, all as the control surface of a detect→recover→verify→elevate loop — is almost entirely a *wiring and surfacing* problem layered over substrate that already self-heals.

## 2. The three affordances, designed

These are not three primitives. They are three facets of **one** agency primitive — the user-declared desired-state *want* (`DevicePin`/acquisition want), reconciled by `GapTracker` to a notarized REA `replicates-commons` Commitment (see §3). (A) reads the gap between want and observed state; (B) re-declares/re-fires the want; (C) applies backpressure to the want's replication.

### (A) See my own health/stats

**Concrete surface:** a navigable `/shefa/health` route (sibling of the existing `/shefa/cluster`, `/shefa/peers`, `/shefa/reciprocity`) — shefa already owns self-state and the `ResilienceSnapshotView`. The existing protocol-omni `ResilienceService` chip (rendered on every page via the shell, per V5) becomes the always-visible entry chip that deep-links into the page. The page answers the diagnostic question **"is it me / my device / a peer / the network?"** across three data sources that already exist:

- **Device-local axes** — mount the orphaned `HealthIndicatorComponent` (`elohim/components/health-indicator/`, V1: DARK, real logic not a stub) which already computes holochain/indexedDb/blobCache/network with a `refresh()` button.
- **Peer/sync axes** — the `P2PStatusView` rich fields (`syncPaused`, `drain`, `pull`, `projectionReconcile{divergentAnchor,healedTotal,caughtUp}`) that today feed *nothing* in the app. This is the exact telemetry that would have shown adam's divergence and the zero-peer anchor-gap.
- **Cluster axis** — the live `MyClusterComponent` device online/offline view.

**p2p-class of any new entity:** none. The health snapshot is a **read-model projection (Category C, observe-only)**, not a new DHT entry and not content-addressed. `ResilienceSnapshotView` and the `/p2p/status` projection block already exist; this is a wiring fix (surface + proxy), explicitly **not** a `recovery_actions`/`health_snapshots` table.

### (B) Reset / re-sync my own state

**Concrete triggers**, grounded in the real incidents:

- **adam's 10-day divergence:** the cure (`run_sweep`) already exists but is timer-only with no entry point. Add `POST /api/v1/reconcile/projection` (storage owns it; calls `projection_reconcile::run_sweep` once, returns the resulting `ProjectionReconcileStatus`). A doorway proxy front-door (`POST /api/v1/me/resync`, auth-gated to the caller's own node) makes it reachable for hosted users. Must be idempotent past the reseed→409 that was adam's literal root cause.
- **Anchor-gap (zero counts):** the want primitive's front door — `POST /api/v1/pins` (already exists, idempotent re-pin) — re-declares a `DevicePin`, re-engaging the provide-loop. Surface this as a "re-host my content" button and as an auto-create on zero-peer/read-miss detection.
- **Identity recovery:** wire the dead `RecoveryRequestComponent` (the flagship reset) — see §4.

**p2p-class:** the reset *action* is **not a new entity** — it is a trigger on an existing controller. The want is the existing `DevicePin` (**Category C, operational, node-local, exists**); the durable notarized outcome is the existing REA `replicates-commons` Commitment (**Category A, notarized; identity = content-derived; created by the provide-loop authoring tick at `main.rs:935`; projected by the acquisition/reconcile streams**). Reject any relational `recovery_actions` table + `GET /api/v1/recovery` — the substrate-native record already exists.

### (C) Pause & wait for catch-up

**Concrete UX:** an app-shell degraded banner ("Your node is catching up, hold on — retrying in Ns") replacing today's silent opaque hang / blank page, plus per-request `Retry-After` so both a human and an auto-loop back off correctly. Reuse `pin-progress`'s tri-state semantics (today scoped to single content links) app-wide.

**Backend**, grounded in the live doorway wedge:

- **The freeze mechanism (prerequisite):** `forward_to_storage` uses `reqwest::Client::new()` with **no timeout at two confirmed sites** (storage_proxy.rs:112, :295). On a wedged storage peer this parks workers forever (the conductor/SSR paths are bounded; this bulk path is not). Wrap `builder.send()`/body-read in `tokio::time::timeout` with a shared timed client — every (C) affordance depends on this.
- **`Retry-After` + structured `{"status":"catching-up","retryAfter":N}`** on the timeout path and existing 503s (grep confirms zero `Retry-After` emits today).
- **Drain-aware health:** plumb `caughtUp` from the `/p2p/status` body that doorway *already parses but discards* (V6: doorway keeps only `connectedPeers`/`peerId`) into `/health`'s p2p block, then render a "catching up…" chip in the live `ConnectionIndicatorComponent` (V2: live, already polls `/health`, already has expanded detail state).
- **User/algorithm-drivable pause:** `pause_sync()` exists as an internal RAII guard with `syncPaused` observable but no toggle. Add `POST /api/v1/sync/pause?ttl=` with auto-resume TTL.

**p2p-class:** the pause/resume "token" is **not a DHT entry and not content-addressed** — it is a **node-local ephemeral flag with a TTL (Category C, agent-scoped)** over the existing `pause_sync()` guard. ("Token" overstates it; it is a flag with an expiry.)

## 3. The self-healing closed loop

**The single foundational internal-agency primitive: the user-declared desired-state WANT.** Concretely the `DevicePin` / acquisition want, tracked by the shared `GapTracker`, reconciled by the existing controllers (`acquisition`, `projection_reconcile`, `custody`) to a notarized REA `replicates-commons` Commitment. The want is what a *person* controls (P1: storage = reconciliation controller, DHT = manifest); `GapTracker` is the engine; the REA Commitment is the notarized outcome. Everything in §2 seeds from it: (A) reads want-vs-observed, (B) re-declares the want, (C) backpressures the want.

The loop, built entirely on existing primitives:

1. **Detect** — read projector lag (`ProjectorStatusView.lagSeconds`, `None` = never projected = danger) + reconcile counts (`divergentAnchor`, `failed`) + `degenerateRate` + `peerCount==0`/`selfCidPresent==false`.
2. **Diagnose** — classify which axis (me/device/peer/network) via the same fields that drive affordance (A).
3. **Bounded-recover** — force a `run_sweep` / re-declare a `DevicePin`; `GapTracker` already bounds this at `MAX_RETRIES=3`.
4. **Verify** — re-measure lag/counts; `caughtUp` is the success predicate.
5. **Elevate** — when bounded recovery exhausts (`projection_reconcile` `mark_failed` past `MAX_RETRIES`, recurring `placement-gap`, `degenerateRate` over threshold), file a finding. **This is the only missing arm.** It rides the existing deterministic ledger+sentinel pattern (`ci-harvest.py` / `deprecation-sentinel.py` → `*.jsonl` fp-keyed; the Findings Sentinel spec explicitly invites this composition as a new instantiation). **Hard constraint (L4):** runtime code must NOT write `.claude/data` directly — the elevate arm is an external poller (`runtime-harvest.py`) that reads the alpha JSON endpoints, which also sidesteps the unresolved "does `ssr_busy` reach Loki" question by reading endpoints, not logs.

## 4. Wiring fixes (present-but-dark)

| Target | Current verdict | Smallest change to make user-facing | Leverage |
|---|---|---|---|
| `HealthIndicatorComponent` (`elohim/components/health-indicator/`) | DARK-WIRED (V1): real logic, mounted nowhere | Add `<app-health-indicator />` to protocol-omni chrome or a `/shefa/health` host; import the standalone component | H |
| `RecoveryRequestComponent` + `RecoveryCoordinatorService` (`imagodei/`) | DARK (V4): fully built, 98.7% covered, two *different* dead routes (`login.html:35`→`/identity/recovery`, `lost-key-entry goRecovery()`→`/identity/recover`) | Add one `recovery` route in `imagodei.routes.ts` → `RecoveryRequestComponent`; unify both entry points onto it | H |
| `/api/v1/status/projector` | Absent from `build_manifest()` → 404 (V3) | Add one line `Route::get("/api/v1/status/projector")` to `build_manifest()` (http.rs:~9469); operator/node-health, no per-user scoping | H |
| `caughtUp` through `/health` | doorway parses the `/p2p/status` body then discards `projectionReconcile` (V6, main.rs:454-472) | Add `caught_up` field to doorway `P2PHealth`, set from `body["projectionReconcile"]["caughtUp"]`, render chip in `ConnectionIndicatorComponent` | H |
| `agency.service.ts:91` `totalPeers` hardcoded `0` | LIVE service, one TODO placeholder (V4) | Read live `peerCount` already polled by connection-indicator | M |
| `selfCidPresent` / `provideLoopEnabled` on `/p2p/status` | Anchor-gap (`self_cid.is_empty()` guard at main.rs:962, confirmed) is invisible AND undiagnosable | Assemble two booleans from `config.self_cid` at the status builder | M |
| `/p2p/status` user reachability | Hard 404 through doorway (V3 refuted SPA-fallthrough) | Point users at the *existing* safe transform route `GET /api/v1/federation/p2p-peers` (V3, confirmed at http.rs:2194) rather than exposing raw `/p2p/status` | M |
| `x-ssr-skipped` shed reason | Set server-side (http.rs:3692), never read by browser | App HTTP interceptor reads the header → degraded banner instead of silent CSR shell | M |

## 5. New primitives

Each entity passes the p2p-design-gate as **NOT a DHT entry type and NOT a relational table** — three are operational node-local/ledger state wiring existing primitives; the only notarized durable record (the REA `replicates-commons` Commitment) already exists.

**N1 — HTTP pause/resume with auto-resume TTL** (`POST /api/v1/sync/pause?ttl=`)
(1) **Category C operational**, agent-scoped node-local. (2) **No DHT entry** — a transient runtime flag over the existing `pause_sync()` guard; no new type. (3) Identity = n/a (singleton node-local flag, not addressable). (4) No coordinator fn; reflected in the existing `syncPaused` status field; auto-resumes on TTL expiry.

**N2 — Force-reconcile trigger** (`POST /api/v1/reconcile/projection`, doorway front-door `POST /api/v1/me/resync`)
(1) The *action* is not an entity at all — a trigger on existing `run_sweep`. The want is the existing `DevicePin` (**C, operational, exists**); the durable record is the existing REA `replicates-commons` Commitment (**A, notarized, content-derived CID**). (2) **No new DHT type** — both endpoints already exist. (3) Identity: want = `DevicePin` logical key; outcome = Commitment `entry_hash` CID. (4) Created by the existing provide-loop authoring tick (`main.rs:935`); projected by acquisition/reconcile streams. **Explicitly reject** a `recovery_actions` table + `GET /api/v1/recovery`.

**N3 — Runtime→finding elevate bridge** (`runtime-harvest.py` + `runtime-triage` agent)
(1) **Category C operational** ledger entry. (2) **No DHT entry** — a `.claude/data/runtime-findings.jsonl` line, same schema as `ci-findings.jsonl`. (3) Identity = **fingerprint** (fp of class+provenance), mirroring ci/deprecation closure-by-deletion for regression-for-free. (4) No coordinator/signal — an external poller reads `/api/v1/federation/p2p-peers` + `/admin/render-stats` (NOT in-process file writes — preserves the no-cluster-write rule and dodges the Loki dependency); NEW fp dispatches `runtime-triage` (clone of `deprecation-triage`).

**N4 — Dedicated pinned-runtime status surface** (capstone)
(1) **Category C operational** — a read-model. (2) **No DHT entry** — a minimal `/status`+`/health` answered from a `tokio::runtime::Runtime` on its own OS thread reading only cached `AppState`. (3) Identity = n/a. (4) No coordinator. **This is the only thing that survives a full worker-pool stall** — the surface the live freeze most needed (today all status routes share the wedged runtime; even the opt-in `DOORWAY_HEALTH_PORT`, unset on alpha, shares it per its own footgun comment).

**N5 — Bounded auto-heal loop** (§3 made concrete)
(1) **Category C operational** background task. (2) **No new entity** — composes existing `ProjectorStatusView` + `run_sweep` + `GapTracker` + the N3 elevate sink. (3)/(4) n/a. The detect→recover→verify→elevate controller; emits an N3 finding on exhaust.

## 6. Ranked opportunity map

| # | Opportunity | Type | Leverage | Effort | Incident/need | p2p-class |
|---|---|---|---|---|---|---|
| 1 | Bound the storage-proxy await (timeout + shared client) | wiring/bug | H | S | live doorway freeze (prereq for all C) | n/a (timeout) |
| 2 | Wire recovery route + unify the two dead paths | wiring | H | S | (B) flagship reset dark | none (UI) |
| 3 | Force-reconcile `POST /api/v1/reconcile/projection` → `run_sweep` | new-primitive (trigger) | H | S | adam 10-day divergence (B) | trigger; want=DevicePin C, record=REA Commitment A |
| 4 | Mount `HealthIndicatorComponent` (→ `/shefa/health` / omni) | wiring | H | S | (A) no my-health surface | C projection |
| 5 | Plumb `caughtUp` through `/health` → connection-indicator chip | wiring | H | S | (C) catching-up invisible | C projection |
| 6 | `Retry-After` + structured "catching-up" body on 503s | manual+auto-loop | H | S | (C) opaque hang/503 | n/a |
| 7 | Surface `selfCidPresent`/`provideLoopEnabled` on status | wiring | H | S | anchor-gap undiagnosable (A) | C projection |
| 8 | HTTP pause/resume + TTL over `pause_sync()` | new-primitive | H | S | (C) doorway wedge | C node-local flag |
| 9 | `/api/v1/status/projector` into `build_manifest()` (1 line) | wiring | H | S | (A) projector lag unreachable | C projection |
| 10 | Runtime→finding elevate bridge (`runtime-harvest.py`) | new-primitive + auto-loop | H | M | all four incidents → no bug path | C ledger (fp) |
| 11 | Bounded auto-heal loop (detect→recover→verify→elevate) | auto-loop | H | M | unifies B+C+elevate | C task |
| 12 | "Network & Sync" panel consuming full `P2PStatusView` | new-primitive | H | M | adam divergence visibility (A) | C projection |
| 13 | App-shell degraded/"catching up" banner | manual+auto-loop | H | M | freeze/blank-page (C) | n/a (UI) |
| 14 | Dedicated pinned-runtime status surface (capstone) | new-primitive | H | M | status dies in its own incident (A/C) | C projection |
| 15 | Re-anchor / re-pin self-heal for seeded content (`POST /api/v1/pins`) | wiring + auto-loop | H | M | anchor-gap (B) | want=DevicePin C → REA Commitment A |
| 16 | Hosted own-node reconcile/pins view via doorway (agent-scoped) | wiring | M | M | (A) hosted MATTHEW-only split | C projection |
| 17 | Serve-stale-on-error in proxy/resolver (`x-served-stale`) | manual+auto-loop | M | M | (C) best content-stays-up | n/a |
| 18 | "Heal this page" CTA on EprRouter-empty/404 | manual | M | S | EprRouter poisoned-row | n/a (UI) |
| 19 | Fix `agency.service.ts:91` `totalPeers` | wiring | M | S | (A) zero-peer visibility | n/a (UI) |
| 20 | Degraded banner driven by `x-ssr-skipped` header read | auto-loop (UI) | M | M | SSR overflow looks-like-success | n/a (UI) |

## 7. Phased path

**P0 — wiring & quick wins (days), ordered by leverage/effort:**
1. **Bound the storage-proxy await (#1)** — the prerequisite; un-wedges workers under a stalled storage peer.
2. **Wire the recovery route + unify the two dead paths (#2)** — a complete, 98.7%-covered service goes from dark to reachable with one route.
3. **Mount `HealthIndicatorComponent` + `/api/v1/status/projector` manifest line (#4, #9)** — two trivial reachability fixes that stand up affordance (A).
4. **Plumb `caughtUp` + `Retry-After`/structured 503 (#5, #6)** — turns the opaque hang into a "catching up" affordance with near-zero code.

**P1 — the three user affordances:**
1. **Force-reconcile trigger #3** (B) — adam's divergence gets a button.
2. **Surface `selfCidPresent`/`provideLoopEnabled` + "Network & Sync" panel #7, #12** (A) — makes the anchor-gap self-detectable.
3. **HTTP pause/resume + app-shell degraded banner #8, #13** (C) — the doorway-wedge affordance, human- and algorithm-drivable.

**P2 — the closed loop + elevate arm:**
1. **Bounded auto-heal loop #11** — composes the P1 triggers into detect→recover→verify.
2. **Runtime→finding elevate bridge #10** — the missing arm; the only thing that turns "self-heal exhausted" into a bug, riding the existing ledger+sentinel pattern.
3. **Dedicated pinned-runtime status surface #14** — so (A)/(C) and the loop's detect step survive a full worker stall.

## 8. Open questions / what to verify with eyes-on

- **The anchor-gap is TWO distinct mechanisms with different fixes** (L5, unresolved in code): (a) `SELF_CID` empty → provide-loop *unspawned* (`main.rs:962`, guard confirmed); (b) `self_cid` set but **no `DevicePin` for bulk-seeded content** → empty desired set → nothing authored. **Whether the bulk seeder creates `DevicePins` decides which fix #15 actually is.** Verify: `grep -rn 'SELF_CID' genesis/` (how prod sets it) + the seeder's pin-creation step.
- **Does `ssr_render_busy_total` reach any durable aggregator?** No prometheus/loki *emit* found in doorway src — only `tracing::warn` (L4). If it lands nowhere durable, the wedge signal is invisible to any log-based elevate arm — which is *why* N3 polls JSON endpoints, not logs. Confirm via the deployment log-scrape config in `genesis/manifests/`.
- **Render the dead recovery routes** to confirm the 404 visually: `pnpm look https://doorway-alpha.elohim.host/identity/recovery` and `/identity/recover`.
- **Confirm `/p2p/status` returns a hard 404 (not SPA shell) through doorway** as V3 concluded: `curl -s https://doorway-alpha.elohim.host/p2p/status`.
- **Confirm `MyClusterComponent` returns non-zero on live alpha** (Epic B may read 0 per `usedPct` zero-guard) before treating `/shefa/cluster`/`/shefa/health` as a trustworthy (A) surface — render once alpha stops flapping.
- **Verify the live SSR path renders vs always-CSR-shell** (`x-ssr-rendered`/`x-ssr-terminal` headers on a `pnpm look` of `/lamad`) to know whether the degraded-banner #20 has a real signal to read.
