---
title: "Ontology Keel Slice 1 — the verdict spine, the epistemic axis, and peer review as value flow"
id: ontology-keel-slice1-verdict-spine-plan
status: Ready
class: protocol-canonical
created: 2026-07-23
domain: D2
topic: [ontology, verdict, ceiling-marker, epistemic, peer-review, rea, keel]
cites:
  - reach-ontology-vocabulary-split-spec | Reach Ontology/Vocabulary Split | sha256:2f5835c40fb02a81 | path: genesis/docs/superpowers/specs/2026-07-22-reach-ontology-vocabulary-split-spec.md
  - genesis/research/owl2-graduation-floor-ceiling-ontology-2026-07-23.md
  - genesis/research/letter-to-rea-practitioners-observed-presence-2026-07-22.md
  - elohim/epr/src/reach.rs
  - elohim/epr/src/witness.rs
  - elohim/epr-rea/src/fold.rs
---

# Ontology Keel Slice 1 — verdict spine + epistemic axis

> **The founding decision (the keel).** Epistemic standing — how a claim moves from emergent truth through peer review to settled canon — is a **derived verdict over witnessed REA events, never a stored status column**. Same two-layer law as reach (spec §2), same ceiling marker (§2a), same narrowing composition. One spine, many gradients: reach (built), epistemic (this slice), compute/trust variance and retention (downstream, same shape). The floor is a deterministic fold any household node can run; **canon can only be conferred by a referenced governance act, never by a threshold alone**; contest routes to `Refer`, never to silence.

## P2P Design Gate: Ontology Keel Slice 1

### Entity: Decision / Verdict / Witness (wire + computation types)
- **Classification**: Operational (C) — *not persisted at all in this slice*. Evaluations are ephemeral and reconstructable by re-evaluation (spec: Category-C projections). A human-resolved `Refer` becomes an attested **decision** — Category A via the content-store attestation `content_type` — **deferred** to a later slice.
- **Content Address Strategy**: n/a (unpersisted); `Verdict.subject` references content by CID.
- **Source of Truth**: recomputation over notarized/witnessed inputs.
- **Coordinator Zome / HTTP Route**: none in this slice.
- **Anti-Pattern Check**: derived-signal-never-written honored (a verdict is never stored as truth); no UUID identity; no route-first design.

### Entity: EpistemicStanding (fold output)
- **Classification**: Operational (C) — in-memory fold result in `epr-rea`; reconstruction = re-fold over events in canonical order. No table, no migration in this slice.
- **Content Address Strategy**: subject is Content-Derived (CID) — claims are content-addressed; review events already reference `object_cid`.
- **Source of Truth**: the witnessed events folded over.
- **Anti-Pattern Check**: no stored status column (the founding law); f64 aggregation ordered canonically (sort by event CID) so two peers fold byte-identically.

### Entity: epistemic-status vocabulary
- **Classification**: not an entity — a closed protocol vocabulary (`elohim/sdk/schemas/v1/enums/`), app-tier in this slice (**no `_dna` block**: adding a DNA constant moves integrity code; graduation to DNA-notarized is declared in the axis card, executed in a later deliberate DNA-lineage event).
- **Anti-Pattern Check**: drift check performed — no existing `EpistemicStatus`; differentiated from witness ladder (evidentiary custody, orthogonal), `place-status` (domain instance, stays), Mishpat `Precedent.status` (free-text `active/superseded/under-review` — flagged for future formalization against this enum, recorded in the axis card).

### Entity: peer-review events
- **Classification**: this slice consumes the **existing** `WitnessedInteraction` wire type (`elohim/epr/src/witness.rs:59`, `EprKind::WitnessedInteraction`, verbs `Cite`/`Affirm`/`Dismiss`) via fixtures. Future persistence: **B2** — local `.eprfs/status/` observation floor graduating to the content-store attestation `content_type` at push (the documented `DepEdge` pattern; imagodei's standalone `Attestation` entry type was removed — canonical attestations are `content_type` values on the elohim content-store). **No new DHT entry type.**

### Entity: canonization record
- **Classification**: future — Mishpat `Precedent` exists (`mishpat_integrity/src/lib.rs:13-31`, 11/~100 headroom, CRUD + link indexes). This slice models it only as an `Option<CanonizationRef>` input to classification.

### Design Constraints Discovered
- `elohim/epr` is the spine's home: pure codec crate, already `COPY elohim/epr ./epr` in the storage Dockerfile, already dep'd by elohim-storage (`Cargo.toml:11`) and epr-rea — a new sibling crate would need new Dockerfile COPY lines (known trap) for zero layering benefit.
- Codegen is glob-based over `enums/` — new schema files are auto-discovered; no registration list.
- `observation-polarity`'s law ("systems cannot ship positive-only feedback") binds the fold: `Dismiss` must be representable and thresholds must weigh negatives.

## Global Constraints

1. **No `_dna` blocks, no DNA changes, no new tables, no new routes.** The keel is types + vocabulary + derivation + tests — deterministic-floor material only.
2. **Canon-requires-governance-act law**: `classify()` can never return `Canon` without a `CanonizationRef`; the maximum mechanical status is `Reviewed`. A threshold alone must be structurally unable to mint canon.
3. **Ceiling law**: contested standing routes to `Decision::Refer(ReferQuestion{layer, reason, note})` — first-class, never a fallthrough, never an error. `ReferReason` starts minimal: `NovelSituation | InsufficientAuthority | ContestedEvidence`.
4. **Determinism law**: the fold sorts events by CID before f64 accumulation; document why in code (two peers must fold byte-identically — floats break under reordering).
5. **Path-limited commits only.** The worktree carries in-flight foreign diffs (`rea_commitment_service.rs`, `seed-commitments.ts`, `qahal_coordinator.rs`) — never `git add -A`; never commit those paths.
6. **CARGO_TARGET_DIR per cargo-pool slot** for every native build (`cargo-pool key` in each crate dir); `RUSTFLAGS=""` for native.
7. House schema format per `reach.schema.json` (`$id: epr:schema:enum:<name>`, description states "Source of truth: …"); `#[serde(rename_all = "camelCase")]` on structs, `lowercase` on wire enums, ts-rs export mirroring `reach.rs`.

## Tasks

- [ ] **Task 0 (branch):** `git checkout -b shift/ontology-keel-slice1` from current HEAD.
- [ ] **Task 1 (Sonnet — schemas):** `decision.schema.json` (`["permit","refuse","refer"]`), `epistemic-status.schema.json` (`["emergent","reviewed","contested","canon","superseded"]`, `_ordinal` false, description carries the partial-order lattice + derived-never-stored law + the three-cousin differentiation), `registries/axes/epistemic.axis.json` (new dir; card: kind `derived`, orthogonalTo witness-ladder + reach, sourceOfRecord = the fold in epr-rea, renamedFrom/related dispositions for Precedent.status + place-status, graduation note re `_dna`). Run `pnpm run schema:test` + codegen; verify auto-discovery.
- [ ] **Task 2 (Opus — verdict spine):** `elohim/epr/src/verdict.rs`: `Decision{Permit,Refuse,Refer(ReferQuestion)}`, `ReferQuestion{layer, reason: ReferReason, note}`, `CheckOutcome{Passed,Failed,Skipped}`, `CheckWitness{check_id, outcome, summary, observed}`, `Witness{checks}`, `Verdict{axis, subject, decision, witness, policy_ref}` — protocol-owned banner, serde/ts-rs per house pattern, unit tests incl. serde round-trip and "Refer is constructible and never collapses to Refuse".
- [ ] **Task 3 (Opus — epistemic fold):** `elohim/epr-rea/src/epistemic.rs`: `EpistemicStanding` fold over `WitnessedInteraction`-shaped events (Affirm/Dismiss/Cite counts + magnitude sums, canonical CID order), `standing_ratio()` continuous, `classify(&standing, Option<&CanonizationRef>, &EpistemicThresholds) -> EpistemicStatus`, `cite_gate(&status) -> Decision` (contested → `Refer(ContestedEvidence)`). Tests: positive-only cannot reach Reviewed without min-review count; Dismiss weight flips to Contested; canon impossible without CanonizationRef; determinism test (shuffled input, identical fold).
- [ ] **Task 4 (Opus — reach joins the spine):** `impl From<ReachVerdict> for elohim_epr::verdict::Verdict` in elohim-storage (`Pending → Refer`, evidence → witness checks); minimal `verdict-view.schema.json` + struct-match test in `schema_contract.rs` following the house 15-line pattern.
- [ ] **Task 5 (Sonnet — capture + gates):** a2o feature capture (`genesis/a2o/features/content/epistemic-standing.feature`): contested-routes-to-Refer, canon-requires-governance-act, ceiling-is-not-a-fallthrough scenarios. Full gates: fmt/clippy/nextest in elohim/epr, epr-rea, elohim-storage (pool slots), `pnpm run schema:test`.
- [ ] **Task 6 (chief):** independent review of diffs, path-limited commits (one per concern), branch left un-pushed (integrator owns push).

## What this slice deliberately does not do
Persist standings or verdicts · touch any DNA · wire eprfs `ValidatorOutcome` conversion (needs an eprfs→epr dep decision) · formalize `Precedent.status` (recorded in card) · thresholds-as-manifest-policy (hardcoded defaults with a `TODO(policy-ref)`) · the announcement/variance organ.
