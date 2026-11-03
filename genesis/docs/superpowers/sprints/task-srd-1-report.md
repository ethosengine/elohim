---
id: task-srd-1-report
status: DONE_WITH_CONCERNS
gap: plans__2026-11-02-shared-rust-discipline#1
actor: agent:implementer@codex
class: implementation-report
created: 2026-11-02
cites: []
---

# Portable policy content and offered standing

Implemented the SRD-1 content boundary in eprfs-meta and opt-in canon-lift export.
The new `epr-policy-content@2` envelope uses canonical JSON with raw CID codec
0x55. Existing registry pins and legacy canon-lift outputs are unchanged.
Scope, binding, enforcement class, establishment provenance and lifecycle fields
remain offered standing outside content. Intrinsic predicate/measure defaults
remain content. Verification establishes exact bytes, canonical format and envelope
version, never adoption authority or host support for arbitrary semantic fields.

Native interfaces:

```
epr canon-lift --portable [--write] [--root DIR] [--out DIR]
epr canon-lift --verify-portable CID --file PATH
```

The portable index records the legacy CID and new content CID without rewriting
history. Objects are created without clobbering; existing mismatched or symlink
objects refuse. Verification is bounded to the registry byte limit. Library and
CLI decision boundaries are recorded in their seam registries.

## Evidence

Base inspected: d7dc62fdd328f3c50b87ded46b34f599802a97bb. No commits made.
Native commands used Rust 1.96.1, empty RUSTFLAGS, CARGO_BUILD_JOBS=1 and
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__eprfs/dev.
Cargo berth was claimed under the actual runtime session and released afterward.

- `cargo test --manifest-path elohim/eprfs/Cargo.toml -p eprfs-meta portable_policy --lib`: 3 passed, `EXIT=0`.
- `cargo test --manifest-path elohim/eprfs/Cargo.toml -p elohim-epr-cli canon_lift::portable --lib`: 3 passed, `EXIT=0`.
- Scoped `rustfmt +1.96.1 --check --edition 2021`: `EXIT=0`.
- Both seam registries validated against the existing JSON schema.
- `cargo build --manifest-path elohim/eprfs/Cargo.toml -p elohim-epr-cli --bin epr`: `EXIT=0`.
- Built native CLI portable export: 42 objects, `EXIT=0`; strict policy verification `EXIT=0`.
- Built native CLI legacy `canon-lift --check`: fresh 42 atoms / 31 heads, `EXIT=0`.

Gate evidence: `just gate eprfs` is the context-selected owning gate; NOT RUN in
this seat, pending the parent integrating SRD-2 and running the full gate once.
Status HOLD preserves that required verification; focused tests do not discharge
network witnessing, cross-space authority, agent acceptance or real Brit adoption.

Durable local export: genesis/local-dev/shared-rust-discipline/portable/.
Strict content CID: bafkreiheu2dri4l5uxhv26jncfxsry26outcolsjw5zd42eazc3sacijxa.

## Residual

The portable representation is generic content, not a policy execution language.
Consumers must explicitly validate supported semantics and authorize local standing.
Legacy JSON-as-DAG-CBOR addresses remain verification-compatible historical inputs;
they are not relabeled in place. No DHT publication or adoption is asserted.

Final source-review follow-up: portable index writes now use the existing
same-directory tempfile/atomic replacement primitive and reject symlink or
nonregular existing indexes. The regression proves a symlink to the legacy
policy registry cannot change its target. Portable CLI suite: 4 passed,
`EXIT=0`; touched-package all-target Clippy after the fix: `EXIT=0`.

## Parent integration closure — 2026-11-02

The local delivery is now verified: normal `just gate rust-style`, `just gate
orchestrator` and `just gate eprfs` all returned `EXIT=0`. EPRFS has 1,970 passing
Rust tests across 130 suite summaries, zero failures and zero ignored tests, plus
workspace formatting and all-target Clippy. Six Rust-style a2o scenarios / 18 steps
pass. Brit's real offline installation and 14-file zero-debt trial pass. Earlier
pending-gate statements above describe the seat's original checkpoint; they are
resolved by this integration evidence. Canonical delivery details and limitations
are in `task-srd-5-report.md`. Network authority/witnessing and later scope/checker
migration remain open; no commit, push, remote CI or appointed acceptance is claimed.
