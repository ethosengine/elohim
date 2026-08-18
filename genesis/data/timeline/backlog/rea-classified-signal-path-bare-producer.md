# `rea_commitments.resource_classified_as` — the DHT signal/reconcile path is a 2nd bare-scalar producer (Option A not yet table-wide)

status: open
discovered: 2026-06-19 (Sprint 1 rust-architect, executing resilience-card-lighting-plan)
domain: D-substrate-distribution
relates: `genesis/docs/superpowers/specs/2026-06-13-non-commons-provide-commitments-design.md` §11 (Option A: uniform JSON-list) · `genesis/docs/superpowers/plans/2026-06-19-resilience-card-lighting-plan.md`

## Finding
Sprint 1 converged the **provide side-projection** writer (`record_provide_from_content_commitment`)
to the JSON-list form, and routed every READER through the typed accessor (so card-lighting is
unaffected — the accessor tolerates bare in-flight). But the census surfaced a producer Sprint 1
left **out of scope**:

- **The DHT signal/reconcile path** — `handle_rea_signal` → `rea_projection.rs:462
  project_commitment_from_wire` → `upsert_with_anchor`/`create_commitment` — writes the column via
  `first_or_none(parse_json_strings(...))`, i.e. it **reduces the wire list to a bare first
  element** for every signal-projected commitment. So runtime-authored commitments still land
  bare-scalar; "`resource_classified_as` is uniformly a JSON list" is **not yet true table-wide**.
- **Latent operate-doorway truncation (pre-existing):** the same `first_or_none` lossily drops all
  but the first capability of a multi-valued `operate-doorway` list on the signal path. Not
  introduced by Sprint 1 — surfaced by its census. Worth confirming whether any operate-doorway
  commitment actually flows through this path (vs HTTP POST) before judging severity.

## Why deferred (correct scope call)
Converging this producer touches the reconcile path + `rea_commitment_service` tests, a wider blast
radius than the card-lighting keystone. The Sprint-1 safe order (all readers accessor-routed FIRST)
means card-lighting does **not** depend on it — the accessor reads bare and list identically. So this
is genuine follow-up coherence, not a card blocker.

## Fix shape (when picked up)
Change `project_commitment_from_wire` to persist the full list (`serde_json::to_string` of the parsed
`Vec`), not `first_or_none`. TDD: a signal-projected commitment round-trips its full classification
list; an operate-doorway signal preserves all capabilities. Then the column is uniformly a list at
every producer and the accessor's bare-tolerance becomes purely defensive (in-flight legacy only).

## Live confirmation + new producer/symptom evidence (2026-08-18, ch07 rotation session)

Mesh-observed on the local 3-peer mesh while validating `services/custody_rotation.rs`
(rotation-authored custody-blob successor, conductor path via `ReaCommitmentService::create`):

- **Third encoding shape live**: the rotation author passes a bare scalar into the db-layer
  input; `to_shefa_input` wraps it (`vec![s]`) so the ZOME stores the correct JSON list — but
  the eager projection (`create_via_conductor`, Gap-F arm) applies the same
  `first_or_none(parse_json_strings(...))` reduction as the signal arm, landing the row
  bare-scalar. Meanwhile the HTTP-create path double-encodes (input view JSON-encodes the
  Vec, `to_shefa_input` wraps the encoded string again) and survives only because
  `first_or_none` unwraps one layer. Three shapes in the column, all from live producers.
- **Symptoms**: `ReaCommitmentView` renders a bare-scalar row as `resourceClassifiedAs: null`
  (fold classification unaffected — `classifications_of` is bare-tolerant, and the ch07
  stocked gauge lit correctly); the resilience card's commitments join reads
  `activePeers: 0` for rotation-authored pledges.
- Fix shape unchanged (persist the full list at every producer, incl. BOTH
  `create_via_conductor`'s eager arm and the signal arm); add the double-encoding HTTP path
  to the TDD list.
