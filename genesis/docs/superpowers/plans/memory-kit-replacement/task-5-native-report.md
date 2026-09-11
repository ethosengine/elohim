---
id: memory-kit-replacement-task-5-native-report
status: DONE_WITH_CONCERNS
cites: []
gap: plans__2026-09-10-memory-kit-replacement-finish#6
actor: agent:implementer@claude-opus-5
session: mk-replace-native-5
commits: []
---

# Station five (native seat) — the recall executor is native and corrections close by identity

Two verbs landed. `epr flow memory recall <op> --session <id>` is the native bounded-evidence
recall executor: budget counters, a locked session continuation, bounded reads with receipt keys,
recipe-selected providers, projection receipts, a paired deterministic footprint lens, and method
pinning by the contract's raw CID. `epr flow concerns --corrections` reads unresolved corrections by
**identity** — every `run:correction` with no later note carrying `closes:<its exact CID>` — which
replaces the date reading the efficacy analysis named as error one. `--closes` was added to
`epr flow note` so a closure is an explicit act rather than an inference.

Nothing in the kit was deleted. `recall-ceremony.py`, `recall-packet.py` and the three libraries
still run, and their 60 tests still pass; station six (integration) retires them once parity is
accepted. The contract MOVED, because it is an algorithm artifact rather than a script neighbour.

Status is `DONE_WITH_CONCERNS`. Two concerns are stated in full below: one divergence I chose
deliberately (the session's method pin is the contract CID alone, per the plan's mandate, so a
changed executor is reported rather than session-fatal), and one act with durable side effects that
a reviewer should look at directly (I migrated eight already-validated chronicle closures into
native closure notes so the two readings agree).

## What landed

| Path | Role |
|---|---|
| `elohim/eprfs/epr-cli/src/flow/memory/recall.rs` | NEW (~1,900 lines). The whole executor: `Contract` (load/validate/`method_cid`), `Execution` (flock'd continuation, attempts/totals/unmetered accounting), `excerpt`, `discover`, `bounded_process`, `retrieve` (local/mempalace/fixture), `save_receipt`/`load_receipt`, `adopt_receipts`, `sample_balance` + `compare_samples`, the sixteen ceremony operations, the text renderer, the CLI. |
| `elohim/eprfs/epr-cli/src/flow/concerns.rs` | EXTENDED. `corrections(root, since)` + `Correction`/`Closure`/`CorrectionCounts`/`Corrections` + `render_text`. The edge view is untouched. |
| `elohim/eprfs/epr-cli/src/flow/note.rs` | EXTENDED. `CLOSES_SLOT_PREFIX`, `resolve_closes`, a seventh `note_slots` parameter, `note_with_options_closing`, `NoteOutcome::closes`, two unit tests. Additive: a note that closes nothing emits exactly the slot vector it emitted before. |
| `elohim/eprfs/epr-cli/src/flow/mod.rs` | EXTENDED. `epr flow concerns` subcommand + `--closes` on `note` + usage lines. |
| `elohim/eprfs/epr-cli/src/flow/memory/mod.rs` | EXTENDED. `pub mod recall;` and the `recall` interception before the shared option parser. |
| `elohim/eprfs/epr-cli/Cargo.toml` | `rustix = { version = "1", features = ["fs"] }` — a SAFE `flock`, so the session lock needs no `unsafe` block in a crate that has none. Already in `Cargo.lock` transitively; no version moved. |
| `elohim/eprfs/epr-cli/tests/flow_memory_recall.rs` | NEW. 32 tests, driven through `CARGO_BIN_EXE_epr` for the ceremony and through the library for the primitives. |
| `elohim/eprfs/epr-cli/tests/flow_concerns_corrections.rs` | NEW. 9 tests. |
| `.epr-meta/elohim/algorithms/recall-contract.json` | RELOCATED from `.claude/scripts/memory-kit/recall-contract.json`; `governance.source` repointed, `governance.native_executor` added, `.epr-meta/elohim/algorithms/` added to `source_roots`, `recall.rs` added to `ceremony.dependencies`. |
| `.gitignore` | `/.eprfs/status/recall/` — terminal, last in the ladder. |

Path references updated for the move: `recall-ceremony.py` (`CONTRACT` constant + the `--contract`
default), `recall-packet.py`, `recall_ceremony_test.py`, `recall_runtime_test.py`,
`.claude/scripts/memory-kit/.epr-meta` (`dedupe-of`), `genesis/build-manifest.json`, and both a2o
step files (`ceremony-reconciliation.steps.ts`, `collective-memory.steps.ts`).

## The privacy line, structurally

The plan's constitutional section and `genesis/docs/architecture/private-thought-governed-fruit.md`
§2 say recall receipts and continuations are private records: never imported, projected, witnessed
or targeted by feedback. Three mechanisms hold that, rather than a convention:

1. **Location.** Every continuation and receipt is written under `.eprfs/status/recall/<session>/`,
   mode `0600`, in a directory the root `.gitignore` excludes **terminally**. The DIRECTORY is
   named, not its contents, because git does not descend into an excluded directory — so nothing
   inside can be re-included by a later `!` rung without first re-including the directory itself.
   The rule sits last in the `.eprfs` ladder, after the two tracked exceptions (`gap-items/`,
   `memory/`), and `the_recall_store_is_ignored_and_its_neighbours_are_unaffected` asks **git**
   (not the `.gitignore` text) about five paths including those two neighbours.
2. **A named gate.** `refuse_private_import(root, candidate)` is the single home of the invariant,
   applied to every path-shaped CLI input (`--path`, `--scope`, `--search-scope`). It is
   mutation-checked: `a_recall_verb_refuses_to_import_or_project_a_private_receipt` accepts the same
   shape outside the store and refuses it inside, asserts the refusal names the canon, drives it
   through the shipped binary, and separately proves `epr flow memory contribute` cannot take a
   receipt as input. Deleting the gate turns the test red rather than leaving it green.
3. **No verb that could.** The five collective-memory operations the Python entry carried
   (`collective`, `memory-project`, `memory-contribute`, `memory-feedback`, `memory-graduate`) are
   NOT among the sixteen. They are already native verbs one directory up and they take an authored
   request file. Re-exposing them through the recall front door would have given one act two
   addresses and put a receipt one flag away from a governed plane.

What a recall session exposes is what was read (bounded excerpts, each with its own fingerprint and
`fingerprint_scope`) and what was concluded (findings, frontier, outcome). `evidence_snapshots` —
the private working copy of each receipt at judgment time — is stripped from every view by
`relevant_findings`, and `accumulated_findings_remain_progressively_recoverable` asserts its absence.

## Method pinning

The algorithm artifact is `.epr-meta/elohim/algorithms/recall-contract.json`. Its **raw CID** is the
`method` on every session and every receipt. Today that is
`bafkreiefupw3jbkzgz5n5h6xse5kmse44vky26xzscd3lhrv3zsuhdtj3m`
(sha256 `85a3edb48559367ade9fd7913aa6489ce5558d7af99087b59e35de65438e69db`, 11,117 bytes).
A session whose stored method differs is refused with the same words the Python used
("algorithm bytes changed; explicitly start a new session and retain prior receipt"), and the prior
receipt survives the refusal untouched.

## Receipt adoption

`epr flow memory recall --adopt-receipts [--from-dir DIR] [--dry-run] [--json]` relocates
`.claude/memory-kit/recall-executions/` into `.eprfs/status/recall/<session>/`. Run on this tree:

```
examined 28, adopted 27, skipped 1, receipt_digests_verified 0, receipt_digests_mismatched 0
```

All 28 entries are continuations (`<session>.json`); the skip is
`collective-memory-observation-20260909.tar.gz`, which is not a JSON receipt. A content-named
receipt (`<session>-<baseline|close|projection>-<64hex>.json`) has its digest **re-verified against
its own name** and is `refused` rather than relocated on mismatch; every relocated row also carries
a recomputed raw CID. `git check-ignore` confirms all 27 are ignored and `git status` shows none.

Bytes are copied **verbatim**. A continuation pinned to the Python executor's digest map keeps it
and is reported `resumable: false` with the reason. Rewriting it to the current contract CID would
have been tampering with the evidence the relocation exists to preserve. The originals stay in the
kit; station six deletes the directory.

## Corrections close by identity

`epr flow concerns --corrections [--since YYYY-MM-DD] [--json]`. A `run:correction` is unresolved
unless a **later** note about the **same subject**, of kind `observation|ruling|verdict|correction`,
carries `closes:<its exact CID>`. Four decisions are worth naming:

- **Subject label, not resource CID.** A repair CHANGES the bytes the correction was written
  against, so requiring resource equality would make every genuine repair unable to close the
  correction it repaired and admit only no-op closures. The exact correction is already named by
  `closes:`; the label check is what stops a closure for one surface reading as a closure on
  another. `a_closure_survives_the_repair_that_changed_the_source` pins this.
- **`failed-approach` cannot close.** An approach that did not work closes nothing; admitting it
  would let "we tried and stopped" read as "resolved" — the same inference the date heuristic made,
  one layer down.
- **A closure written as `--kind correction` is not itself a new open concern.** Without that, the
  list could never reach zero however much work was discharged.
- **`--since` filters the DISPLAY only.** Suppressed rows are counted (`omitted_by_since`), never
  resolved.

**Live reading, this tree:** `corrections 79, unresolved 71, resolved 8, stale_unresolved 0,
surfaces 51, issues []`. `stale-record.py --json` on the same tree reports `0` unresolved STALE and
`8` resolved — the two agree on the STALE surface, and the native reading additionally surfaces the
71 non-STALE corrections the date reading never showed at all.

That agreement required an act with a durable side effect, called out under **Concerns**: the eight
closures lived in `genesis/data/timeline/chronicle/*.md` frontmatter (`stale_record_resolutions`),
which the native reading cannot see. I migrated each one into a native closure note whose reason
names the chronicle, the resolution text, the evidence path and its body fingerprint — the exact
four fields the Python validator had already checked. Without the migration the native command
would have reported eight verified repairs as unresolved, which is a worse falsehood than the one
being replaced.

## The 60-case parity map

The Python suite is 17 (`recall_runtime_test.py`) + 31 (`recall_ceremony_test.py`) +
12 (`recall_packet_test.py`) = 60. All 60 still pass unchanged after the contract move.

### `recall_runtime_test.py` (17)

| Python case | Rust counterpart |
|---|---|
| `test_cli_continuation_accumulates_arbitrarily_many_named_packets` | `continuation_accumulates_every_named_packet` |
| `test_cli_contract_change_refuses_reset_and_preserves_previous_receipt` | `contract_change_refuses_reset_and_adopt_carries_prior_accounting` |
| `test_session_rejects_changed_executor_identity_without_losing_totals` | `a_changed_executor_is_reported_without_stranding_the_session` (RESHAPED — see Concerns) |
| `test_session_requires_a_question_and_cannot_escape_or_follow_state_symlink` | `a_session_needs_a_question_and_refuses_escape_or_a_symlinked_state` |
| `test_failed_operation_is_retained_and_next_success_does_not_erase_it` | `a_refused_operation_is_retained_in_attempt_accounting` |
| `test_invalid_utf8_whole_source_still_charges_bytes` | `a_short_or_undecodable_excerpt_is_withheld_but_still_charged` |
| `test_output_overflow_withholds_sources_but_charges_them` | `a_short_or_undecodable_excerpt_is_withheld_but_still_charged` |
| `test_discovery_exact_tags_and_groups_describe_only_returned_window` | `discovery_filters_exactly_and_describes_only_its_returned_window` |
| `test_incomplete_metadata_window_reports_unresolved_instead_of_false_absence` | `only_a_complete_frontmatter_boundary_establishes_membership` |
| `test_name_filter_avoids_unrelated_metadata_reads_but_retains_exact_tag_filter` | `discovery_filters_exactly_and_describes_only_its_returned_window` |
| `test_metadata_delimiter_must_be_a_complete_yaml_document_boundary` | `only_a_complete_frontmatter_boundary_establishes_membership` |
| `test_metadata_cut_inside_fake_delimiter_does_not_make_membership_evidence` | `only_a_complete_frontmatter_boundary_establishes_membership` |
| `test_discovery_respects_scan_bytes_and_skips_symlink_escapes` | `discovery_respects_its_scan_budget_and_skips_symlinks` |
| `test_excerpt_range_unicode_and_fingerprint_are_exact` | `an_excerpt_range_and_its_fingerprint_are_exact` |
| `test_excerpt_never_returns_partial_line_or_invalid_utf8_as_evidence` | `a_short_or_undecodable_excerpt_is_withheld_but_still_charged` |
| `test_provider_output_limit_includes_stderr_and_stops_before_buffering_all` | `a_foreign_provider_is_bounded_by_bytes_and_by_seconds` |
| `test_provider_timeout_and_nonzero_exit_preserve_diagnostics` | `a_foreign_provider_is_bounded_by_bytes_and_by_seconds` |

### `recall_ceremony_test.py` (31)

| Python case | Rust counterpart |
|---|---|
| `test_purpose_story_and_per_assertion_observations` | `every_view_carries_purpose_scope_and_linked_next_actions` |
| `test_resume_rehydrates_and_invalidates_changed_receipts` | `resume_rehydrates_and_invalidates_changed_receipts` |
| `test_method_changes_require_explicit_continuation_with_prior_accounting` | `contract_change_refuses_reset_and_adopt_carries_prior_accounting` |
| `test_provider_substitution_and_refusal_preserve_intent` | `provider_substitution_and_refusal_preserve_intent` |
| `test_revalidation_total_budget_retains_pending_frontier` | `revalidation_budget_leaves_an_executable_pending_frontier` |
| `test_prepared_repair_does_not_execute_and_finish_does_not_accept` | `prepared_repair_does_not_execute_and_finish_does_not_accept` |
| `test_output_bounds_apply_to_both_actual_renderings` | `the_output_budget_bounds_both_renderings` |
| `test_newer_contrary_judgment_blocks_historical_repair_readiness` | `repair_readiness_requires_current_evidence_and_a_current_judgment` |
| `test_native_context_expands_without_overwriting_saved_next_action` | `native_context_expands_and_refuses_a_stale_navigation_pin` |
| `test_oversized_finding_refusal_is_retained_in_attempt_accounting` | `a_refused_operation_is_retained_in_attempt_accounting` |
| `test_context_page_offset_does_not_skip_selected_edge_revalidation` | `display_pagination_never_controls_exact_slot_revalidation` |
| `test_changed_native_context_refuses_an_old_indexed_navigation_choice` | `native_context_expands_and_refuses_a_stale_navigation_pin` |
| `test_source_escape_and_unproven_evidence_ready_refuse` | `a_source_outside_the_declared_scope_is_refused` + `repair_readiness_requires_current_evidence_and_a_current_judgment` |
| `test_rereading_changed_evidence_does_not_revalidate_an_old_judgment` | `rereading_changed_evidence_does_not_revalidate_an_old_judgment` |
| `test_accumulated_findings_remain_progressively_recoverable` | `accumulated_findings_remain_progressively_recoverable` |
| `test_doc_repair_never_routes_to_sidecar_reseal` | `a_doc_plane_repair_never_routes_to_a_sidecar_reseal` |
| `test_healthy_selected_edge_cannot_be_offered_for_reseal` | `a_healthy_selected_edge_is_not_offered_for_reseal` |
| `test_finish_preserves_later_batch_finding_and_rechecks_changed_source` | `accumulated_findings_remain_progressively_recoverable` + `prepared_repair_does_not_execute_and_finish_does_not_accept` |
| `test_pending_selected_evidence_has_executable_continuation_without_restart` | `revalidation_budget_leaves_an_executable_pending_frontier` |
| `test_native_claim_and_standing_changes_are_visible_without_source_change` | `native_standing_changes_are_visible_without_a_source_change` |
| `test_selected_resume_and_adopt_skip_population_and_ignore_display_pagination` | `display_pagination_never_controls_exact_slot_revalidation` |
| `test_exact_slot_pages_share_deadline_and_stdout_stderr_budget` | **RETIRED** (see below) |
| `test_measurement_pair_survives_resume_and_counts_consolidation` | `the_paired_lens_pins_both_halves_and_refuses_an_unpaired_close` |
| `test_measurement_rejects_changed_scope_and_tampered_baseline` | `the_paired_lens_pins_both_halves_and_refuses_an_unpaired_close` |
| `test_measurement_incomplete_is_not_a_zero_delta` | `the_paired_lens_pins_both_halves_and_refuses_an_unpaired_close` |
| `test_measurement_receipt_directory_symlink_is_refused` | `the_paired_lens_pins_both_halves_and_refuses_an_unpaired_close` + `a_session_needs_a_question_and_refuses_escape_or_a_symlinked_state` |
| `test_existing_receipt_fifo_refuses_without_blocking` | `a_receipt_pins_the_contract_files_raw_cid_as_its_method` (RESHAPED: `save_receipt` is exclusive-create-then-compare and refuses a non-regular file via `symlink_metadata`; the Python's `O_NONBLOCK` FIFO guard is a libc-level concern that `std::fs` does not reproduce) |
| `test_shared_resume_skips_population_and_keeps_exact_request` | **RETIRED** |
| `test_collective_and_retained_request_pin_charge_native_output` | **RETIRED** |
| `test_shared_source_read_returns_to_shared_context_without_edge_selection` | **RETIRED** |
| `test_shared_finding_and_frontier_survive_reset_without_edge` | **RETIRED** |

### `recall_packet_test.py` (12)

| Python case | Rust counterpart |
|---|---|
| `test_whole_sources_obey_shared_byte_budget_and_do_not_reset_fallback` | `revalidation_budget_leaves_an_executable_pending_frontier` (the shared per-pass scan/byte/file budget lives in `revalidate`) |
| `test_file_budget_and_duplicate_aliases` | `revalidation_budget_leaves_an_executable_pending_frontier` (`revalidate` dedupes named receipt keys before spending the file budget) |
| `test_missing_out_of_scope_absolute_and_escaped_sources_stay_unresolved` | `a_source_outside_the_declared_scope_is_refused` |
| `test_located_sources_are_not_an_acceptance_judgment` | `prepared_repair_does_not_execute_and_finish_does_not_accept` |
| `test_method_identity_pins_contract_and_implementation_bytes` | `a_receipt_pins_the_contract_files_raw_cid_as_its_method` |
| `test_fingerprint_describes_returned_content_even_if_source_changes` | `an_excerpt_range_and_its_fingerprint_are_exact` |
| `test_failed_decoding_still_consumes_read_budget` | `a_short_or_undecodable_excerpt_is_withheld_but_still_charged` |
| `test_post_read_disappearance_still_consumes_read_budget` | `a_short_or_undecodable_excerpt_is_withheld_but_still_charged` (the "source disappeared while reading" frontier — added during this station precisely so the usage is charged rather than lost to an error) |
| `test_native_packet_strips_legacy_context_and_accounts_both_payloads` | **RETIRED** |
| `test_native_command_failure_and_timeout_propagate` | `a_foreign_provider_is_bounded_by_bytes_and_by_seconds` |
| `test_native_missing_malformed_or_oversized_facets_are_refused` | `an_unsupported_contract_is_refused_before_any_session_exists` |
| `test_explicit_followup_requires_question_and_retains_each_packet_budget` | `a_session_needs_a_question_and_refuses_escape_or_a_symlinked_state` + `continuation_accumulates_every_named_packet` |

### The six retired cases, and why

All six are artefacts of the Python being an **out-of-process** driver of `epr`, not properties of
the algorithm. They have nothing to assert against a native executor.

1. `test_exact_slot_pages_share_deadline_and_stdout_stderr_budget` — the exact-slot lookup shelled
   out to `epr flow context --concerns` once per page, so its stdout AND stderr had to share one
   byte budget across pages. Natively it is `concerns::concerns_with(...)`, an in-process call with
   no second process and therefore no stderr. The **deadline** half survives in `refresh_selected`'s
   `native_timeout_seconds` loop and its byte-budget exhaustion frontier, exercised by
   `display_pagination_never_controls_exact_slot_revalidation`.
2. `test_native_packet_strips_legacy_context_and_accounts_both_payloads` — same cause. There is no
   second payload to strip a legacy context from, and `charge_native` accounts the one projection
   actually consumed.
3–6. `test_shared_resume_skips_population_and_keeps_exact_request`,
   `test_collective_and_retained_request_pin_charge_native_output`,
   `test_shared_source_read_returns_to_shared_context_without_edge_selection`,
   `test_shared_finding_and_frontier_survive_reset_without_edge` — all four exercise the
   `memory-project` / `collective` operations, which are **already native verbs**
   (`epr flow memory project|collective|contribute|feedback|graduate`) and are deliberately absent
   from the sixteen. Their semantics are covered by `tests/flow_memory.rs` and
   `tests/flow_memory_import.rs`, which test the verbs themselves rather than a shell around them.

## View-shape parity

`recall-ceremony.py --json` was run on its own fixture and every operation's top-level key set was
pinned, then compared against the native executor on a live tree. The sets are **identical** for
all sixteen operations:

| op | keys |
|---|---|
| `open` | actions, concerns, continuation, cumulative, execution_method, frontier, measurement, next_action, operation, orientation, unresolved, usage |
| `select` / `context` | + input_choices, node, selected_current (and native_context on `context`) |
| `read` | + evidence, receipt_keys |
| `remember` | + retained |
| `recipe` | + contract, recipe |
| `source` | + source_outline |
| `search` | + retrieval |
| `history` | + history |
| `compare` | + comparison |
| `resume` / `adopt` | + continuation, evidence_check, measurement, selected_current |
| `prepare` | + evidence_check, prepared_action, selected_current |
| `reconcile` | + meaning, native_walk |
| `measure` | + measurement |
| `finish` | + current_edges, evidence_check, measurement, outcome, reconciliation |

Nested shapes are preserved too (`orientation` carries the same nine fields including the two
verbatim `constraints` strings; `cumulative` carries `attempts`/`totals`/`unmetered_attempts`/
`accounting_scope` with the same wording; `evidence_check` carries the same eight fields and the
same `meaning` sentence; every `unresolved` string that names a limit is verbatim).

**One usage-key divergence, by design.** The Python charged `native_raw_bytes` and
`native_stderr_bytes` for each shelled-out `epr` call. Natively there is no second process, so
`native_raw_bytes` is charged as the serialized size of the projection actually consumed and there
is **no `native_stderr_bytes` slot**. Emitting a constant zero for a stream that does not exist
would be theatre.

**One `execution_method` divergence, by mandate.** The Python pinned a map of five file digests.
The native payload is `{method, recall-contract.json, contract_path, executor, native_executable,
measurement_lens}` — the contract CID is the pin, and the binary digest is reported beside it.

## The paired footprint lens

`measure --phase baseline|close` invokes the declared foreign lens
`genesis/scripts/memory_balance.py` through `bounded_process` (bounded by `native_raw_bytes` and
`native_timeout_seconds`), saves the snapshot as a private receipt, pins it via
`epr flow memory pin` (in-process), and — when `--actor-session` is given — appends an
`epr flow note --kind observation` naming the receipt. `compare` is ported natively rather than
re-invoking the lens with `--baseline`, because the lens's `--baseline` flag folds the comparison
INTO the sample and would change what the close receipt pins.

Two honest limits, both surfaced on the view rather than only recorded here:

- The lens CLI accepts `--max-files`, `--max-bytes` and `--max-entries` but not `max_seconds` or
  `max_depth`, which the contract also declares. Those two fall back to the lens's own defaults, and
  each one emits an `unresolved` line naming itself.
- Balance receipts land under `.eprfs/status/recall/<session>/receipts/`, not in the
  `balance-sheets/` directory the Python used. They are receipts OF a recall session and therefore
  private. Station six's relocation of `.claude/memory-kit/balance-sheets/` to
  `.eprfs/status/balance/` concerns the standalone lens runs, not these.

## Gate

Run from a clean tree with the cargo berth claimed (`berth claim cargo --session mk-replace-native-5`).

```
env RUSTFLAGS= CARGO_TARGET_DIR=/tmp/eprfs-gate-target CARGO_BUILD_JOBS=1 \
  cargo fmt --all --check                                      FMT_EXIT=0
env RUSTFLAGS= CARGO_TARGET_DIR=/tmp/eprfs-gate-target CARGO_BUILD_JOBS=1 \
  cargo clippy --workspace --all-targets -- -D warnings         CLIPPY_EXIT=0
env RUSTFLAGS= CARGO_TARGET_DIR=/tmp/eprfs-gate-target CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 \
  cargo test --manifest-path elohim/eprfs/Cargo.toml --workspace   TEST_EXIT=0
EPR_BIN=/tmp/eprfs-gate-target/debug/epr python3 -m unittest discover \
  -s .claude/scripts/memory-kit/__tests__ -p 'recall*_test.py'  → Ran 60 tests, OK
```

Workspace totals on the green run: 49 test binaries, **638 tests passed, 0 failed**. New counts:
`flow_memory_recall` 32 passed, `flow_concerns_corrections` 9 passed, plus two `note.rs` unit tests
for the closure slot and its resolver.

**One transient red, not mine.** The first workspace run had
`flow_memory_import::importing_the_corpus_twice_appends_exactly_one_round_of_events` fail with
`left: "repository", right: "workspace"`. That assertion belongs to the concurrent station-four
seat and no longer exists in the tree — they replaced the literal with a rung relation while my run
was in flight. Re-running that test file alone after their edit: 12 passed. It is recorded here
because a reviewer reading only the first log would otherwise attribute it to this station.

Additionally, `stale_record_test.py` (14 tests) still passes — the Python reading it validates is
untouched by this station.

Binary installed over `/opt/rust/cargo/bin/epr` (31,655,264 bytes), sha256
`a2f4a4d5b32211fdd4961cca913ff24730051db4753d77249b5088c40feef151`, byte-identical to
`/tmp/eprfs-gate-target/debug/epr`. Verified after install:
`epr flow concerns --corrections --json` reads
`{corrections 79, unresolved 71, resolved 8, stale_unresolved 0, surfaces 51}`. Cargo berth
released.

## Concerns

**1. The session's method pin is the contract CID alone.** The plan mandates that "every receipt
pins its raw CID as `method`", and I implemented exactly that. The consequence is a real
weakening against the Python, which pinned five file digests (contract, executor, and three
libraries) and REFUSED a session when any of them changed. Natively a changed executor is
**reported** — `executor_digest` is stored on the continuation, a mismatch appends an `unresolved`
line and the totals survive — rather than stranding a session on a rebuild. I chose report-not-
refuse because during this station the binary was rebuilt a dozen times mid-investigation and a
refusing pin would have made the executor unusable on itself. A reviewer who thinks the stronger
pin is worth the friction should say so; it is a one-line change in `Execution::hydrate`.

**2. I migrated eight validated closures into the ledger.** `.eprfs/status/flows.jsonl` gained eight
`run:correction` notes carrying `closes:`, attributed to
`agent:implementer@claude-opus-5` under session `mk-replace-native-5`. Each reason names its source
chronicle, the resolution text, the evidence path and the body fingerprint the Python validator had
already verified. This is a durable append with real consequence — it moves eight corrections from
open to closed in every future reading — and it is the one act in this station that changes shared
state rather than adding a capability. My reasoning: the closure evidence existed and was
validated, the native reading could not see it, and shipping a command that reported eight verified
repairs as unresolved would have replaced one falsehood with another. A reviewer should check the
eight notes directly (`epr flow concerns --corrections --json | jq .resolved`) and, if any
migration is wrong, the remedy is a further correction note rather than a ledger edit.

**3. The kit's closure home is now bypassed, not migrated wholesale.** `stale-record.py` still reads
`stale_record_resolutions` frontmatter and will keep working. Anyone writing a NEW closure should
write a native note; a new chronicle frontmatter entry will not be seen by
`epr flow concerns --corrections`. Station six should say this in whatever surface names the
ceremony, and that is an integration-seat act, not one I can make from `elohim/eprfs/**`.

**4. `epr flow concerns` is a new subcommand with one flag.** Station three's plan text names
`epr flow concerns --stamp <doc>` as a separate contract, which the cite-writer seat owns. I built
the subcommand shell so that lands as another arm; a bare `epr flow concerns` prints usage and
refuses rather than guessing which view was wanted.

**5. `--adopt-receipts` verified zero content-named receipts on this tree.** All 28 legacy entries
are continuations, so the digest-re-verification arm ran against fixtures only
(`receipt_adoption_reports_counts_before_it_moves_anything` plants both a matching and a mismatched
receipt). If a projection receipt ever existed in that directory it is already gone.

**6. The gap was already discharged when I closed it, and not against this report.**
`epr flow fulfill --on plans__2026-09-10-memory-kit-replacement-finish#6 --report <this file>
--status DONE_WITH_CONCERNS` returned `(already discharged — no-op)`. A `produce` /
`report:DONE_WITH_CONCERNS` event for this gap already sat at `.eprfs/status/flows.jsonl` line
7804, pinning resource/evidence `bafkreieu34327ikipoe7sisexwo2y4vbx347nc3xgd4ngfwavfa5dnflvy` —
which is NOT this report (body CID
`bafkreidnrwn2actkoxiuceswwgd6jx6muopoojhcmdh745watbtbtmvcaa`) and which I could not locate on
disk. Every station #1–#6 has exactly one such discharge and all six carry provider
`agent:implementer@claude-opus-5`, the identity every implementer seat in this shared tree uses,
so the two runs cannot be told apart by attribution. I did NOT mint a second discharge — a note
annotates, a fulfill discharges once — and instead recorded the correct evidence as a
`run:observation` on the gap naming this report's path and body CID. **A reviewer accepting this
station should read the report at that CID, not follow the discharge's evidence pin.** The
underlying seam is worth naming for the story graph: a body CID is a snapshot, so a discharge that
pins one becomes unresolvable the moment the report is edited, and the flow plane offers no way to
say "this discharge names the wrong evidence" other than a note beside it.

## What this station did NOT do

- Did not delete `recall-ceremony.py`, `recall-packet.py`, `recall_runtime.py`, `recall_lenses.py`
  or `recall_providers.py`. They still run and their 60 tests still pass.
- Did not touch hooks, packages, skills, `.claude/memory/**`, `.claude/settings.json`,
  `flow/cites.rs`, `flow/report.rs`, `flow/measures.rs`, `flow/memory/import.rs` or
  `flow/memory/index.rs`.
- Did not re-author the memory-ceremony skill or the memory-kit package to name the native verbs.
  That is the integration seat's half of this station.
- Did not commit or push.


---

# Round two — review response (changes-requested on gap #6)

The review verdict (`bafyreie5qfbykiverrslt6eqyskx3jsbm6tkj5lcc6mngip7zjj6pun54a`) returned
`changes-requested` with two Importants and two Minors; the station-five integration seat then
raised two more. All six are addressed below. **Everything above this line is the round-one record
and is left unedited**, including the numbers it quotes, so the two rounds can be read against each
other — where round two moved a number, it says so here.

## IMPORTANT 1 — a changed executor could silently continue a session. FIXED.

`Execution::hydrate` now carries **two** refuse-then-adopt pins, and both fire before the attempt
counter is incremented and before anything is written:

- the contract's raw CID (unchanged from round one), and
- the **executor digest**, which is now resolved BEFORE the session is opened and passed into
  `Execution::open(root, session, need, method, executor, state_limit)`.

A mismatch refuses with the exact command that continues the work —
`epr flow memory recall adopt --from-session <prior> --session <new-session>` — and, critically,
**leaves the stored digest intact**, so the receipt `adopt` carries forward still says what the
prior counters were accumulated under. `prior_receipt` now records `executor_digest` alongside
`method`, `attempts` and `totals`. The round-one behaviour (overwrite the digest, push one
`unresolved` line, keep going) is gone; so is the `executor_changed` warning block.

Test: `a_changed_executor_refuses_the_session_and_adopt_carries_both_digests` — moves the recorded
pin, asserts the refusal names `executor bytes changed` and the literal adopt command, asserts the
continuation is byte-unchanged (attempts, totals and the stale digest all survive), then adopts and
asserts the new session pins the running executor while `prior_receipt.executor_digest` holds the
old one and the inherited investigation resumes.

**Confirmed live, and it is exactly the behaviour the review asked for.** The integration seat's
edit moved the contract CID (`bafkreiefupw3j…` → `bafkreigpbzag5p77klfxpvxv4lbwuptdba7zci2vwsnloafgnyrhkkuydi`,
sha256 `cf0e406e…`, 11,143 bytes). Every relocated continuation under `.eprfs/status/recall/` now
refuses rather than continuing:

```
$ epr flow memory recall recipe --session ceremony-dev-a --json
unresolved: algorithm bytes changed; explicitly start a new session and retain prior receipt —
            epr flow memory recall adopt --from-session ceremony-dev-a --session <new-session>
next:       The pinned method changed. Retain this receipt and continue explicitly: …
```

The tests read the LIVE contract (`live_contract()` reads
`.epr-meta/elohim/algorithms/recall-contract.json` from disk at test time and re-scopes a copy), so
the CID move needed no test edit and none was made.

## IMPORTANT 2 — the move left dangling sealed slots. RESTAMPED; residue disclosed, not cleared.

What I did:

1. Re-declared the three live relationships at the contract's new home with
   `epr flow seal … --governor cite-seal --on .epr-meta/elohim/algorithms/recall-contract.json`.
2. Appended one `epr flow note --kind correction` on the plan
   (`bafyreie…ssg4`) recording the move, the eight path references, the restamp, and the residue.

**What I could not do, and why — the review's proposed remedy does not reach these records.** The
four offending records are **sidecar `DepEdge`s**, not doc-plane `cites:` envelopes.
`epr flow cites stamp|verify` operates on document frontmatter and never sees them, and
`epr flow hold` — the verb whose whole purpose is a declared deviation — **refuses them**:

```
$ epr flow hold .claude/scripts/memory-kit/recall-packet.py \
    --on .claude/scripts/memory-kit/recall-contract.json --reason "upstream relocated …"
epr flow: invalid arguments: cannot cite-seal `…/recall-contract.json`:
          upstream unreadable (dangling at birth)
```

The gate is correct for a NEW edge and wrong for an ALREADY-dangling one, but widening it is a
change to a shared verb (`flow/seal.rs`) that no reviewer asked for, so I did not make it.

**The cause also widened while this was in flight.** The station-five INTEGRATION seat retired the
Python recall suite during this round — `recall-ceremony.py`, `recall-packet.py`,
`recall_runtime.py`, `recall_lenses.py`, `recall_providers.py`, `stale-record.py` and every
`recall*_test.py` / `stale_record_test.py` are deleted. All four slots therefore now have **both**
endpoints unreadable and are residue of the retirement, not of the move alone. The live page:

```
1. …/recall-contract.json  -> …/__tests__/recall_packet_test.py   [dangling, sidecar]  both unreadable
2. …/recall-contract.json  -> …/__tests__/recall_runtime_test.py  [dangling, sidecar]  both unreadable
3. …/recall-packet.py      -> …/recall-contract.json              [dangling, sidecar]  both unreadable
4. …/recall-packet.py      -> …/recall_runtime.py                 [dangling, sidecar]  both unreadable
```

**Second disclosure, against myself.** The three re-seals in step 1 were authored against a tree in
which those consumer files still existed; minutes later they did not. They are inert (verdict `ok`,
`consumer_readable: false`, never offered as concerns) but they assert a relationship for files that
are gone. Both facts are in the correction note rather than left to be found.

**Handoff, named:** the flow plane has no retraction verb for a sidecar slot. Clearing all seven
records is a **sidecar-maintenance act for station six** (the station that removes the kit and
reconciles what its removal orphans), not a cite-writer act — the doc plane the cite writer owns is
not where they live. The round-one report's path-reference list, Concerns and did-NOT-do section
omitted the sealed cite plane entirely; that omission is corrected here.

## IMPORTANT 3 (integration seat) — the footprint lens was anchored to `--root`. FIXED.

`resolve_lens` replaces `root.join(BALANCE_LENS_REL)`. The lens is a **repository tool**, not a
member of the tree being measured — the retired Python imported it from the repository and scoped
only its SAMPLE by `--root`. Resolution order, each rung checked for an actual file:

1. an explicit `--lens` (a binding override, returned even if missing, so a caller who named a path
   gets a refusal about **that** path rather than a silent substitution);
2. the repository holding the **contract** (four levels up from
   `<repo>/.epr-meta/elohim/algorithms/<file>`);
3. `--root`, for a tree carrying its own copy;
4. the repository the process is standing in, by walking up from the working directory.

And an unreachable lens is now an **unavailable measurement**, not a refused operation: `measure`
returns `{available: false, reasons: […], meaning: "Unavailable is not zero…"}` and `open`
proceeds. That is what actually answers the complaint — a fixture root with no lens no longer
refuses.

The absolute-`--lens` workaround is removed from both a2o step files
(`genesis/a2o/steps/devflow/{collective-memory,ceremony-reconciliation}.steps.ts`): only the two
`--lens` argument lines and the two stale comments explaining the workaround, per the instruction.

Test: `the_footprint_lens_needs_no_flag_in_a_tree_that_does_not_carry_it` — a fixture that provably
carries no lens opens with no `--lens` anywhere, and accepts either honest answer (a resolved
sample with its receipt under `/receipts/baseline-`, or an explicit `available: false` that says
unavailable is not zero); a named-but-missing `--lens` still refuses about that path.

## IMPORTANT 4 (integration seat) — `--tag` had no native equivalent. ADDED.

`--tag <t>` is repeatable on `search` and `source`, threaded through `retrieve` into `discover`'s
existing exact-membership filter (every named tag must be present).

- `search --tag` filters the local provider's candidates and echoes the tags it applied.
- `source --tag` **without** `--path` answers "which sources carry this": bounded traversal grouped
  by tag, with a linked outline action per candidate, opening nothing on its own. With `--path` it
  behaves exactly as before, and with neither it now says so (`…or --tag to discover one`).
- A provider that cannot read frontmatter is **refused** rather than quietly ignoring the filter:
  `--tag is exact frontmatter membership and only the local provider reads frontmatter`.
- One subtlety worth naming: `search` substitutes the evidence question for a missing `--query`,
  which would have silently emptied a correctly-tagged result. When tags are named and no `--query`
  is, the query is empty — the tags **are** the filter.

Test: `tag_filters_discovery_exactly_and_is_refused_where_it_would_be_ignored`.

## MINOR 1 — `selection_rule` said "resource". FIXED.

`concerns.rs` now says "under the same subject label. The label, not the resource CID: a repair
changes the bytes the correction pinned", in both the rendered `selection_rule` and the function
doc.

## MINOR 2 — every refusal pointed at the method-change remedy. FIXED.

`remedy_for(message, session)` names the remedy for the fault that actually fired: method/executor
change → the adopt command; a private path → "a receipt is never an input to any verb"; out of
scope → "name a path inside the declared source_roots, `recipe` lists them"; a held lock, a missing
ceremony, an unselected concern, a double adopt, a changed intent/scope, a changed measurement
cohort, a full continuation — each its own line; and an unclassified fault says something true
("correct the named input; the session and its prior receipt are unchanged") rather than something
wrong. Argument-shape and contract-load refusals now also arrive in the **same structured
envelope** (`print_refusal`) instead of as bare stderr with no `next` at all.

Test: `each_refusal_points_at_its_own_remedy`.

## Round-two gate

```
cargo fmt --all --check                                    FMT_EXIT=0
cargo clippy --workspace --all-targets -- -D warnings      CLIPPY_EXIT=0
cargo test --manifest-path elohim/eprfs/Cargo.toml --workspace   TEST_EXIT=0
```

Workspace totals: 49 test binaries, **641 tests passed, 0 failed**.

One fixture flake was found and fixed by this round's own suite, and it is worth naming because it
would have bitten anyone: a note's `occurred_at` is git HEAD's author date and therefore part of its
content address, so two fixture repositories built a second apart minted different CIDs for the same
authored note. `the_closes_slot_is_additive_and_leaves_existing_addresses_alone` passed on a fast
machine and failed on a slow one. Both fixture harnesses now pin `GIT_AUTHOR_DATE`/
`GIT_COMMITTER_DATE`, so the assertion is about the slot vector rather than about the clock.

Binary installed at `/opt/rust/cargo/bin/epr`, **31,714,968 bytes**, sha256
`4dafb9b3ad95357b2b99fc5b35e4d439ba0c1fa7547ce97d49bbaff7c87ad45f` (round one was
`a2f4a4d5…`). Verified after install: the sixteen-op usage line carries the new `--tag` and `--lens`
lines, and `epr flow concerns --corrections` reads `{corrections 80, unresolved 72, resolved 8,
stale_unresolved 0, surfaces 51}` — one more correction than round one, which is this round's own
disclosure note about the move and the residue. Cargo berth released.

`flow_memory_recall` is now **35 tests** (33 + the lens test + the tag test; the round-one executor
test was rewritten in place, and `each_refusal_points_at_its_own_remedy` is new), plus
`flow_concerns_corrections` 9 and three `note.rs` unit tests.

**The Python recall suite leg is GONE, and I am not claiming it passed.** In round one it ran
`Ran 60 tests … OK`. During round two the integration seat deleted the suite and its libraries, so
`python3 -m unittest discover -s .claude/scripts/memory-kit/__tests__ -p 'recall*_test.py'` now
collects nothing. The 60-case parity map above stands as the record of what those cases asserted and
where each one now lives; it is no longer re-runnable against the Python, and station six's parity
gate should read it as a map, not as a live oracle.

## What round two did NOT do

- Did not widen `epr flow hold` to admit an already-dangling slot, though that is the change that
  would let the four residue records be retired. It touches a shared verb no reviewer asked me to
  change.
- Did not run `epr flow cites stamp --all --write`. The doc-plane cites naming the old path are one
  legacy bare-path entry in a chronicle that already carries a PRE-EXISTING unrelated defect
  (`unresolvable slug: 'ceremony-efficacy` — a quoting artefact, not mine), and a corpus-wide
  stamped write while another seat is live is not a change I will make on the strength of a fault I
  did not cause.
- Did not touch `flow/cites.rs`, `flow/report.rs`, `flow/measures.rs`, `flow/memory/import.rs`,
  `flow/memory/index.rs`, hooks, packages, skills or `.claude/memory/**`.
- Did not commit or push.


---

# Round three — the footprint lens is native

Round two's `resolve_lens` was a path-search ladder, and the ladder was the wrong shape: a cucumber
fixture runs with its cwd under `/tmp` and a fixture contract, so rung 2's `ancestors().nth(4)` is
`None` and rungs 3 and 4 miss. `just gate memory-ceremony` was red at
`collective-memory.steps.ts:335` on `measurement.evidence`. A measurement primitive the ceremony
depends on cannot be conditional on where the process happens to be standing, so the lens is now
part of the executor.

## What landed

| Path | Role |
|---|---|
| `elohim/eprfs/epr-cli/src/flow/memory/footprint.rs` | NEW (~560 lines). `genesis/scripts/memory_balance.py` transcribed: `Limits` (+ `from_declared`), `method`, `normalize_scope`, the bounded `Walk`, `snapshot` and `compare`. |
| `elohim/eprfs/epr-cli/tests/flow_memory_footprint.rs` | NEW. 10 tests — the nine Python cases ported plus field-by-field byte parity against the oracle. |
| `elohim/eprfs/epr-cli/src/flow/memory/recall.rs` | `resolve_lens` DELETED. `sample_balance` samples natively; `external_sample` is the subprocess path and is reached only by an explicit `--lens`. `compare_samples` delegates to `footprint::compare` — one implementation, in the lens. |

`--lens <script>` is now what it should always have been: an override naming an **external** lens,
never a default path. A sample taken through it carries an `unresolved` line saying so, because its
method pin, budgets and omissions are its own and not this executor's. `execution_method.measurement_lens`
reports the lens's `method` object when native, and the override path when overridden — what
measured it, not a path someone hoped to find.

`genesis/scripts/memory_balance.py` is left in place; station six retires it once this is reviewed.

## What is preserved, and what differs

Preserved exactly, because these are the measurement's promises: `complete` goes false the moment
anything is omitted and a delta is refused on an incomplete pair (unknown is never zero); a file
that changed mid-read is `stable: false` **and** omitted, keeping its bytes in the total while the
total stops claiming completeness; symlinks are refused including on intermediate components;
`same_content_relocations` keeps a MOVE from reading as a shrink; and the scope digest keys the
pair, so `compare` names the field that disagreed instead of returning a number.

One difference, stated rather than hidden: the Python walks with fd-relative
`openat(…, O_NOFOLLOW)`, closing the TOCTOU window on ancestors. The native lens builds each path
component by component from the root and refuses any component whose `symlink_metadata` says it is a
symlink, using only `std::fs` — the same refusal, checked one instant earlier. The sample was
already documented as "sequential, not an atomic filesystem snapshot"; this widens that existing
window on ancestors and creates no new class of claim. Also by design: `method.sha256` is the native
module's own bytes, so a Python baseline and a native close are correctly **not** comparable —
they are different methods, and `compare` refuses the pair.

## The nine Python cases

| `genesis/scripts/__tests__/memory_balance_test.py` | Rust counterpart |
|---|---|
| `test_special_paths_lines_and_overlap` | `special_paths_line_counting_and_cohort_overlap` |
| `test_archive_is_not_net_shrink_and_new_overhead_counts` | `archiving_is_not_a_net_shrink_and_new_overhead_counts` |
| `test_consolidation` | `a_real_deletion_is_a_real_negative_delta` |
| `test_mismatch_and_missing_refuse_comparison` | `an_incompatible_pair_or_a_missing_input_refuses_comparison` (widened: all seven keyed fields, plus phase ordering) |
| `test_bounds_and_symlinks` | `budgets_bind_symlinks_are_refused_and_escapes_do_not_parse` (widened: six escaping spellings, not one) |
| `test_concurrent_change_is_incomplete` | `a_file_that_changed_during_sampling_is_incomplete_but_still_counted` — RESHAPED. The oracle patches `os.read` to mutate the file mid-read; there is no `unittest.mock` here, so the same condition is produced by its own definition (observed bytes ≠ size at close of read) and the test carries an honest-completion control beside it. |
| `test_declared_exclusions_and_depth_limit` | `declared_exclusions_are_visible_and_depth_and_time_budgets_bind` (widened: five out-of-range budgets refused rather than clamped) |
| `test_flag_semantics_and_exclusive_output` | `sampling_writes_nothing_at_all` — RESHAPED. That case is about the shell wrapper `genesis/scripts/memory-balance.sh` (`--json --no-save` writes nothing; `--output` creates exclusively; a repeat fails; an unknown flag fails). The native lens has no wrapper and no output flag — it returns a value and the caller decides — so the invariant that survives is the load-bearing one, **sampling writes nothing**, asserted by a before/after listing of the whole tree. The exclusive-create rule now lives where the write does, in `save_receipt`. |
| `test_retention_protects_ignored_tracked_and_unknown_pins` | RETIRED, with the reason. That case does not test the lens: it loads `.claude/scripts/memory-kit/memkit-retention.py` and asserts the RETENTION planner never removes pinned evidence — a different tool, still in the kit, and a station-one bound rather than a footprint measurement. Its lens-shaped half (a git tree samples completely because `.git` is a declared exclusion) is covered by `a_git_repository_samples_completely_because_dot_git_is_a_declared_exclusion`. |

## Byte parity

`the_native_snapshot_matches_the_python_oracle_field_for_field` runs both implementations over one
fixture (nested dirs, an excluded `__pycache__`, an empty file, an unterminated final line, and
identical content at two paths) and asserts fourteen fields plus the two traversal counters are
**equal**, normalizing away only `method` (a digest of the implementation, which is supposed to
differ), `root` (a temp path) and the two timestamps plus `elapsed_seconds`. It ran green against
the oracle on this machine.

The digest half carries the assertion where the oracle is not installed:
`EXPECTED_NORMALIZED_SHA256 = fc5f4cf78c67cc838490fa0ef9a5d4e9c786871b385edca5ccf27de9594616d7`,
recorded 2026-09-10 and cross-checked against the oracle in the same run. The oracle leg skips with a
printed reason when python3 or the script is absent; the digest never skips.

## The lens test no longer accepts either branch

`the_footprint_lens_needs_no_flag_in_a_tree_that_does_not_carry_it` accepted a resolved sample OR an
honest "unavailable", which meant a lens that never resolved anywhere would have passed it — the
review is right that such a test cannot fail. It is replaced by
`the_footprint_lens_is_native_and_needs_no_script_in_the_tree`, which asserts **unconditionally**
that a fixture provably carrying no script produced a real sample: a baseline receipt on disk,
`complete: true`, non-zero observed files and bytes, and `measurement_lens` naming the native
method. The paired-lens test also dropped its `--lens` argument entirely and now exercises the
default path.

## Round-three gate

```
cargo fmt --all --check                                          FMT_EXIT=0
cargo clippy --workspace --all-targets -- -D warnings            CLIPPY_EXIT=0
cargo test --manifest-path elohim/eprfs/Cargo.toml --workspace   TEST_EXIT=0

cd genesis/a2o && EPR_BIN=/tmp/eprfs-gate-target/debug/epr pnpm exec cucumber-js \
  --profile collective-memory      CM_EXIT=0   4 scenarios (4 passed), 17 steps (17 passed)
  --profile ceremony               CER_EXIT=0  5 scenarios (5 passed), 22 steps (22 passed)
```

Both cucumber profiles are green with **no `--lens` argument anywhere** — which is the fix, observed
rather than argued. Workspace totals: 50 test binaries, **651 tests passed, 0 failed**.

`just gate memory-ceremony` — the gate the review reported red — now exits **`GATE_EXIT=0`**, with
both profiles inside it green (5/22 and 4/17). Re-run once more against the INSTALLED binary rather
than the target-dir one, to prove the shipped bytes are what passed:

```
EPR_BIN=/opt/rust/cargo/bin/epr … --profile collective-memory   CM_EXIT=0   4 scenarios, 17 steps
EPR_BIN=/opt/rust/cargo/bin/epr … --profile ceremony            CER_EXIT=0  5 scenarios, 22 steps
```

Binary installed at `/opt/rust/cargo/bin/epr`, **32,124,816 bytes**, sha256
`579aa40a68ac1d012cd22b76e6f6006c935f63ce957eb4f8ca999f862d5b44ba` (round two was `4dafb9b3…`,
round one `a2f4a4d5…`). The usage line was corrected in the same pass — it still said the lens was
"resolved from the contract's repository", which stopped being true the moment the lens became
native, and a help line that describes a deleted mechanism is a small lie the next reader would
have believed. Cargo berth released.

## What round three did NOT do

- Did not delete `genesis/scripts/memory_balance.py` or its nine tests, per the instruction: station
  six retires them once the native lens is reviewed. Both still run.
- Did not touch `memkit-retention.py`, which the ninth Python case exercises and which is a
  station-one bound rather than a footprint measurement.
- Did not commit or push.
