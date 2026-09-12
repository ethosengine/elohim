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
| Focused open | `focused_open_is_byte_identical` | `62b0037d87ea035b97dc2aeebb858e0cd50714c2c6e22720c28c365e917f2e6d` |
| Whole open | `whole_open_is_byte_identical` | `6f55d1ba8e0e0ba5b0cb13b050998435fe8053b0b51905cb5f9424d92e7d0f63` |
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

## 2026-09-11 — fix round 1 of Task 1.1's review

Two independent changes to the `lens:` line landed in the same round, so both are re-baselined
together (once). (1) `render_lens` now appends a short CID and a `renew:` slot to the line —
`lens: {level} · stated {…} · revealed {…} · {defaults} · cid {short} · renew: none (no tending
record)` — the short CID via the existing `crate::flow::short_cid` (never a wall-clock date: that
would re-baseline this digest daily; real expiry arrives with station 4's tending record). (2)
`tests/common/mod.rs`'s `contract_value()` now declares an EXPLICIT `lens_table` (identical
values to `lens::builtin_table()`) rather than only inheriting whatever the live contract file
happens to carry, so `provenance.defaults` on these two golden fixtures reads `"declared"`
instead of the earlier round's `"builtin"` — a value that appears in the line and therefore in
the digest, even though neither fixture's resolved level, choices, density or scaffold changed.
`GOLDEN_REFUSAL` is unchanged again, for the same reason as round 1.

## 2026-09-11 — station 1, Task 1.2

`render()` gained the honesty floor: one new line inserted immediately after the `lens:` line at
every lens — `recipe <short cid> · lens <short cid> · selection: <rule, clipped> · omissions: N
· receipts: N` (see `src/flow/memory/recall/render.rs`'s `render_floor_line`, and
`src/flow/memory/recall/lens.rs`'s `RenderFloor::declared`, a fixed constant never read from a
flag or the contract). Both golden fixtures resolve to `standard` (neither session claims an
actor), so they exercise the "keep today's rendering, insert one floor line" path — the
`minimal`/`simple` two-line-orientation collapse and the content-floor candidate truncation
this task also adds are untouched by these two fixtures and are covered instead by
`flow_memory_recall_lens.rs`'s new tests. `GOLDEN_FOCUSED` and `GOLDEN_WHOLE` re-baselined;
`GOLDEN_REFUSAL` is unchanged, for the same reason as both earlier rounds — a refusal never
reaches `render()`'s orientation/lens/floor preamble at all.

## 2026-09-11 — station 3 (Task 3.1)

No rendering CODE changed. `.epr-meta/elohim/algorithms/recall-contract.json` bumped `version`
11 -> 12, added a `"question_bank"` pointer, and moved `lens_table.levels.minimal.density_bytes`
1500 -> 2000. `tests/common/mod.rs`'s `contract_value()` builds its fixture from
`live_contract()` (the real file on disk) and overrides only `source_roots`,
`ceremony.defaults.scope`, `ceremony.providers.alternative` and `lens_table` — `version` and the
new `question_bank` key pass through unchanged. Every rendered view prints a `recipe <short cid>`
in its honesty-floor line (Task 1.2), and that CID is `Contract::method_cid()` over the WHOLE
contract's raw bytes, so ANY byte in the live contract moving — not only the `lens_table` this
fixture happens to override — moves it. `GOLDEN_FOCUSED` and `GOLDEN_WHOLE` re-baselined;
`GOLDEN_REFUSAL` is unchanged, for the same reason as every earlier round.
