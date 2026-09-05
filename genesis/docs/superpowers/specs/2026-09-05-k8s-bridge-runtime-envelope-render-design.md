---
title: "k8s bridge — the runtime manifest declares the compute envelope; deployments.json becomes its lockfile-render; capacity is observed, not hand-promoted"
id: k8s-bridge-runtime-envelope-render
status: Draft
class: protocol-canonical
context-tier: disclosed
steward: rust-architect
graduation-trigger: the alpha fleet's per-human k8s requests/limits are byte-identical to the k8s-bridge render of a content-addressed RuntimeManifest whose CID deployments.json pins; the pre-push gate refuses drift between the pin and the render; the Rakia ledger's cluster envelope is produced by an observation command, and the aggregate-capacity ask has not fired on a stale envelope since.
created: 2026-09-05
domain: seam atlas 3.3 runtime/footprint (declaration) × 3.6 bridge (render outward) × 3.15 resource governance (aggregate policy, ratification)
serves: dev-system-equilibrium
informed-by:
  - genesis/docs/superpowers/specs/2026-09-02-compute-envelope-tevah-design.md (§3 declaration, §5.2 quota, §7 gate, §11 sequencing, register 21 the epr-pvc guide-star)
  - genesis/data/timeline/backlog/2026-08-29-compute-envelope-virtual-peer-contract.md (the six-field envelope contract)
  - genesis/orchestrator/data/deployments.json ($computeEnvelopeRatification — the prose no code reads)
  - genesis/data/rakia/compute-capacity.json (promoted 2026-09-05 from Prometheus by hand after four months stale)
cites:
  - "compute-envelope-tevah | Tevah | sha256:ac9364d4b024290f | path: genesis/docs/superpowers/specs/2026-09-02-compute-envelope-tevah-design.md"
  - genesis/data/timeline/backlog/2026-08-29-compute-envelope-virtual-peer-contract.md
  - "elohim-seam-map-concern-routing | The Elohim Seam Map | sha256:fd5ced9f996ff5af | path: genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md"
  - bridges/CLAUDE.md
  - elohim/ark/core/src/manifest.rs
  - elohim/epr-rea/src/model.rs
  - scripts/ci/conductor-split-budget.sh
  - genesis/data/devices/archetype-resource-budgets.json
  - .claude/scripts/_lib/epr_meta.py
  - genesis/orchestrator/scripts/snapshot-capacity.sh
---

# k8s bridge — runtime envelope render

## 0. Why now (the 2026-09-05 integration push)

A 331-commit push to `dev` was refused by the `.epr-meta` compose-gate's one ask-class verdict:
`test-bench-aggregate-capacity` on `deployments.json`, limits.cpu_m=53750 > allocatable=46000.
The 46000 came from a capacity ledger promoted 2026-05-04 with shem cordoned; the live cluster
had 70 cores (shem back since the PSU fan was kept running). The ratification the gate asked for
already existed as prose in the same file (`$computeEnvelopeRatification`, 2026-07-16) that no
validator reads. The only accepted answer was a push-time environment variable, which the
auto-mode classifier refuses. A peer's compute envelope has three homes that cannot read each
other; this spec collapses them into one declaration, one render, and one observation.

## 1. What the grounding found (three readers, 2026-09-05)

**WIRED.** `RuntimeManifest {schema, kind, supersedes, reach, processes}` is content-addressed:
DAG-CBOR canonical bytes → `elohim_epr::cid::compute_cid` → `bafyrei…`, order/whitespace-
insensitive, three tests pin it, and the three household-mesh berths share one manifest CID
(`bafyreihagg75k…`) — "peers on one declaration share one CID" is measured. `validate()` is the
refusal seam (five rules today). `ark manifest cid` prints the CID. The death witness already
carries `bounded_by` and `pain: AlgedonicEvidence::Breach{stock, limit, bound_ref}`, round-tripped.
`ProcessSample` already measures rss/cpu/fds/io. `epr-rea` already owns the graded-bound ontology:
`Bound{limit, unit, threshold_pct, sense, source}`, `LimitSource::Folded{rule}`,
`Composition::Sum.fold()`. The `delegates-compute` Mishpat commitment's `bounds` is
`additionalProperties: true` at every layer (schema, integrity key-probe, storage `serde_json::Value`,
view) — extra quota keys validate and persist today, DNA-hash-neutral, and nothing reads them.
`ram-guard` is the six-field contract in Python on the dev box.

**SPEC-ONLY.** `envelope`, `ResourceQuota`, `ChildSpec.quota`, the Σ rule, `Berth.effective`,
any cgroup write, `RuntimePassport` resource fields (it has none), manifest DHT anchoring
(no `create_content` call site), `elohim/ark/seam-registry.yaml` and `.epr-meta` (required at
birth by tevah §7/§8, absent).

**The k8s side.** `deployments.json` `edgenode{Memory,Cpu}{Request,Limit}` are declared THE
source of truth per human; `elohim/holochain/Jenkinsfile` splits them 5/8:3/8 memory and 1/2:1/2
cpu (remainder to storage) via `scripts/ci/conductor-split-budget.sh` into eight sed placeholders;
adam's resources are hard-coded in his explicit manifests; `archetype-resource-budgets.json` is
the floor per `deviceArchetype`; `validate-deployments.ts` (TS) enforces floor + sum-neutrality,
`epr_meta.py` and `epr-cli/src/repository_validators.rs` (native) enforce archetype alignment and
the aggregate ceiling. Twenty-odd consumers read `deployments.json` (name/suspended/nodeTypes/
runtimeConfig/humanId…); `scope-reconcile.py` depends on its TEXT layout. `snapshot-capacity.sh`
is kubectl-only, writes a raw snapshot that is not the ledger schema, and never updates the ledger.

**Contradiction to resolve, not paper over.** Tevah §7 lists "modeled in the k8s plane" as an
anti-pattern and §11 says "k8s becomes one packaging of the envelope" — but today the arrow runs
k8s → runtime, and making deployments.json a render is a NEW decision. This spec is that decision
(tevah register 30, below).

## 2. The decision

1. **The manifest declares the envelope.** `RuntimeManifest.envelope: RuntimeEnvelope {bound:
   ResourceQuota{memory_bytes, cpu_millis, pids, disk_bytes}, headroom_bytes, measure: Committed,
   protected, shed_order, graded{soft,high,hard}}` and `ChildSpec.quota: ProcessQuota{memory_max_bytes,
   cpu_share_millis, oom_group, oom_score_adj}`. All additive, `serde(default)` + skip-if-none, so
   a manifest without an envelope keeps today's CID (the S0 CID test is the discipline).
   `validate()` gains the tevah §5.2 refusal: Σ child `memory_max_bytes` + `headroom_bytes` ≤
   `bound.memory_bytes`, naming both totals. The projection to `epr-rea` is
   `RuntimeEnvelope::memory_bound(threshold_pct) -> Bound{source: Folded{Sum}}` — inherit, never
   mint a second bound type. The passport's `effective_tier` stays `None`: declared is not enforced,
   and the manifest field must never imply enforcement (tevah §4 honesty).

2. **deployments.json becomes a lockfile-render, not a generated file.** Each active human gains
   `runtimeManifest: {cid, path}` pinning a manifest under
   `genesis/orchestrator/manifests/runtime/<name>.manifest.json` (one per archetype budget —
   `family-node-base`, `recycled-laptop`, `chromebook-edu`, `home-nuc`, `raspberry-pi-4` — plus a
   superseding manifest for any human with a justified `resourceOverride`, e.g. adam, whose
   manifest `supersedes` the archetype one). The four `edgenode*` fields stay in the file, byte-
   identical for every existing consumer, but they are now DERIVED: `k8s-bridge render` computes
   them from the pinned manifest (`bound.memory_bytes`/`cpu_millis` → k8s quantities, the split
   arithmetic reproduced exactly from `conductor-split-budget.sh` and pinned by a golden test
   against the script's own output), and `k8s-bridge verify` refuses drift between pin, file
   bytes, and rendered fields — the berth's exit-65/66 discipline lifted to the fleet. The pin is
   the **declared head** at the k8s packaging (tevah §7: "which manifest applies is a declared head,
   never recency"); DHT anchoring of the manifest stays tevah S1 station 4, out of this slice.
   The rendered Deployment carries `elohim.protocol/runtime-manifest: <cid>` as an annotation.

3. **Capacity is observed by a command and promoted by running it.** `k8s-bridge observe`
   reads Prometheus (kube-state-metrics `kube_node_status_allocatable`, `kube_node_status_condition`)
   and writes the ledger's `cluster.{totalAllocatable, readyNodeCount, notReadyNodes, nodeTypes[].
   nodes[].{ready,allocatable}, totals}` plus `snapshotTimestamp`/`snapshotMethod`, leaving the
   hand-curated commitments/actuals untouched and marked with their own snapshot date. The
   validator gains a freshness leg: a ledger older than 30 days downgrades the aggregate verdict
   from `ask` to `refer` naming `k8s-bridge observe`, so a stale envelope can never again block a
   push by asking a question the operator answered in May. The Rakia ledger is not retired here —
   retirement needs the runtime to REPORT allocatable (a `HostPassport` resource block, tevah S3+),
   which is the next rung and is named in §6.

4. **Ratification becomes a typed field both validators read.** `compute-capacity.json` gains
   `cluster.ratifications: [{policy: "test-bench-aggregate-capacity", dimension: "limits.cpu_m",
   overcommit_pct, ratifiedBy, ratifiedOn, reason}]`; the aggregate check applies the ratified
   overcommit to that dimension only (requests never ratify — they reserve; memory never ratifies —
   it is incompressible). The prose `$computeEnvelopeRatification` comment is retired into that
   record. This mirrors the on-chain precedent `bounds.reach_elevation_acknowledged` on
   `delegates-compute`: an exception is acknowledged in the signed payload, never in a comment or
   a push-time flag. The protocol-native form — the same acknowledgement as a key on a
   `delegates-compute` commitment's `bounds` — is the seam this record graduates into (§6).

## 3. The bridge crate — `bridges/k8s` (`k8s-bridge`)

Bridge seam (atlas 3.6): a library crate that translates the runtime manifest OUTWARD into k8s
resource shapes, consumed by the orchestrator's render path through a thin CLI (`k8s-bridge
render|verify|observe`) invoked from `scripts/ci/` — IaC-shaped traffic, so the consumer is the
orchestrator, not `elohim-storage` or `doorway-service`. Birth rule: `bridges/k8s/Cargo.toml`
workspace, `seam-registry.yaml`, `.epr-meta`, and the S3.6 row's `registry_crates` in
`.claude/epr-meta/seam-catalog.yaml` (empty today — `seam_matrix_test.py` counts an unrouted
registry as zero cells). `elohim/ark/` gets its own `seam-registry.yaml` in the same slice.

**Guide-star check (tevah register 21).** `epr-pvc` — the network of peers offering external
actors a collective agreement for persistent volumes backed by this runtime — stays unminted.
This crate renders resources only; `Berth.data_root` remains a reserved volume head with no
semantics; `STORAGE_CLASS_PLACEHOLDER` stays the operator's fixed `openebs-hostpath`. Nothing
here precludes a later `k8s-bridge` arm that turns a volume head into a PVC agreement; the
crate name is chosen so that arm lands in the same seam rather than a second bridge.

Decision points registered at birth (seam-registry rows, concern canon answered per row):
- `render_envelope` (verdict-fn): manifest + split rule → eight k8s quantities. C0 plane: bridge,
  pure. C4 honest absence: a manifest without envelope renders NOTHING (pass-through, today's
  behaviour) rather than zeros. C6a bounded / C6b idempotent: pure function, golden-tested. C8:
  every render prints the manifest CID it rendered from. C10 contract evolution: the `schema`
  field; a manifest with an unknown schema is refused, never guessed.
- `drift_verdict` (boundary-answer-type): `{Fresh, PinMismatch{expected, actual}, RenderDrift
  {field, pinned, rendered}, Unpinned{human}}`. C5 evidence-not-authority: names bytes, does not
  rewrite them. C13 graduated authority: `Unpinned` is `refer` while humans migrate, `refuse` once
  every active human is pinned (a dated flip in `.epr-meta`, never a silent default).
- `capacity_freshness` (pure-decision-predicate): ledger age vs 30d → `ask`/`refer` class.

## 4. P2P design gate

### Entity: RuntimeManifest (with envelope)
- Classification: Notarized (A) — tevah §7's answer stands (rides `Content` with
  `metadata_json.kind = runtime-manifest`, DNA-hash-neutral). THIS SLICE DOES NOT ANCHOR IT: it
  stays a content-addressed file whose CID is pinned; anchoring is tevah S1 station 4.
- Head-plane cost: one declared head per manifest lineage — five archetype manifests plus a
  handful of overrides per fleet; tens, not thousands.
- Network stakes: Constitutional (manifests) — floor-protected; never cheapens at Simulacra.
- Address: Content-derived CID (dag-cbor `bafyrei…`), minted by `RuntimeManifest::cid()`; the
  applicable head is DECLARED by the deployments.json pin (lockfile), never by recency.
- Source of truth: the manifest bytes (DHT once anchored). SQLite/Automerge: none in this slice.
- Coordinator/route: none in this slice (the existing `/epr/{cid}` serves it once anchored).
- Anti-patterns checked: "modeled in the k8s plane" — inverted on purpose (k8s is the render);
  no UUID; no second address (the pin carries the CID, the path is a convenience).

### Entity: the quota declaration on chain
- Classification: Linked (A2) — extra keys on an existing `delegates-compute` Commitment's
  `bounds`; zero new entry types; coordinator-only validation. Not written by this slice; the
  manifest's `envelope.bound` is the value those keys will carry (§6).

### Entity: capacity observation (the Rakia ledger's cluster block)
- Classification: Ephemeral (C) — a projection of the k8s plane (hardware), reconstructable from
  Prometheus by `k8s-bridge observe`; provenance recorded in `snapshotMethod`. k8s is not the
  architecture: this is an observation of compute, not a protocol entity. Source of truth: the
  cluster, until the runtime reports it (§6).

### Entity: ratification record
- Classification: Ephemeral (C) today (repo governance, operator-signed via git, read by both
  validators); graduates to Linked (A2) as `bounds.overcommit_acknowledged` on the commitment.

### Entity: RenderedEnvelope / DriftVerdict
- Classification: Ephemeral (C) — pure functions of manifest + rule; never persisted.

Back-fill detector: no route is added; no coordinator function is added; the 1-year item count
is tens of manifests.

## 5. What was rejected

- Generating deployments.json wholesale from manifests — breaks `scope-reconcile.py`'s text-
  structure dependency and twenty parsed-field consumers for no gain; the lockfile-render keeps
  every consumer byte-identical and still makes the fields derived.
- Enforcing quota (cgroup writes) in this slice — tevah §11 orders quota at S3 behind S1/S2, and
  alpha's container cannot delegate `cgroup.subtree_control`; a declaration-only slice is
  compatible with the ordering, an enforcing one is not.
- Retiring the ledger outright — the runtime has no resource observation to replace it with yet.
- A push-time flag as the ratification path — it is exactly the mechanism the classifier refuses
  and the operator cannot audit later; a typed record is reviewable in the diff.
- Reading the prose comment with a regex — prose is not a record.

## 6. Next rungs (named, not taken)

1. `HostPassport` gains an allocatable/committed block reported by the runtime; `k8s-bridge
   observe` then folds passports instead of Prometheus; the ledger's producer flips (tevah S3+).
2. The manifest is anchored (tevah S1 station 4) and the pin becomes a declared head on the DHT.
3. The quota declaration rides `delegates-compute` bounds (`memory_bytes`, `cpu_millis`,
   `overcommit_acknowledged`) and `bounds_validator` learns the keys.
4. The `epr-pvc` arm: `Berth.data_root` as a volume head → PVC agreement (the guide-star).

## 7. Evidence and gates

- `just gate elohim-ark` (ark-core tests incl. the S0-CID-unmoved test and the Σ refusal).
- `just gate k8s-bridge` (new manifest project): golden test vs `conductor-split-budget.sh`
  output for every archetype budget; `verify` green on the current deployments.json once pinned.
- `.claude/scripts/_lib/__tests__/seam_matrix_test.py` green with `k8s-bridge` routed to S3.6.
- Pre-push: `k8s-bridge verify` wired beside `validate-deployments.ts`; the aggregate policy reads
  the ratification record and the freshness leg; `EPR_META_ACK=1` is no longer part of any
  documented push procedure for this concern.
- Tevah spec register entry 30 recorded (this decision), guide-star check answered.

## 8. Risks

- The split arithmetic lives in bash and is reproduced in Rust: the golden test IS the contract;
  if they ever diverge, the script is the authority until the Jenkinsfile calls the bridge.
- Two validators (Python `epr_meta.py`, native `repository_validators.rs`) must read the
  ratification record identically — one fixture, two harnesses.
- A stale Prometheus (Loki/Prometheus 502s read as untrustworthy zeros — probe rails memory)
  must not promote zeros: `observe` refuses to write when any node's allocatable is missing.
