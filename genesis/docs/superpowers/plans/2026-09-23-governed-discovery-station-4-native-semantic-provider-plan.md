---
title: Governed discovery — station 4 at L0 — a native semantic provider under a pinned model, the fold as a measure, and the mine gate retired
id: governed-discovery-station-4-native-semantic-provider-plan
status: proposed
class: devflow
serves: recall-reaches-authority
process_subdomain: memory
date: 2026-09-23
refines: genesis/docs/superpowers/specs/2026-09-12-memory-search-scale-three-seams-design.md
informed-by:
  - genesis/docs/superpowers/specs/2026-09-11-governed-discovery-journey-lens-graduation-design.md
  - .epr-meta/elohim/algorithms/recall-contract.json
cites:
  - "memory-search-scale-three-seams-design | the spec this plan refines: §2 index-as-measure, §3 native providers and the visitor rule, §8 station 4 row; two store/embedder deviations declared against its §3 table | sha256:c119695543a2854f | path: genesis/docs/superpowers/specs/2026-09-12-memory-search-scale-three-seams-design.md"
  - "governed-discovery-journey-lens-graduation-design | the carrier: the Provider trait the Semantic provider implements, the lens the fused first screen renders under, and the L0 rung this station sits on | sha256:77030654da24c3e0 | path: genesis/docs/superpowers/specs/2026-09-11-governed-discovery-journey-lens-graduation-design.md"
  - "governed-discovery-stations-0-3-plan | the predecessor whose stations 0-3 are all checked; its gate command, seam files and golden fixtures are this plan baseline | sha256:a0be77a4bc3f6dc7 | path: genesis/docs/superpowers/plans/2026-09-11-governed-discovery-stations-0-3-plan.md"
  - "recall-codex-trail-sprint | the sprint that left the bank at 5 of 6 with q-hook-binary the named lexical miss this station semantic route reaches at rank 1 | sha256:8fc535d804aab415 | path: genesis/docs/superpowers/plans/2026-09-22-recall-codex-trail-sprint.md"
  - genesis/research/search-discovery-incumbent-power-and-p2p-inversion-2026-09-11.md
  - genesis/data/timeline/backlog/arch-dataplane-borrows-backlog.md
  - genesis/data/timeline/backlog/agentic-context-tooling-consolidation-queue.md
---

# Governed discovery — station 4 at L0

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Tasks carry the checkbox (`- [ ]`) — that is the commitment grain `epr flow project` mints and `epr flow claim` takes; steps are numbered and tracked inside the task.

**Goal:** Give the recall executor a second candidate route whose method is declared, so a question phrased in words the authority does not use still reaches it. The route is a native `Provider` (`ranking_known: true`) that ranks by cosine under a pinned embedding model, over a fold that is a rebuildable projection of the declared source roots, declared as an `IndexMeasure`. Its freshness is a fold-lag bound that replaces the manual MemPalace mine gate on the SessionStart headline. The palace stays a declared visitor and second opinion; the contract's `semantic_provider` stops naming it.

**Why now:** the three-seams spec registers this as station 4 and schedules it once stations 0–3 land; all 0–3 tasks are checked. The habit `recall-reaches-authority` is red on its rolling window; the deterministic bank reaches 5 of 6 lexically, and the one miss is exactly the class a semantic route answers (below).

**Spec:** `genesis/docs/superpowers/specs/2026-09-12-memory-search-scale-three-seams-design.md` §2 (an index is a measure), §3 (seam 1 — native providers, the visitor rule), §8 (station 4 row). Carrier: the governed-discovery design's `Provider` trait and L0→L3 ladder.

## Spike evidence (2026-09-23, this checkout)

The palace's own model already on disk (`~/.cache/chroma/onnx_models/all-MiniLM-L6-v2/onnx/model.onnx`, sha256 `4f148ba8ae9c2c7f…`, Apache-2.0, 384 dims; tokenizer sha256 `da0e79933b9ed517…`) was run through Python `onnxruntime` 1.30 + `tokenizers` 0.23 (both installed) over each bank question's own declared scope, chunked by markdown heading / Python def / 1,500-byte windows, mean-pooled, cosine. Script: session scratchpad `semantic_spike.py`.

| Question | Scope | Files / chunks | Lexical today | Semantic rank of the declared authority |
|---|---|---|---|---|
| q-remine | `.claude/skills` | 67 / 654 | reached (ranked `converge` first before S1) | 2 |
| q-corrections | `.claude/skills` | 67 / 654 | reached | **1** |
| q-top-red | `.` (capped at 700 files) | 700 / 3,988 | reached via the root authority set | 7 |
| q-hook-binary | `.claude/hooks` | 46 / 316 | **the named miss** | **1** |
| q-body-scan | `.epr-meta/elohim/algorithms` | 2 / 16 | reached | 2 |
| q-journey-folds | `.claude/epr-meta` | 7 / 66 | reached | 3 |

Two facts the plan is built on: the two routes miss *different* questions, so fusion for order plausibly reaches 6 of 6; and the fold is expensive on this CPU (700 files took 223 s, 46 files 17 s) while a single query embedding is well under the provider budget — so the fold is incremental and off the query path, never rebuilt at `open`.

## Decisions taken here (the operator questions the spec left by name)

- **Q2 native semantic route timing** — now; station 1's trait is the seam and station 3's bank is the measure.
- **Q4 which model, how pinned** — `all-MiniLM-L6-v2` (Apache-2.0, 90 MB fp32 ONNX, 384 dims), the bytes already on this machine. License and size pass. Pinned as a `ModelPin` whose `model_bytes` is the CID of the ONNX file; the tokenizer is pinned beside it. A different model is a new measure version, judged by the bank's reach rate folded against each model's CID — never a config edit.
- **The model is a governed artifact, not a setting** (operator, 2026-09-23: "their fit-for-purpose feels like it should eventually be an EPR-governed artifact"). The pin resolves to a Manifest EPR — the same kind the `IndexMeasure` declaration is — carrying bytes CID, tokenizer CID, license, dims, pooling, max tokens and source. Fitness is *evidence on that artifact*: `recall-bank-reach@1` folds with `env model=<cid>`. This station declares and measures it; attestation and any hop beyond `Private` stay station 9 (the visitor test / confidentiality-plane rows 10–11).
- **Two declared deviations from the spec's §3 table, with the reason and the way back:**
  1. *Store*: the spec names sqlite-vec in the storage service's database (the L2 home; elohim-storage already links `rusqlite` through holochain_types). At L0 the fold store IS SQLite: `rusqlite 0.37` + `libsqlite3-sys 0.35` with the bundled `sqlite3.c` are already in the cargo cache (`/opt/rust/cargo`), resolve `--offline` (verified 2026-09-23 in a scratch crate), and the bundled build compiles with `SQLITE_ENABLE_FTS5`. So the fold lives at `.eprfs/status/index/<measure-cid>/fold.sqlite` — a `chunks` table (path, section, fingerprint, demoted_at, vector BLOB) plus an FTS5 virtual table — and the only deviation is **sqlite-vec is not cached and crates.io returns 403 from this container**, so vectors are ranked by brute-force cosine in Rust over the BLOB column (sub-100 ms at the tens-of-thousands-of-chunks scale one checkout has). Adding sqlite-vec later changes the ranking query, not the schema or the `Provider` interface. Cost: the bundled SQLite compile adds a one-time ~1–2 min to the eprfs gate at `CARGO_BUILD_JOBS=1`.
  2. *Embedder*: no Rust ONNX runtime is reachable (`ort`, `candle` absent from the cache; registry blocked). The embedding step is a **declared fold procedure**: a small Python program pinned by its own CID in the model manifest, invoked through the existing `bounded_process` envelope under `provider_bytes` / `provider_seconds`, refusing to run when the model bytes on disk do not hash to the pin. This is *not* the visitor shape — the ranking is native and the method is fully declared, so `ranking_known: true` is honest. Swapping to a native runtime later is a procedure version change with no interface change. Runtime access is **checked, not inferred**: the provider probes interpreter, modules and model at call time and reports `unavailable: <reason>` on the screen.

## Global constraints

- Native gate after every task that touches Rust, EXIT echoed on its own line: `env RUSTFLAGS= CARGO_TARGET_DIR=/tmp/eprfs-gate-target CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 cargo test --manifest-path elohim/eprfs/Cargo.toml -p elohim-epr-cli; echo EXIT=$?`, plus `cargo fmt --all --manifest-path elohim/eprfs/Cargo.toml -- --check` and `cargo clippy --manifest-path elohim/eprfs/Cargo.toml -p elohim-epr-cli --all-targets -- -D warnings`. Claim the cargo berth first (`berth claim cargo`).
- Integrated gate before any commit that touches hooks, measures, a2o or the contract: `just gate memory-ceremony; echo EXIT=$?`.
- Nothing mints a kind. `IndexMeasure`, `ModelPin`, `FoldAttestation`, `ShardManifest`, `FoldState`, `RankingMethod::{Vector, Fused}`, `ReachBound`, `SurfaceRule`, `Retention` come from `elohim_epr_rea::index`; the model manifest and the measure declaration are Manifest EPRs.
- Honesty floor: every candidate prints its producer and method CID; a fused screen prints each producer's rank beside the result; standing never enters any score (asserted at the render floor). A stale fold is used and *labelled* (`fold N files behind`), never silently trusted and never silently dropped. The private chain (`.eprfs/status/recall/`, `.claude/worktrees/`) never enters the fold.
- The recall contract's `version` moves on every byte change (v14 → v15 at task 4.1); every receipt pins the new method.
- One new Rust dependency, `rusqlite = { version = "0.37", features = ["bundled"] }`, already cached and offline-resolvable; nothing else (the registry is unreachable). No `#[ignore]` tests: the live-model test runs where the model is present (`just gate memory-ceremony` on a developer checkout) and the fixture-embedder test runs everywhere.
- Path-limited commits (`git add -- <paths>`); no pushes from this lane.

## File structure

| Path | Responsibility |
|---|---|
| `.epr-meta/elohim/algorithms/embedding-models/all-minilm-l6-v2.json` | the model as a governed Manifest EPR: bytes CID, tokenizer CID, license, dims, pooling, max tokens, source, resolve paths, procedure CID |
| `.epr-meta/elohim/algorithms/recall-semantic-index.json` | the `IndexMeasure` declaration (`recall-semantic-index@1`): chunk rule, `ModelPin`, `Vector{cosine}`, reach ceiling `SelfScope`, surfaces, retention, `fold_lag` bound |
| `.epr-meta/elohim/algorithms/recall-contract.json` | v15: `ceremony.providers.semantic`, `discovery.semantic_provider: "semantic"`, fusion recipe for the focused first screen |
| `elohim/eprfs/epr-cli/embedder/embed.py` | the pinned fold procedure (stdin JSON texts → stdout f32 vectors; refuses on pin mismatch) |
| `elohim/eprfs/epr-cli/src/flow/memory/recall/index.rs` | the fold in SQLite (`fold.sqlite`: `chunks` + FTS5): incremental build, fingerprints, demotion, `FoldAttestation`, lag |
| `elohim/eprfs/epr-cli/src/flow/memory/recall/providers.rs` | `Lexical` (FTS5/BM25) provider (task 4.8) |
| `elohim/eprfs/epr-cli/src/flow/memory/recall/providers.rs` | `Semantic` provider; `Fixture` embedder for tests |
| `elohim/eprfs/epr-cli/src/flow/memory/recall/discovery.rs` | fusion for order in `first_screen` |
| `elohim/eprfs/epr-cli/src/flow/memory/recall/render.rs` | producer ranks and method CID on every candidate line |
| `elohim/eprfs/epr-cli/src/flow/report.rs` | `HEADLINE_ORDER`: `mempalace` → `index` |
| `.claude/epr-meta/measures.yaml` | `index-fold-lag@1`, `index-fold-lag-ceiling@1` (headline `index`), `index-fold-files-per-run@1`, `recall-bank-reach@1` |
| `.claude/hooks/pickup-semantic-surfacing.py` | reads the semantic provider through `epr`; spawns a detached bounded fold when lag > 0 |
| `elohim/eprfs/epr-cli/tests/flow_memory_recall_index.rs` | station 4 tests |

---

## Station 4 — the native semantic provider

### Task 4.1: Declare the model and the measure as governed artifacts (no behaviour change)

- [ ] Task 4.1: Declare the model and the measure as governed artifacts

1. **Step 1:** Write `embedding-models/all-minilm-l6-v2.json`: `artifact_type: model-manifest` (a Manifest EPR), `model_bytes` = CIDv1 (raw, sha256) of `model.onnx`, `tokenizer_bytes` likewise, `license: Apache-2.0`, `dims: 384`, `pooling: mean`, `normalize: true`, `max_tokens: 256`, `source: sentence-transformers/all-MiniLM-L6-v2`, `resolve: [~/.cache/chroma/onnx_models/all-MiniLM-L6-v2/onnx, $EPR_EMBED_MODEL_DIR]`, `procedure: <CID of embed.py>` (filled at 4.2), `fitness: recall-bank-reach@1` (the fold that judges it).
2. **Step 2:** Write `recall-semantic-index.json` as an `IndexMeasure`: `measure: recall-semantic-index@1`; `chunk_rule` = CID of the declared rule object (`{markdown: heading ≤ 4, python: def/class, other: passage_window_bytes windows, max_chunk_bytes: 2000, max_chunks_per_file: 12}` — the same sectioning `passage.rs` already applies); `embedding: ModelPin{model_bytes, license, dims}`; `ranking: Vector{Cosine}`; `reach: ceiling SelfScope`; `surfaces: SurfaceRule::new(kinds, contract.source_roots ∖ discovery.exclude_directories)`; `retention: DemoteAfter{count: 1, per: fold}`; `fold_lag: Bound{limit: 25, unit: files, sense: Ceiling, source: Declared}`.
3. **Step 3:** Contract v15: `ceremony.providers.semantic = {kind: "semantic", measure: ".epr-meta/elohim/algorithms/recall-semantic-index.json", ranking: "cosine under the declared ModelPin", optional: true}`; `discovery.semantic_provider: "semantic"`, `discovery.semantic_output: "native fold candidates; method CID printed"`; the `mempalace` provider stays declared with `kind: mempalace` and a new `role: visitor`. Bump `version` to 15.
4. **Step 4:** Test (`flow_memory_recall_index.rs`): the measure file deserializes into `IndexMeasure` and `validate()` passes; a copy with `embedding: null` is refused (vector ranking without a pin); the contract's `semantic_provider` no longer equals `"mempalace"`; the contract CID moved and `recall-questions.json`'s `recipe` is updated to it.
5. **Step 5:** Commit — `chore(recall): declare the embedding model and the semantic IndexMeasure as governed artifacts; contract v15`.

### Task 4.2: The embedder as a pinned, bounded fold procedure

- [ ] Task 4.2: The embedder as a pinned, bounded fold procedure

1. **Step 1:** `embedder/embed.py` (stdlib + `onnxruntime` + `tokenizers` + `numpy`): reads `{"model_dir", "pin": {"model_bytes", "tokenizer_bytes"}, "texts": [...]}` on stdin; hashes both files and **refuses with exit 3** on pin mismatch; batches of 32, truncation 256, mean pooling over the attention mask, L2 normalise; writes `{"dims": 384, "vectors": [[f32…]]}`. No network, no writes.
2. **Step 2:** In `providers.rs`, `trait Embedder { fn embed(&self, texts: &[String], contract) -> FlowResult<Embedding> }` with two impls: `PinnedProcedure` (resolves the model dir from the manifest's `resolve` list, runs `embed.py` through `bounded_process` under `provider_bytes`/`provider_seconds`, maps exit 3 to `unavailable: model bytes do not match the pin`, a missing interpreter/module to `unavailable: <reason>`) and `Fixture` (deterministic hashed bag-of-words vectors, 384 dims — test interchange only, declared like the existing `fixture` provider, never live-fit).
3. **Step 3:** Fill the manifest's `procedure` with the CID of `embed.py`; `PinnedProcedure` refuses when the file on disk does not hash to it (the procedure is part of the method).
4. **Step 4:** Tests: fixture embedder is deterministic and normalised; `PinnedProcedure` against a temp dir with wrong bytes reports the pin refusal, not a panic; against a missing interpreter reports `unavailable`. Live test (model present): a two-text batch returns two 384-vectors with cosine(self) ≈ 1.
5. **Step 5:** Commit — `feat(recall): pinned embedding procedure behind a bounded envelope; fixture embedder for tests`.

### Task 4.3: The fold — incremental, fingerprinted, attested, bounded per run

- [ ] Task 4.3: The fold — incremental, fingerprinted, attested, bounded per run

1. **Step 1:** `recall/index.rs`: `epr flow memory index fold [--scope <dir>] [--max-files N] [--embedder fixture]` walks the measure's surfaces with the contract's `exclude_directories` and `first_screen_globs`, fingerprints each file (canonical body sha256, the cites convention), diffs against the `chunks` table in `.eprfs/status/index/<measure-cid>/fold.sqlite`, re-chunks and re-embeds only changed/new files (up to `--max-files`, default from `index-fold-files-per-run@1`), marks removed files `demoted_at` (never deleted — `Retention`), appends vectors to `vectors.f32`, and writes a `FoldAttestation{measure, shard: ShardManifest{atoms, bytes, manifest cid}, heads_at, state, attested_by: actor sidecar ref, at}` — `Complete` when no file remains behind, `Degraded{retried}` when the run cap stopped it, `Failed{why}` on procedure refusal.
2. **Step 2:** `epr flow memory index status [--json]`: lag (files behind or absent), last attestation state and time, measure CID, model CID, chunks, bytes. The store is derived and gitignored (`/.eprfs/status/*` already ignores it); a missing or corrupt `fold.sqlite` is rebuilt, never repaired by hand.
3. **Step 3:** Tests with the fixture embedder on a temp tree: first fold attests `Complete`; editing one file → status lag 1 → a second fold re-embeds exactly one file (manifest diff); deleting a file → its chunks carry `demoted_at` and are excluded from ranking; `--max-files 1` on three changes → `Degraded{retried: 0}` and lag 2; the private recall store under the tree is never folded.
4. **Step 4:** Commit — `feat(recall): incremental semantic fold with FoldAttestation and a per-run bound`.

### Task 4.4: The `Semantic` provider ranks natively

- [ ] Task 4.4: The Semantic provider ranks natively

1. **Step 1:** `Semantic` implements `Provider`: embeds the question once (the *question text*, not the term list — terms are the lexical route's shape), cosine over every non-demoted chunk vector in the fold, best chunk per file, top `search_results` (or the lens's `choice_count`) files; each candidate carries `path`, `score`, `producer: "semantic"`, `method: <IndexMeasure CID>`, `model: <model CID>`, `fold_lag` at answer time, and `best_section` resolved through the existing `best_section` path so the linked `read` choice lands on the passage.
2. **Step 2:** `retrieve()` gains a `"semantic"` arm and `search --provider semantic` works; `providers_for` returns it when declared; absent fold → `unresolved: ["semantic: no fold — run epr flow memory index fold"]`, empty candidates, no error.
3. **Step 3:** Tests: with a fixture fold, a question worded unlike the target's terms ranks the target first; every candidate has `method == measure.cid()` and `ranking_known == true`; a fold 30 files behind still answers and the result carries `fold_lag: 30`.
4. **Step 4:** Commit — `feat(recall): native semantic provider — cosine over the declared fold, method CID on every candidate`.

### Task 4.5: Fusion for order on the focused first screen

- [ ] Task 4.5: Fusion for order on the focused first screen

1. **Step 1:** Contract v15 `discovery.first_screen_fusion = {recipe: "rrf-v1", k: 60, producers: ["local", "semantic"]}` (a `RankingMethod::Fused` declared in the recipe). In `first_screen`, when a focused area or the root authority screen has lexical candidates *and* the semantic provider answers within budget, fuse by reciprocal rank for **order only**; each candidate keeps `ranks: {local: n|null, semantic: n|null}`. When the semantic provider is unavailable or has no fold, the screen is the lexical screen plus one omission line naming why.
2. **Step 2:** `render.rs`: a candidate line prints its producer ranks (`local #2 · semantic #1`) and the method CID handle at `standard` and above; at `minimal`/`simple` one collapsed tag (`fused`). The honesty floor line names the fusion recipe. Assert at the render floor that no standing or human signal enters the order (a unit test feeds a candidate with a `standing` field and checks the order is unchanged).
3. **Step 3:** `open --purpose bootstrap` is untouched (no `--need` → no first screen → no semantic call), so the SessionStart 6 s budget is unaffected. Re-baseline the golden renderings only if a line changed on the fixture, and say so in `tests/fixtures/recall-golden/README.md`.
4. **Step 4:** Tests: fused order equals RRF over the two producer orders on a fixture; a lexical-only target and a semantic-only target both appear on one screen; a stale fold prints `fold N files behind` in omissions.
5. **Step 5:** Commit — `feat(recall): rank fusion for order on the focused first screen; producer ranks printed, standing never summed`.

### Task 4.6: Fold-lag freshness replaces the mine gate

- [ ] Task 4.6: Fold-lag freshness replaces the mine gate

1. **Step 1:** `measures.yaml`: `index-fold-lag@1` (family `index`, unit files, `derive: fold-manifest-vs-tree` — native, re-derivable), `index-fold-lag-ceiling@1` (`headline: index`, `hard: 25`, consumes `index-fold-lag@1`, `skipped` when no fold exists — never zero), `index-fold-files-per-run@1` (default 40, the per-run cap), `recall-bank-reach@1` (fraction of bank questions reached, folded with `env model=<cid>`).
2. **Step 2:** `report.rs`: `HEADLINE_ORDER` replaces `mempalace` with `index`; `mempalace` stays in the vocabulary (as `memkit` did) so a reader asking by name gets its own word back. The headline reads `index: N files behind the fold within hard 25 ✅` or `⚠ failed`, or `skipped — no fold`.
3. **Step 3:** `pickup-semantic-surfacing.py`: replace the `mempalace search` call with `epr flow memory recall search --provider semantic --json` (bounded by the contract, once per session), keep the cosine floor and the recall-hints-not-truth footer, and when `index status` reports lag > 0 spawn a **detached** `epr flow memory index fold --max-files <index-fold-files-per-run@1>` so the lag converges across sessions without blocking a prompt. The palace's own `mine` remains the visitor's maintenance and is no longer a headline gate.
4. **Step 4:** Memory-ceremony skill package (package-first, projected with `just codegen agents write`): the re-mine phase names `epr flow memory index fold` and `index status`; the three palace commands move to a "visitor maintenance (optional)" note. Verifier green.
5. **Step 5:** Tests: 34 existing hook tests plus: the surfacing hook with `epr` reporting lag spawns exactly one detached fold and exits 0; `epr flow report --headline` prints `index:` in `mempalace`'s slot position and `skipped` with no fold.
6. **Step 6:** Commit — `feat(recall): fold-lag freshness on the headline retires the manual mine gate; surfacing reads the native provider`.

### Task 4.7: Evidence, fitness fold, and the close

- [ ] Task 4.7: Evidence, fitness fold, and the close

1. **Step 1:** Build a real fold over the declared surfaces (`index fold` to `Complete`; record wall-clock and chunk count in the atom delta). Run the deterministic bank: `epr flow memory recall sample --question <id> --reader agent:steward@fixture` for all six. Target: ≥ 6 of 6 reached with fusion (baseline 5 of 6; spike ranks in the table above). Fold `recall-bank-reach@1` with `env model=<model cid>` — the first fitness evidence on the model artifact.
2. **Step 2:** The palace removal test: with `ceremony.providers.mempalace` deleted from a copy of the contract, the bank result is unchanged — removing the visitor loses only the second opinion (the spec's done-test for station 4).
3. **Step 3:** Fresh reader of a different tier (context-reset, entry only) on a question phrased without the authority's own terms; judged by a seat of another tier; folds on the recall-journey measures.
4. **Step 4:** `just gate memory-ceremony; echo EXIT=$?` and `just gate eprfs; echo EXIT=$?` green; a2o ceremony profile re-run; one DELTA line in `.epr-meta/recall-reaches-authority.habit.md` (status flips only on the window bound, not here); `python3 .claude/scripts/habits-project.py`; consolidation-queue item 24 gets its disposition; dataplane borrows row 14 gains an "L0 landed; L2 sqlite-vec/FTS5 remains" note.
5. **Step 5:** Commit — `habit(recall-reaches-authority): station 4 at L0 measured — semantic provider under a pinned model`.

### Task 4.8: The FTS5 lexical provider (BM25) behind the trait

- [ ] Task 4.8: The FTS5 lexical provider (BM25) behind the trait

1. **Step 1:** `Lexical` implements `Provider` over the fold's FTS5 table: BM25 (`bm25()` rank), the question's `question_terms` as the match expression with prefix terms, top `search_results` files by best chunk, `ranking_known: true`, `method` = a second `IndexMeasure` (`recall-lexical-index@1`, `ranking: Bm25`, no `ModelPin` — `validate()` refuses a pin nothing uses) declared beside the semantic one and sharing the fold.
2. **Step 2:** Contract v15 declares `providers.lexical {kind: "lexical", measure: ".epr-meta/elohim/algorithms/recall-lexical-index.json", optional: true}`. The existing `local` metadata traversal stays the always-present default; `lexical` is a third producer the fusion recipe may name (`producers: ["local", "semantic", "lexical"]`) once its bank evidence is in.
3. **Step 3:** Tests: on a fixture fold a multi-term question ranks the chunk carrying all terms first; a stemmed inflection matches through the FTS5 tokenizer; the bank sample with `lexical` fused does not lower any question's rank below the two-producer result.
4. **Step 4:** Commit — `feat(recall): FTS5/BM25 lexical provider over the shared fold; three-producer fusion declared`.

---

## Definition of done

- The contract's `semantic_provider` no longer names `mempalace`; a candidate prints its measure CID (the spec's §8 probe for station 4).
- The deterministic bank reaches 6 of 6 through the fused first screen, and the same with the palace undeclared.
- `epr flow report --headline` shows `index:` fold lag in the retired `mempalace` slot; the surfacing hook reads the native provider.
- The model exists as a Manifest EPR with a CID pin, a license, and one fitness fold against its CID.
- Bundled `rusqlite` is the only new Rust dependency (cached, offline-resolvable); every gate above green; one delta in the habit atom; register re-projected.
- The FTS5 lexical provider (task 4.8) is declared and tested; whether it enters the fusion recipe is decided by its bank evidence, recorded in the atom delta.

## Out of scope (captured, not absorbed)

- sqlite-vec (not cached; the ranking query changes, not the schema) and the L2 mirror of both providers in elohim-storage's own database (dataplane borrows row 14 remainder).
- Embedding-model attestation and any hop beyond `Private`; the palace's six-part admission (station 9).
- Human lens negotiation and bi-temporal `as_of` (station 5).
- A native Rust embedding runtime (a procedure version change once the registry is reachable; interface unchanged).
