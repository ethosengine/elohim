---
id: task-brit-feedback-source-report-20261111
gap: plans__2026-11-11-brit-feedback-operation#2
actor: agent:orchestrator@gpt-6-astra
status: DONE
commits: [4487b67b2f80def79bd851c9845e0461fea6c9c5, 7ba27c3f497b382e72431d75a294339ac16bf54c]
---

# Retained feedback source implementation

Implementation and owning-gate verification are complete for the retained-source station. Publication, installed qualification and experienced recovery remain separate sprint stations; this report does not claim them.

`EvidenceResources` now exposes explicitly supplied feedback entries. The repository path implementation uses the existing contextual policy loader and verified `FeedbackStore` reader. A `feedback:repair` Intent selects that boundary; the reader checks the exact occurrence CID and record kind. Descriptive labels cannot replace native evidence. Ordinary document sources retain their body-CID verification. Captured immutable views default to an unavailable private resource rather than reading device files.

Retained records establish availability only. Original Intent, commitment, actor and occurrence CIDs remain unchanged; acceptance still requires the existing appointed process. Corruption is reported separately from an unavailable record. The store intentionally has no permanent reclamation tombstones, so a missing CID reports **not retained; absent or reclaimed history is unknown**. No new history store, retention policy, source label or network entity was introduced.

## Verification

Gate-evidence: `env RUSTFLAGS= RUSTUP_TOOLCHAIN=1.96.1 CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__eprfs/dev CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 cargo test --manifest-path elohim/eprfs/Cargo.toml -p elohim-epr-cli --test flow_feedback_source --test flow_reconciliation` — **`EXIT=0`**, five new source tests and eight existing reconciliation tests passed. Log: `genesis/a2o/reports/recovery/brit-feedback-20261111/source-tests-verified.log`.

Gate-evidence: same environment, `cargo test --manifest-path elohim/eprfs/Cargo.toml -p elohim-epr-cli --lib captured_view_never_opens_private_feedback_as_an_implicit_resource` — **`EXIT=0`**, one isolation test passed. Log: `source-view-isolation.log` in the same proof directory.

The live candidate's `flow context` on the original installation observation returned **`EXIT=0`**, `issues: []`, original actor eligibility true, and `acceptance-unestablished`. The original occurrence `bafyreiapmrlppcifcprcswp75f6a5agxgc5dxh5u6hrprc2n7jgzkl3hbi`, Intent `bafyreifby43qgjba66pnmcbjim572r55c5lgupwnwd4lxne4u3jqtoj47e`, commitment `bafyreifzvj7zvkpzntflxmxt2yxl2vcyfvvhcnoypzzkcdv6rte47nejuq` and actor claim `bafyreibz52l5ey42o74rsv4xikxjmzccu2ijf3mdfoh5c7kyzzgjyecswa` are preserved. Evidence: `source-live-context.json`.

The initial focused batch used jobs=2 instead of the configured eprfs cap=1 and failed because fixture actor registration lacked a Git worktree. A second batch used the correct cap but exposed the expected reference-admission guard: feedback Intents cannot be appended without reading retained feedback in the same transaction. These failures remain in `source-tests.log` and `source-tests-final.log`; neither is green gate evidence. Both runs independently executed the existing 14 acceptance tests successfully before failing the source fixture suite. Final fixtures use lawful reference admission for live context and explicitly supplied in-memory historical snapshots for unavailable sources.

## Integration boundary

Changed paths: `src/flow/acceptance.rs`, `src/flow/reconciliation.rs`, the shared `load_policy` extraction in `src/feedback/mod.rs`, a private-view unit test in `src/view/readings.rs`, `tests/flow_feedback_source.rs`, and the existing reconciliation entry in `seam-registry.yaml`, all under `elohim/eprfs/epr-cli/`.

Owning gate: **`just gate eprfs` passed, EXIT=0**, at integrated commit `7ba27c3f497b382e72431d75a294339ac16bf54c`: 2,292 Rust tests across 147 suites, zero failures or ignored tests, plus the invoked Python fixtures. Evidence: `delivery-eprfs-boundary-retry.log`. The first full run is retained in `delivery-eprfs-final.log`: the dependency-boundary test read the new source subdirectory as a file. The repaired walker recursively checks production Rust files and preserves its assertions and test-file exclusion. The package-only focused attempt was interrupted while rebuilding a different feature closure; it is not passing evidence.

The integrated executable re-read the original installation occurrence and returned the unchanged Intent and commitment CIDs, eligible actor attribution, `issues: []` and `acceptance-unestablished` (`integrated-source-live-context.json`). Independent integration and delta reviews found no actionable issue. Rust-style, the 460-test EPR core gate, the five-scenario/29-step story, a2o lint/format/types, Gherkin and 2,372 package projection checks also passed within their recorded source boundaries. Technical verdict and appointed acceptance remain separate native acts.

Story review after the wording revision returned READY with correctness 0, interpretability 1 low and preference 0 in one fresh round. It resolved the fifth scenario's ambiguous source-reading wording; the retained low note is that “capacity returns below reset” leaves the retained-byte quantity implicit. The preceding round had correctness 0 and interpretability 1 on the fifth scenario. Evidence: `story-blind-final.md` and `story-cucumber-isolated.log`.
