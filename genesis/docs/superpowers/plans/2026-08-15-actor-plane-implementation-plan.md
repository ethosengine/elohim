---
title: Actor plane — in-flight identity claims, acceptance at ratification, adverse attestation — Implementation Plan
id: actor-plane-implementation-plan
status: Draft
class: process-meta
domain: D9
sprint: actor-plane
requires_env: [household-nodes]
cites:
  - genesis/data/timeline/backlog/commons-holonic-stewardship-backlog.md
  - "computation-attestation-graduated-rigor-design | Computation Attestation & Graduated Proof Rigor | sha256:d767f5c1eb04c841 | path: genesis/docs/superpowers/specs/2026-05-01-computation-attestation-graduated-rigor-design.md"
  - "eprfs-witnessed-interaction-primitive | The eprfs Witnessed-Interaction Primitive | sha256:6a24773ffd7b83f4 | path: genesis/docs/superpowers/specs/2026-07-15-eprfs-witnessed-interaction-primitive-design.md"
  - "sense-respond-governance-classifier | The Sense-and-Respond Governance Classifier | sha256:c716a519ee6cc953 | path: genesis/docs/superpowers/specs/2026-07-15-sense-respond-governance-classifier-design.md"
  - "contributor-presence-commons-stewardship | Contributor-Presence Commons Stewardship | sha256:9551a506a01ab1b7 | path: genesis/docs/superpowers/specs/2026-07-21-contributor-presence-commons-stewardship-design.md"
  - "commitment-dispatch-puller | Commitment-to-dispatch puller | sha256:608803ebc8811e4a | path: genesis/docs/superpowers/specs/2026-08-13-commitment-dispatch-puller-design.md"
  - "elohim-ceiling-design | 2026-06-23-elohim-ceiling-design | sha256:24925a4c8e1d9420 | path: genesis/docs/superpowers/specs/2026-06-23-elohim-ceiling-design.md"
  - "did-bridge-identity-resolution | DID Bridge | sha256:5769f6cd4c7163ca | path: genesis/docs/superpowers/specs/2026-07-17-did-bridge-identity-resolution-design.md"
---

# Actor plane — implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this
> plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give agents in-flight a way to register an honor-system identity claim
(`agent:<role>@<model>`), have the governance harness stamp who acted on every gated decision, and
make plural authorship visible in the developer valueflow — the dev-harness instantiation of
backlog rows 16/16a's capability-attestation design (operator directive 2026-08-15). The trailer
roster on commit `e4c4accf3` is the degenerate form this plan supersedes: written by the
dispatcher about others, after the fact; the actor plane lets the agent claim for itself, in
flight, with the steward attached.

**Operator steering:** this is a system primitive — captured close to the byte layer in
performant, composable Rust crates (`epr-rea`, `epr-cli`); Python hooks stay thin plumbing. The
native `epr govern` binary is already the decision authority (dual-evaluator pattern); the actor
plane extends it, never bypasses it.

**Three legs, composed from existing canon (no new primitive family):**

1. **Claim (floor, honor-system, never blocks).** The honors-contract signatory (sense-respond
   classifier §10.4A) lifted from field to session-scoped registration. `definition_cid` = content
   address of the agent's own package — the `evaluator_identity()` pattern applied to a
   persona-build; honest-narrow (identifies the build, not the instance). The steward (git-signing
   human) stays attached to every attribution — the stewarded-minor claim shape,
   contributor-presence's witness≠steward separation.
2. **Acceptance (ceremony, Witness tier).** Ratification-at-dev-merge is the acceptance act —
   graduated-rigor's Witness station, recorded not judged. No plausibility predicate is invented
   (canon has none; a mechanism nobody can audit is what elohim-ceiling Principle 8 rejects).
   Contest escalates the next acceptance to Audit, riding graduated-rigor unchanged. Fills the
   commitment-dispatch-puller spec's C9 hole (session holder → identity), respects its C1
   anti-self-election floor.
3. **Adverse attestation (evidenced, advisory).** Classifier §11.2 vocabulary: adverse =
   `correction` with evidence CID, filed against the claimed identity — never a bare accusation;
   aggregates key only on evidenced corrections. Teeth are propagation/standing effects at DHT
   graduation; v1 records, never blocks.

**Graduation path (design, not built here):** `ContributorPresence(unclaimed,
steward_id=operator)` per persistent agent identity; co-authorship as
`attestation:witnessed-ascription` manifest kind (zero new entry types); counterfeit identity via
the named `impersonation-claim` signal GAP; cryptographic ceiling = brit `AgentKey` / DID bridge.
Explicit non-paths: KeyRotation/CryptographicQuorum (red-team-refuted), `governance` as a
`SubstrateSignal` member (DNA-hash move), escalation-provenance in canonical bytes, self-issued
credentials as apex.

## Tasks

### Task 1 — `epr actor claim` / `epr actor current` (foundation)

- [ ] New `elohim/epr-rea/src/actor.rs`: `ActorClaim { claimed: AgentRef, session, claimed_at,
      definition_cid: Option<String> }` — `claimed` validated `agent:<role>@<model>`; `claimed_at`
      from git HEAD author date (never `now()`); `definition_cid` = `sha256:<hex>` of the raw
      bytes of `.epr-meta/elohim/packages/agents/<role>.json`, `None` = honest absence. Canonical
      bytes via existing `canonical_bytes`/`atom_cid` dag-cbor.
- [ ] `SidecarActorStore` mirroring `SidecarFlowStore` on `.eprfs/status/actors.jsonl`
      (append-only `{cid, record}`, CID re-verified on read); `current_for(session)` = last claim
      per session. Claims stack; latest wins; history never rewritten.
- [ ] New `elohim/eprfs/epr-cli/src/actor.rs` (hand-rolled arg loop, `govern.rs` idiom):
      `claim --as agent:<role>@<model> --session <id> [--json] [--root DIR]` (refuse malformed
      shape, refuse no-HEAD; CID-dedupe → idempotent re-claim reports `appended: false`;
      outcome names `superseded` prior claim) and `current --session <id>` (exit 0 with-or-without
      claim; answer in payload; exit 2 = could-not-run). Route `"actor"` in `main.rs`.

### Task 2 — `epr flow note --as` + steward slot

- [ ] `flow/mod.rs run_note`: optional `--as`, `--session` (env fallback
      `CLAUDE_SESSION_ID`/`ELOHIM_SESSION_ID`, resolved in the CLI shell so `note()` stays
      env-free), passed as a `NoteActor` struct.
- [ ] `note.rs` provider resolution, three arms: `--as` given → validate (refuse malformed, never
      silently fall back to email), provider = agent ref, `steward:<git-author-email>` appended as
      final classified_as slot; session resolved → `current_for`, claim found → same, no
      claim/unreadable sidecar → fall through with rendered notice, exit 0; neither →
      byte-identical current behavior (golden `note_event_cid_is_stable` unchanged).

### Task 3 — Actor stamp on governance decisions

- [ ] `govern.rs`: `--session` arg; `actor_stamp(root, session)` → `{claimed, session,
      definitionCid, source: "claim"|"unclaimed"}`; any sidecar read error → `"unclaimed"` (the
      identity sidecar must never break the decision authority); `"actor"` key emitted only when
      `--session` given.
- [ ] Python plumbing (additive): `epr_client.govern(session=None)`;
      `epr_meta.witness(actor=None)` — key present only when supplied (the `evaluator`-field
      precedent); `epr-meta-resolver.py` threads `data.get("session_id")` and captures
      `native.get("actor")`; `epr-meta-git-gate.py` resolves session from env best-effort.
- [ ] Agent-prompt plumbing: scribe/blind-reader dispatch templates gain the one-line
      self-registration instruction.

### Task 4 — Co-Authored-By trailers in `epr flow project`

- [ ] `flow/mod.rs`: `producing_commit` returns `Provenance { author, occurred_at, co_authors }`
      via format `%ae%x1f%aI%x1f%(trailers:key=Co-Authored-By,valueonly,separator=%x1e)`;
      defensive guard for old git (literal `%(trailers` → empty). Both callers updated
      (`project.rs`, `stocks.rs`). `head_commit_provenance` untouched.
- [ ] Pure `normalize_co_author`: `noreply@anthropic.com` → `agent:<name-slug>`,
      `noreply@ethosengine.com` → `collective:<slug>`, unknown domain → lowercased email, junk →
      `None` (one bad trailer never fails a projection).
- [ ] `project.rs derive_process_doc`: co-authors → sorted, deduped `co-author:<normalized>`
      classified_as entries on Produce events; provider stays the signing author (the steward);
      empty stays `Vec::new()` (byte-identical for trailer-less commits).
- [ ] **Load-bearing dedupe guard**: alongside CID dedupe, build
      `EventKey = (verb, provider, resource_cid, process_cid, occurred_at)` from existing Event
      records; staged event with new CID but present key counts `present`, not appended. History
      is NOT retro-attributed — enrichment applies to newly-minted events only; retro-attribution
      is a deliberate separate migration.

### Task 5 — Spec, seal, story

- [ ] Spec `genesis/docs/superpowers/specs/2026-08-15-actor-plane-inflight-identity-claims-design.md`
      authored via the scribe loop (technical spec → scribe prose → coherence review → fresh
      blind-reader).
- [ ] This plan cite-sealed (cite-gen tooling; envelopes never hand-written).
- [ ] Storyteller writes the genesis story citing this sealed plan; its commit carries the full
      co-author roster and, once Task 1 lands, storyteller's own `epr actor claim` — the actor
      plane's first exhibit.
- [ ] Habits one-line delta; commits path-limited with plural co-author rosters; no push
      (integrator authority).

## Verification

- `cargo test` in `elohim/epr-rea` and `elohim/eprfs` (pool-slot `CARGO_TARGET_DIR`; plain
  `cargo test`, exit echoed). Unit goldens: ActorClaim CID; unflagged-note CID regression pin;
  normalize_co_author table; the dedupe-guard double-count test.
- Live: register a claim → note `--as` → re-project flows → gated edit shows actor-stamped
  governance row; commit `e4c4accf3` (3-trailer roster) is the trailer fixture.
- Blind-reader verdict on the spec; scribe-loop rounds recorded.
