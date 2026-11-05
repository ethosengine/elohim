# Brief T10 + T11 — steward/node and ark (Sonnet 5.5; one leg, two write-sets, two commits)

Worktree: `/projects/elohim/.worktrees/rg1-steward` (branch `regenesis/steward`, `.family=sprint`, submodules initialised). Envelope (`agent:steward-ark-implementer@claude-sonnet-5-5`, `--task regenesis-1-T10`). brit first. **Prerequisite:** `crates/elohim-address` + `elohim-epr 0.2` on your branch (rebase onto L1); T2's shard protocol shape (`/elohim/shard/2.0.0`, `shard_cid: ByteCid`, stripes) from `task-rg1-storage-report.md` — else code to plan §2.5.7 and state the assumption.

Read first: plan §0, **§2.5 (items 2, 7, 14 govern)**, §2.2 rows T10 and the T11 line in §2.5.14; `design-gate.md` byte-profile 2 (external checksums stay evidence; ark's pin crosses), dna-blast 7 (steward has no gossipsub; shares the shard protocol); `steward/node/src/pod/actions/storage.rs`, `pod/cli.rs:300-335`, `p2p/protocols.rs:15-25`, `p2p/transport.rs:40-55`; `elohim/ark/core/src/task.rs:220-250` and its tests; `.claude/memory`-style note in CLAUDE.md: steward/node uses `RUSTFLAGS=""` (native) and its own cargo slot (`cargo-pool key`).

## Deliverables
T10 (commit 1): `pod/actions/storage.rs` — the 23 `blob_hash` sites → `blob_cid: ByteCid` parsed with `ByteCid::parse` at the boundary; its own RS keyed by byte CID and **striped** per §2.5.7 (reuse `elohim_epr::profile::Erasure` ids); `pod/cli.rs:317,327`; `p2p/protocols.rs:20` → `/elohim/shard/2.0.0`; `p2p/transport.rs:49`; tests re-minted from kernel vectors. `just gate steward-node` → `gate-T10-steward-node.log` with `EXIT=`.
T11 (commit 2): `elohim/ark/core/src/task.rs:234` — the artifact pin accepts a byte CID (`ByteCid::parse`) as the native identity; an externally supplied sha256 remains labelled evidence verified on ingestion (keep a `sha256_evidence: Option<[u8;32]>` field or equivalent — read the surrounding contract before choosing); the "supplied CID must equal raw sha256 CID" rule is replaced by "supplied CID must be a canonical byte CID; if sha256 evidence is present it is checked against the bytes too"; tests `task_pin_accepts_byte_cid`, `task_pin_refuses_sha2_raw_cid_as_identity`, `sha256_evidence_is_verified_when_present`. `just gate ark` (or the manifest's name) → log with `EXIT=`.

## Hand-off
Report `task-rg1-steward-ark-report.md` (`status: claimed-not-verified`; both `landed_commit`s; `verified_by:` blank; `concerns:`; `actor`).

Do NOT: write a prefix validator; keep `blob_hash`; edit storage/doorway/epr files; run anything in the main checkout.
