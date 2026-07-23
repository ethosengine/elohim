---
title: "Discipline 1 — .epr-meta governance speaks the verdict spine; agents drive within the ladder"
id: epr-meta-discipline1-governance-spine-plan
status: Ready
class: protocol-canonical
created: 2026-07-23
domain: D2
topic: [epr-meta, governance, verdict, refer, dispatch, agency, parity, keel]
cites:
  - ontology-keel-slice1-verdict-spine-plan | Ontology Keel Slice 1 | sha256:059f604e7ebc7821 | path: genesis/docs/superpowers/plans/2026-07-23-ontology-keel-slice1-verdict-spine-plan.md
  - reach-ontology-vocabulary-split-spec | Reach Ontology/Vocabulary Split | sha256:2a1ef52c1ced3c48 | path: genesis/docs/superpowers/specs/2026-07-22-reach-ontology-vocabulary-split-spec.md
  - elohim/sdk/schemas/v1/registries/governance-parity-vectors.json
---

# Discipline 1 — the repo governs itself on the spine

> **The move.** `.epr-meta` becomes the first production consumer of the keel: `ask` IS `Refer` (routed, witnessed, never a collapsed warn); every non-trivial gate outcome lands in the already-declared-but-empty `governance-findings.jsonl`; the dead `dispatch` class comes alive on the sentinel idiom; policies get content pins; and the **agency charter** is codified — agents author `measure`/`inject` freely (observation cannot harm), `dispatch` with a named agent, and may NEVER self-grant `ask`/`deny` (escalation is operator-ratified). Cross-runtime law: the **golden-vector corpus** (`governance-parity-vectors.json`, chief-authored) is the correspondence theorem every evaluator must pass — Python hook, git-gate (the universal floor: fires for Claude, Codex, and humans via husky), and Rust eprfs-meta/epr-cli. Codex edit-time gating stays deferred on the runtime's own limitation (no PreToolUse), already recorded in the HookPackage's `runtimeTargets`.

## P2P Design Gate (delta — everything else per keel-slice-1 gate)

### Entity: governance-findings.jsonl entries (gate verdicts, witnessed)
- **Classification**: Operational (C) today — append-only **local observation floor**, the documented `DepEdge`/`.eprfs/status` pattern: B2 graduation (content-store attestation `content_type`) when governance verdicts go peer-visible. NOT reconstructable (they witness moments) — justified as the observation-floor exception, same as `.eprfs/status/` sidecars.
- **Shape**: `{ts, runtime: claude|git-gate, gate, subject, decision: permit|refuse|refer, class, ruleId, policyRef, refer: {layer, reason}|null, witness: [checks]}` — the keel `Verdict` projected to JSONL; `decision` values are `epr:schema:enum:decision`.
- **Anti-pattern check**: no verdict stored as truth (these are witnessed *events about evaluations*, not authoritative states); routine clean allows are NOT logged (silence when no rule fired is clean; silence when a rule misfired is the sin).

### Entity: policy contentHash pins
- 7 rows in `.claude/epr-meta/policies.yaml` gain `contentHash: sha256:<hex>` over the canonical-JSON policy body. Loader verifies; mismatch → the binding **refers** (authoring polarity: a tampered registry routes to judgment, never bricks the repo, never silently proceeds).

## Global Constraints

1. **The package is authoritative.** The resolver is a planted HookPackage (`master: "package"`). Code changes happen in `.epr-meta/elohim/packages/hooks/epr-meta-resolver.json` `source.body` and PROJECT to `.claude/hooks/epr-meta-resolver.py` byte-identically (plant-eprfs-hook flow). Never edit the projection directly. `registration` is recorded, never auto-written to settings.json.
2. **Polarity law.** Authoring may fail open but never silently: every downgrade, unresolvable ref, malformed-input exception, and fail-open path emits a witness line. Serving/validation surfaces (not this slice) fail closed.
3. **Ceiling law.** `ask` outcomes are `decision: "refer"` with `{layer, reason}`. epr-cli stops collapsing `Deny|Ask` into one status. `refer_reason` vocabulary starts: `rule-fired`, `unresolvable-validator`, `governance-manifest-malformed`, `policy-pin-mismatch`, `escalation-requires-ratification`.
4. **Agency charter (the ladder) — ratification is an "us" act (operator correction, 2026-07-23).** New policy `governance-escalation-ladder@1`: agents self-grant `measure`/`inject` (+ `dispatch` with named agent+prompt); `ask`/`deny` authorship or promotion requires **deliberation provenance** — `established_by: deliberated-*` recording the deliberating community (today: operator+agents working this tree, the "us" of the pre-p2p era; tomorrow: a qahal — the shape stays identical, only the peer set grows). Legacy `operator-*` rows predate the convention and remain accepted. **Ratification is not a solo stamp; it is peer acceptance at the branch rung**: the branch ladder is the repo's reach axis (`shift/*` = self · `dev` = community · `main` = commons), a rule on a shift branch is structurally pending, and the dev-merge acceptance IS the ratification event — the CanonizationRef of repo governance (accepting a merge includes accepting the rules we govern ourselves by; governance diffs deserve first-class surfacing at the merge gate). Enforced by validator `epr:validator-escalation-ladder` (Python AND Rust, both frontmatter-tolerant) on `.epr-meta`-file writes: introducing/raising to ask|deny without a deliberated pin → **refer** (`escalation-requires-ratification`).
5. **Parity is the law, vectors are the text.** Both runners consume `elohim/sdk/schemas/v1/registries/governance-parity-vectors.json` verbatim. A vector a runner cannot execute is an EXPLICIT skip with reason in test output — never silent green.
6. **Declared runtime-scoping ≠ unresolvable.** Python declares `epr:validator-eprfs-meta-domain-neutrality` as `rust-only` (skips clean); only genuinely unknown refs refer.
7. Path-limited commits; foreign in-flight diffs untouched; `RUSTFLAGS=""` + pool slots for native builds.

## Tasks

- [ ] **T1 (Sonnet — Python + package + registry):** (a) resolver via package: emit verdict-ledger lines (shape above, flock append like `_handle_measures`) for deny/ask/advise/measure/dispatch/downgrade/exception paths; ask→refer shape; malformed-manifest→refer; unresolvable→refer (kill the inject-degrade); implement `dispatch` class (ledger line + additionalContext self-dispatch directive, deprecation-sentinel idiom verbatim); re-project byte-identical. (b) `epr_meta.py`: `RUNTIME_SCOPED_VALIDATORS`, contentHash verify on policy load (mismatch→refer verdict), combine unchanged. (c) `epr-meta-git-gate.py`: witness lines (`runtime: "git-gate"`), keep `EPR_META_ACK`. (d) `policies.yaml`: 7 pins + `governance-escalation-ladder@1` row (pending ratification) + pin tool `.claude/scripts/epr-meta-pin.py`. (e) Python `epr:validator-escalation-ladder`. (f) first live dispatch rule: `.claude/epr-meta/.epr-meta` — writes to `policies.yaml` dispatch `code-reviewer` (audit: version-pin discipline, in-place-mutation check). (g) Python parity runner over the vectors.
- [ ] **T2 (Opus rust-architect — Rust parity):** (a) epr-cli `check.rs`: `Ask` distinct from `Deny` (referral surfaces as its own finding status/exit class). (b) Rust `epr:validator-escalation-ladder` in `repository_validators.rs`. (c) policy contentHash verify in eprfs-meta binding expansion (mismatch → Ask-class verdict, reason `policy-pin-mismatch`). (d) Rust parity runner: eprfs-meta test materializing each vector into a temp tree, asserting `{decision, winning_class, rule_id}`; explicit skips with reasons where eprfs-meta lacks the surface (e.g. manifest-integrity checking) — report them.
- [ ] **T3 (chief):** review diffs, run both parity runners independently, gates, path-limited commits.

## Deliberately not in this slice
Codex PreToolUse (runtime lacks hooks — picked up via HookPackage `runtimeTargets` when it grows them) · moving `combine()` to Rust-canonical (trigger: golden vectors diverge irreconcilably) · B2 graduation of the ledger · policies→Mishpat `Precedent` graduation · any new deny-class rule.
