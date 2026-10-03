---
epr-meta-version: 1
id: elohim-storage-governance
purpose: >
  The truth layer: domain services, diesel persistence, and the dual P2P transport (libp2p and
  iroh) where offline-correct, P2P-native state actually lives. This manifest exists to host the
  habit atoms declared here — six of the register's twelve, which is not an accident of placement
  but a measurement: convergence, blob custody, cross-signed attribution, the operator's runtime
  verbs, sync cost, and conductor capacity are all promises this crate keeps or breaks. It
  carries two author-time rules — the scale-risk pointer on the lineage bridge sweep, a measured
  shape (fan-out ∝ peers × records), and the plane rule for chain writes, born from the 2026-10-03
  fleet reading; every other gate that holds this tree
  is structural (the schema contract harness, `just gate elohim-storage`, and the cargo-pool
  CARGO_TARGET_DIR rail), and a rule with no lived drift or measured risk behind it is furniture.
rules:
  - id: scale-risk-lineage-bridge
    class: inject
    when:
      write: "lineage_bridge.rs"
    dedupe-of: genesis/data/timeline/backlog/arch-scale-risk-backlog.md
    retire-when: >
      when row 3 of the scale-risk cluster reads `retired` — a courier election (one courier per
      neighbour) or a compaction of carried v1 facts has landed and the Station 5 receipt carries
      its catch-up minutes.
    why: >
      SCALE RISK ON THIS PATH — row 3 of the scale-risk cluster: the sweep has EVERY crossed peer
      held-carry EVERY neighbour's v1 records (DHT writes ∝ peers × records), one 16-record page
      per 30 s tick, so a 3.5k-record neighbour is ≈110 min per courier and the window stays open
      at least that long. Three peers on node_registry hides it. Prefer changes that elect a
      courier or compact what is carried over changes that only tune the page size, and put the
      catch-up minutes on the receipt. Advisory only.
  - id: chain-speaks-in-the-persons-voice
    class: inject
    when:
      write: "*.rs"
      contains-any: ["record_peer_status", "issue_attestation", "WriterKind::", "write_serialized", "PEER_STATUS_STALENESS_SECS", "authorize_signing_credentials"]
    dedupe-of: genesis/docs/superpowers/specs/2026-10-03-plane-separation-design.md
    retire-when: >
      when the write gate refuses a machine write on a timer by type (voice and trigger declared on
      every chain write), substrate liveness is armed on iroh, and an idle household reads zero
      chain actions per hour on every cell.
    why: >
      A PERSON'S CHAIN SPEAKS IN THE PERSON'S VOICE — plane-separation §1, §5, §6. Storage signs as
      the steward's own agent, so every chain write from this crate is a statement the person makes
      and can never unmake. Before adding or moving one, name its voice (person, delegate, machine)
      and its trigger (act, transition, period close, timer): a machine write on a timer is the
      anti-pattern (`record_peer_status` cost about 50 h of conductor call time per day on alpha).
      Liveness is not a notary fact — a killed peer never writes that it left — so do not read
      freshness from a DHT timestamp; read substrate presence, and change the consumers BEFORE the
      writer (transition-only heartbeats drop every healthy peer out of placement after 900 s). An
      attestation is immutable and has no head: it must never enter head adoption or be re-authored.
      Name your `WriterKind`; `Other` is a gap. Advisory only.
cites:
  - genesis/docs/content/elohim-protocol/architecture/2026-07-12-substrate-trust-contract-runbook.md
---
# elohim-storage — governance package

Habits declared here (`*.habit.md`) describe THIS crate's behaviour; their checks live wherever
the evidence does — a2o scenarios, cargo tests, live fleet probes — because a habit is the
practice and a suite is only ever evidence for it.

The register is projected from these atoms into `genesis/manifests/habits.yaml` by
`.claude/scripts/habits-project.py`. Edit the atom, never the projection.
