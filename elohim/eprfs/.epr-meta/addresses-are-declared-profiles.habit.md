---
epr-habit-version: 1
id: addresses-are-declared-profiles
invariant: >
  Every address this system mints or admits names its own profile, and is minted and checked by
  the one Rust addressing kernel or a TypeScript twin held to its vectors: payload bytes, chunks
  and shards under the byte profile (CIDv1 raw 0x55 + BLAKE3-256), records under dag-cbor +
  sha2-256, citation fingerprints under raw + sha2-256. A byte surface admits an address only by
  full decode, a verifier fails closed on any profile it does not know, and an object addressed
  under an older profile stays verifiable under it, never re-addressed and never read as a newer
  one.
status: red
active: false
checks:
  - "L0 vectors: `cargo test -p elohim-epr` (chunker_vectors, profile_vectors) and `just gate elohim-address`; `EPRFS_WRITE_VECTORS=1 cargo test -p eprfs-core --test vectors` once, then without (the `address` and `chunker` sections of elohim/eprfs/fixtures/tree-vectors.json byte-stable, `write_vectors_refuses_to_change_a_pinned_entry`); `just gate eprfs-ts` and `just gate epr-ts` reproducing the same sections"
  - "a2o tag lane: `just test mesh '@concern:addresses-are-declared-profiles'`"
  - "scripts/ci/address-profile-lint.sh: no native address spelled outside the kernel's constructors"
refs:
  - genesis/local-dev/regenesis-1/plan.md
  - genesis/local-dev/regenesis-1/design-gate.md
  - genesis/docs/superpowers/plans/2026-10-29-native-dev-tooling-over-peers-plan.md
retire-when: >
  every address constructor in the tree is the kernel's and the lane has read zero
  legacy-spelling refusals for a quarter
---
DELTA 2026-11-05: born 2026-11-05 unwired; L0 vectors red until T0/T3 land.
DELTA 2026-11-05 (T3): declared red, not unwired — the census refuses `unwired` beside declared checks, and the L0 checks now run: eprfs-core `tests/vectors.rs` re-derives the `address` and `chunker` sections and eprfs-ts reproduces both. Red because the a2o lane has no `@concern:addresses-are-declared-profiles` scenario yet and `scripts/ci/address-profile-lint.sh` does not exist (T0b). Evidence: genesis/docs/superpowers/plans/task-rg1-eprfs-report.md.
