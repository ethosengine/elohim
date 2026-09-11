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
| Focused open | `focused_open_is_byte_identical` | `a3b304920260fc1a38d903e5645bf985c36603234805cb5c3e20a75e9d189678` |
| Whole open | `whole_open_is_byte_identical` | `8ca20e6f1ae913f42e06b6cac20533121c6e0259e5f84b089463a7379b676b4d` |
| Refusal | `refusal_is_byte_identical` | `882890b4af2e5f60f4d6fbc322377fe4eb9bc12ad495fe4b435fb8d251b1a061` (unchanged — see below) |

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

## 2026-09-11 — station 1 (Task 1.1)

One `lens:` line added after `Guiding context` on every rendered view (the reader lens: WHO is
reading, resolved from the actor sidecar and the recipe's declared `lens_table`, printed with
its stated/revealed provenance and a content-addressed CID — see `src/flow/memory/recall/lens.rs`
and `flow_memory_recall_lens.rs`). `GOLDEN_FOCUSED` and `GOLDEN_WHOLE` re-baselined; `GOLDEN_REFUSAL`
is unchanged because a refusal never reaches `render()`'s orientation/lens preamble — verified by
re-running `refusal_is_byte_identical` unmodified after the lens line landed. The lens CID itself
does not vary run to run: it is `BlobCid::compute_raw` of canonical JSON over `{level,
choice_count, density_bytes, scaffold, provenance}` alone (no timestamp, no path), and neither of
these two fixture sessions registers an actor claim, so both resolve to the same `stated: ["none"]`
/ `standard` lens on every run — confirmed stable by re-running the capture command twice.
