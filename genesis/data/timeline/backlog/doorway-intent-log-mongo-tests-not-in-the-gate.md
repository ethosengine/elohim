---
id: "backlog-doorway-intent-log-mongo-tests-not-in-the-gate"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "The provisioning intent log's Mongo tests are not run by the doorway gate"
slug: "doorway-intent-log-mongo-tests-not-in-the-gate"
written: "2026-10-28"
author: "port/c-hosted (pre-landing review rev-5, finding 4)"
status: "backlog"
priority: "medium"
cites:
  - doorway/doorway-service/src/conductor/provision_intent.rs
  - elohim/holochain/build-manifest.json
tags: [doorway, hosted, provisioning, mongo, gate, test-coverage]
---

# The intent log's Mongo tests are not in the gate

`doorway/doorway-service/src/conductor/provision_intent.rs` has five tests named `mongo_*`. They exercise `MongoIntentStore` against a real mongod: the unique index, concurrent first inserts, versioned replace under contention, counts against the memory store, and a reopened store.

Before 2026-10-28 they printed `NOT RUN` and **passed** when `DOORWAY_TEST_MONGODB_URI` was unset. In every gate run that is a green with no mongod behind it. They are now `#[ignore]`d with a reason, so the test run reports them as ignored instead of passed. Run with `--ignored` and no URI, they now panic instead of passing.

Run them by hand from `doorway/doorway-service`, with the gate's cargo slot:

```bash
DOORWAY_TEST_MONGODB_URI=mongodb://127.0.0.1:27017 \
  cargo test --lib -- --ignored conductor::provision_intent::tests::mongo_
```

## What is missing

The doorway gate (`gate.projects.doorway` in `elohim/holochain/build-manifest.json`, recipe `gate` in `doorway/doorway-service/justfile`) starts no mongod. The Mongo path of the log is the one deployed doorways use, and no gate proves it.

## Cure

Give the doorway gate an isolated, throwaway mongod: a unix-socket `mongod` under the cargo pool slot, as the household mesh already runs one. Export `DOORWAY_TEST_MONGODB_URI` and run the `mongo_*` tests with `--ignored` as a separate step that fails the gate on failure.
