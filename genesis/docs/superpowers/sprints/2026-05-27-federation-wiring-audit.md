# Sprint Result — Federation Wiring Audit

**Shift ID:** `2026-05-27T18-50-federation-wiring-audit`
**Branch:** `sprint/cross-pillar-cleanup`
**Operator:** Matthew Dowell
**Date:** 2026-05-27

## Outcome

**Phase 1 landed:** per-human primary doorway routing wired in
`elohim/holochain/Jenkinsfile`. Family-three personas (matthew/jessica/james)
get doorway-A; the 11 remote personas (adam included) get doorway-B. Measure
script `genesis/agentic/bin/federation-routing-dryrun.mjs` drives 0 → 5/5.

**Phase 2 audit:** 8 surfaces inspected. Three are complete, three are
stubbed, one is end-to-end-wired-but-blocked-by-other-work, one is
operator-ground-truth that I can't verify from the dev environment.

## Phase 1 — implementation summary

### What changed

`elohim/holochain/Jenkinsfile` (2 hunks):

1. **`getEnvConfig()` alpha block (L302-329):** added `doorwayA` + `doorwayB`
   sub-maps with each doorway's bootstrap+signal URL pair. Kept the top-level
   `bootstrapUrl`/`signalUrl` as the single-doorway fallback for staging/prod
   (which still run single-doorway envs).
2. **`deployHumanManifest()` selection (L575-593, L595-606):** added a
   `primaryDoorway` local that selects between `envConfig.doorwayA` and
   `envConfig.doorwayB` based on `humanConfig.nodeTypes.contains('remote')`.
   Falls back to a synthesized pair from the env's top-level URLs when neither
   sub-map is declared (staging/prod single-doorway path). Replaced the sed
   substitutions at L601-602 to interpolate from `primaryDoorway`.

The same sed pipeline already reaches Adam's hand-rendered manifest
(`adam-firstman.yaml` L79-81 carries the placeholders) — no separate code path
needed.

### Persona routing (verified by the dry-run script)

| Persona | nodeTypes | primary doorway | bootstrap URL substituted |
|---------|-----------|------------------|-----------------------------|
| matthew | operations, edge, performance | A (on-prem) | `https://doorway-alpha.elohim.host/bootstrap` |
| jessica | operations, edge, performance | A (on-prem) | same as matthew |
| james | operations, edge, performance | A (on-prem) | same as matthew |
| adam | remote | B (shem apex) | `https://elohim.host/bootstrap` |
| pete | remote | B | same as adam |
| terrance | remote | B | same as adam |
| frank | remote | B | same as adam |
| gertrude | remote | B | same as adam |
| susan | remote | B | same as adam |
| caleb | remote | B | same as adam |
| daniel | remote | B | same as adam |
| emma | remote | B | same as adam |
| eve | remote | B | same as adam |
| nancy | remote | B | same as adam |

3 personas on doorway-A, 11 on doorway-B. The genesis pair (matthew on-prem,
adam on shem) is now co-located with its respective doorway, which removes the
cross-WireGuard signaling hop the previous single-doorway-for-everyone shape
imposed on every shem-side human.

### Landed by commit

Phase 1 was committed by the operator as **`91f300663`** *"infra(jenkins):
federation prep — per-human primary doorway routing + drop phantom timothy"*.
That commit bundles a complementary cleanup beyond this shift's named scope:
phantom `timothy` references (suspended persona from deployments.json long ago)
were removed from `genesis/Jenkinsfile`'s fallback storage-URL + conductor-URL
lists (`timothy` → `adam`), from the M1 cross-pod fetch test (`timothy` →
`jessica`), and from `elohim/holochain/Jenkinsfile`'s `fullDistribution`
defaults (dropped `timothy:'staging'`, added `james:'alpha'` to mirror
deployments.json reality). Worth noting because the timothy drop also explains
the Genesis seeder polling a non-existent workload — the symptom that was
showing up in orchestrator #1069's timothy-tutor replication-timeout signal is
a compound of the InvalidHashFormat verifier bug (Phase 2h) AND this phantom
poll. The audit below treats them separately, but the post-91f300663 baseline
should resolve the phantom-poll component cleanly even without the verifier
fix.

### Measure output (target = 5)

```
$ node genesis/agentic/bin/federation-routing-dryrun.mjs --verbose
federation-routing-dryrun: 5/5 passing
  [PASS] alpha.doorwayA.urls
  [PASS] alpha.doorwayB.urls
  [PASS] deployHumanManifest.selection
  [PASS] sed.BOOTSTRAP_URL.per_human
  [PASS] sed.SIGNAL_URL.per_human
```

Commit SHA will be appended after push.

### What Phase 1 explicitly does NOT do

- Does **not** wire fallback or secondary doorway per human. Conductor config
  schema is singular — a single bootstrap_url + signal_url per human. This is
  intentional: per the stop conditions, Phase 1 stays the simplest shape that
  decouples the shem-side cluster from doorway-A's signal server. The harder
  multi-registration question lives in the Phase 2a sequenced backlog.
- Does **not** rewrite sessions, recovery flows, or doorway-binding throughout
  doorway-service / elohim-storage (Phase 2e is the bug surface; out-of-scope
  per the Objective's path scope).

## Phase 2 — audit findings (a–h)

### a) Multi-registration / fallback — *stubbed-at-substrate*

**State:** Holochain conductor config (`bootstrap_url` / `signal_url` in
`adam-firstman.yaml` L79-81 and the consolidated template) accepts singular
string fields, not arrays. `elohim-storage` Cargo.toml pins
`holochain_client = 0.9.0-dev.5` and `holochain_types = 0.7.0-dev.5` — HC 0.7
series. The deployed conductor-config.yaml shape uses singular `bootstrap_url:`,
which is the only shape the schema accepts. Multi-bootstrap at the conductor
layer would need an upstream schema extension; multi-bootstrap at the substrate
layer would need elohim-storage to maintain its own parallel peer-discovery
channel and merge results into the conductor's view (a substantial design).

**Verdict:** *stubbed-at-substrate — `elohim/elohim-storage/src/p2p` does not
yet maintain a parallel peer-discovery channel that could provide fallback
signal discovery when the conductor's configured signal_url is unreachable*

### b) FEDERATION_PEERS protocol — *complete (outbound side)*

**State:** End-to-end wired on the outbound side:

- `doorway/doorway-service/src/config.rs:207-208` — `FEDERATION_PEERS` parsed
  as `Vec<String>`.
- `doorway/doorway-service/src/main.rs:871-924` — non-empty triggers
  `spawn_peer_discovery_task` + `register_doorway_in_dht` (if doorway_id +
  doorway_url present) + `spawn_heartbeat_task`.
- `doorway/doorway-service/src/services/federation.rs` — full surface:
  `register_doorway_in_dht` (L125), `spawn_heartbeat_task` (L205),
  `get_all_doorways` (L580), `refresh_peer_cache` (L721),
  `spawn_peer_discovery_task` (L766), `get_cached_peers` (L786).
- `doorway/doorway-service/src/routes/federation.rs` — admin API for
  add/remove/refresh/list peers (L409, L448, L492, L527).
- `doorway/doorway-service/tests/dashboard_topology.rs:62` —
  `dashboard_topology_reports_federation_peers_from_cache` asserts the cached
  peer surface reaches the dashboard.

**Verdict:** *complete — FEDERATION_PEERS is fully consumed by the discovery +
heartbeat + admin-mutability layers; not a dangling env var. Bidirectional
inbound reconciliation is a separate finding — see (f).*

### c) DHT-derived doorway discovery — *complete*

**State:** `elohim/holochain/dna/infrastructure/zomes/infrastructure/src/lib.rs`
implements:

- `register_content_server` (L724) — creates `ContentServer` entry + four
  discovery links: `HashToContentServer`, `AgentToContentServer`,
  `CapabilityToContentServer`, `RegionToContentServer`.
- `find_publishers` (L903) — queries by hash/capability/region; returns
  `FindPublishersOutput` (multi-publisher by construction).

Both alpha doorways have `ELOHIM_STORAGE_AUTO_REGISTER=true` (alpha.yaml L131,
alpha-b.yaml L159), so both call `register_content_server` with wildcard
`content_hash="*"` + capabilities `[blob, html5_app]`. Result: two distinct
`ContentServer` entries, each linked from the wildcard and capability anchors;
a third-party peer calling `find_publishers` will get both.

**Verdict:** *complete — multi-publisher discovery is structurally supported at
the DHT layer. Requires both doorways to be alive + registered to actually
populate two entries, but the path is wired.*

### d) Doorway-B ingress + cert + signal subdomain — *manifests-wired, cluster-state-needs-operator-verification*

**State (manifests):** `alpha-b.yaml` declares the ingress at L361-418:
- `host: elohim.host` → service `elohim-doorway-alpha-b` port 8080
- `host: signal.elohim.host` → same service (tx5/SBD WebRTC relay)
- TLS hosts both, secretName `elohim-host-apex-tls`, cluster-issuer
  `letsencrypt-production`

**Cluster-state caveat:** `prod.yaml` carries a parking note (L13-22) that the
prod-pipeline still claims `doorway.elohim.host` (separate from `elohim.host`
which has been re-bound). The `alpha-b.yaml` parking note (L33-37) says the
operator must `kubectl delete ingress -n elohim-prod elohim-doorway-prod` once
to free `elohim.host` so cert-manager can reissue `elohim-host-tls-cert` (the
old prod secret) into the new `elohim-host-apex-tls` slot in `elohim-alpha`.
**I cannot verify whether that operator action has happened — no kubectl from
this environment.** From recent commit history (e2124fe64 "realign backend to
adam to match shem placement", 92686971e "generous health probes + 2× CPU
limit") it looks like doorway-B is in the operator's active rollout window; the
ingress dependencies may or may not be settled.

**Verdict:** *manifests-wired (every required ingress + cert binding is
declared correctly); cluster-state-pending-operator-verification (kubectl get
ingress -n elohim-alpha elohim-doorway-alpha-b -o yaml + kubectl get
certificate -n elohim-alpha elohim-host-apex-tls would confirm).*

### e) Account-recovery ceremonies — *stubbed-at-session-binding*

**State:** Recovery infrastructure exists on both layers:

- doorway-service: `orchestrator/disaster_recovery.rs` (DisasterRecoveryCoordinator,
  RecoverySummary, NATS-driven), `routes/auth_routes.rs:389+`
  (`/// Doorway URL for future recovery`), recovery request/response types at
  L445+.
- elohim-storage: `services/recovery_flow_projector.rs` (Phase 2 M4 projector
  for recovery + key-revocation projections), `p2p/recovery_invitation.rs`,
  `p2p/recovery_revocation.rs`.

**The bug Phase 1 implicitly exposes:** session + recovery binding is
single-doorway throughout:
- `elohim/elohim-storage/src/db/local_sessions.rs:32` — `LocalSession.doorway_url: String`
  (mandatory single string).
- `elohim/elohim-storage/src/db/local_sessions.rs:297` — `doorway_url TEXT NOT NULL`
  in the schema.
- `doorway/doorway-service/src/auth/jwt.rs` — JWT claim carries
  `doorway_url: Option<String>` (single).
- `doorway/doorway-service/src/conductor/chaperone.rs:459`,
  `routes/identity.rs:107+`, `routes/federation.rs:347` —
  `state.args.doorway_url.clone()` everywhere; single URL.

Today every human is bound to one doorway URL at session creation. Phase 1
makes the assignment placement-aware (remote→B, on-prem→A) but the asymmetry
still means a human who registered through doorway-A cannot recover through
doorway-B (the session lookup keys on `doorway_url`).

**Verdict:** *stubbed-at-session-binding — `elohim/elohim-storage/src/db/local_sessions.rs:32`
+ `doorway/doorway-service/src/auth/jwt.rs:95,128,222,263,440,517,541,555` all
require the human's single registered `doorway_url` to match the recovery
attempt's `doorway_url`. Doorway-agnostic recovery is the next-layer ask and is
out-of-scope for this shift.*

### f) Federation reconciliation — *partially-wired (outbound discovery yes; inbound bidirectional reconciliation stubbed)*

**State:** The peer-discovery + heartbeat + cache pieces are real (see (b)
above). What's stubbed is the actual reconciliation of shared content between
peer doorways. `doorway/doorway-service/src/services/dashboard_topology.rs`
header comment (L10-12) explicitly says:

```
- `federation_peers.online` — always `true` (cached = known).
- `federation_peers.direction` — always [`FederationDirection::OutboundOnly`].
- `federation_peers.shared_cid_count` — always `0`.
```

And `dashboard_topology.rs:150` has a `TODO(Phase 5): constructed once
collect_federation_peers performs …` — explicit unfinished work. Distinct
`MONGODB_DB` between A and B (`doorway-alpha` vs `doorway-alpha-b`) means
projection-cache divergence is observable, but no code reconciles the diverged
state.

**Verdict:** *partially-wired — outbound discovery, registration, heartbeat
exist; inbound bidirectional reconciliation (shared content state, direction
inference, shared_cid_count) is hardcoded as constants in
`doorway/doorway-service/src/services/dashboard_topology.rs:10-12` and gated on
"TODO(Phase 5)" at the same file L150.*

### g) Adam-as-shem-backend sizing — *complete (just landed)*

**State:** Recent commit `ea35f82fa "infra(adam): size to genesis-pair parity
with matthew"` (HEAD) sized `adam-firstman.yaml` L283-289 to:
- requests: memory 2Gi, cpu 1000m
- limits: memory 8Gi, cpu 3000m

This matches matthew's bumped envelope (deployments.json L39-42:
2Gi/8Gi + 1000m/3000m). Both genesis-pair peers now carry symmetric resource
budgets, appropriate to each servicing one doorway's reads plus its share of
P2P gossip + DHT validation.

**Verdict:** *complete — sized in commit ea35f82fa as part of doorway-B
realignment to adam. No further sizing change recommended until soak data
shows actual usage shape.*

### h) `is_blob_hash_shaped` InvalidHashFormat verifier — *confirmed-still-blocker*

**State:** `elohim/elohim-storage/src/p2p/inventory_gossip.rs:131-134`:

```rust
/// Sha256 hex shape check: 64 lowercase hex chars (defensive structural rule).
fn is_blob_hash_shaped(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}
```

Called at **two** points in `verify_structural`: line 91 (snapshot path) and
line 123 (delta path). Both reject `sha256-<hex>` wire format (which is 71
chars: `sha256-` prefix + 64 hex). Tests at L145+ continue to use bare hex
(`"a".repeat(64)`), reinforcing the old shape.

Genesis pipeline #1045 / orchestrator #1069 timothy-tutor replication timeout
(UNSTABLE) is consistent with this verifier rejecting valid hashes at the
delta-verify step → empty replications → 0 items replicated after 5+ minutes.

**Verdict:** *confirmed-still-blocker — every federation gossip path that
carries a `sha256-` prefixed hash is rejected at structural verify. Until this
lands, neither inbound (b) reconciliation nor outbound (c) multi-publisher
gossip will actually move bytes between doorway-A and doorway-B's backends.
This is out-of-scope per Objective constraints but is the critical
sequencer for everything else federation-related.*

## Sequenced backlog (what to land next)

In order of dependency:

1. **(h)** Land the `is_blob_hash_shaped` fix — accept canonical `sha256-<hex>`
   wire format. Until this, federation moves no bytes and (b)/(c)/(f) all
   appear "wired" but produce nothing. **Owner candidate:** existing
   substrate-rea shift or a focused inventory-gossip shift.
   *Operator decision:* none — known fix, known location.

2. **(d)** Operator-side: confirm doorway-B ingress + cert state on the alpha
   cluster — `kubectl get ingress -n elohim-alpha elohim-doorway-alpha-b`,
   `kubectl get certificate -n elohim-alpha elohim-host-apex-tls`. If the
   prod-pipeline parking note hasn't been actioned, `kubectl delete ingress
   -n elohim-prod elohim-doorway-prod` once. *Operator decision:* none — runbook
   already documented in `alpha-b.yaml` L33-37.

3. **(f)** Replace the three hardcoded constants in
   `doorway/doorway-service/src/services/dashboard_topology.rs:10-12` with real
   peer-state reads. Requires inbound capabilities probe + shared_cid set
   intersection. Blocked-by (h) — without gossip working, shared_cid_count
   computation has no input. *Operator decision:* none — fully scoped to
   doorway-service.

4. **(e)** Doorway-agnostic session + recovery binding. Bigger lift — schema
   change to `local_sessions.doorway_url` (single TEXT → JSON array OR
   separate `session_doorways` table), JWT claim schema change, every code
   path that reads `state.args.doorway_url` for routing logic. Coupled with
   the multi-registration substrate question in (a). **Operator decision:**
   *would you rather (4a) keep single-doorway sessions but allow session
   migration across doorways at recovery time (smaller code change), or (4b)
   refactor to multi-doorway sessions from the start (bigger but lasts)?*

5. **(a)** Multi-registration / fallback. Two routes:
   - **(5a)** DNS-layer failover: `signal.elohim.host` resolves to a list of
     IPs; if the configured signal server stops responding, kitsune2 retries
     via DNS round-robin. Minimal substrate change, requires DNS coordination
     across A + B clusters. **Operator decision:** *is the operator willing to
     run signal.elohim.host as a multi-record A/AAAA pointing at both clusters'
     signal services, with health-checked DNS removal?*
   - **(5b)** Substrate-layer dual-registration: elohim-storage maintains a
     parallel peer-discovery channel (e.g. libp2p mDNS within the cluster, or
     iroh/pkarr-based discovery), merging results into the conductor's view.
     Bigger lift, more powerful. **Operator decision:** *is this work
     sequenced with the iroh stack rollout (memory:
     project_iroh_phase11_all_backends_wired) or independent?*

6. **(after 4 + 5)** Update memory `project_multi_doorway_human_registration`
   from "single-primary today, three layers block multi" to whatever the chosen
   multi-registration shape converges on.

## CI dispatch validation (per skill principle 7)

Phase 1's commit will touch:

- `elohim/holochain/Jenkinsfile`
- `genesis/agentic/bin/federation-routing-dryrun.mjs` (new)
- `.claude/memory/project_multi_doorway_human_registration.md`
- `.claude/memory/MEMORY.md` (one-line index update)
- `genesis/docs/superpowers/sprints/2026-05-27-federation-wiring-audit.md` (this file)
- `.claude/shifts/2026-05-27T18-50-federation-wiring-audit.*` (gitignored — won't land)

Expected orchestrator dispatch:
- **elohim-edge** (`elohim/holochain/Jenkinsfile` changed) — yes, definite.
- **elohim-orchestrator** itself — yes (file change drives webhook).
- **genesis** — possible, depends on whether `genesis/docs/**` or
  `genesis/agentic/bin/**` are in the pipeline's `paths.includes` set.
- NOT elohim-app (no app changes).
- NOT elohim-dna-lamad / -mishpat (no zome source changes).
- NOT sophia (no submodule pointer change).
- NOT steward (manual-only trigger).

graph-walker pre-flight will be invoked before push to confirm.

## Stop-condition status

- Did not break existing alpha deploy: Phase 1 change preserves
  `envConfig.bootstrapUrl`/`signalUrl` as the single-doorway fallback, so
  staging + prod (unchanged envs) keep the historical behavior. The selection
  branch only activates when both `doorwayA` and `doorwayB` are declared
  (alpha-only today).
- No finding from Phase 2 makes Phase 1 wrong. The closest miss is (a) — the
  conductor schema is singular — but Phase 1 is *exactly* the
  singular-per-human shape, not multi. The harder multi-registration design
  sits in the sequenced backlog above.
- Wall-clock well under 6h.

## Memories touched

- `project_multi_doorway_human_registration.md` — added current-state section
  with three blocking layers (conductor singular bootstrap_url, doorway
  OutboundOnly stub, session.doorway_url single-pin).
- `MEMORY.md` — one-line index updated to honor today's gap.
