---
id: "backlog-conductor-sqlcipher-key-logged-by-sqlx-slow-statement"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "The conductor's SQLCipher database key is written to its log in clear whenever PRAGMA key takes over a second: sqlx logs slow statements at WARN with their full text"
slug: "conductor-sqlcipher-key-logged-by-sqlx-slow-statement"
written: "2026-10-12"
author: "claude-fable-5-1 (sprint 2026-10-11-idle-cpu-residual-sprint)"
status: "open"
priority: "high"
severity: high
nodes: [alpha, household]
tags: [conductor-fork, security, secrets, logging, sqlite, sqlcipher, observability, loki, trustful-self, friction-verify, plane-notary, unit-once, phase-transition, lane-operator, custody-everyone]
cites:
  - genesis/docs/superpowers/sprints/2026-10-11-idle-cpu-residual-sprint.md
  - genesis/data/timeline/backlog/conductor-residual-cpu-full-chain-read-and-perpetual-republish.md
---

# The database key in the conductor log

**What was seen.** While taking a bounded SQL statement-log window on the household (sprint
`2026-10-11-idle-cpu-residual-sprint`, 2026-10-12 ~02:40Z), the conductor log at the DEFAULT level — the
household's `MESH_RUST_LOG` begins `warn,…`, the fleet's conductor containers run
`RUST_LOG=info,elohim_storage=debug,…` (`genesis/orchestrator/manifests/humans/_edgenode-conductor.template.yaml:288`)
— carried lines of the shape

```
WARN sqlx::query: …/sqlx-core-0.9.0/src/logger.rs:121: slow statement: execution time exceeded alert threshold
  summary="PRAGMA key = \"x'<64 hex chars>'\"; …" db.statement=…
```

on three separate days of ordinary operation (2026-10-04, 2026-10-08, 2026-10-12, james's log; counted, not
printed). The hex is the SQLCipher key for that database. The evidence copies under
`genesis/a2o/reports/recovery/idle-residual-20261011/stmt-*.log` have the hex redacted; the live household logs
under `genesis/local-dev/household-dowell/conductors/.sandbox_run_log.*` still carry it (gitignored, local,
dev-fixture keys — but the same mechanism runs on alpha, whose logs ship to Loki).

**Why.** `holochain_data` (sqlx 0.9.0, `crates/holochain_data/Cargo.toml:12`) applies the cipher as connect
pragmas: `crates/holochain_data/src/key.rs:192` `opts.pragma("key", format!("\"x'{}'\"", self.key_hex()))`.
sqlx-core executes connect pragmas as ordinary statements and its logger
(`sqlx-core-0.9.0/src/logger.rs:108-149, 201-208`) emits every statement at DEBUG and every statement slower
than 1 s at **WARN**, with `summary` (the first words — which for this statement is the whole key) and
`db.statement`. The connect options never call `log_slow_statements` or `disable_statement_logging`
(`crates/holochain_data/src/lib.rs:209-273`), so the sqlx defaults stand. Key derivation on `PRAGMA key` under a
busy disk routinely crosses 1 s at start-up, which is exactly when every database is opened.

**Exposure.** Anyone who can read conductor logs (pod logs, Loki, a shipped crash bundle, a pasted log in a
ticket) holds the key to that conductor's encrypted databases. The key is per-conductor and derived from the
keystore's material, so a leaked key does not sign as the agent, but it opens the encrypted DHT and authored
databases at rest, which is the whole purpose of encryption-at-rest. The hosted fleet (adam, matthew, …) is the
exposed surface; the household is the place it was seen.

**Cure (fork, one line; carried on `perf/scheduler-latch-v2` as its fifth commit in the sprint above).** On the
`SqliteConnectOptions` holochain_data builds, `.log_slow_statements(log::LevelFilter::Debug, Duration::from_secs(1))`
so the default level never prints a statement body; `RUST_LOG=sqlx::query=debug` still yields the full
statement log an operator opts into deliberately (and knows carries keys at connect). Not chosen:
`disable_statement_logging()` (loses the operator's window) and redacting in the log formatter (treats the
symptom; sqlx's `summary` is computed before any layer sees it). Upstream Holochain should carry the same line;
until a pin that includes it rolls, every conductor log on the fleet is to be treated as holding the keys of the
databases it opened.

**Operational follow-through (operator's, not this sprint's).** Decide whether alpha's Loki retention of
conductor WARN lines since the sqlx 0.9 move warrants a key rotation; the fork's `hc_db_*` metrics, not the slow
statement log, are the slowness signal to keep.

**Done-when.** A fleet conductor log at the default level contains no `PRAGMA key` statement text after a start
with cold databases (grep `PRAGMA key` over the first 10 min of a pod's log = 0), with the pin carrying the cure.
