---
id: portable-app-migration-plan
status: Draft
class: process-meta
context-tier: disclosed
steward: human:matthew
serves: release-retention
date: 2026-10-14
graduation-trigger: All four stages have owning gates and real delivery receipts; pending runtime stories are no longer wip
cites:
  - "portable-app-lifecycle-design | 2026-10-14-portable-app-lifecycle-design | sha256:2ca34a5e34b985fc | path: genesis/docs/superpowers/specs/2026-10-14-portable-app-lifecycle-design.md"
  - "live-session-plane-design | The live session | sha256:d1adbf86e46fd487 | path: genesis/docs/superpowers/specs/2026-10-14-live-session-plane-design.md"
  - "live-session-slice-1-own-devices-direct | Live session, slice 1 | sha256:d256fb581707f08d | path: genesis/docs/superpowers/plans/2026-10-14-live-session-slice-1-own-devices-direct.md"
---

# Portable app migration: execution order

The design owns the boundaries. This plan names the build order and proof owed. A completed
foundation does not make Che attachment, offline execution or private reconciliation delivered.
Do not add shell/Python orchestration as a substitute for a missing Rust API. CLI entrypoints
invoke the same native interfaces that the SDK and runtime adapters consume.

Recovered from `0ee5dfde8` on 2026-10-28. This remains a draft migration contract; its foundation runtime is a separate port. The newer [What every app inherits](../specs/2026-10-20-what-every-app-inherits-placement-contract-design.md) governs placement and manifest needs. A need becomes declarable only with its first reader; this recovery does not reopen undeclarable fields.

## Stage 1 — foundation contracts

Producer: SDK schemas, elohim-views and lower-level seam-contracts. Consumers: both SDKs,
browser bridge and workspace bridge. Habit: release-retention (readiness and residency remain
separate); the other stages retain their own existing habits.

- Compile lifecycle interfaces with explicit unsupported defaults and generated TS types.
- Keep release resolution, execution, residency, continuation, browser attachment, observation
  journal and synchronization as separate capabilities so a read-only adapter inherits no writes.
- Implement generic bounded demand leases below the SDK; freeze app stream targets in the SDK
  binding. Report per-device retained progress independently from remote reachability.
- Mint owner-local governance and seam declarations; register actual decision predicates and
  manifest-owned gates. No DNA or app-manifest capability claim is added by an interface.
- Verify: `just gate app-lifecycle-contracts`, `just gate elohim-sdk`, `just gate seam-contracts`,
  `just gate browser-bridge`, `just gate workspace-bridge`; manifest/seam validation and Gherkin.
- Proof is contract proof. Record real gate results in the owning habit, keeping runtime stories
  tagged `@wip`. Generated types must reproduce Rust's actual serialized fixture shapes.

## Stage 2 — authorized Che access

Depends on stage 1. Producers: peer app host/admission and doorway ingress. Consumer: node's
workspace adapter and ordinary authenticated browser. Habits: workspace-device-arrival,
idle-is-free, identity-cross-signed.

- Add the node adapter for ensure-instance, reusing Ark and deduplicating the operation identity.
  The first consumer is the existing native declared-view reader; no per-card proxy or capture loop.
- Compose device enrollment, native/hosted session handoff and resource permission independently.
  Validate scope at the peer-owned operation boundary. A recognized device and a roster entry
  cannot by themselves authorize another person's view.
- Implement scoped outbound WSS service attachment to doorway's existing HTTPS listener. The
  route lease identifies a configured peer surface, not a user-supplied URL/filesystem path.
  Define bounded frames, requests, concurrency, expiry, reconnect and revocation in the producer's
  schema before implementing transport. Preserve HTTP statuses and streaming backpressure.
- Keep the peer API functional without doorway. Doorway terminates browser ingress and relays
  the admitted surface; it does not become app serving, session logic or resource authority.
- The live-session slice's loopback-only routes stay local. Remote session exposure requires its
  explicit peer admission adapter; do not rewrite headers or defeat `caller_is_local`.
- Wire one run command to native start/reuse and return the reachable URL. Keep Che environment
  interpretation in workspace-bridge. Remove the secret fragment from the ordinary access flow
  only after direct and doorway paths pass; preserve the working launcher during migration.
- Prove real sign-in, repeat launch, failed refresh retaining the last reading, revocation,
  different-person refusal, origin/header tampering, reader restart and network interruption.
  Gates: owning node/hub, doorway, SDK and a2o. Receipt must use the real Che hostname.

## Stage 3 — Rakia adoption

Depends on stage 2 and verified artifact acquisition. Producers: storage release/hold APIs,
hub readiness and Ark execution. Consumers: Rakia composition through generic Avodah, browser
connector and native shell. Habits: release-retention, epr-atom-home, runtime-upgrade-propagation.

- Extend release artifact declaration and compatibility selection for supported native targets;
  derive from the elected release, not checkout bytes or renderer platform hints.
- Acquire and verify the complete executable dependency set under standing permission. New
  capabilities need their own consent. Hold the old usable release until startup succeeds.
- Add the minimal extension/native-host binding over browser-bridge with transport-observed
  origin and an authenticated session. No credentials in protocol URLs; no browser history capture.
- Implement continuation preparation over the existing private working version and activation
  with a durable operation identity. A failed activation cannot stop the old runtime or duplicate
  effects. Preserve tab navigation/unsaved work without copying signing keys between authors.
- Wire acquisition pins, explicit archive and policy-controlled idle reclamation through storage.
  Protect running/pinned bytes; never remove personal/unsynced data. Retention requires verified
  code AND data before an offline claim, and complete checked writes before replacing a saved copy.
- Prove coding and orchard with the same client, browser and shell takeover, fallback without
  connector, cold offline reopen, corruption/full-disk/interruption recovery and pin protection.
  Use pinned production bundles and visual review. Gates follow all producers plus frontend.

## Stage 4 — platform integration

Depends on stage 3. Producers: resident node/Ark composition, shared SDK persistence and the
existing observation service. Consumers: manifest-inventoried first-party apps and attention
dashboard. Habits: attention-witnessed-privately, sync-scale-honesty, operator-runtime-surface.

- Inventory actual app bundles from manifests; migrate main client, Avodah, Lamad and shell.
  Keep identity portals available for bootstrap. Shared libraries bind SDK contracts rather than
  creating their own stores, queues, process managers or session channels.
- Keep the resident runtime alive independently of app windows, bounded by device power and
  resource admission. Zero demand has no app-specific polling; presence exists only in a joined
  live session and follows that work's expiry/no-storage rules.
- Implement AppObservationJournal around ObservationEmitter's existing intent/receipt path.
  Checked local append precedes acknowledgement; retry handles survive restart; successful
  acknowledgement preserves provenance and signature status and cannot delete a different item.
- Reconcile private observations only through explicit own-device admission, never by widening
  public reach or gossip. Refuse revoked devices, cross-person joins and device-private disclosure.
- Dashboard open/refresh promotes the existing worker through demand leases. Show local results
  first, fixed-target progress and current availability separately. Coalesce worker work, preserve
  each caller's target, and release only priority on completion/cancellation/failure. Never replay
  presence or allow unknown remote ends to claim current freshness.
- Prove cold restart, every serving process stopped, duplicate deliveries, offline accumulation,
  reconnect, low power, cancellation, concurrent refresh and idle-at-rest behavior. All apps must
  retain their local functionality when the hub disappears. Each final result includes a real
  browser receipt, independent acceptance-story review, habit delta and regenerated projection.
