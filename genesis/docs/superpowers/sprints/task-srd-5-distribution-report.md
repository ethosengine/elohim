---
id: task-srd-5-distribution-report
status: DONE_WITH_CONCERNS
gap: plans__2026-11-02-shared-rust-discipline#5
actor: agent:implementer@codex
class: implementation-report
created: 2026-11-02
serves: rust-style
cites: []
---

# Standalone addressed checker delivery

Brit now holds the existing checker as an immutable addressed source distribution,
and has a verified private installation built from that holding without an Elohim
checkout dependency or network access. This completes the distribution prerequisite;
the parent owns final gate acceptance and the remainder of SRD-5. No publication,
commit, push or network adoption was performed.

## Exact holding and installation

Source archive CID:
`bafkreiekgcq6dzpw2r5asipbeyjlvbffrac72e4pjtjulvviwmghvpcx2u`.
The archive is 283,187 bytes and two separate assemblies produced identical bytes.
It lives in Brit's `.epr-meta/rust-style/objects/`, selected by local `source.json`.

The producer retains original checker/core/meta/seam-contracts source, manifests,
locks, tests and checked-in Cargo configuration. It explicitly projects the eprfs
workspace to the two dependency members while preserving inherited declarations.
The archive carries the original workspace CID, per-file identities and license
declarations; existing component/root license files are included when present.
This repository has no separate license text file for these four crates; their
declared SPDX identifiers and available README notices are retained, not rewritten.

The checker remains `publish = false`. Its compiled method reads first-party source
inventory, so a normalized registry package would not carry the needed sibling
source layout. The earlier registry-preparation report is superseded by this
explicit source-distribution mode; no registry package is called qualified.

The Bash installer verifies the archive CID and digest, rejects links, traversal,
duplicate paths and oversized archives, and requires a normal absolute private
prefix with no symlink ancestry. Its explicit physical `/tmp` scratch root refuses
repository/Cargo-config ancestors and ignores consumer `TMPDIR`. It builds with
Rust 1.96.1, cleared native RUSTFLAGS and one job in an artifact-specific Cargo
target directory, then records the binary digest. `--verify` runs no Cargo and
refuses changed source or binary bytes. There is no PATH or sibling fallback.

Installed binary:
`elohim/brit/.eprfs/tools/rust-style/bin/epr-rust-style`.
Its local receipt records binary SHA-256
`9be036e4518e7e6822a63a614e79f04336eba18c7f5aeb589754af0531d0950d`.
This digest identifies this device's build, not a promise of reproducible binary
bytes across hosts or compiler environments. The checker toolchain changes no
Brit crate MSRV requirement.

## Qualification evidence

- `python3 -m unittest discover -s genesis/orchestrator/rust-style -p test_source_distribution.py`: four tests pass, `EXIT=0`. This suite now runs in the existing rust-style verification recipe.
- `bash elohim/brit/scripts/ci/test-install-rust-style.sh`: PASS, `EXIT=0`. Fake-Cargo fixtures prove refusal/caching boundaries only; they are not the real build evidence below.
- Two `source-distribution.py` assemblies to different output directories: identical archive CID and SHA-256, `EXIT=0`.
- Actual installer with `CARGO_NET_OFFLINE=true` and the declared native pool slot: built from `/tmp/brit-rust-style.*`, `EXIT=0` (3m54s). Every first-party dependency came from the archive's layout. Third-party sources were available in the device cache; a cold consumer needs those locked crates from crates.io.
- Installed `--export-discipline` using Brit's held guide: exact equality with the reviewed parent's exported policy/guide/method references, `EXIT=0`. Method CID is `bafkreidmczh47krdmia4qsv4sgmovx6cyxj4d4bjvtybpot6qb2ft36h2y`.
- `test-rust-style.sh --native <installed-binary>`: PASS, `EXIT=0`, including explicit enrollment, unexplained suppression refusal, dirty repair not hiding committed failure, explained exception and corrupted-guide refusal with committed closure isolation. The parent subsequently strengthened this fixture's baseline to retain inherited debt; its later rerun belongs to parent integration evidence.
- `cargo test --locked --all-targets --manifest-path genesis/orchestrator/rust-style/Cargo.toml` from a separate `/tmp/brit-style-qualification.*` archive extraction: 18 ratchet and 12 reach tests pass, `EXIT=0` (1m05s). Cargo was offline and inherited `GIT_*` overrides were cleared, with system/global Git configuration disabled. This is controlled-environment qualification, not a hostile-environment isolation claim.
- Installer `--verify` after qualification: `EXIT=0`; installed binary still matches its receipt. Cargo berth released before parent full gates.

Gate-evidence: brief `epr flow context` returned no current assertions and an
ambiguous gate list. Parent explicitly selected the existing `just gate rust-style`
owning gate and is running final integration gates. This report does not replace
their exit verdict with the focused evidence above; HOLD preserves that boundary.
Base inspected: `d7dc62fdd328f3c50b87ded46b34f599802a97bb`. Scoped commits: none.

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
