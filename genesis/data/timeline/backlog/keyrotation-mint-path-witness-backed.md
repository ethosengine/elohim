---
id: "backlog-keyrotation-mint-path-witness-backed"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "KeyRotation mint path — no coordinator fn mints a valid witness-backed KeyRotation (blocks identity-lineage end-to-end recovery)"
slug: "keyrotation-mint-path-witness-backed"
written: "2026-07-18"
author: "identity-head arc (Wave B review, operator: defer-the-mint)"
status: "open"
priority: "high"
area: "imagodei/identity-recovery"
domain: "D2"
jobs: [elohim-edge]
relatedNodeIds:
  - "memory:project_rea_compute_commitment_primitive"
cites:
  - genesis/docs/superpowers/plans/2026-07-17-identity-head-key-lineage-plan.md
  - genesis/docs/superpowers/specs/2026-07-17-identity-head-key-lineage-design.md
tags: [identity, key-rotation, recovery, humanity-witness, key-stewardship, cryptographic-quorum, m4-migration]
---

# KeyRotation mint path — the deferred last mile of identity-lineage recovery

## Why this exists
The identity-head arc (Waves A–C) ships the lineage primitive: chain-root over the
KeyRotation DAG, the `binds-identity` controller declaration, the `rotate_identity_key`
authorization gate, and did:elohim resolution of real controllers + lineage. All
DNA-hash-neutral, reviewed, banked. **But no coordinator fn mints a valid `KeyRotation`
entry**, so the *success* path (an authorized/recovery-quorum rotation actually appending a
node + advancing the head) is proven only by pure-logic unit tests + the authorization gate
— never an end-to-end conductor mint.

Root cause (confirmed, Wave B review): post-M4 cross-DNA migration, `submit_intimate_witness`
synthesizes a `HumanityWitness` for signal emission but **writes none to imagodei's DHT**
(`imagodei/zomes/imagodei/src/lib.rs:3913`); `recovery_m3::m3_happy_path_intimate_quorum` is
a bodyless `TODO` stub. `validate_key_rotation`'s `IntimateQuorum` branch resolves
`witness_hashes` via `get()` against entries no coordinator fn creates.

## Consequence
The grandma-recovery a2o (identity-head plan Wave D) lands as `@wip` partial — authorization
gate + wiring + read-side chain-walk proven — with the true end-to-end mint deferred here.
Operator decision 2026-07-18: defer the mint, ship the primitive.

## The narrower path (Wave B review finding) — ⚠ REFUTED; see "Design review (2026-07-18)" below
> **This section's premise is UNSAFE and is retained only for history.** The 2026-07-18
> design + adversarial red-team review (below) found the `CryptographicQuorum` path is NOT a
> safe coordinator-only unblock — as it stands it is a **universal identity-forgery
> primitive**. Do NOT build the paragraph that follows. Read "## Design review (2026-07-18)".

The **`CryptographicQuorum` recovery variant does NOT depend on `HumanityWitness`** — an M-of-N
*cryptographic* controller quorum could mint a real `KeyRotation` without rebuilding the
witness/M4 attestation path. ~~That is the likely-minimal unblock~~ (refuted): a `rotate_identity_key`
success path gated on `CryptographicQuorum` controller signatures, aligned with the
recovery-quorum controllers `binds-identity` already declares. The full witness-backed
(`IntimateQuorum`) mint remains the larger, M4-dependent piece.

## When picked up
Flip the identity-head plan Wave D from `@wip` to green; route through `p2p-design-gate`
(this mints a notarized entry) and the DNA-hash-neutrality gate (coordinator-only). Prefer the
CryptographicQuorum path first as the minimal end-to-end proof.

## ⚠ HARD PRECONDITION — this mint path MUST NOT land without plan task B1b
(Whole-arc review, 2026-07-18.) Today every identity is a degenerate single-node chain, so the
storage `identity_root_cid(k)` (trim-only, returns `k`) and the imagodei `identity_chain_root(k)`
(walks the DAG) are *accidentally equal* — the ONLY reason the Wave-A re-pointings (REA
provider/receiver, `claimed_agent_id`) and the identity-head `chain_root` coincide. The instant a
real KeyRotation can be minted, `identity_chain_root(new_key)` walks back to genesis `G` while
storage `identity_root_cid(new_key)` still returns `new_key` → every re-pointing diverges from the
head's `chain_root` and **silently breaks** (a claim stored under `G` won't resolve for a lookup
routed through `new_key`; an REA provider written as `new_key` won't match a head with
`chain_root=G`). So landing this mint path REQUIRES the plan's **B1b** (upgrade storage
`identity_root_cid` to walk the DAG AND route the read filters through the resolved root) in the
SAME change — otherwise the arc's data integrity silently regresses. See
`genesis/docs/superpowers/plans/2026-07-17-identity-head-key-lineage-plan.md` Wave B, task B1b.

---

## Design review (2026-07-18) — build-ready design + 4-lens adversarial red-team

A design pass produced a build-ready mint+B1b design; four independent red-team lenses
(crypto-quorum-forgery, b1b-completeness, dna-hash-neutrality, chain-root-stability) attacked it
against the tree. **Verdict: the CryptographicQuorum mint MUST NOT be landed as a
coordinator-only change. Its central premise is refuted.** Evidence (all code-grounded):

### Why the "coordinator-only, DNA-hash-neutral" framing is fatally wrong
The design's selling point is that `validate_cryptographic_quorum` (`imagodei_integrity/src/recovery_v2.rs`)
stays byte-identical (no DNA-hash move). But that validator is precisely what leaves the forgery
open. **Every fix that closes the forgery is an integrity-zome edit → DNA-hash move → forced prod
re-key + lineage governance.** The two are mutually exclusive.

### The forgery (BLOCKER — universal identity takeover)
`check_cryptographic_quorum_rules` (`recovery_v2.rs:245-302`) verifies ONE Ed25519 signature over
`M = new_agent_pubkey.raw39 ‖ recovery_request_hash.raw39` against a verifying key it reads from a
`KeyStewardship.shard_commitment_hash`. It **never references `human_agent_pubkey`,
`superseded_agent_pubkey`, or `stewardship.human_id`.** An attacker:
1. Mints their OWN `KeyStewardship` (`validate_key_stewardship`, `imagodei_integrity/src/lib.rs:1494-1532`,
   accepts any base64 `shard_commitment_hash`; the `threshold_m`/`total_shards_n` fields are
   **decorative** — nothing ties them to the committed key, so "5-of-5" = one self-chosen key).
2. Signs `M` with their own key; calls `rotate_identity_key(human=VICTIM_G, superseded=VICTIM_head,
   new=K_attacker, controller_policy="recovery-quorum", authority=CryptographicQuorum{...})`.
3. Every gate passes (`superseded==current_head` — attacker read it off the DHT;
   `authorize_rotation` needs only `has_recovery_authority`; integrity `verify` succeeds under the
   attacker's own key). → attacker's key becomes the victim's head; with B1b, inherits the victim's
   entire REA/economic history + `did:elohim` lineage. **Full account + economic takeover of ANY
   identity, authorized solely by a stewardship the attacker minted for themselves.**

### Compounding blockers
- **Single-use guard is dead code:** the `stewardship.rotated_at.is_some()` reject can never fire
  (`append_key_rotation_entry` never supersedes the stewardship; validation always resolves the
  pristine original by fixed hash) → one self-minted stewardship = permanent skeleton key.
- **Grindable byte-min tiebreak → chain-root hijack:** `chain_root_of`/`chain_head_of` pick the
  byte-minimal parent/tip. An attacker grinds a genesis key byte-minimal (free; ~2 tries), mints a
  `KeyRotation` with `new_key = victim's key` (unconstrained), creating a merge; the walk flips the
  victim's root to the attacker's chain. Integrity (HDI, no `get_links`) **cannot** defend this.
- **`.verify()` not `.verify_strict()`** at `recovery_v2.rs:296` (design claimed strict) — malleable;
  fixing it is also an integrity edit.
- **`rotate_identity_key` runs NO freeze-floor and NO revocation gate** (both of which
  `commit_key_rotation` has) — the network's emergency stop gives zero protection.
- **`controller_policy` is attacker-supplied** — the victim's declared policy is never read, so a
  `self`-policy identity is rotated by simply passing `"recovery-quorum"`.
- **DNA-neutrality deletes the deployment fence:** all nodes share one DHT; a premature mint is a
  permanent entry every node sees, but storage nodes not yet on B1b keep `identity_root_cid==key`
  → split-brain roots across the fleet, unrollback-able. A DNA-hash *move* would isolate
  un-upgraded nodes (safe); neutrality trades that away.

### True scope — do it right (the integrity change is fine in dev)
The sound mint is an **integrity change** that: binds `human_agent_pubkey` +
`superseded_agent_pubkey` into the signed `M`; enforces `stewardship.human_id` resolves to the
target human; enforces the group-key invariant (or honestly downgrades the claim to "single
cryptographic controller"); switches to `verify_strict`; adds a real one-time nonce for single-use;
de-grinds the merge/head tiebreak (earliest-`rotated_at`, or fail-closed on multi-parent); reads
the victim's notarized `controller_policy`; and runs freeze + revocation gates in
`rotate_identity_key`. It is NOT the "witness-free coordinator-only minimal unblock" the backlog
originally promised. **But the DNA-hash move it requires is NOT a blocker here:** everything is
dev, so it's a normal `ALLOW_DNA_REINSTALL` reinstall (both genesis peers flagged together, per the
DNA-upgrade-governance doc), not a prod migration. So: do it right — design the integrity validator
properly, accept the hash move, reinstall the dev DHT. Re-scope + route through `p2p-design-gate` +
`red-team` again on the revised integrity design before building.

### Reachability note (dev context — NOT an incident)
`rotate_identity_key` AND `commit_key_rotation` are `#[hdk_extern]` in-tree, so the forgery is
reachable by construction wherever this coordinator runs. But everything is dev — the repo IS the
state; there is no separate prod fleet to race. So this is a **correctness bug to fix before the
mint is considered shippable**, not a live security incident. No kill-switch scramble; just don't
ship the CryptographicQuorum success path until the integrity binding below lands.

### B1b is also unsound as designed (must be redesigned, not just "routed")
The mint's HARD PRECONDITION (B1b) as specified stores the *resolved root* in the
`provider`/`receiver` column — but `identity_root_cid` is a function of a **moving, eventually-
consistent** edge set, so a row written before its rotation edge arrives is **permanently
orphaned**. The "exhaustive 5-read + 1-write" set is **incomplete** (missed:
`create_replicates_dwelling_commitment` write; `peer_capacity_service.rs:203` +
`replication_prioritizer.rs:74` reads). And the resilience-card join is **column==column**
(`humans.agent_pub_key == rea_commitments.provider`, `household_resilience.rs:172-176`) — root-
normalizing only one side re-creates the all-zeros regression; input-routing cannot fix it.
**B1b redesign:** store the RAW key, resolve the equivalence class **read-side** (root→members),
so a late edge never orphans; sweep the *columns* (`provider.eq|receiver.eq`), not `identity_root_cid`
call-sites; root-normalize BOTH partners of any identity join (or add an explicit `chain_root`
join column). This is a bounded, testable storage session — but bigger than the plan's B1b, and
it is a prerequisite the mint cannot be sound without.
