# bridges/valueflows — Local Guidance

The hREA / VF-GraphQL bridge for the Elohim Protocol. Consumed by
`elohim-storage`; mounted at `/api/v1/vf-graphql`.

## Workspace structure

- `valueflows-types/` — stable type definitions (TranslationPoint, enums).
  Standalone crate so analysis tooling can depend on the ledger schema
  without pulling async-graphql + hyper.
- `valueflows-bridge/` — library; GraphQL schema + handler entry point.
- `valueflows-tests/` — integration tests (schema-level + HTTP-level).

## Current state (M1)

- `/api/v1/vf-graphql` mounted on elohim-storage.
- `EconomicEvent` query returns deterministic fixture data with VF wire
  field names (`provider`, `receiver` — not `providerId`).
- Every resolve writes a `TranslationPoint` to the
  `translation_observations` table via `tokio::task::spawn_blocking`
  (avoids blocking the async runtime on r2d2 pool acquisition).
- The bridge uses `as_ledger_str()` on each enum (defined in
  `valueflows-types`) for stable string values that match the
  CHECK constraints in the migration — Debug format is never used
  for SQL writes.
- hREA DNA role added to happ manifest with `deferred: true` (cells
  provision lazily in M2; binary fetched out-of-band per
  `elohim/holochain/dna/hrea/workdir/README.md`).
- No mutations yet. No identity bridge. No authority gate. No real
  hREA reads or writes.

## Reference docs

- Spec: `genesis/docs/superpowers/specs/2026-05-20-wave3-valueflows-hrea-interop-design.md`
- M1 plan: `genesis/docs/superpowers/plans/2026-05-20-wave3-m1-valueflows-substrate-readiness-plan.md`

## Build / test

```bash
cd bridges/valueflows
cargo check --all
cargo test --all
```

For elohim-storage integration, use the storage workspace's build:

```bash
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' \
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
cargo build --features graph-native --lib
```

## Sequencing

- M1 (this) — substrate, fixture EconomicEvent.
- M2 — identity bridge: VfBinding entry, handshake, per-Human hREA cells,
  `elohim/qahal-authority` crate.
- M3 — authority gate + write path for Proposal+Intent.
- M4 — remaining VF types.
- M5 — learning ledger reports.
- M6 (optional) — Apollo Federation.
