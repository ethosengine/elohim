# Recall golden capture — station zero

Task 0.1 of the governed-discovery stations 0–3 plan (from the repository root:
`genesis/docs/superpowers/plans/2026-09-11-governed-discovery-stations-0-3-plan.md`; the task's
working brief was an untracked scratch file and no longer exists):
pin today's rendering of `elohim/eprfs/epr-cli/src/flow/memory/recall.rs` by digest, before
any function in it moves. `tests/flow_memory_recall_golden.rs` runs the shipped `epr` binary
against the station-five fixture (`tests/common/mod.rs`'s `repo()`) and asserts the sha256 of
three renderings against the pinned constants below. When a later station's split changes a
digest, that is either a real regression (fix the split) or an intentional rendering change
(re-baseline here, in a commit that says why).

**How to read this file.** It is three things in order: a reference (what is pinned and how it
was captured), a runbook (**Re-baselining**, below the normalization note), and a dated history
of every re-baseline. If your golden test just failed, go to **Re-baselining**. The `GOLDEN_*`
constants in `tests/flow_memory_recall_golden.rs` are authoritative; the table here mirrors them
and is updated in the same commit.

Terms the history uses. *Stations* are the numbered steps of the plan that split the recall
executor into smaller files; station zero pinned today's output before anything moved, and the
fixture predates that plan — an earlier plan's fifth station built it — and lives entirely in
`tests/common/mod.rs` (`repo()`: a small repository with two stale citation edges). The *recall entry* is `epr flow memory recall`; its *contract*
(`recall-contract.json`) is the declared algorithm, and its content hash (CID) is printed on
every view as the *recipe*, so any contract edit moves every digest. A *lens* is the reader's
detail level, which bounds how much a view prints; a *floor* is a line no lens ever suppresses.

## Captured

- Source commit (the original station-zero capture; each later re-baseline names its own reason
  in the newest dated entry below): `f8e1aaa700c19c5eeeeeb6b3571917211bc83f43`
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
| Focused open | `focused_open_is_byte_identical` | `46e2d00a401baa78dd2af568958c92c3af2b2ca4012bbf0c7e07cd493a83499f` |
| Whole open | `whole_open_is_byte_identical` | `562a9cd85cf9df15c7ec2bfa8289bb58e2a0fd204e17f9c67563f1750fe6c6b5` |
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

Done when `cargo test --manifest-path elohim/eprfs/Cargo.toml -p elohim-epr-cli --test
flow_memory_recall_golden` (with the capture command's environment) reports all three tests
passing. (`left:` is the value `assert_eq!` actually produced; `right:` is the pinned one.)


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

## 2026-09-22 — contract v13 (recall — Codex's trail sprint)

Focused and whole open re-baselined for ONE reason: the fixture contract's method CID moved
(`bafkreia…65q4` → `bafkreih…yene`) because contract v13 declares new `discovery` keys
(`first_screen_globs`, `short_terms`, `stemming`, `passage_window_bytes`), `limits.resume_commits`
and two source roots. Proof that nothing else changed: substituting the v12 fixture CID back into
each new rendering reproduces the previous pins (`62b0037d…`, `6f55d1ba…`) byte for byte. The
refusal digest is unchanged.

## 2026-09-22 — contract v14 (search spans the declared globs; phrase matches across identifier separators; Python sections are def/class; generated directories excluded)

Focused and whole open re-baselined for ONE reason: the fixture contract CID moved (bafkreih…yene → bafkreib…pv4m). Proof: substituting the previous fixture CID back into each new rendering reproduces the previous pins byte for byte. The refusal digest is unchanged.

## 2026-09-23 — contract v15 (station 4, Task 4.1: the `semantic` provider over `recall-semantic-index@1`; the palace a declared visitor)

Focused and whole open re-baselined for ONE reason: the fixture contract CID moved (bafkreib…pv4m → bafkreih…udfe) because contract v15 declares `ceremony.providers.semantic`, gives `mempalace` `role: "visitor"`, and moves `discovery.semantic_provider`/`semantic_output`. The rendered diff against the v14 capture is that one `recipe` token on line 7 of each view. Proof: substituting the previous fixture CID back into each new rendering reproduces the previous pins (`ec03406e…`, `f8c8970e…`) byte for byte. The refusal digest is unchanged.

## 2026-09-23 — contract v16 (station 4, Task 4.2: `limits.fold_procedure_bytes`/`fold_procedure_seconds`/`fold_batch_texts` declared for the embedding fold procedure)

Focused and whole open re-baselined for ONE reason: the fixture contract CID moved (bafkreih…udfe → bafkreic…gbvi). Proof: substituting the previous fixture CID back into each new rendering reproduces the previous pins (`46e2d00a…`, `562a9cd8…`) byte for byte. The refusal digest is unchanged.
