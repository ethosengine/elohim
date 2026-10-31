---
id: "backlog-local-view-delivery-deferred-steward-closure-conflict"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "port/local-view-delivery is deferred: a local-view binary inside steward/node cannot satisfy both the closure and read-set contracts"
slug: "local-view-delivery-deferred-steward-closure-conflict"
written: "2026-10-31"
author: "claude-opus-5-5"
status: "open"
priority: "high"
area: "steward/node · elohim-hub · orchestrator build-input contracts"
domain: "D5"
tags: [local-view, steward-node, elohim-hub, build-manifest, gate-input-audit, deferred]
---

# local-view-delivery is deferred, not dropped

Train 2 of the 2026-10-27..31 overnight integration merged `port/local-view-delivery`
(33eeb81cb hub + app launcher, 75264f4f6 steward `local-view-only` build, ported from the
0ee5dfde8 rescue) and then reverted that merge (873f289c9), because the pre-push orchestrator
gate refused it and no manifest edit satisfies both contracts:

- `genesis/orchestrator/build-inputs-closure.test.mjs` requires every gate and build step to
  watch its whole Cargo path-dependency closure. It counts optional dependencies regardless of
  features, so steward-node must watch `elohim-hub → elohim/eprfs/epr-cli → …` (10 paths), and
  `elohim/elohim-hub/build-manifest.json` was missing 27 closure paths of its own.
- `genesis/orchestrator/gate-input-audit.test.mjs` requires `REAL steward-node read set has no
  unresolved reads or repository-wide path`. Watching epr-cli puts its `repo_root()` reads
  (`flow/scope.rs`, `frames.rs`) into steward's read set, which then keys the whole tree.

The branch is intact; nothing of the port is lost.

| Row | What | State |
|---|---|---|
| 1 | Move the local-view binary out of steward/node into a bin of `elohim-hub` itself (or its own crate), so steward never depends on epr-cli; point `app/scripts/local-view-launcher.sh` and `just dev view` at it | open: the port owner's design call |
| 2 | Complete `elohim/elohim-hub/build-manifest.json`'s closure globs (the 27 paths; the list is reproducible with `node --test genesis/orchestrator/build-inputs-closure.test.mjs`) | open |
| 3 | Re-run the household receipt after re-landing: the port also changes doorway serving paths (`server/http.rs`, `server/local_peer.rs`, `server/mod.rs`, `main.rs`) | open |
| 4 | Alternative to row 1, only if the contract owner agrees: teach the closure test to resolve a step's enabled features instead of counting every optional dependency | open: judge change, needs the orchestrator owner |

## Done when

- The local view ships on dev with steward-node's read set still free of unresolved reads and
  every manifest watching its closure.
