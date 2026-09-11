# Recall golden capture — station zero

Task 0.1 of the governed-discovery stations 0–3 plan
(`.superpowers/sdd/2026-09-11-governed-discovery-stations-0-3-plan/task-0.1-brief.md`):
pin today's rendering of `elohim/eprfs/epr-cli/src/flow/memory/recall.rs` by digest, before
any function in it moves. `tests/flow_memory_recall_golden.rs` runs the shipped `epr` binary
against the station-five fixture (`tests/common/mod.rs`'s `repo()`) and asserts the sha256 of
three renderings against the pinned constants below. When a later station's split changes a
digest, that is either a real regression (fix the split) or an intentional rendering change
(re-baseline here, in a commit that says why).

## Captured

- Source commit: `f8e1aaa700c19c5eeeeeb6b3571917211bc83f43`
- `epr` binary (debug, `CARGO_TARGET_DIR=/tmp/eprfs-gate-target`) sha256:
  `03377c027d2f83b750c3cd70e7dfc3dd723d1fce8f6815f9a38655a0a1d6da13`
- Capture command:
  ```
  env RUSTFLAGS= CARGO_TARGET_DIR=/tmp/eprfs-gate-target CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 \
    cargo test --manifest-path elohim/eprfs/Cargo.toml -p elohim-epr-cli \
    --test flow_memory_recall_golden -- --nocapture
  ```

## Digests (sha256 of stdout, two fields normalized out — see below)

| Rendering | Test | Digest |
|---|---|---|
| Focused open | `focused_open_is_byte_identical` | `7e4b4a3e2b6b7502f05d5498e84dab53fdb2a1e2841835e519410bb3ad5dfc53` |
| Whole open | `whole_open_is_byte_identical` | `52f65a4ee15db636ae50474a72b3de0088d1c257f00d43a3d8030af9878e6781` |
| Refusal | `refusal_is_byte_identical` | `882890b4af2e5f60f4d6fbc322377fe4eb9bc12ad495fe4b435fb8d251b1a061` |

## A discovered seam: two ambient, non-algorithmic fields had to be normalized

The naive "hash raw stdout" design in the task brief's skeleton digests two fields the
recall renderer legitimately produces but that are **not** part of the pinned rendering shape:

1. `--root <path>` / `--contract <path>` — the renderer echoes the fixture's own
   `--root` value back into its "Linked choices" next-command suggestions.
   `tests/common/mod.rs`'s `repo()` returns a `tempfile::tempdir()` with a random suffix, so
   this path differs every run.
2. `Usage: elapsed_seconds <n> · …` — a measured wall-clock duration, sub-millisecond and
   never bit-identical across two invocations of the same binary on the same fixture.

Both are real product behaviour (not test bugs), so `flow_memory_recall_golden.rs`'s
`run_text()` normalizes them (`<FIXTURE_ROOT>`, `<FIXTURE_ELAPSED>`) before hashing — pinning
what the renderer *decides to say*, not incidentals a given run happened to land on. Verified
stable across 3+ independent `cargo test` invocations after normalization (previously: 2 of the
3 renderings changed digest on every run). If a future split introduces another ambient field
(another timestamp, another absolute path, a PID, …), add it to the same normalization rather
than widening this note.

## Re-baselining

Re-run the capture command above, take the `left:` values from the panic output (one per
failing test — `focused_open_is_byte_identical`, `whole_open_is_byte_identical`,
`refusal_is_byte_identical`, in that order when run single-threaded), paste them into the
`GOLDEN_*` constants, and update this table plus the source commit. A re-baseline commit must
say *why* the rendering changed.
