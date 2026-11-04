---
id: "backlog-native-dev-track-c-brit-signs-derivation-record-exact-byte-verify"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Track C [integrity]: brit signs DerivationRecord canonical bytes; verification becomes exact-byte signature plus lenient decode"
slug: "native-dev-track-c-brit-signs-derivation-record-exact-byte-verify"
written: "2026-11-04"
author: "claude (design pass, native-dev-plane spec; steward human:matthew)"
status: "backlog"
priority: "high"
relatedNodeIds:
  - "native-dev-plane-one-derivation-earned-reach-design"
tags: [native-dev-plane, integrity, brit, derivation, attestation, track-c, d7]
---

Source spec: `native-dev-plane-one-derivation-earned-reach-design` (genesis/docs/superpowers/specs/2026-11-04-native-dev-plane-one-derivation-earned-reach-design.md).

## Item (spec §11 Track C, ruling D7) [integrity: Claude-family adversarial review before landing]

Write-set: `elohim/brit/brit-epr`, `brit-build-ref`, `elohim/brit/gitoxide-core/src/repository/push.rs`, `elohim/ark/cli/src/brit_index.rs` (widened 2026-11-04, adversarial review; see below).

1. brit signs `DerivationRecord` canonical bytes; notes carry records.
2. Verification: exact-byte signature over bytes as received, then lenient decode; unknown fields retained as opaque, never authorizing. Producers lint strictly. This keeps the anti-field-smuggling property of today's strict decode.
3. Delete the inert Jenkins helper arm.

**Verified 2026-11-04.** `elohim/brit/brit-epr/src/elohim/attestation/build.rs:5-29`: `BuildAttestationContentNode` is `deny_unknown_fields`; fields are manifest_cid, step_name, inputs_hash, output_cid, agent_id, ..., and a free `signature: String`. Re-verified by the session model 2026-11-04: `elohim/brit/brit-build-ref/src/build_cmd.rs:88-103` signs the node's canonical bytes with the signature blanked, so the signature DOES bind `step_name`, `inputs_hash` and `manifest_cid`. It binds no commit. The cure is motivated by `deny_unknown_fields` (mixed versions cannot read each other), the one-note-per-step-and-commit overwrite shape, the absence of `basis` and typed misses, and the third key and address format, not by an unbound signature.

**Correction to the brief.** The Jenkins helper `genesis/orchestrator/scripts/brit-helper.sh` has no `attest` arm. Its arms are `verify`, `plan` and `build-ref`; `build-ref` is the inert one (exits 0 with "attestation skipped" when `brit-build-ref` is not installed, and swallows any failure as advisory). Delete `build-ref`.

**Receipt.** Superseded by the 2026-11-04 (adversarial review) receipt list below. Authored against Track A types; integrates when A lands.

**retire-when.** `BuildAttestationContentNode` and `build-ref` are gone; one record shape remains.

## 2026-11-04 (adversarial review): write-set, D7 verification, receipts (spec §2 D1, §7 D7, §8 D3, §11 track C)

- **push.rs.** `elohim/brit/gitoxide-core/src/repository/push.rs:707`: an explicit lease skips the fast-forward check. That is a defect for the forge-mirror path (D3). Fix: brit push keeps the fast-forward check under an explicit lease. Track F3's mirror depends on this; `native-dev-d3-fork-preserving-space-head-and-ff-only-mirror` depends on it.
- **brit_index.rs.** `elohim/ark/cli/src/brit_index.rs:428` builds a new key and `deterministic` value and copies the old brit signature onto the rebuilt bytes; that signature authenticates only the original note. Converted records are therefore **unsigned**, carry a witness pointer to the original note's CID (verifiable over the original bytes), and are admitted at most at `self` on the original signer's device. Test: a substituted method or input never verifies.
- **Verification follows D7's three cases.** Framing is `{body, sig}`: the signature covers the body bytes exactly as received, is never embedded in them, and the body is never re-encoded before verifying. Within an epoch, writers add fields only under `ext` (cannot restrict, scope or authorize). Reader cases: known epoch + known fields -> decide; known epoch + unknown field outside `ext` -> refuse (producer-lint violation); unknown epoch -> verify, retain bytes, name the record once, treat as absent for every decision.
- **Receipt (per §11 track C).** Signed-record vectors: tampered bodies and unknown non-`ext` fields refused, unknown epochs held; substituted method/input in a converted note never verifies; brit push refuses a non-fast-forward under an explicit lease; a Claude-family adversarial review before it lands.
- Soft dependency: authors against track A's `DerivationRecord`/`ProjectedRoot` with fixture roots; integration waits on A (retained `ext` decoding, `project_view`). A and C both touch ark in different files.
