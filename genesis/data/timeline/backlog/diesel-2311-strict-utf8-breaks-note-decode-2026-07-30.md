---
id: "backlog-diesel-2311-strict-utf8-note-decode"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "diesel 2.3.5→2.3.11 (vuln-lane bump) makes SQLite TEXT decode strict — economic_events note round-trip test fails"
slug: "diesel-2311-strict-utf8-note-decode"
written: "2026-07-30"
author: "agentic-developer"
status: "wip"
priority: "high"
area: "elohim-storage"
domain: "dev"
severity: "gate-blocker"
tags: [diesel, sqlite, utf8, vulnerability-remediation, pre-push-gate, elohim-storage]
relatedNodeIds:
  - "memory:feedback_concurrent_sessions_shared_worktree"
shift_objective: |
  PROVENANCE CORRECTED 2026-07-30: this entry originally read "the Codex session's UNCOMMITTED
  Cargo.lock bump". Both halves were wrong. The bump is `85370bdc4`
  ("fix(storage): close protobuf/keccak/diesel/tar/bytes/quinn/yamux advisories via
  cached-mirror bumps"), authored by the Claude Opus 5 vulnerability-cluster campaign
  (session_018e5qyRbBTcbC8DipEECwmC, cluster 08) — and it is COMMITTED **and already on
  origin/dev**, not sitting in a working tree. So "must not land on dev until green" is moot:
  it landed. Treat this as a regression on origin/dev, not a pending bump. Owner is that
  campaign; see the escalation note below the frontmatter.

  Original body: the elohim/elohim-storage/Cargo.lock bump (diesel 2.3.5 → 2.3.11, part of the
  rustls/reqwest vuln sweep) breaks the committed test
  db::economic_events::tests::list_load_path_decodes_every_constructible_row_shape:
  "Error deserializing field 'note': invalid utf-8 sequence of 1 bytes from index 0".
  Deterministic (fails in isolation in 0.21s), NOT a flake. The test is the tree's guard that
  every constructible row decodes on the .load() path — diesel 2.3.11 made TEXT→String decode
  strict where 2.3.5 tolerated/lossy-decoded invalid UTF-8. Committed dev (diesel 2.3.5) is
  green; only the working tree with the bumped lock fails.
  Resolution options for the lane owner, in preference order: (a) sanitize/refuse non-UTF-8
  `note` bytes at the INSERT path and migrate/clean any legacy rows, keeping the test's
  guarantee honest under 2.3.11; (b) custom lossy deserializer for the note column; (c) pin
  diesel below the strictness change ONLY if the vuln advisory allows. The bump must not land
  on dev until this test is green under the new lock — the pre-push gate enforces exactly this
  (it blocked the 2026-07-30 saga push; the saga lane pushed its own payload independently).
---

Evidence: pre-push gate log 2026-07-30 ~00:20Z (2247 passed / 1 failed / 521s) + isolated
re-run. Lock diff also drops reqwest 0.11/rustls 0.21 chain and bumps rand/getrandom families —
the diesel bump is the only observed test breakage.

## Escalation 2026-07-30 — this is an availability regression, not a test-fixture problem

The test is not asserting that malformed data is acceptable. It inserts `x'ff'` into `note` and
non-numeric TEXT into a REAL column **via raw SQL on purpose**, then asserts
`list_economic_events` still returns `Ok` with the clean sibling row present. Its invariant is
**resilience: one corrupt row must not break the whole list query.**

Under diesel 2.3.11's strict TEXT→String decode, a single non-UTF-8 `note` byte fails the entire
`.load()` — so one bad row takes down the whole economic-events list endpoint in production, not
just in tests. That is the same failure class already paid for once in this repo (one poisoned
scope row emptying `EprRouter`; see `memory:project_epr_router_empties_on_poisoned_scope`, cured
by fail-closed per-row collection). The bump silently re-introduced it.

Consequence for the resolution options above: **(a) sanitize-at-INSERT does not fix this.** The
hostile bytes are already in the database, and sanitizing writes cannot repair existing rows —
which is precisely what the test simulates. (c) pin-below-strictness restores the invariant but
gives back the four advisories the bump closed (#535/#539/#540/#543).

Preferred resolution is **(b), per-row resilient decode**: keep diesel 2.3.11 and make the
text-column decode lossy/fail-soft so a corrupt row degrades to a lossy value (or is skipped
with a counter) instead of failing the batch. That satisfies both the advisory and the
invariant. Until that lands, reverting the diesel bump alone is the correct interim state,
because a real availability regression outranks four advisory rows that are not otherwise
urgent.

Note the verification gap that let this through: the campaign's closure standard was
`cargo check --locked --all-targets`, which **compiles** tests without running them. A
dependency bump that changes runtime decode behaviour passes that gate cleanly. Dependency
closures on elohim-storage need `cargo test`/`nextest`, not `cargo check`.

## Resolution 2026-07-30 — resilient TEXT decode; diesel stays at 2.3.11, advisories stay closed

Took the **forward-fix path (b)**. `diesel 2.3.11` is unchanged in
`elohim/elohim-storage/Cargo.lock`, so **#535 / #539 / #540 / #543 remain closed** — no
advisory was reopened, and `VULNERABILITY_CLUSTER_08_RUST_STORAGE_RUNTIME.md` needs no edit.

**Reproduction (before the fix):** `EXIT=101`, `0 passed; 1 failed`, 0.20s, exact reported
error. The wrapper script's own exit code read `0` while cargo returned 101 — the explicit
`echo "EXIT=$?"` on its own line is what caught it, confirming the piped-output trap this
campaign hit three times.

**Root cause (mechanism, confirmed against vendored diesel source).** Not a "strictness
policy" change — a **soundness fix**. diesel 2.3.5 decoded `String` from `Text` via
`impl FromSql<VarChar, Sqlite> for *const str` → `SqliteValue::read_text()` +
`str::from_utf8_unchecked` (UB on non-UTF-8 bytes, but tolerant). 2.3.11 routes
`String: FromSql<Text, Sqlite>` through `<&str as FromSqlRef>::from_sql` →
`SqliteValue::as_utf8_str()`, a strict `str::from_utf8`
(`type_impls/primitives.rs:129-135`, `sqlite/types/mod.rs:28-31`). Correct fix upstream —
but it converts a per-value oddity into a whole-batch `.load()` failure. The old test's
"documented negative result" was resting on the UB.

**The seam that made this cheap:** `SqliteValue::read_text()` is *still* public and *still*
lossy in 2.3.11 (`parse_string` → `String::from_utf8_lossy`; diesel's own unit test asserts
`x'fffefd'` → three U+FFFD). Only `String`'s path became strict.

**Fix** — `elohim/elohim-storage/src/db/lossy_text.rs` (new): `LossyText` /
`LossyTextOpt`, one shared `deserialize_as` shim pair decoding via `read_text()`. Applied to
`EconomicEvent.note` and `EconomicEvent.metadata_json` (`db/models.rs`) with
`#[diesel(deserialize_as = LossyTextOpt)]` — **field types stay `Option<String>`, so there is
zero ripple** into services, views, `views_convert`, or ts-rs output (generated TS byte-identical;
no codegen churn). Non-obvious requirement worth recording: diesel's `FromSqlRow` blanket routes
through **`Queryable`**, not `FromSql` — a `deserialize_as` target needs `Queryable<ST, Sqlite>`
with `type Row = Self` (the shape diesel itself uses for `*const str`) or it fails with
`FromSqlRow ... is not satisfied`. `LossyTextOpt` must also override `from_nullable_sql`, whose
default returns `UnexpectedNullError`.

**Blast-radius survey (the reason this is 2 columns, not ~800).** Every `TEXT` column in the
crate is written from a Rust `String`/`&str`, which is UTF-8 by construction, so **no in-crate
write path can produce a non-UTF-8 `TEXT` value**. Verified: binary payloads are declared
`Binary` in `diesel_schema.rs` (`sealed_blob`, `signer_pubkey`, `canonical_bytes`,
`payload_bytes`, `proof_bytes`, `share_data`, `request_nonce`) and land in BLOB columns, not
TEXT; the only non-test raw-SQL writes are in tests (`relationships_diesel.rs:675`,
`content_diesel.rs:2619`); the dynamic EPR projector (`projector/mod.rs`) resolves every column
to a `String` from `serde_json::Value` and inlines it as an escaped TEXT literal; no
`from_utf8_unchecked` outside doc comments. Residual exposure is therefore a foreign writer to
the SQLite file, storage corruption, or a future path binding bytes to a TEXT column — so
blanket-wrapping ~97 model structs would be churn without a matching threat. Wrapped the two
columns whose *contract* is opaque text; the shim is now a one-line opt-in for any future column.

**Residual gap (deliberately not closed here, follow-on).** The shim makes *`TEXT`* decode
fail-soft, not every column class: a genuinely un-coercible `INTEGER`/`REAL`/`Binary` value, or a
`NULL` in a non-`Option` field, still fails the whole `.load()`. (The test's Shape 2 —
non-numeric TEXT in a REAL column — still passes only because SQLite coerces it to `0.0`.) The
general cure is per-row loading at the batch-load call sites: `load_iter()` + `filter_map` +
`warn!` + a skipped-row counter, i.e. the same per-row degradation that cured the `EprRouter`
poisoned-scope incident. That is a broader change across many `.load()` sites and wants its own
scoping; it is **not** required to restore this test's invariant.

**Adjacent observation, not fixed (out of scope, no orphan intended):**
`src/api/standing.rs:118-119` builds `evaluator_cid`/`subject_cid` via
`String::from_utf8(bytes).unwrap_or_default()` over `Binary` pubkey columns — for real key bytes
that silently yields `""` rather than an error. Pre-existing, unrelated to this bump; worth its
own triage.

**Verification.** All from `elohim/elohim-storage`, `RUSTFLAGS='--cfg getrandom_backend="custom"'`,
`CARGO_BUILD_JOBS=4`, cargo-pool slot, output redirected with an explicit `EXIT=` line (never piped):
- target test: `list_load_path_decodes_every_constructible_row_shape ... ok`
- full lib suite: `EXIT=0` — **2253 passed / 0 failed / 2 ignored** in 133.37s, vs the pre-push
  baseline of 2247 passed / 1 failed. Delta accounts exactly: the repro run totalled 2250
  tests, +5 new `db::lossy_text` unit tests = 2255 = 2253 passed + 2 ignored.
- `cargo fmt --check`: clean (`FMT_EXIT=0`).
- 29 pre-existing clippy lints (incl. `await_holding_lock`) deliberately untouched — documented,
  out of scope.

`cargo-nextest` is **not** installed in this container (`/opt/rust/cargo/bin` holds only rustup
shims), so the closure standard for a dependency bump here is plain `cargo test --lib` with a
redirected log — never a pipe, which masks cargo's exit code.
