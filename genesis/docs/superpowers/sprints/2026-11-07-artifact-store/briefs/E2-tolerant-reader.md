---
id: artifact-store-brief-e2-tolerant-reader
status: active
---

# Brief E2 — A11 tolerant reader (Sonnet 5.5)

Worktree `as-e2`, branch `artifact-store/e2`. Read `common.md`, then plan §7.3 E2 (`genesis/docs/superpowers/plans/2026-10-29-native-dev-tooling-over-peers-plan.md:298-299`) and design D7 in `genesis/docs/superpowers/specs/2026-11-04-native-dev-plane-one-derivation-earned-reach-design.md` (§7, "the law of honest evolution").

**Write-set:** `elohim/eprfs/eprfs-storage/src/tier_blob.rs` (the body of the private `decode_manifest` and the epoch check only; E3 appends a `pub fn read_tier_manifest` to the same file in a separate hunk), `elohim/eprfs/eprfs-storage/tests/local_tier_blob.rs`, `elohim/eprfs/eprfs-core/src/derive.rs` only if `DerivationKey` decode needs the retained-`ext` rule.

**Deliverables.**
1. Mixed-epoch manifests decode per D7: the epoch is read before the epoch-specific schema; epoch 2 decodes as today; any other epoch is HELD and answered `TierBlobUnavailable::UnsupportedEpoch` (or a renamed `UnsupportedManifestVersion`, printable as `unsupported-manifest-version`), never a parse error and never a hit.
2. Unknown `ext` bytes are retained byte-exactly through decode and re-encode.
3. A held manifest records an `unavailable` `Use` like any other non-hit (I7).

**Tests (add to `local_tier_blob.rs`):** `epoch_3_manifest_is_held_with_version_named`, `unknown_ext_bytes_survive_roundtrip`, `held_manifest_is_never_a_hit_and_records_unavailable_use`. Existing tests stay green, including `epoch_1_manifest_is_held_not_read_and_not_a_parse_error`.

**Gate:** `just gate eprfs`. **Reviewer:** Codex astra read leg (not integrity class).
