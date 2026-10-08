---
epr-habit-version: 1
id: liveness-has-one-source
invariant: >
  On a peer, "is this peer here now" has one writer and one cadence — substrate presence,
  armed on every transport with a short TTL — and every consumer (shard placement, the
  resilience card, network posture, the doorway roster) reads it; posture (lifecycle, pool
  flags, archetype) is a standing declaration written on change; a killed peer never has to
  write that it left; unarmed presence reads unmeasured, never alive.
status: red
active: false
checks:
  - "a2o @concern:liveness-has-one-source (genesis/a2o/features/observation/liveness-has-one-source.feature — a killed peer leaves placement within 35 s with zero chain actions about liveness (plane-separation §8 row §5.5 verbatim); unarmed presence reads unmeasured; all @wip; default profile: just test mesh features/observation/liveness-has-one-source.feature)"
  - "epr flow report --bound liveness-sources-ceiling@1 (hard 1 over liveness-sources@1: distinct liveness writers that produced a fact about one peer in a 10-minute household rest; reads skipped until folded)"
refs:
  - "canon: genesis/docs/content/elohim-protocol/observability/epic.md §3 (posture vs liveness), §7 step 5, §11 (five writers at four cadences)"
  - "ruling: genesis/docs/superpowers/specs/2026-10-03-plane-separation-design.md §5 ruling 5 — order is binding: arm presence on iroh, define the unarmed fallback, move the readers, only then change the writer"
  - "the five writers today: elohim/elohim-storage/src/heartbeat.rs (60 s → infrastructure.record_peer_status, a DHT PeerStatus, stale at 120/900 s); src/services/peer_liveness.rs (35 s in-memory, armed only from the libp2p path at src/p2p/mod.rs ~3821); doorway ProbeRoster (attestation per ~5 min); infrastructure:doorway-heartbeat (declared, no producer); doorway self_uptime (hourly strip in Mongo)"
  - "the absence bug this atom also owns: elohim/elohim-storage/src/heartbeat.rs ~368–372 defaults free storage to 100% when the probe fails (absence read as health)"
  - "sibling: dataplane-convergence (active) owns placement; the reader move in step 3 of the consolidation order is sequenced inside its work"
retire-when: >
  when the transport layer is the only thing that can say a peer is present and no code path
  can write a liveness fact to a chain — the habit then describes the transport.
---
DELTA 2026-10-08 (BORN red): liveness is derived five ways at four cadences and notarized twice; an idle household writes chain actions about peers that did not change. Cargo test the first code pass adds: `cargo test --lib services::peer_liveness -- armed_on_iroh_and_unarmed_reads_unmeasured`. FIRST MOVE: arm presence on iroh and define the unarmed fallback (ruling 5 step one), with the readers moved before the writer. Planned in the Observability epic §7 step 5. NO status change.
