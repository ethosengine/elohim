---
id: "backlog-death-witness-runtime-harvests-a-dying-conductors-last-words"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Death witness — the peer runtime's supervisor harvests a dying conductor's last words (ring-buffered log tail, exit status, resource + DB-pool snapshot, readiness attempts) into a content-addressed report on its own disk, mirrors it to a declared care partner over the p2p plane before it exits, and attests it when a conductor is next available — so a crash that today only a k8s admin can read is visible to the household operator and the recovery partner"
slug: "death-witness-runtime-harvests-a-dying-conductors-last-words"
written: "2026-09-02"
author: "shift 2026-09-02T02-20-land-rung5-batch (operator-directed)"
status: "backlog"
priority: "high"
domain: "D-runtime-operations"
roadmap_rung: "self-healing control plane — observe pillar; k8s-parity crosswalk (kubectl logs --previous / crash dump)"
relatedNodeIds: []
tags: [death-witness, crash-report, process-manager, supervisor, conductor, observability, k8s-parity, care-partner, custody-commitment, attested-private, household-operator, recovery-partner, self-healing]
cites:
  - elohim/elohim-storage/src/conductor/process_manager.rs
  - elohim/elohim-storage/src/reconcile/custody.rs
  - genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md
  - genesis/data/timeline/backlog/alpha-conductor-crash-loop-after-wave4-roll-and-moved-dna-hashes.md
  - genesis/data/timeline/backlog/conductor-websocket-flap-breaks-deploy-write-path.md
---

## Why (operator, 2026-09-02, during the alpha conductor crash-loop)

Seven conductors crash-looped for two hours and the only way to learn WHY was Prometheus +
Loki — instruments a household does not have. The runtime already *supervises* the conductor
(`process_manager` spawns it, pipes its stdout/stderr, parses its DB-pool saturation lines, and
polls its admin socket for readiness) and then throws all of that away when it gives up. If we
are giving k8s-like powers to p2p, `kubectl logs --previous` + `describe` for a dead child is a
runtime capability, not a cluster-admin privilege: visible to the household's elohim operator,
and to a high-trust replication/recovery partner (matthew ↔ adam), so a node that cannot speak
for itself still has a witness.

## P2P Design Gate: Death Witness

### Entity: DeathWitness (the report)
- **Classification**: Attested-Private (B2). The raw report belongs to one node (its own log
  tail carries paths, content ids, peer ids); its EFFECT — "this node's conductor died at T,
  class X, report CID Y, restart N of incident I" — must be verifiable by the care partner and
  the operator. Raw stays private + mirrored; a small attestation is notarized.
- **Head-Plane Cost Budget**: attestations only. Deaths are rare in steady state (tens per node
  per year); a crash-loop would mint hundreds per hour, so the unit of attestation is the
  **incident** (first death opens it, each restart increments it, readiness closes it) — one
  head per incident, never per restart. Bundling shape: **composite root** (incident) over
  per-death reports (private, unheaded). Order of magnitude at 1 yr across a 7-peer fleet:
  <100 heads. No quiesce impact worth pricing.
- **Network Stakes**: all four stages. The report write and the partner mirror are
  **floor-protected** (`CounterEvidence`-class: a node's own failure evidence must reach the
  operator un-filterably, at any stage). The attestation's verification is stage-priceable.
- **Content Address Strategy**: Content-Derived — the report is canonical DAG-CBOR
  (`bafyrei…`); the attestation is Agent-Scoped Composite (node agent key, report CID,
  kind=death-witness). The applicable head is declared by the incident root, never "latest".
- **Transport Affinity**: `auto` — the report bytes ride the existing blob/inventory plane to
  the partner; a dying node must use whichever swarm is up. **This path must not touch the
  conductor**: the conductor is the thing that died. libp2p/iroh only.
- **Source of Truth**: the report bytes on the node's own disk (PVC / data dir) are truth at
  birth; the partner's mirror is a replica; the attestation on the DHT is the public proof,
  written **later** (amber → green) when a conductor is next available. A report with no
  attestation is honest absence, not corruption.
- **Integrity Zome + DNA-hash class**: none new — DNA-hash-NEUTRAL. The attestation rides the
  consolidated attestation content type on the elohim DNA (`content_store_integrity`;
  `attestation:*` content-typed entries), kind `death-witness`. The care-partner binding is an
  attribute of an existing `Mishpat::Commitment` (custody commitment, `reconcile/custody.rs`)
  with kind `witness-mirror` — Linked (A2), no new entry type.
- **Coordinator Zome**: `content_store::create_content` (attestation content) → EntryHash, via
  the existing attestation authoring path; care-partner binding via the existing commitment
  authoring (`salvage_commitment_author` shape) → the commitment's EntryHash + a link.
- **Projections**: SQLite `death_witnesses` (per node: incident_id, report_cid, died_at,
  exit_class, restart_n, attestation_anchor NULL until witnessed — source of truth: local
  operational for the raw report, DHT for the attestation); Automerge sync: no (raw is private;
  the mirror is a blob, not a doc); reach for the attestation: `unresolved — reach vocabulary in
  declared drift` (intent: the household/care-circle tier, never commons).
- **HTTP Route** (declared in elohim-storage `build_manifest()`): `GET /admin/witness/deaths`
  (own, incident-rooted list + `?incident=<cid>` for the report body) and
  `GET /admin/witness/deaths?of=<agent_cid>` on a partner (mirrored). The doorway projects the
  household's list into the operator view; it authors nothing.
- **Anti-Pattern Check**: not modeled in the k8s plane (the k8s pod restart is one *observer*
  of the same event, not its home); no UUID (incident + report are CIDs); no conductor call on
  the death path (the uncancellable-call trap); no per-host authoring of the attestation (one
  node attests its own death, the partner attests *receipt*, both agent-scoped).

### Entity: CarePartnerBinding (who receives my witnesses)
- **Classification**: Linked (A2) — an attribute of an existing custody commitment between two
  agents; kind `witness-mirror`. Consent is explicit (C12): a partner is declared, never
  inferred from replication topology.
- **Address**: Agent-Scoped Composite (self, partner, kind). **Source of truth**: DHT (the
  commitment), projected locally so the supervisor can read the partner list without a
  conductor. DNA-hash-NEUTRAL.

### Concern canon (Step 4)
C0 plane: runtime/footprint seam (supervisor) + bridge to the p2p plane — answered. C3 liveness:
the witness is written and offered within a bounded budget BEFORE the supervisor exits —
answered by construction (bounded ring buffer, bounded mirror attempt, then exit regardless).
C4 honest absence: a death with no report (SIGKILL, OOM, disk full) is recorded by the partner
as a gap on next contact, never as "healthy" — partial until the partner-side gap record lands.
C6a bounded work: ring buffer of N lines + one snapshot; no conductor calls — answered. C8
observability-per-decision: this entity IS the answer for the supervisor's exit decision.
C11 backpressure: the harvest reads pipes the supervisor already owns; it adds no load to a
dying child — answered. C12 consent: partner is a declared commitment — answered. C14
witnessed residual: the incident root carries restart count and the partner's receipt —
answered. C1/C2/C5/C7/C9/C10/C13: n-a (no election, no authority move, no contract change).
Registration: a row in `elohim/elohim-storage/seam-registry.yaml` at implementation.

## What the supervisor captures (composes what exists)

`process_manager` already: spawns with piped stdout/stderr, forwards lines, parses
`Database read connection is saturated. Util N%`, polls the admin socket 60×2 s. Add:
1. a ring buffer of the last N (default 400) child lines per stream, in memory;
2. on child exit OR readiness give-up: exit status/signal, uptime, readiness attempts, the
   last DB-pool saturation samples, `/proc/<pid>` snapshot (rss, threads, fds, nice) taken on
   the last poll before death, the runtime passport (image, happ hashes) — serialised as
   DAG-CBOR, CID-named, written to `<data_dir>/witness/deaths/<incident>/<cid>.cbor`;
3. an offer of the report CID + bytes to each declared care partner over the p2p plane
   (bounded, best-effort, before exit);
4. on the next successful readiness: attest the incident (own death + partner receipts).

This also replaces the "give up after 120 s and exit" behaviour with a witnessed decision: if
the child is still alive at the readiness ceiling, keep waiting and say so in the witness; if
it died, say how. (Companion fix on the crash-loop escalation atom, §1.)

## Done when

A conductor killed on the household mesh (`just mesh recovery cold <peer>` or a forced OOM)
produces a death witness on the peer's disk, the same CID on the declared partner within the
bounded budget, `GET /admin/witness/deaths` lists the incident on both, and the attestation
appears on the DHT after the peer recovers — as an a2o scenario in `features/recovery/`
tagged `@concern:death-witness`, bound to a habit under `elohim/elohim-storage/.epr-meta/`.
