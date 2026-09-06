---
epr-habit-version: 1
id: nachalah-allotment
invariant: >
  Verified membership, consent, and witnessed assignments determine scoped holding;
  every assigned authority fully validates. Changes preserve sector-by-sector recovery
  floors through live writes and failures. Holding grants neither readability nor
  canonical-version authority. Runtime support arrives through the existing elected
  release and lineage ceremonies before actuation, with verified local rollback.
status: unwired
active: false
checks: []
first_move: >
  Connect the existing release adopter to durable supervised activation through ark,
  preserving the previous executable, runtime manifest, database compatibility, berth,
  keys, and credentials; prove readiness, identity, failed-adoption rollback, and outcome
  attestation before enabling any earned-arc actuation.
refs:
  - "genesis/docs/superpowers/plans/2026-09-05-nachalah-supervised-activation-sprint-handoff.md — next sprint: conductor adoption, local rollback, and repeated household ceremony proofs"
  - "genesis/a2o/features/delivery/nachalah-allotment.feature — 20 scenarios/outlines, all @wip; no holding or adoption check is runnable yet"
  - "genesis/docs/superpowers/specs/2026-09-05-nachalah-allotment-epic-design.md"
  - "genesis/docs/superpowers/specs/2026-09-01-runtime-artifacts-elected-content-design.md"
retire-when: >
  when scoped admission, assignment transitions, and release adoption enforce these
  invariants by construction and their constituent reach, recovery, and upgrade habits
  cover every failure boundary without a separate cross-seam rehearsal.
---
DELTA 2026-09-05: ark's executable-identity readiness rung and witnessed readiness-failure path pass `just gate elohim-ark` (120 tests); the mesh launcher declares the rung. Full supervised activation, scoped admission/migration, fork assignments, performance comparison, and alpha delivery remain unimplemented and unmeasured; this habit stays UNWIRED.
DELTA 2026-09-05 (handoff): authored the supervised-activation sprint handoff with the uncommitted base warning, five delivery stations, existing code homes, and repeated household adoption/rollback exit proofs; no additional runtime or holding result is claimed.
DELTA 2026-09-06 (authority audit): source audit at 8c32aa11e identifies missing authenticated identity bindings and revocation/stop ordering inside #1b-b; three refusal scenarios/outlines added to the existing acceptance feature, all @wip. Gherkin parser passed for all 196 feature files (EXIT=0); no scenario executed, process changed, activation permission issued, or habit status changed. Details and probes: supervised-activation sprint handoff, Authority/status audit.
DELTA 2026-09-06 (#1b-b verifier landed + reviewed): `services/commitment_record.rs` authenticates the exact signed grant record — author pin, recomputed ActionHash and EntryHash against both the pin and the record's cached hashes, Ed25519 over the encoded Action — with real negative tests (same entry bytes under a different author, each pin mismatch, mutated action/entry/signature, wrong zome/index/visibility, Update/Delete, malformed and oversized wire); mishpat coordinator gains one readback extern (integrity zome and dna.yaml untouched — DNA hash unmoved, hot-swap class). Independent Opus review: changes-requested on one MEDIUM (the verifier was `pub` and DNA-unbound; now `pub(crate)` with the mishpat-cell precondition documented) plus nits, all applied this session; gate re-run is the approval. Orchestrator RULING recorded in the handoff: revocation dominates — permits are claims against the grant, no outstanding-lease semantics, fresh status is the last check before the first destructive action, and the revoke-after-status race is closed by witnessed rollback, not a permit; the concealed-revocation scenario is reshaped to end at rollback + attestation. Still unwired: identity bindings (routed to identity-cross-signed), activation, rollback, both household ceremonies; earned-arc actuation disabled.
