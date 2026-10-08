---
epr-habit-version: 1
id: sensor-labels-bounded
invariant: >
  A peer's self-sense never leaves the host carrying a participant: every label on every
  exposition the runtime serves (the conductor's metrics port, storage /metrics, doorway
  /metrics) is drawn from a declared bounded vocabulary, no label value is an agent key, a cell
  id or anything derivable to one person, and on a per-human conductor the exposition itself is
  served to the host and folded by the peer, never scraped off it.
status: red
active: false
checks:
  - "a2o @concern:sensor-labels-bounded (genesis/a2o/features/observation/sensor-labels-bounded.feature — three scenarios, all @wip: no exposition on a household peer carries a participant label; a refused label is counted by key; the fleet's Prometheus holds no per-participant conductor series (@act:ii @requires:observability); default profile: just test mesh features/observation/sensor-labels-bounded.feature)"
  - "epr flow report --bound sensor-participant-labels-ceiling@1 (hard 0 over sensor-label-cardinality@1: distinct label values matching ^uhCAk or a cell id across the three expositions on a peer; reads skipped until folded, never zero)"
refs:
  - "canon: genesis/docs/content/elohim-protocol/observability/epic.md §7 step 1, §8 promise 3, §9 boundary 5"
  - "the leak: elohim/holochain-conductor crates/holochain/src/core/ribosome.rs ~891–905 (agent label on zome-call duration) and crates/holochain/src/core/ribosome/host_fn/emit_signal.rs ~50 (cell_id on emitted signals); the exporter crates/holochain_metrics/src/prometheus.rs renders whatever the instrument pushed"
  - "the scrape that leaves the host: genesis/orchestrator/manifests/infra/alpha-edgenode-podmonitor.yaml (storage :8090 and conductor :9464 every 30 s)"
  - "boundary: genesis/docs/architecture/private-thought-governed-fruit.md §4 boundary 6 (participation is sensitive; the surveillable surface is a design quantity)"
  - "sibling: zome-call-cost-bounded (its ceiling's metric may carry zome and fn, never agent); attention-witnessed-privately (the person plane; this atom is the machine plane)"
guard: >
  Regression risks: (1) hashing the agent label instead of deleting it — a stable hash of a key is
  a pseudonym linkable over time and fails this invariant; (2) stripping at render without
  re-aggregating, which emits duplicate series and leaves the per-agent cardinality alive in the
  SDK aggregator; (3) greening this off the storage exposition alone — it already passes; the
  conductor's is the one that leaks.
retire-when: >
  when the conductor's exporter registers its label vocabulary at compile time (a KeyValue with
  an undeclared key does not build) and the peer is the only reader of its own conductor's
  exposition, so nothing is left for a scrape test to catch.
---
DELTA 2026-10-08 (BORN red — the invariant is measured NOT held): read from the fork at pin c916eddbe, `ribosome.rs` pushes `agent` (the source-chain agent pubkey) onto every zome-call duration measurement when a workspace exists, and `emit_signal.rs` labels every emitted signal with `cell_id`; the fleet scrapes :9464 every 30 s, so today a cluster Prometheus holds a per-participant activity ledger for every hosted human on alpha. Storage's own /metrics carries no participant label (closed vocabularies, read from src/metrics.rs). Cargo tests the first code pass adds: `cargo test -p holochain --lib metrics::label_vocabulary` (the KeyValue set at the two sites equals the declared vocabulary; fails on any key named agent or cell_id and on any value matching ^uhCAk) and `cargo test --lib metrics::label_vocabulary` in elohim-storage and doorway-service. FIRST MOVE: delete the `agent` push, replace `cell_id` with `dna_hash`, add the render denylist test, change the documented listen example to loopback. Planned in the Observability epic §7 step 1; declared here where the fork's habits live. NO status change until the fork test is green at the pin and a household scrape reads 0 twice.
