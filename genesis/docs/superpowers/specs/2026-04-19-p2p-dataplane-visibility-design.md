# P2P Dataplane Visibility — North-Star & Sprint Design

**Date:** 2026-04-19
**Status:** Design (review pending)
**Scope:** One north-star doc stitching existing P2P / resilience / peer / device / agent design work into a layered architecture, plus a single sprint scope ("Option B: your fabric, lit up") that makes four broken production surfaces show real household-scale resilience data using real peers on shem.

---

## 1. What triggered this doc

Four production surfaces on `alpha.elohim.host` and `doorway-alpha.elohim.host` show nothing meaningful:

| Surface | Symptom | Surface lives in |
|---|---|---|
| `/shefa/devices` | `node_registry_coordinator::get_my_nodes` zome fn doesn't exist; viewer crashes with `e.filter is not a function` | `app/elohim-app` shefa pillar |
| `/shefa/resources/category:content` | Empty list; user should see content they steward | `app/elohim-app` shefa pillar |
| `/shefa/dashboard` | Network Health tab empty; no posture, no peer summary | `app/elohim-app` elohim pillar (via shefa route) |
| `doorway-alpha/threshold/dashboard` | No registered peers; `/admin/routes` HTTP parsing failure; `/admin/users` 403 | `doorway/doorway-app` admin UI |

Adjacent noise: `GET /wasm/elohim-cache-core/elohim_cache_core.js 404` (asset missing from deploy), `POST /api/v1/mastery` and `/api/v1/mastery/engagement` 405 (routes exist in elohim-storage per `http.rs:6606`, but proxy manifest not registering POST method).

None of these is an isolated bug. They are the mirrors of a dataplane whose design work is largely complete but whose integration — across DNA, storage projection, doorway registry, and UI — is partial. The resilience icon on `content-viewer.component.html` showing `⚪ "No stewards assigned"` is the same class of problem at a smaller surface.

This doc is deliberately one document, not one-per-dashboard, because the fix is one coherent activation — turning existing designs into lit surfaces on shem using real household peers.

## 2. The north star — grandma-grade P2P, layered

The ambition stated by the user ("Deploying grandma on p2p and be as reliable with family photos as Google") is the acceptance horizon. The layers below must all work before that claim is true. Each layer has design work in the tree; few are fully activated end-to-end.

```
┌────────────────────────────────────────────────────────────┐
│ L7  Guardian loop & autonomous operations                  │  aspirational
│      elohim-agent reasons about mesh health, requests       │
│      diagnostic attestations, nudges operators, routes     │
│      compute, files GateDecisionAttestations                │
├────────────────────────────────────────────────────────────┤
│ L6  Household fabric / elohim-operator                     │  partial (design)
│      household as the unit. multi-blade clusters.          │
│      elohim-operator places elohim-node instances with     │
│      roles (edge/archival/inference) and balances storage, │
│      compute, and model-diversity budgets. k8s today,      │
│      native operator tomorrow. grandma gets a first-class   │
│      private p2p cloud without knowing she has one          │
├────────────────────────────────────────────────────────────┤
│ L5  elohim-node (hard topology) + device archetypes        │  design complete
│      elohim-node is the HARD layer — the physical host     │
│      boundary. like `kubectl get node`. has a fixed        │
│      hostname, committed resources, a role in the          │
│      household fabric. 15+ archetypes across cap levels    │
│      0–5 describe what a node CAN do                       │
├────────────────────────────────────────────────────────────┤
│ L4  PeerStatus (soft availability) + agency phases         │  design complete
│      PeerStatus is the SOFT layer — peer-authored, short-  │
│      TTL, derived from live state. like `kubectl get pod`. │
│      one peer per running conductor; may come and go       │
│      against a stable elohim-node below it. agency phases  │
│      (visitor/hosted/device/node/doorway) describe what    │
│      a peer IS DOING. doorway subscribes, not authors       │
├────────────────────────────────────────────────────────────┤
│ L3  Identity-driven replication & recovery                 │  partial
│      pull-mode: peers reconstitute their slice from DHT.    │
│      content filtered by identity (stewardship + commons). │
│      recovery protocol phases 2–5 remain                    │
├────────────────────────────────────────────────────────────┤
│ L2  Shard distribution & resilience projection             │  partial
│      RS 4+3 encoding. shard_manifests + shard_locations    │
│      local tables. /api/v1/resilience endpoint live.       │
│      periodic verification + auto-redistribution pending    │
├────────────────────────────────────────────────────────────┤
│ L1  Content addressing, EPR heads, blobs                   │  shipped
│      CID-addressed content, EPR publication, blob store,    │
│      automerge sync for metadata                            │
└────────────────────────────────────────────────────────────┘
```

**Hard vs. soft separation (L5 / L4).** The node/pod analogue is load-bearing. An elohim-node is a hostname with committed storage, compute, and model capacity — grandma's mini-PC, Matthew's home NUC, a shem blade. It changes rarely, and when it does change it's an operator event (blade added, drive replaced, node drained). PeerStatus is what a running conductor claims about itself right now — alive, degraded, leaving, accepting stewardship reserves. Many peers can run over their lifetime on one elohim-node (conductor restart, version bump, agent re-registration); the node is the durable fact, the peer is the current vital sign. The storage-↔-node surface: elohim-node publishes its hard shape (capacity, archetype, role) once at boot and on change; elohim-storage publishes PeerStatus every ~60s derived from live state. Network Health posture joins both.

**Peer diversity has two independent axes** (per `project_compute_and_model_independent_diversity_surfaces`): **compute** (hardware — this is L5) and **model** (which elohim, what context, what specialties — this is L7). They correlate (a Level-5 archetype can host a bigger model) but stay independent. The resilience picture this sprint builds sits on the compute axis; model diversity comes when the guardian loop lands.

The resilience story the UI must eventually tell at L2+L3+L4 is not peer-to-peer — it is **household-to-household**. A peer dies, another peer in the same household holds the shard, life goes on; the protection claim that matters is "N households reciprocally steward this content, my household can lose any one of them." Per-peer detail exists as drilldown.

The visibility layer (dashboards) reads across L2–L6 and is how operators, households, and stewarded humans experience all of it. When any layer is half-activated, the dashboards show nothing.

## 3. What already exists — inventory map

The below is cited from the design-doc survey. Names are paths; status is inferred from both doc state and code state.

**L2 (shard distribution / resilience projection):**
- `genesis/plans/2026-04-04-p2p-resilience-proof-design.md` (+ sprint-a/b/c plans). Sprint A mostly landed (resilience service + view shipped). Sprints B/C (auto-distribution, periodic verification, reconstruction endpoint) pending.
- `genesis/plans/2026-03-11-human-resilience-profile-plan.md`.
- Code: `elohim/elohim-storage/src/api/resilience.rs`, `src/sharding.rs`, `src/p2p/shard_protocol.rs`.
- UI: `ResilienceService`, `ResilienceView` type shipped; viewer loads `this.resilience` but the tooltip still only reads `this.stewardship`.

**L3 (identity-driven replication / recovery):**
- `genesis/plans/2026-04-06-identity-driven-replication-design.md` (+ plan). Design complete; commons-only scope (encryption deferred).
- `genesis/docs/plans/2026-04-17-peer-self-bootstrap-from-dht-design.md` — breadcrumb only.
- `doorway/doorway-service/RECOVERY-PROTOCOL.md`, `RECOVERY-SPRINT-PLAN.md` — 4-layer recovery, phase 1 done.
- Code: `app/elohim-app/src/app/imagodei/services/recovery-coordinator.service.ts` at 98.7% coverage; backend pull-mode stubs.

**L4 (peer-stewarded availability):**
- `genesis/docs/specs/2026-04-17-peer-stewarded-availability-design.md` — approved.
- `genesis/docs/plans/2026-04-17-peer-stewarded-availability-phase-1-plan.md` — task-by-task.
- `genesis/plans/2026-04-09-peer-status-schema-contract-design.md` (+ plan) + `2026-04-09-tier3-peer-metrics-design.md`. Tier 1/2 shipped; Tier 3 partial.
- Code: `infrastructure` DNA (PeerStatus entry type scaffolding in flight).

**L5 (device topology / agency):**
- `genesis/plans/2026-04-13-device-archetypes-design.md` (+ plan) — 15+ archetypes across levels 0–5, fixture data pattern parallel to humans.
- `genesis/plans/2026-04-10-agency-phase-registration-design.md` — 5 agency phases.
- `genesis/plans/2026-04-11-humans-schema-design.md`, `2026-04-10-collectives-schema-design.md`.
- `genesis/plans/2026-03-12-stewarded-node-topology-design.md` (older, informs `node-registry` DNA).
- Code: `genesis/data/humans/*.json` (personas live), `genesis/data/devices/` (archetype fixtures pending).

**L6 (fabric / operator):**
- `genesis/plans/2026-04-16-elohim-node-consolidation-design.md` (+ plan) — single-container elohim-node spawns holochain as subprocess; ready for Phase 1 rollout.
- Memory: `project_household_fabric.md`, `project_household_horizontal_scaling.md`, `project_elohim_node_role.md`.

**L7 (agent / gates):**
- `elohim/elohim-agent/spec/2026-04-18-gate-interface.md` + companion theory doc.
- `genesis/plans/2026-04-18-elohim-agent-gate-interface-plan.md`.
- Memory: `project_elohim_agent_sense_respond_architecture.md`, `project_elohim_active_observed_not_flagged.md`.

**Cross-cutting principles (load-bearing):**
- `feedback_schema_first_ioc.md` — wire contracts are JSON schemas; Rust/TS comply.
- `project_doorway_manifest_driven_routes.md` — doorway is web2-only (federation/CDN/DNS/bootstrap/signal); resilience/recovery/topology routes live in apps.
- `project_no_sovereignty_stewardship_over_ownership.md` — vocabulary discipline throughout.
- `project_household_is_resilience_unit.md` (new) — household not peer is the resilience grouping.
- `project_shem_is_p2p_live_canvas.md` (new) — shem has compute headroom to run all personas as real peers; dashboards lighting up on shem is the acceptance bar.

**Drift risk to flag:** there are two overlapping node-stewardship schemes — the older `node-registry` DNA (`NodeRegistration`, `NodeHeartbeat`, `CustodianAssignment`…) and the newer `infrastructure` DNA `PeerStatus` surface. They target the same concern from different angles. The `/shefa/devices` view is wired to `node_registry_coordinator::get_my_nodes`, which the coordinator zome does not implement. Before building more on either, the sprint must declare which one is canonical for the household/device story. Recommendation in §6.

## 4. Story anchors — a2o scenarios that govern this work

A2O scenarios are the specifications for the sprint. Existing coverage:

**Primary anchors (ready or near-ready step defs):**
- `genesis/a2o/features/content/stewardship-allocation.feature` — 6/6 steps shipped. Governs `/shefa/resources/category:content`. If it passes at the API layer but the UI shows nothing, the pipe is broken, not the spec.
- `genesis/a2o/features/deployment/doorway-self-registration.feature` — 3/3 steps shipped. `/admin/nodes` returns ≥1 node, hardware capacity populated. Governs the `threshold/dashboard` nodes tab; blocked on the missing `get_my_nodes` zome fn.
- `genesis/a2o/features/deployment/conductor-visibility.feature` — 4/4 steps shipped. Pool list, agents on conductor, user→conductor lookup, 403 for non-admin. Governs `/admin/users` and operator-scoped admin reads.
- `genesis/a2o/features/browser/doorway-dashboard-health.feature` — partial step defs (`ui/doorway-dashboard.steps.ts`). Dashboard renders without errors; tab switching clean.

**Primary anchors (aspirational — step defs missing):**
- `genesis/a2o/features/elohim/network-health-posture.feature` — 21 scenarios, all @wip. Canonical spec for `/shefa/dashboard` Network Health tab (active peers, stale peers, storage pressure, always-on counts, compute availability, operator attestation surface at info/debug/trace levels).
- `genesis/a2o/features/federation/peer-advertisement.feature` — 16 scenarios, all @wip. Canonical spec for gossipsub capacity heartbeat, neighbor table construction, stale eviction, cache readiness updates. Prerequisite to both `/shefa/devices` and Network Health.
- `genesis/a2o/features/shefa/human-resilience.feature` — 11 scenarios, all @wip. Canonical spec for household resilience status (at-risk → partial → protected), mutual aid activation, Elohim reasoning about institutional attestation needs.
- `genesis/a2o/features/deployment/peer-diversity.feature` — 20 scenarios, all @wip. Device capability gradient, memory pressure backpressure, stewardship boundaries.
- `genesis/a2o/features/deployment/human-device-mapping.feature` — 10 scenarios, all @wip. humans×devices×deployments referential integrity.

**Adjacent (sets up fixtures / upstream or downstream flows):**
- `federation/shard-tracking.feature`, `federation/epr-cross-peer-resolution.feature`, `federation/cross-doorway-content.feature`, `delivery/peer-mesh.feature`, `delivery/client-resilience.feature`, `elohim/compute-allocation.feature`, `elohim/compute-coordination.feature`, `elohim/elohim-presence.feature`.

**Gaps — new scenarios the sprint must author:**
1. `shefa/device-stewardship.feature` — "Matthew opens `/shefa/devices` and sees his household's nodes (matthew-home, matthew-laptop, jessica-phone) plus connected peer households (Adam/Eve, Pete, Timothy, Nancy) with archetype labels, lifecycle states, and serve-family/serve-public capability badges."
2. `shefa/network-health-dashboard.feature` — UI-layer scenario wrapping `network-health-posture` output into the `/shefa/dashboard` tab (peer-count card, storage-pressure gauge, always-on badge, household-reciprocation stat).
3. `shefa/stewarded-resources-visible.feature` — "Matthew opens `/shefa/resources/category:content` and sees the content his household stewards, grouped by affinity, with per-item resilience badge."
4. `doorway/admin-routes-visible.feature` — `GET /admin/routes` returns the registered route table with source types (Steward/DNA/Agent/Builtin) and counts; admin auth gates it.
5. `doorway/admin-users-visible.feature` — `GET /admin/users` returns registered humans with agency phase + assigned conductor; 403 for non-admin.
6. `shefa/resilience-tooltip.feature` — "Matthew hovers on a content's resilience icon and sees household stewards, shard encoding, peers online, health score — wired from `ResilienceView`."

**Fixture personas already in tree and usable on shem:** Matthew (doorway/manager), Jessica (spouse, device), Susan (sister, household), Adam (firstman/node), Eve (firstwoman/device), Pete (pastor/device), Timothy (ward of Matthew/device with capability grant), Nancy (neighbor/hosted), Gertrude (grandmother/core-family — large-text + simple-nav), Maria (newcomer), Ezra (newcomer). Enough diversity to prove household-to-household reciprocation with a couple, a ward, a pastor, a neighbor, and a newcomer — exactly the test matrix the resilience story needs.

## 5. Why nothing lights up — the integration gap

The design work is strong and the scenarios are in place for much of it. The reason production dashboards are blank is that integration across four boundaries has not completed:

1. **DNA ↔ storage projection.** `node_registry_coordinator::get_my_nodes` was assumed by the frontend anchor (`app/elohim-app/src/app/elohim/integrity/node-registry.anchor.ts:31`) but never implemented in the zome; `infrastructure` DNA's `PeerStatus` is the newer answer but its post-commit signal → projection → HTTP read path isn't yet the source for `/shefa/devices`. Result: no device data.
2. **Storage HTTP ↔ doorway proxy manifest.** `/api/v1/mastery` exists in storage (`http.rs:6606`) but returns 405 through doorway. The doorway route registry advertises the route's methods from storage's `build_manifest()`; if POST isn't declared there, doorway serves GET-only. `/admin/routes` handler exists (`admin.rs:798`) but isn't wired in `server/http.rs` — so the admin UI's `getRouteRegistry()` fails. These are integration oversights, not design gaps.
3. **Stewardship allocation ↔ UI.** The API passes a2o (`stewardship-allocation.feature` green), and the service layer (`StewardshipAllocationService`) is wired into the viewer, but no route surface renders them at `/shefa/resources/category:content`. The resource-explorer component needs to call the allocation API scoped to the signed-in human.
4. **Seed topology ↔ live cluster.** The fixture data describes 11+ personas with households and devices, but shem is not currently running them all as real peers. The dashboards have nothing to display because nothing is active to display.

Three of four issues are pipe-connection problems. The fourth is about activating the living test — which is why the user wants shem used.

## 6. Canonical decisions this sprint must lock in

Not exploratory. These are the decisions that unblock everything else and must be confirmed before the implementation plan is written.

**D1. PeerStatus is canonical for device VISIBILITY, elohim-node is canonical for device TOPOLOGY.** The `infrastructure` DNA `PeerStatus` entry is the canonical peer-availability record per the 2026-04-17 peer-stewarded-availability design. But PeerStatus answers "is this peer alive right now?" — not "what hardware is the household running?" That second question is answered by elohim-node, which publishes a durable node-shape (hostname, archetype, committed storage/compute/model, household binding) to storage on boot and on change. The two compose: `/shefa/devices` reads the node inventory for the household (hard list) and joins in PeerStatus for each node's currently-running peer (current vital signs). The older `node-registry` DNA custodian-assignment pieces remain for scheduled shard custody but are not the source of truth for the devices view; the frontend `NodeRegistryAnchor` retires. A follow-up task before implementation: clarify the storage↔elohim-node publish surface (which fields elohim-node owns, which are derived from PeerStatus, which are elohim-operator placements) so it mirrors the kubectl node/pod separation cleanly.

**D2. Household reuses `collectives`.** `Household` is a hard collective type — grounded by place, bound to shared physical infrastructure. The `2026-04-10-collectives-schema-design.md` schema covers the grouping; household gets a distinguished `kind: "household"` value. The household's place-groundedness may warrant an app-manifest hook at the schema boundary (place as first-class attribute, not tag), but that extension is a v2 concern and does not block this sprint. The `humans.json` fixture's spouse/family edges become the household membership links; a derivable `household_id` surfaces on humans (computed, not stored) so the query path `humans → household → nodes → peers` is clean for the UI.

**D3. Resilience UI displays household-first.** The resilience tooltip, the shefa dashboard, and the device page all lead with household counts. Per-peer detail is drilldown. This is the single place where every layer's data consolidates for a human.

**D4. Doorway stays web2-only.** Confirmed from memory and `doorway/CLAUDE.md`. This sprint adds ZERO per-domain proxy files to doorway-service. All new routes land in elohim-storage's `build_manifest()` and auto-register. The only doorway-service changes permitted: wire the existing `/admin/routes` handler into `server/http.rs` and fix `/admin/users` authorization.

**D5. Shem is the acceptance target — full roster, real peers.** Shem has >100 GB RAM and ~4 TB storage, ample to run the full persona roster as independent peers. Dashboards must light up on shem with real peers before the sprint can be called done; a2o scenarios remain the logic spec, shem is the behavioral spec. The demo specifically proves resilience/recovery for Matthew's family (household) — Timothy offline doesn't lose the family's content, Timothy's return restores `protected` status, and no content Jessica or Matthew or James stewards becomes unreachable during the dip.

## 6.A. Node-shape publish surface (concrete sketch)

The building blocks already exist; what's missing is the boot-time handshake and a canonical schema tying them together.

**Already in tree (do not rebuild):**
- `node-registry` DNA defines `NodeRegistration` with hostname/cpu/memory/storage/bandwidth/region/zomes_hosted/steward_tier/custodian_opt_in/max_custody_gb/claim_status/signature (`elohim/holochain/dna/node-registry/zomes/node_registry_integrity/src/lib.rs`).
- `stewarded_nodes` SQLite projection with CRUD routes `GET/POST /db/nodes`, `GET /db/nodes/{id}`, `POST /db/nodes/{id}/stewardship` in `elohim-storage/src/http.rs` (lines 4489–4602) — writes `dht_anchor_hash` back on projection.
- `devices.json` fixture carries capabilityLevel (0–5), memoryGb/storageGb/cpuCores/gpu, canSteward/canInfer/canDoorway, natType, bandwidth, healthSurfaces, attestationCapabilities, degradationMode per archetype.
- `PeerStatus.archetype_class: Option<String>` field in the integrity zome (`infrastructure_integrity/src/peer_status.rs`) already accepts an archetype label.

**What's missing (the actual sprint work):**
1. Elohim-node reads `DEVICE_ARCHETYPE` (env or `--device-archetype`) at boot, looks up the archetype in `devices.json`, and derives capabilityLevel/memoryGb/storageGb/cpuCores/canSteward/canInfer/canDoorway.
2. Elohim-node reads hostname via `gethostname()`, household via `HOUSEHOLD_ID` (env, matches a `collectives` entry with `kind: "household"`), role via `NODE_ROLE` (env — edge/archival/inference/doorway).
3. Elohim-node POSTs once at boot to a new schema-first publish route that lands as a NodeRegistration DHT entry and stewarded_nodes projection row. Subsequent updates are idempotent upserts.
4. Elohim-node passes the archetype label through to the heartbeat task so PeerStatus.archetype_class is populated on every tick (currently scaffolded but not fed from config).

**Schema sketch — `elohim/sdk/schemas/v1/views/node-shape-view.schema.json` (new, schema-first per memory):**

```jsonc
{
  "$id": "node-shape-view.schema.json",
  "type": "object",
  "required": ["nodeId", "hostname", "deviceArchetypeId", "householdId",
               "capabilityLevel", "role", "committed", "signature", "signedAt"],
  "properties": {
    "nodeId":             { "type": "string" },        // hardware-stable (MAC or UUID)
    "hostname":           { "type": "string" },
    "deviceArchetypeId":  { "type": "string" },        // FK into devices.json
    "householdId":        { "type": "string" },        // FK into collectives (kind=household)
    "role":               { "enum": ["edge", "archival", "inference", "doorway"] },
    "capabilityLevel":    { "type": "integer", "minimum": 0, "maximum": 5 },
    "committed": {
      "type": "object",
      "required": ["cpuCores", "memoryGb", "storageTb"],
      "properties": {
        "cpuCores":        { "type": "integer" },
        "memoryGb":        { "type": "integer" },
        "storageTb":       { "type": "number"  },
        "bandwidthMbps":   { "type": "integer" },
        "maxCustodyGb":    { "type": "number"  },
        "canSteward":      { "type": "boolean" },
        "canInfer":        { "type": "boolean" },
        "canDoorway":      { "type": "boolean" }
      }
    },
    "stewardTier":        { "enum": ["caretaker", "guardian", "steward", "pioneer"] },
    "custodianOptIn":     { "type": "boolean" },
    "region":             { "type": ["string", "null"] },
    "signature":          { "type": "string" },        // self-signed over payload
    "signedAt":           { "type": "string", "format": "date-time" },
    "dhtAnchorHash":      { "type": ["string", "null"] }  // populated after DHT commit
  }
}
```

**Ownership split (the kubectl-node/pod analogue in concrete terms):**
- **Operator declares (env vars, elohim-operator manifest):** `DEVICE_ARCHETYPE`, `HOUSEHOLD_ID`, `NODE_ROLE`, `REGION`.
- **Elohim-node self-determines at boot:** `nodeId`, `hostname`, derived `capabilityLevel` + `committed` fields from archetype lookup, `signature`, `signedAt`. Elohim-node is the only party that can sign for the host.
- **PeerStatus projects live (every ~60s):** `status` (Starting/Online/Degraded/Maintenance/Leaving), `flags` (general_pool_member, accepting_stewardship_reserves), `archetype_class` (echoes the archetype so the subscriber doesn't need to join).
- **Storage computes for visibility:** `/shefa/devices` reads `stewarded_nodes` filtered by household, LEFT JOINs `peer_statuses` on `nodeId`/agent. Nodes without recent peer status render as "offline" — which is itself the protection claim working.

**Route shape:** `POST /api/v1/nodes/shape` (schema-first, camelCase, owned by elohim-storage; proxied by doorway via manifest auto-register; matches existing POST-to-storage patterns). `GET /api/v1/households/{id}/devices` returns the join. Both declared in `build_manifest()` so doorway picks them up without code changes.

**Why not reuse `POST /db/nodes` directly:** the existing `/db/nodes` handler accepts a NodeRegistration payload but doesn't know about archetype or household binding, and doesn't drive a DHT commit today. The new `/api/v1/nodes/shape` is a thin wrapper that (a) validates against the schema, (b) upserts stewarded_nodes with the archetype/household fields, (c) triggers the dormant `register_node` zome call to commit to DHT, (d) returns the dht_anchor_hash. Existing `/db/nodes` GETs remain for read-side callers.

## 6.B. Maintenance-safe data movement

Resilience is the P2P dataplane's job, not the storage driver's. But maintenance — draining a blade, restarting a node, moving a persona to different hardware — requires a protocol-level choreography so the household never loses protection during the operation. k8s is a modeling convenience; the real contract is what PeerStatus + household-resilience + doorway routing must coordinate.

**The six-step choreography:**

1. **Announce.** Operator marks the node for maintenance. Elohim-node flips `PeerStatus.status = Maintenance`, clears `general_pool_member` and `accepting_stewardship_reserves`. Doorway route-registry subscribers stop routing new work to this peer.
2. **Verify household protection holds.** Before the drain proceeds, household-resilience computation (sprint step 8) must report the household as `protected` or at-worst `partial` *assuming this node is gone*. If the projection would drop to `at-risk`, the maintenance is blocked or requires an operator override with an attested trail.
3. **Quiesce.** In-flight work drains. Holochain source chain flushes. SQLite WAL checkpoints. Every durable fact is on disk before shutdown.
4. **Move (if hardware change).** Copy the data directory — agent key, source chain, SQLite projections, local blob store — to the destination volume. Content-addressed blobs are fungible (re-obtainable from peers), but the agent key + source chain are the irreducible identity piece.
5. **Verify integrity at destination.** Fingerprint check — hash of source-chain head + key pubkey + schema version matches origin. Refuse to start on mismatch. Identity continuity is unambiguous or the node doesn't come up.
6. **Rejoin.** Elohim-node boots, re-publishes node-shape (same `nodeId`, possibly new `hostname`), PeerStatus runs `Starting` → `Online`, flags restore. Doorway subscribers see it again, routing resumes.

**What this sprint delivers:** steps 1, 2, 3, and 6. PeerStatus lifecycle is Phase 1 complete; doorway routing exclusion for non-Online peers is the peer-stewarded-availability design's direct output; household-resilience computation and `Starting→Online` on rejoin are sprint deliverables 7 and 8; node-shape publish on boot is deliverable 4. The announce + protect-check + rejoin legs emerge naturally from what's being shipped.

**What this sprint defers:** steps 4 and 5 as an implemented flow — the blade-to-blade move with source-chain migration and fingerprint verification. That's elohim-operator native territory (SP-4 appliance packaging) and depends on Holochain's source-chain migration story, which is not ours to invent. For this sprint, a node that goes into `Maintenance` comes back up on the same volume on the same host; moving a persona across hosts is a deferred ops concern.

**Shem volume posture (transitional):** k8s is a modeling convenience, not the production target. Shem uses **StatefulSet per persona + local-path-provisioner stable-bound volume**. No dynamic cross-zone provisioning, no CSI driver bets. The pod rescheduling can happen in place (stop → start on same volume). If a shem blade dies hard, the persona's state is lost on that blade — but household-resilience covers the content-side, and the agent-key/source-chain loss is the one genuine gap (which is the same gap SP-4 addresses with Shamir-household split-custody). For the demo, we accept that gap and declare it out of scope; we do not build an elaborate volume-backup apparatus that will be thrown away when elohim-operator goes native.

**The genuine irreducible risk for grandma:** loss of the agent key + source chain without a Tier-1 recovery mechanism. Until Recovery Protocol Phases 2–5 ship (SP-1), that loss IS a human-intervention event. This sprint does not fix it; it makes the protection claim *around it* honest — the household can lose any one node's content contribution without impact, but losing the last copy of that node's key material is still a problem worth flagging explicitly in the UI.

## 7. The sprint — "Your fabric, lit up"

**Goal.** By end-of-sprint, a user signed in as Matthew on `alpha.elohim.host` (projected through his doorway running on shem) sees:
- `/shefa/devices` shows Matthew's devices + connected households (Adam/Eve, Pete, Timothy, Nancy) with lifecycle, archetype, and capability flags sourced from live PeerStatus.
- `/shefa/resources/category:content` shows the content Matthew's household stewards, grouped by affinity, each with a resilience badge (household count + health).
- `/shefa/dashboard` Network Health tab shows peer posture, storage pressure, always-on summary, household reciprocation count — all computed live from PeerStatus + stewardship allocations + resilience projection.
- Any content viewer's resilience tooltip shows real data: shard encoding, household stewards, peers online, health score — not "No stewards assigned."
- `doorway-alpha.elohim.host/threshold/dashboard` shows registered peers (via doorway's PeerStatus subscription), route registry populated, users list populated (admin-gated).

Running on shem with the full persona roster as real peers. Demo scenario: bring one peer (Timothy) offline; household resilience drops to `partial`; bring them back; status restores.

**What ships (ordered by dependency):**

1. **Canonical decisions locked.** D1–D5 above committed as decision record in the spec tree (new file in `genesis/docs/superpowers/specs/decisions/`). Retires `NodeRegistryAnchor`.
2. **PeerStatus Phase 1 closeout (small).** Phase 1 is ~95% shipped — integrity types, link types, validation, all three coordinator functions, diesel model + migration with source-of-truth comment, policy engine, heartbeat tokio task, TCP forwarder, example config, post-commit signal wiring into `peer_statuses` projection, config field, lib/main spawn — all present and running (verified file-by-file against the Phase 1 plan). Two gaps remain: (a) sweettest integration test at `elohim/holochain/tests/peer_status.rs` is missing (file doesn't exist), (b) no HTTP read route for the `PeerStatusView` — the view struct and schema (`peer-status-view.schema.json`) exist but no route in `build_manifest()` serves them. This sprint lands both as Phase 1 closeout: write the sweettest test, add `GET /api/v1/peer-statuses` + household-filtered variant to storage. Small scope.
3. **Household grouping.** Extend `humans` + `collectives` to surface a queryable `household_id`. Fixtures updated in `genesis/data/humans/*.json` for Matthew (+ Jessica, + James, + Susan), Adam (+ Eve), Pete (solo), Timothy (under Matthew's stewardship), Nancy (solo), Gertrude, Maria, Ezra.
4. **Node-shape publish + device list.** Implement the §6.A sketch: `node-shape-view.schema.json`, `POST /api/v1/nodes/shape` (upserts `stewarded_nodes` with archetype + household fields, triggers `node-registry::register_node` DHT commit, returns `dht_anchor_hash`), `GET /api/v1/households/{id}/devices` (LEFT JOIN stewarded_nodes × peer_statuses on nodeId, returns hard shape + live vitals). Elohim-node boot reads `DEVICE_ARCHETYPE`/`HOUSEHOLD_ID`/`NODE_ROLE`/`REGION` env vars, derives committed fields from archetype lookup, POSTs to the shape route, and passes archetype_class through to the heartbeat task so PeerStatus tagging is populated per tick. Manifest declarations only; no doorway code changes.
5. **Stewarded-resources endpoint & page wiring.** `/shefa/resources/category:content` calls existing stewardship allocation API filtered by the signed-in human's household; resource-explorer component renders by affinity with resilience badge.
6. **Resilience tooltip.** `getResilienceTooltip()` in `content-viewer.component.ts` reads `this.resilience` (already loaded) and displays household count + shard encoding + health score. Fallback states honest.
7. **Network Health posture.** Compute posture from PeerStatus + shard_locations + stewardship_allocations. HTTP endpoint `GET /api/v1/network/posture`. `/shefa/dashboard` tab renders the posture card. Info-level no attestation; debug-level requires `compute:debug` attestation (per `network-health-posture.feature`).
8. **Resilience-is-household computation.** Service-level: for any content, compute `householdsStewarding: int`, `householdsReciprocated: int`, `protectionStatus: at-risk|partial|protected`. This becomes the top-level resilience claim in UI.
9. **Doorway admin gaps.** Wire `/admin/routes` handler in `server/http.rs`. Restore `/admin/users` authorization (admin passes, others 403). (The mastery 405 is tabled as a separate fix — out of scope here.)
10. **Shem activation.** Deploy the household roster as real peers on shem: Matthew household (3 devices: matthew-home always-on, matthew-laptop intermittent, jessica-phone intermittent), Adam household (adam-node always-on, eve-laptop intermittent), Pete (pete-laptop intermittent), Timothy (timothy-laptop intermittent under Matthew's stewardship grant), Nancy (hosted via doorway-alpha), doorway-alpha itself. Drive the human-resilience demo scenarios against the cluster.
11. **New a2o features.** Author the six gap scenarios from §4. Step defs for `network-health-posture`, `peer-advertisement`, `human-resilience` feature files (turning @wip into executable).
12. **WASM asset fix (small).** Resolve `elohim_cache_core.js 404` in deployment. Single-commit, not a sprint-shaping piece.

**What does not ship (explicitly deferred):**
- Recovery Protocol Phases 2–5 (DHT entry types in imagodei DNA, shard reconstruction, work-while-recovering, verification). Stays in recovery sprint planning.
- Private-content encryption for pull-mode replication (identity-driven-replication design § future sprint).
- Hardware appliance packaging ("the black-box mails to grandma's router").
- Guardian loop autonomous operations (L7) beyond what gate-interface v1 already specifies.
- elohim-operator native implementation (L6). Shem stays k8s for now.
- Tier 3 peer metrics phases 7–10 (bandwidth, NAT, extended RTT). Info/debug attestation split stays in this sprint; bandwidth metrics stay deferred.
- Auto-distribution on ingest (resilience Sprint C1) and periodic shard verification (C3). Follows this sprint; visibility first, active distribution next.

**Acceptance bar.**
- All six new a2o scenarios (§4) green.
- `human-resilience.feature`'s "Matthew + Susan + Pete" scenario green — protection_status reports `protected` with trust_circle ≥ 2 and three-household reciprocation, computed from live data.
- `peer-advertisement.feature`'s "Heterogeneous network handles mixed availability" scenario green — timothy-laptop offline, matthew-home + adam-node stay online, routing excludes stale.
- All four dashboards on `alpha.elohim.host` + `doorway-alpha.elohim.host` display real data with no console errors.
- Shem cluster runs the full persona roster and sustains the demo scenario (Timothy offline → partial → Timothy online → protected) over at least 10 minutes.
- **Maintenance choreography demo (§6.B steps 1/2/3/6):** Timothy's elohim-node can be flipped to `Maintenance` via operator action → doorway stops routing to it within 60s → household-resilience reports the projected state before the drain proceeds → restart on same volume → `Starting` → `Online` → routing resumes. Household protection never drops to `at-risk` during the operation.

## 8. Sub-project decomposition (for next sprints)

Once this sprint ships visibility, the following sub-projects become fundable with clear scope:

- **SP-1: Recovery Protocol Phases 2–5.** Pull-mode reconstitution, social recovery, shard reconstruction, work-while-recovering, verification. Depends on PeerStatus projection (this sprint).
- **SP-2: Active shard redistribution.** Auto-distribution on ingest, periodic verification, reconstruction endpoint. Picks up resilience-proof Sprint C.
- **SP-3: Private-content encryption.** Key exchange for non-commons pull-mode replication. Deferred per identity-driven-replication design.
- **SP-4: Appliance packaging.** elohim-node single-container (design in tree) compiled for ARM mini-PC; mails-to-grandma onboarding flow; elohim-operator native (exits k8s).
- **SP-5: Guardian loop.** L7 — elohim-agent reasons about mesh health, requests attestations, nudges operators. Depends on gate-interface v1 landing and PeerStatus + posture data (this sprint).
- **SP-6: Device archetype policy generation.** v2 of peer-stewarded-availability — auto-sensing shape report instead of explicit config per archetype.

## 9. Sprint cadence

**Single integration push, with one internal checkpoint.** Deliverables 1–7 ship the surfaces lit on local/seeded data (locked decisions, PeerStatus end-to-end, household grouping, device endpoint, stewarded-resources wiring, resilience tooltip, network posture). Checkpoint here: all four dashboards show real data locally, all primary a2o scenarios green. Then deliverables 8–12 activate shem, author the new a2o scenarios, and drive the household-resilience demo to acceptance. Separating the phases into two sprints risks losing the tight feedback loop between integration and activation — shem is not a "later" target, it is the behavioral spec. One push keeps the team honest about what "done" means.

## 10. Risks

- **DNA coordination between node-registry and infrastructure.** Retiring a frontend call path. Mitigation: D1 is a decision, not a code change; the retirement happens behind a flag where needed, and the frontend anchor is replaced not removed.
- **Storage↔elohim-node publish surface is not yet specified.** D1 hinges on elohim-node publishing durable node-shape to storage; that interface needs writing before the node-registry anchor retires cleanly. Treat this as the first design artifact inside the sprint, not a blocker to starting.
- **a2o scenarios written but not runnable.** The 21 network-health-posture scenarios, 16 peer-advertisement, 11 human-resilience are a lot of step-defs. The sprint does not need ALL of them green — only the demo-critical handful named in §7 acceptance. The rest feed SP-5.
- **PeerStatus Phase 1 closeout.** Verified ~95% shipped via file-by-file audit against the Phase 1 plan. The two remaining pieces (sweettest integration test + HTTP read route for PeerStatusView) land inside this sprint as step 2, not as a preceding Phase 1 effort. No real risk of slip here.
- **Household as collective kind needs schema verification.** The `collectives` schema should accept `kind: "household"` with place-groundedness; if it can't without v2 changes, the sprint scope must either make the minimal schema change or fall back to a derived household view computed from humans.json edges.
