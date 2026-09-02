---
id: "backlog-death-witness-runtime-harvests-a-dying-conductors-last-words"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Death witness — the peer runtime's supervisor harvests a dying conductor's last words (ring-buffered log tail, exit status, resource + DB-pool snapshot, readiness attempts) into an EPR atom with a declared reach, written to its own disk and offered along its custody plane before it exits, attested when a conductor is next available — so a crash that today only a k8s admin can read is readable by exactly whom the node's reach admits: the household operator, its recovery partners, and nobody else"
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

## The worked example — what the witness would have said, and what it cost not to have it

Keep this with the design; it is why the item exists. On 2026-09-02 seven conductors died the
same way and the distance between what the peer runtime KNEW and what anyone could SEE was the
whole incident.

**What the peer runtime had in hand, and threw away.** The supervisor (`process_manager`) spawned
holochain, held both its output pipes, and polled its admin socket. Two point three seconds
after spawn the child wrote `FATAL PANIC … Could not initialize Conductor from configuration:
InternalCellError(CellWithoutGenesis(CellId(DnaHash(uhC0kkLdCTgRoh…), AgentPubKey(uhCAk8Obz…))))`
to stderr and exited 1. The supervisor kept polling for 120 s, logged sixty `Conductor not ready
yet … Connection refused` lines, then `Conductor failed to become ready, attempts: 60`, and
exited — reporting nothing the child had said. The same runtime, on the previous boot, had
itself logged `DNA content drift vs bundle (ALLOW_DNA_REINSTALL=true) — reinstalling`, then
`uninstall_app failed: … DatabaseError(Timeout(Elapsed))` — the exact cause, in its own log, one
boot earlier. Nothing joined the two.

**What it took to see it instead.** From the developer seat, read-only, over two hours:
Prometheus for `kube_pod_status_ready` (0/7), `kube_pod_container_status_restarts_total` (3–14
each, 64 in 2 h after 28 h of none), `kube_pod_container_status_last_terminated_reason`
(`Error`), pod ages and image tags; Loki for the doorway's `DNS resolve … Name or service not
known` per human, storage's `projection-reconcile: OPENED the unresponsive-conductor circuit`,
and the conductor's own tail — and that tail still led to the WRONG mechanism (a load storm),
because the crash line was above the retention window I was reading. The right diagnosis needed
a cluster admin with `kubectl describe` and `kubectl logs --previous` (the panic, the exit
code), a shell on the node (`ls -la databases/authored/` → five files of exactly 139264 bytes,
mtime = first post-restart boot; `dht/` 393–891 MB intact), and cgroup/ulimit reads to rule out
the load hypothesis. A household has none of those. Its elohim operator would have seen a peer
that "won't start" and nothing else, forever.

**What the witness makes true.** The same facts, harvested by the process that already owns
them, at the moment of death, without a conductor: the child's exit status and its last
stderr lines (the panic and the cell id), the supervisor's own preceding decision (the
drift-reinstall and the torn uninstall — the previous incident's tail is the current one's
context), the authored-DB sizes and mtimes it can `stat` itself, the passport (image tags,
bundle vs installed hashes), the readiness attempts. Content-addressed, on disk, offered to the
node's custodians before exit, rendered at `/epr/{cid}` for whoever the reach admits. The
household operator reads: *"conductor died 2.3 s into boot: no genesis for five cells; the
previous boot deleted the source chains during a reinstall that timed out; DHT data intact;
recovery: clear conductor state, rejoin as a new agent"* — the sentence the k8s dev produced
after an hour with root. `kubectl describe` is a capability of the hyperscaler plane
(seam-map crosswalk); this item is its peer-native equivalent, and the self-healing control
plane's observe pillar cannot exist without it.

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
- **Transport Affinity**: `auto` — the report bytes ride the existing blob/inventory + custody
  plane like any other atom the node authors; a dying node must use whichever swarm is up.
  **This path must not touch the conductor**: the conductor is the thing that died. libp2p/iroh
  only.
- **Source of Truth**: the report bytes on the node's own disk (PVC / data dir) are truth at
  birth; every custodian's copy is a replica; the attestation on the DHT is the public proof,
  written **later** (amber → green) when a conductor is next available. A report with no
  attestation is honest absence, not corruption.
- **Who reads it — the reach, not an enumerated pair.** The atom is authored with a declared
  reach at the node's local-relationship / household tier (name `unresolved — reach vocabulary
  in declared drift`; the intent is "the people already standing in relationship to this node",
  never commons). `LocalRelationship` reach is **floor-protected** — it never cheapens or
  filters at any stage — which is exactly the property a node's own failure evidence needs.
  Whoever that reach admits can read it at the universal address `/epr/{cid}` (the atom home
  that landed 2026-09-02) on any peer or doorway that holds a copy; the household operator's
  view is a query (witnesses by subject agent), not a privileged surface. Replication needs no
  new binding: the custodians the node already has (custody commitments in
  `reconcile/custody.rs`) custody its witnesses the way they custody its blobs — a recovery
  partner is simply a custodian inside the reach. An operator who wants a specific always-on
  witness (matthew ↔ adam) declares it as an ordinary custody commitment; nothing in the
  witness itself knows about pairs. (The "two readers" pair was the MVP framing; this is the
  native one.)
- **Integrity Zome + DNA-hash class**: none new — DNA-hash-NEUTRAL. The witness is an ordinary
  EPR content atom (a new `contentType` in the app manifest, e.g. `runtime:death-witness`, with
  its three-leg coupling declared like every other type); the attestation rides the
  consolidated attestation content type on the elohim DNA (`content_store_integrity`;
  `attestation:*` content-typed entries), kind `death-witness`. No binding entity exists:
  custody commitments already name who replicates a node's atoms.
- **Coordinator Zome**: `content_store::create_content` for the witness atom (when a conductor
  is available; before that it is a disk-resident, custody-offered atom in the amber window)
  and for the attestation → EntryHash, via the existing authoring paths.
- **Projections**: SQLite: the ordinary content projection row for the atom (source of truth:
  local operational until anchored, DHT once witnessed — the standard amber/green derivation
  from `dht_anchor_hash`, nothing bespoke) plus an incident index (incident root cid,
  restart_n, exit_class) for the operator query; Automerge sync: no (the atom is
  reach-gated below broadcast); reach: `unresolved — reach vocabulary in declared drift`.
- **HTTP Route**: none new. The witness is reachable at the universal `/epr/{cid}` (reach
  gate applies); the operator's list is the existing content query by type + subject agent,
  declared in elohim-storage `build_manifest()` if a dedicated filter is needed. The doorway
  projects it like any atom; it authors nothing.
- **Anti-Pattern Check**: not modeled in the k8s plane (the pod restart is one *observer* of
  the same event, not its home); no UUID (incident + report are CIDs); no conductor call on the
  death path (the uncancellable-call trap); no per-host authoring of the attestation (one node
  attests its own death, a custodian attests *receipt*, both agent-scoped); no enumerated
  reader list (readers are the reach).

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
3. an offer of the atom (CID + bytes) along the node's existing custody plane — the same
   inventory/blob offer any authored atom gets, bounded and best-effort, before exit;
4. on the next successful readiness: author the atom through the conductor (amber → green)
   and attest the incident (own death + custodians' receipts).

This also replaces the "give up after 120 s and exit" behaviour with a witnessed decision: if
the child is still alive at the readiness ceiling, keep waiting and say so in the witness; if
it died, say how. (Companion fix on the crash-loop escalation atom, §1.)

## Done when

A conductor killed on the household mesh (`just mesh recovery cold <peer>` or a forced OOM)
produces a death witness on the peer's disk, the same CID on every custodian inside its reach
within the bounded budget, `/epr/{cid}` renders it for a household member and refuses a
stranger, the operator's query lists the incident on the peer and on a custodian, and the
attestation appears on the DHT after the peer recovers — as an a2o scenario in
`features/recovery/` tagged `@concern:death-witness`, bound to a habit under
`elohim/elohim-storage/.epr-meta/`.

## Design canonized (2026-09-02) — corrections to the gate above

The witness is the first output of the compute envelope; the sealed design is
`genesis/docs/superpowers/specs/2026-09-02-compute-envelope-tevah-design.md` (§6 the path, §7 the
re-run gate, §16 the review disposition). The grounding and three adversarial reviews overturned
five claims in this atom's own gate; the spec carries the corrected answers:

1. **"a new `contentType` `runtime:death-witness` … DNA-hash-NEUTRAL" is wrong.** `Content::validate`
   refuses any type not in `ALL_CONTENT_TYPES` (integrity zome): a new content type MOVES the DNA
   hash. The witness rides `issue-report` + `metadata_json.kind: death-witness`; the passport rides
   `node-context`. First-class types are batched into the constitutional DNA change.
2. **"custody commitments already name who replicates a node's atoms" is wrong.** They are per exact
   blob hash; a fresh witness has no custodian. The spec adds a standing `custody-spool` commitment
   (coordinator-only action; DNA-hash-neutral, verified) that becomes binding only when a custodian
   outside the node's control counter-signs it (`attestation:custodian-commitment`, its first
   producer) — the capture guard against a steward naming its own second box.
3. **Classification: not B2 — Notarized (A) at INCIDENT grain.** A `Content` entry is a public notary
   record and every pantry file is advertised, so there is no private-chain raw to attest from. The
   incident is one content row whose head moves as deaths are appended; verdicts, tails, snapshots,
   and passports-at-death are bytes inside the incident blob; the custodied copy is a redacted
   summary; the raw stays local behind a per-incident grant.
4. **Attestation deferred.** `attestation:death-witness` is absent from `ATTESTATION_KINDS`;
   `attestation:device-health` has a closed schema already ridden by release attestations. Green =
   the anchored witness row; the incident attestation waits for the constitutional batch.
5. **Done-when named the wrong drill.** `just mesh recovery cold <peer>` kills STORAGE and never a
   conductor; `features/recovery/` does not exist and collides with key recovery. Home:
   `features/resilience/death-witness.feature` (four @wip stations, written 2026-09-02); habit
   `runtime-death-witnessed` (born unwired — nothing is provable until the mesh launches conductors
   under the envelope).

Also: the ring is 200 lines (this atom said 400); the give-up path still carries no tail; the
node-local authoring identity is the transport keypair (amber, `private`, `unbound` until its
`AgentPeerBinding` resolves) with the agent key at green; readers are the custody-spool set, and no
reach tier is canonized while the vocabulary is in declared drift.
