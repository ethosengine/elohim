---
title: "Identity Head + Agent-Key Lineage — implementation plan (Grandma-recovery rung)"
id: identity-head-key-lineage-plan
status: Draft
created: 2026-07-17
maintainers: Matthew Dowell + Claude Fable 5
class: protocol-canonical
domain: D2
sprint: Sprint-2 (Grandma recovery + mutual-aid pair)
topic: [identity, agent-key, lineage, chain-root, binds-identity, key-rotation, group-control, community-recovery, rea-agent, contributor-attribution, agent-peer-binding, did]
context-tier: disclosed
sovereignty-frame: descriptive
steward: rust-architect
graduation-trigger: decompose-complete OR all-waves-green
refines:
  - genesis/docs/superpowers/specs/2026-07-17-identity-head-key-lineage-design.md
cites:
  - lens-version-dag-epr-policy-dependency-design | the primitive this plan implements an identity-instance of — version_parent DAG, chain-root, declared head | sha256:62e0f37f8f57c0ed | path: genesis/docs/superpowers/specs/2026-06-27-lens-version-dag-epr-policy-dependency-design.md
  - did-bridge-identity-resolution | the phase-1 bridge whose §5 phase-2 hook Wave C upgrades in place (self-only projection -> real controllers + lineage + verified transport ids) | sha256:5769f6cd4c7163ca | path: genesis/docs/superpowers/specs/2026-07-17-did-bridge-identity-resolution-design.md
requires_env: [household-nodes]
---

# Identity Head + Agent-Key Lineage — implementation plan

Implements `genesis/docs/superpowers/specs/2026-07-17-identity-head-key-lineage-design.md` (9 gap-items;
#9 DNA-reinstall is a scoped-out hard follow-on and is NOT in this arc). **Compose-don't-build**: most
substrate is WIRED (Collective, Membership{Steward}, KeyRotation, RecoveryAuthority, AgentPeerBinding,
did:elohim assembly) — the plan adds a chain-root identifier + a `binds-identity` action discriminator
and re-points three raw-keyed references.

> REQUIRED SUB-SKILL for agentic workers: superpowers:subagent-driven-development (or executing-plans).
> Steps use checkbox (- [ ]) syntax. Story-first: the a2o RED (Wave D) is authored before Wave B lands,
> so the DNA core is built against a failing recovery scenario.

## Dependency shape (why the waves are ordered this way)

The re-pointings (#5/#6) must point *at* a chain-root, so a **thin chain-root identifier** (the
degenerate single-node case, derived from the current key) lands first — the indirection is installed
and testable immediately, and becomes load-bearing the moment real rotation (Wave B) exists. DNA core
before resolution/binding; the culminating recovery scenario proves the whole chain.

```
Wave A (thin seam, no DNA-hash move) ─┐
  A1 chain-root identifier (degenerate)│→ Wave B (DNA core, sweettest) ─┐
  A2 REA re-point   A3 contributor     │   B1 version_parent DAG        │→ Wave C (resolution+binding)
                                       │   B2 binds-identity            │   C1 did:elohim upgrade
Wave D RED authored FIRST ────────────┘   B3 rotate_identity_key       │   C2 signed AgentPeerBinding
  D1 grandma-recovery a2o (RED → GREEN after C) ──────────────────────┘
```

## Wave A — the thin chain-root seam + re-pointings (gap #1-partial, #5, #6)
No DNA-hash move; storage + projection only; testable on household-nodes now.

- [ ] **A0 (Wave D RED first).** Author the grandma-recovery a2o scenario as a RED before any code:
  `genesis/a2o/features/auth/` — *"A human loses their key; the community-recovery quorum authorizes a
  key rotation; their contributor attribution, REA standing, and presence claims all resolve unbroken."*
  `@requires:household-nodes`. It fails today (no rotation path); it is the Wave-D acceptance gate.
- [ ] **A1 — chain-root identifier (degenerate slice of #1).** Add `identity_root_cid(agent) -> Cid`:
  for an un-rotated identity, root = a stable derivation from the genesis/current agent key (single-node
  chain). Storage-side resolver + a stable column/concept the re-pointings target. Pure function +
  unit test (root is deterministic and stable for a fixed key). NO DAG yet — that's B1.
- [ ] **A2 — REA re-point (#5).** When an economic party is a collective, populate
  `rea_commitments.provider/receiver` with the collective's chain-root cid (its content-CID identity),
  not a raw key. Reconcile SQLite `collectives.id` slug ↔ DHT content-CID via a `dht_anchor_hash`-style
  mirror column (the pattern `rea_commitments` already uses). Test: a collective-party commitment stores
  the chain-root; the resilience/REA join resolves it.
- [ ] **A3 — contributor re-point (#6).** Re-anchor `claimed_agent_id` / `ClaimedAgentToPresence` on the
  identity_root_cid so a claim resolves through the root, not a point-in-time key. Presence↔EPR edges
  and recognition accrual are already root-safe — leave them. Test: a claimed presence resolves via the
  root; simulate a key change and confirm the claim still resolves (indirection proven before real
  rotation exists).
- [ ] **A-gate:** `cargo build && cargo test --lib` (elohim-storage, ambient RUSTFLAGS, pooled target);
  the A2/A3 rotation-survival unit tests green.

## Wave B — the DNA core (gap #1-full, #2, #3)
Mishpat coordinator-zome changes → **DNA-hash-neutral** (payload-in-`payload_json`, coordinator hot-swap;
the `author-lens`/`binds-policy` precedent). Sweettest, local — no cluster env.

> **B0 — architecture decision (lineage home; resolved 2026-07-17, compose-don't-build).** The identity
> lineage DAG is realized on the **already-wired imagodei `KeyRotation` edges** (`recovery_v2.rs` —
> old→new + `RecoveryAuthority`), NOT a new `version_parent` on `mishpat_integrity::Commitment`. Each DNA
> keeps its proven home: **imagodei owns the key-rotation edges + chain-root derivation** (recovery is its
> domain, `RecoveryAuthority` already lives there); **mishpat owns the `binds-identity` declaration** (the
> controller-set + head selection referencing the chain-root — parallel to `binds-policy`, "which head
> applies is declared"). This avoids adding a second key-rotation mechanism (the `KeyRotation`-vs-new-
> `version_parent` drift) and avoids widening the mishpat integrity struct (a hash-move risk). So B1's
> "version_parent DAG" is the `KeyRotation` superseded-chain made queryable with a derived stable root;
> B2's `binds-identity` (mishpat) points at that root. If the imagodei↔mishpat cross-DNA reference proves
> unworkable in sweettest, escalate — do not silently collapse both into one DNA.

- [ ] **B1 — version_parent DAG (#1 full).** Model `KeyRotation` (`recovery_v2.rs`) edges as a
  `version_parent` DAG (SET, for the merge/recovery case); derive the chain-root over a multi-node chain
  so A1's degenerate root generalizes. Sweettest: a rotation appends a node; the root is stable across it.
- [ ] **B1b — storage read-routing + Wave-A carryover (when root≠key becomes real).** Once B1 makes the
  root value differ from the raw key, route the storage READ filters through the resolved root too (Wave A
  only routed WRITE paths; raw `.eq()` reads must now normalize via `identity_root_cid`), OR make read-time
  normalization idempotent so raw-vs-root storage resolves equal. **Wave-A carryover (review-confirmed):**
  `record_provide_from_content_commitment` (`rea_commitments.rs:518-566`) was intentionally left un-routed
  in Wave A (self-provide, always the conductor key) — safe while root==key, but it silently orphans under
  a human-key rotation once reads route. Route it (or prove its keyspace is isolated from rooted joins) here.
- [ ] **B2 — `binds-identity` discriminator (#2).** New `Mishpat::Commitment` action discriminator
  declaring *chain head = key K; controllers = {set}; controller-policy = self | Steward-set |
  RecoveryAuthority M-of-N*. Reuse the wired `RecoveryRequest`/`RecoveryAuthority` for the human quorum.
  Validator (`commitments.rs`) + schema. Sweettest: valid `binds-identity` accepted; malformed refused.
  Confirm DNA hash unchanged (the coordinator-only class — verify via `hc dna hash` before/after).
- [ ] **B3 — `rotate_identity_key` coordinator fn (#3).** Append a version node authorized by the current
  controllers per the `binds-identity` policy. Sweettest (the load-bearing proof): controller-authorized
  rotation appends + updates head; an unauthorized rotation is REFUSED; a RecoveryAuthority-quorum
  rotation (the grandma case) is accepted.
- [ ] **B-gate:** sweettest suite green (`RUSTFLAGS="" ... cargo test ... --run-ignored all`); DNA-hash
  neutrality confirmed; `just pack` refreshes the bundle.

## Wave C — resolution + witnessed binding (gap #4, #7)
Builds on Wave B; upgrades the already-built did:elohim assembly in place.

- [ ] **C1 — did:elohim assembly upgrade (#4).** `ElohimIdentityStore` now resolves real `controller`
  entries + lineage (chain-root + head) from the `binds-identity` declaration, replacing the phase-1
  self-only projection. The bridge crate's phase-2 hook (DID bridge spec §5). Conformance: assembled
  document still passes `bridges/did/schemas/did-document-1.1.schema.json`; controllers populated.
- [ ] **C2 — sign the AgentPeerBinding (#7).** Replace `STAGE1_SIGNATURE_SENTINEL` with a real
  challenge/response over the transport channel, cross-signed by the current head key; a signed
  `AgentPeerBinding` adds a **verified** `alsoKnownAs` transport-id to the head (the did:elohim assembly
  then emits verified, not self-asserted, transport ids). Test: unsigned/forged binding rejected; a
  correctly cross-signed one accepted and surfaced in resolution.
- [ ] **C-gate:** storage `cargo test --lib` + `bridges/did` conformance suite green; a resolved
  `did:elohim` document shows real controllers + a verified transport `alsoKnownAs`.

## Wave D — the culminating proof (gap #8)
- [ ] **D1 — grandma-recovery a2o GREEN.** The A0 RED now passes end-to-end: key loss → community-recovery
  quorum authorizes `rotate_identity_key` → contributor attribution + REA standing + presence claims all
  resolve unbroken through the chain-root. `@requires:household-nodes` (multi-controller leg; household is
  a live multi-peer mesh — the stable floor). This scenario IS the acceptance criterion for the whole arc.

## Definition of done
- [ ] Waves A–D green; the grandma-recovery a2o scenario passes ×2 fresh runs.
- [ ] DNA-hash neutrality confirmed for Wave B (no re-key on deploy).
- [ ] did:elohim resolves real controllers + lineage + verified transport ids (phase-1 self-only retired).
- [ ] #9 (DNA-reinstall migration) remains a captured hard follow-on — NOT closed here.

## Watch-outs
- **DNA-hash neutrality is load-bearing** — `binds-identity`/`rotate_identity_key` MUST stay
  coordinator-only (integrity zomes + modifiers untouched), or a hash move forces re-key on prod. Verify
  with `hc dna hash` before/after, per the DNA-upgrade-governance doc.
- **Chain-root stability is the contract** — the root cid must NEVER change across rotation/recovery, or
  every re-pointing (A2/A3) silently breaks. Property-test it.
- **Ontology guard** — the recovery quorum is a *controller*, not an override; keep it in the same
  controller-policy field as self (structural imago-dei, not a bolt-on).
- **Sweettest env** — `RUSTFLAGS=""`, `--run-ignored all` (`#[ignore]` is a CI no-op otherwise), `just
  pack` (not build) to refresh the `.dna`.
