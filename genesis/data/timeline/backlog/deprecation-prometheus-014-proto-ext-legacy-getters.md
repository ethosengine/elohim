---
id: "backlog-deprecation-prometheus-014-proto-ext-legacy-getters"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "prometheus 0.14 proto_ext legacy getters — one new storage test is the tree's only user"
slug: "deprecation-prometheus-014-proto-ext-legacy-getters"
written: "2026-09-04"
author: "deprecation-triage"
status: "backlog"
priority: "low"
deprecation_status: open
severity: low
fingerprints: [61fa1af687e2, 6fa93a843711, 4e9dc83316f5]
relatedNodeIds: []
tags: [deprecation, rust, cargo, prometheus, elohim-storage, metrics, mechanical]
cites:
  - elohim/elohim-storage/src/metrics.rs
  - elohim/elohim-storage/Cargo.toml
  - elohim/elohim-storage/justfile
  - genesis/docs/superpowers/plans/2026-09-05-epr-app-deliverability-verdict-slice1.md
---

## What is deprecated

Three rustc warnings, emitted while compiling the `elohim-storage` test target:

```
warning: use of deprecated method `prometheus::proto_ext::<impl prometheus::proto::MetricFamily>::get_name`: Please use `.name()` instead
warning: use of deprecated method `prometheus::proto_ext::<impl prometheus::proto::LabelPair>::get_name`: Please use `.name()` instead
warning: use of deprecated method `prometheus::proto_ext::<impl prometheus::proto::LabelPair>::get_value`: Please use `.value()` instead
```

`prometheus` 0.14.0 marked the legacy protobuf-2-era `get_*` accessors on the
generated proto model `#[deprecated(since = "0.14.0")]` in favour of the
protobuf-3 generated accessors. Verified against the vendored crate source
(`prometheus-0.14.0/src/proto_ext.rs`): the deprecated set is `get_name`,
`get_help`, `get_value`, `get_timestamp_ms`, `get_quantile`,
`get_cumulative_count`, `get_upper_bound`, and `Summary::get_sample_count` /
`Summary::get_sample_sum`. **`get_metric()`, `get_label()`, `get_counter()`,
`get_gauge()`, `get_histogram()`, `get_field_type()` carry no deprecation
attribute** — they are not part of this concern.

### Causal note — this is fallout from the RUSTSEC-2024-0437 bump, surfacing late

`85370bdc4` (2026-07-30) bumped `elohim-storage` from `prometheus 0.13` to
`0.14` to evict `protobuf 2.28.0`. That commit correctly recorded "zero
call-site edits" because storage's metrics code used only
`Registry` / `IntGauge*` / `IntCounter*` / `Opts` / `TextEncoder::encode`, none
of which changed. The deprecated surface is the **proto model read path**, which
no code in the tree touched until 2026-09-04. It is not a regression in the
bump; it is the first code to reach the newly-deprecated accessors.

## Usage inventory

Repo-wide grep for the deprecated accessors across all Rust sources
(`get_name()`, `get_value()`, `get_help()`, `get_timestamp_ms()`,
`get_cumulative_count()`, `get_upper_bound()`) — **one site**, four lines, all
inside a single `#[cfg(test)]` function:

| File | Line | Call | Replacement |
|---|---|---|---|
| `elohim/elohim-storage/src/metrics.rs` | 5849 | `f.get_name()` (`MetricFamily`) | `f.name()` |
| `elohim/elohim-storage/src/metrics.rs` | 5854 | `l.get_name()` (`LabelPair`) | `l.name()` |
| `elohim/elohim-storage/src/metrics.rs` | 5854 | `l.get_value()` (`LabelPair`) | `l.value()` |

All three sit in `mod tests::app_deliverability_verdict_counter_is_registered_and_labelled`,
added by `67da5542d` ("count deliverability verdicts by verdict and reason class
(C8)", Task 3 of the 2026-09-05 epr-app-deliverability-verdict-slice1 plan).

**This is a new pattern, not an existing one.** The dispatch hypothesis was that
other counter-registration tests in `metrics.rs` already used these getters —
they do not. `REGISTRY.gather()` has exactly two callers in the file: the new
test, and `gather_text()` (line ~3029), which hands the families straight to
`TextEncoder::encode` and never reads a field. The neighbouring histogram test
calls `.get_sample_count()` on a `prometheus::Histogram` — the crate's own
metric type, *not* `proto::Summary` — which is not deprecated. So the deprecated
proto-read idiom has exactly one instance in the tree, and it is one commit old.

Unrelated homonyms deliberately excluded: `elohim/brit/src/{porcelain,plumbing}/main.rs`
call `app.get_name()` on a **clap** `Command`, which is not deprecated.

## Migration path

Purely mechanical, three token substitutions in one function, no logic change.
The replacements are drop-in with identical signatures — verified against the
generated model `prometheus-0.14.0/proto/proto_model.rs`:

- `LabelPair::name(&self) -> &str` (line 53) ← replaces `get_name() -> &str`
- `LabelPair::value(&self) -> &str` (line 89) ← replaces `get_value() -> &str`
- `MetricFamily::name(&self) -> &str` (line 1697) ← replaces `get_name() -> &str`

Return types are unchanged (`&str` in all three), so the surrounding
`== "elohim_app_deliverability_verdict_total"`, `== "verdict"` and `== "broken"`
comparisons compile untouched. There is no API-shape decision to make and no
upstream guide to follow beyond the deprecation notes themselves.

Leave `f.get_metric()` and `m.get_label()` alone — not deprecated, and swapping
them would be churn.

## Current decision

**Open — canonicalized, not fixed this run, deliberately deferred to a mechanical
follow-up. This is warn-only noise, not a gate red.**

Three reasons the fix was not landed here:

1. **Concurrency fence.** `elohim/elohim-storage/src/metrics.rs` and `src/http.rs`
   are being finalized by a concurrent session completing Task 3 of the
   epr-app-deliverability-verdict-slice1 plan. A background triage run editing
   those two files would collide with in-flight work.
2. **The code is not in this worktree.** This agent is isolated in
   `.claude/worktrees/agent-a4a388fbe3bc78414`, which predates `67da5542d`; its
   `metrics.rs` has no such test. The fix cannot be written *or verified* here
   without reaching into the shared checkout, which the isolation contract
   forbids.
3. **Nothing is red, so nothing is urgent.** `just gate elohim-storage` runs
   `cargo clippy -- -D warnings` **without `--all-targets`**, so `cfg(test)` code
   is never linted under deny-warnings. The three warnings appear only during
   `cargo test`'s compile, which does not deny. Confirmed by the fact that
   `67da5542d` landed on `dev` at all. Left alone, this costs three lines of
   compile noise and nothing else.

**Next step (bounded, ~2 minutes, whoever next touches storage metrics):** apply
the three substitutions above, then
`RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=<pool slot> cargo test --lib metrics::tests::app_deliverability_verdict`
— echoing `EXIT=$?` on its own line, unpiped, per the repo's cargo-exit-code
trap. Green run with the three `warning: use of deprecated method` lines gone is
the whole verification. Then delete this entry and the three ledger fingerprints.

**Watch item for the follow-up:** if `cargo clippy` is ever widened to
`--all-targets` (a reasonable hardening), these three warnings become a **gate
red** rather than noise. That change and this fix should land together, or this
fix should land first.

**Correction at integration (2026-09-05).** The three call sites had already been
moved to `.name()` / `.value()` in commit `49972934b` on 2026-09-04 ("fix(storage):
metrics tests use prometheus 0.14 name()/value()", `elohim/elohim-storage/src/metrics.rs`,
5 lines) — that commit names fingerprints `fe49ee2127a3 19709474b9b5 cd1e5e2f933b`
and reports the accessors as *failing* the crate gate, so the "nothing is red"
reasoning above was already stale when this entry was written from an older
worktree. What is still outstanding is only the proof: no verified
`cargo clippy -- -D warnings` plus `cargo test` run on `elohim-storage` has been
recorded since, so this entry stays open on the verification leg rather than the
edit leg. A duplicate entry with slug `deprecation-prometheus-014-proto-getname-getvalue`
(same three fingerprints) covered the identical concern and was retired unmerged
in favour of this one; the three ledger rows still point at that retired slug and
should be repointed here when the ledger is next writable.

## Verification

N/A — not fixed. Closure evidence will be: the three substitutions applied, the
storage test target compiling with zero `proto_ext` deprecation warnings, and
the named test green. Delete this entry and ledger lines `61fa1af687e2`,
`6fa93a843711`, `4e9dc83316f5` at that point — a reintroduction then correctly
re-fires the sentinel as new.
