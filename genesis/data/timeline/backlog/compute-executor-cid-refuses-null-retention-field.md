---
id: "backlog-compute-executor-cid-refuses-null-retention-field"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "compute-executor `cid` refuses any task whose retention carries an explicit JSON null — `maxRuns: null` (a shape the contract validator allows and operations.md documents) fails with `serde error: Unit is not supported`, so a count-unbounded or age-unbounded task can never be submitted or run"
slug: "compute-executor-cid-refuses-null-retention-field"
written: "2026-09-07"
author: "delegated-compute AUTHORITY leg, household mesh 2026-09-07"
status: "open"
priority: "medium"
jobs: [elohim-edge]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "habit:operator-runtime-surface"
tags: [delegated-compute, rakia-executor, cid, retention]
---

**Evidence (household mesh, 2026-09-07):**

```
$ compute-executor cid task-expiry.json      # retention {"maxAgeSeconds":60,"maxRuns":null,"expireWhen":"either"}
compute-executor: serde error: Unit is not supported
EXIT=1
$ compute-executor cid task.json             # retention {"maxAgeSeconds":3600,"maxRuns":2,"expireWhen":"either"}
bafyreifmcy6h5xlwsaniuhvqff77cs5n6imhm2wlpctsrfx7s5spayvrdq
EXIT=0
```

**Root cause:** `elohim/rakia/rakia-executor/src/main.rs:59` reads the task file into a
`serde_json::Value` and hands it to `contract::cid`, and
`elohim/rakia/rakia-executor/src/contract.rs:142-145` calls `ipld_core::serde::to_ipld(value)`.
`serde_json::Value::Null` serializes through `Serializer::serialize_unit`, which
`ipld-core-0.4.3/src/serde/ser.rs:190-192` rejects with `Err(ser::Error::custom("Unit is not
supported"))`. (`Option::None` on a typed struct is fine — `serialize_none` at ser.rs:244 yields
`Ipld::Null`; only an explicit JSON null reaches `serialize_unit`.) The same `Value` path is used
by `Commands::Run` (`main.rs:69` → `runtime::run(task, original, …)`), so such a task also cannot
execute even if it were somehow admitted.

**Why it matters:** `contract.rs:117-123` explicitly ACCEPTS a retention policy with one of the two
limits absent (it only refuses when *both* are `None`, or when either is `Some(0)`), and
`genesis/agentic/compute/operations.md` documents both
`{"maxAgeSeconds":432000,"maxRuns":null,"expireWhen":"either"}` and
`{"maxAgeSeconds":null,"maxRuns":5,"expireWhen":"either"}` as supported shapes. Neither can be
submitted today. `genesis/agentic/compute/retention.mjs:22-25` and `payloads.mjs:9-13` both handle
`null` correctly, so the JS side is already written to the documented contract.

**Workaround used for the AUTHORITY leg:** the expiry fixture was authored with
`{"maxAgeSeconds":60,"maxRuns":1,"expireWhen":"either"}`. The a2o assertion only requires
`maxAgeSeconds <= 60 && expireWhen === "either"`, so the scenario measured honestly — but it did
not exercise the count-unbounded shape.

**Cure candidates:** (a) serialize the JSON envelope through a wrapper whose `Serialize` maps
`Value::Null` to `serialize_none` (exact for every other variant, since it delegates to
`serde_json`'s own impl) and use it at both `main.rs:59` and the `runtime::run` original-value CID;
(b) decide instead that null-valued fields are excluded from the CID preimage — a contract change,
not a bug fix, and it would move every existing task CID shape, so it needs an explicit decision.
Do NOT add `skip_serializing_if` to `Retention` without a matching `#[serde(default)]`: the struct
is `deny_unknown_fields` and `atomic()` round-trips it through the same serializer.

**Done when:** `compute-executor cid` and `compute-executor run` accept a task carrying
`"maxRuns": null` (and one carrying `"maxAgeSeconds": null`), with a unit test in
`rakia-executor` pinning both shapes, and a mesh run measures the count-only retention policy
end to end.
