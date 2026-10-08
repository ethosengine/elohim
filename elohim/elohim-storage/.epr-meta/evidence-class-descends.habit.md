---
epr-habit-version: 1
id: evidence-class-descends
invariant: >
  Every observation row, attestation and summary event carries evidence_class ∈ {fixture,
  real}; the class only descends through graduation — a fixture observation graduates into a
  fixture attestation and never into a summary EconomicEvent that settles; a row with no class
  is refused at write, never defaulted to real.
status: unwired
active: false
first_move: >
  Add evidence_class to elohim/sdk/schemas/v1/manifest/observation-kind.schema.json's payload
  rules and to elohim/sdk/schemas/v1/attestation/attestation-metadata.schema.json, then write
  `cargo test --lib graduation::evaluator -- fixture_observations_never_settle` red; only then
  declare checks (that test; pnpm run schema:test; a2o @concern:evidence-class-descends in
  genesis/a2o/features/observation/evidence-class-descends.feature, already scaffolded @wip).
refs:
  - "canon: genesis/docs/content/elohim-protocol/observability/epic.md §10 (keep fixtures out of settlement), the one hard rule of the Prometheus survey"
  - "the rows: genesis/data/timeline/backlog/measure-family-borrows-backlog.md row 28 (the class) and row 29 (domain identity, which rides the same evaluator change but is dedupe, not class); genesis/data/timeline/backlog/arch-authority-in-integrity-backlog.md row 15 (the integrity floor rides the next [dna:migrate])"
  - "today: fixture-ness is known only network-wide (crates/seam-contracts/src/freshness.rs NetworkStage) and in a2o lane tags; no row says it came from a seeder"
retire-when: >
  when the integrity zome refuses an unclassed or class-raising attestation (row 15 landed),
  leaving the storage-side refusal a shim.
---
DELTA 2026-10-08 (BORN unwired — no column, no schema field; a test today fails to compile, which is not a failing check, so the census admits no checks): the only legal first move is the schema field, after which this atom is written red with its three checks. Planned in the Observability epic §10 and §12 rung 2. NO status change.
