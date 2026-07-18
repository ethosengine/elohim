---
id: "backlog-keyrotation-crypto-quorum-forgery-live-check"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "URGENT verify: are rotate_identity_key / commit_key_rotation externs LIVE on deployed conductors? — if so, a CryptographicQuorum universal-identity-takeover forgery may be reachable NOW"
slug: "keyrotation-crypto-quorum-forgery-live-check"
written: "2026-07-18"
author: "identity-head arc (2026-07-18 design red-team — dna-hash-neutrality + crypto-quorum-forgery lenses)"
status: "open"
priority: "urgent"
area: "imagodei/identity-recovery-security"
domain: "D2"
jobs: [elohim-edge]
cites:
  - genesis/data/timeline/backlog/keyrotation-mint-path-witness-backed.md
  - genesis/docs/superpowers/specs/2026-07-17-identity-head-key-lineage-design.md
tags: [security, urgent, key-rotation, forgery, identity-takeover, cryptographic-quorum, kill-switch, operator-owned, red-team]
---

# URGENT — is the CryptographicQuorum mint forgery reachable on the live fleet?

## The finding (2026-07-18 design red-team, code-grounded)
`rotate_identity_key` (`elohim/holochain/dna/imagodei/zomes/imagodei/src/identity_lineage.rs:304`)
and `commit_key_rotation` (`elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs:3481`) are BOTH
`#[hdk_extern]` in-tree and BOTH call the shared `append_key_rotation_entry`
(`lib.rs:3572-3611`) which really commits a `KeyRotation` and advances the head. The integrity
gate for the `CryptographicQuorum` authority (`imagodei_integrity/src/recovery_v2.rs:245-302`)
verifies ONE Ed25519 signature over `new_key.raw39 ‖ recovery_request_hash.raw39` against a
verifying key the **attacker chooses** (via a self-minted `KeyStewardship`), and never binds the
rotation to the victim's `human_agent_pubkey` / `superseded` key / `stewardship.human_id`.

**Consequence if the coordinator WASM carrying these externs is deployed:** any agent can rotate
ANY identity's head to a key they control (universal account + economic takeover). See
`keyrotation-mint-path-witness-backed.md` "Design review (2026-07-18)" for the full attack.

## Why this is separate + urgent
The mint *build* decision (defer/rescope) is one thing; whether the forgery is **already
reachable in production** is an independent, time-sensitive security question. The prior arc
recorded "no coordinator fn mints a valid KeyRotation" — but that was about the *IntimateQuorum*
(witness) path being dead post-M4. The **CryptographicQuorum** path is structurally completable,
and the externs are shipped in-tree.

## Verification steps (operator/security-owned — NO kubectl from dev)
1. **Are the externs deployed?** Query the deployed imagodei coordinator WASM's exported function
   list on an alpha/prod conductor: does it export `rotate_identity_key` and/or
   `commit_key_rotation`? (Coordinator WASM is hot-swapped via `update_coordinators`; whether a
   prior coordinator update landed these is the crux.) If NOT exported → forgery not reachable via
   app interface; risk is latent-in-tree only.
2. **Is `KeyStewardship` creation reachable** by an arbitrary agent (the forgery needs the attacker
   to mint their own stewardship)? Confirm `create_key_stewardship` (or equivalent) is callable and
   `validate_key_stewardship` accepts an attacker-chosen `shard_commitment_hash`.
3. **Is any recovery_request precondition enforced** end-to-end that would block a self-served
   request? (Integrity verifies the signature *covers* `recovery_request_hash` but not that the
   request exists / is unfrozen / matches `superseded` — so likely not a barrier.)

## Mitigation if reachable (precautionary regardless)
Add a **default-OFF kill-switch gating `append_key_rotation_entry` itself** (the shared chokepoint,
so it covers BOTH externs) — coordinator env/config or a storage-side pre-emit gate. Do NOT gate
only one extern. This is coordinator-only (hot-swap, no DNA-hash move) and can ship ahead of any
mint decision. Real fix = the integrity re-scope in `keyrotation-mint-path-witness-backed.md`.

## Note
Alpha currently has active genesis-pair rekey / recovery churn (`gitStatus`: "matthew genesis-pair
rekey orphaned anchors"; self-heal doorway breaker item). Confirm this verification is prioritized
against that live context — a legitimate rekey path and a forgeable one share `append_key_rotation_entry`.
