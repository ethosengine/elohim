---
id: "backlog-local-view-delivery-port-followups"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Local-view delivery is ported dormant; its browser stories, security review and owner binding remain"
slug: "local-view-delivery-port-followups"
written: "2026-10-29"
author: "claude (rust-architect port, session 3f9afd7a, steward human:matthew)"
status: "open"
priority: "medium"
ci_status: none
jobs: []
tags: [local-views, elohim-hub, doorway, steward-node, rescue, devflow, security-review]
---

# Local-view delivery is ported dormant; what remains

The local-view delivery cluster from the 2026-10-16 power-loss rescue (`0ee5dfde8`) landed
on `port/local-view-delivery` (base `land/big-train`). Every live path keeps its behaviour by
default:

- `elohim/elohim-hub`: the crate, its `local_views` reader, the `local-view` example and its
  tests, in the `elohim/Cargo.toml` workspace, gated by `just gate elohim-hub`.
- `steward/node`: the existing dependency set sits behind a default `runtime` feature, and
  `--local-view <config>` exists only in the `--no-default-features --features
  local-view-only` build. The default binary has no new argv form and does not link
  `elohim-hub`.
- `doorway-service`: `/api/local-views` answers 404 with `Cache-Control: private, no-store`
  unless `LOCAL_VIEW_READER_URL` is a bare loopback `http` URL, read once per process.
  `--local-view-relay <loopback addr>` is a separate argv mode. No other path changes.
  Its gate predicate `reader_target` is registered in `doorway/doorway-service/seam-registry.yaml`.
- `app/scripts/local-view-launcher.{mjs,sh,test.mjs}`, `just dev view <declaration>`, and
  `elohim/elohim-storage/.epr-meta/idle-is-free.view.json`.

## Remaining work, in order

1. **Security review of the new listeners** (the rescue triage asked for it:
   `stranded-branches-after-2026-10-18-power-loss.md` row for `0ee5dfde8`). Scope: the hub
   reader (`elohim/elohim-hub/src/local_views.rs`: token check, loopback-only bind, view-id and
   CID validation, history file handling), the doorway relay
   (`doorway/doorway-service/src/server/local_peer.rs`: loopback-only target, path allow-list,
   no query, GET/POST only, Authorization required, no redirects), and the launcher access
   adapter (`app/scripts/local-view-launcher.mjs`: single-use capability, session cookie, origin
   and fetch-site checks, the `0.0.0.0` viewer listen). Record the verdict here; any finding
   becomes a fix on the owning file with a test. The relay buffers a whole reader response
   in memory (a snapshot bundle can approach the reader's 256 MiB capture bound plus base64);
   the review decides whether to stream it or cap it.
2. **Run the browser stories and lift `@wip`.** `genesis/a2o/features/devflow/local-view-board.feature`
   and `work-composition.feature` stay `@wip` (commit `70a05f987`). To run them: build the
   production app (`cd app/elohim-app && pnpm exec ng build --configuration production`), build
   the reader (`cd steward/node && cargo +1.96.1 build --profile local-view --no-default-features
   --features local-view-only --bin elohim-node`, pool slot, `RUSTFLAGS=''`, `CARGO_BUILD_JOBS=1`;
   the rescue measured a 20 min cold build), optionally build doorway for the relay leg, then
   `ELOHIM_NODE_BIN=<slot>/local-view/elohim-node LOCAL_VIEW_RELAY_BIN=<doorway bin>` with the
   features' named cucumber profiles. Removing `@wip` is a feature edit, so it needs a
   blind-reader pass (`story-legibility-rescued-lifecycle-and-declared-views.md` covers the
   neighbouring stories).
3. **Verified signed-in identity and owner binding** for the launcher (`TODO(identity)` in
   `app/scripts/local-view-launcher.mjs`): replace the capability/session adapter with the
   human's verified identity and bind each view's history to its owner.
4. **`steward/node/Dockerfile`** copies only `Cargo.toml` and `src`, so neither the existing
   path dependencies nor the new optional `elohim-hub` resolve inside it. It was already unable
   to build before this port; fix it together with the node's publication
   (`hub-enablement-dial-readiness-2026-06-21.md`).
5. **Blind-reader pass on `elohim/elohim-hub/README.md`.** The port changed its status
   paragraph, the "What to do with the answer" lead, one debt-table row and the "What goes
   here" list. The README review rule asks for a fresh-context blind-reader with the
   `readme` profile until READY; the porting session had no way to dispatch one. Run it
   before this branch reaches `dev`, and record rounds and per-class findings in the commit
   that closes it.
6. **Brand vocabulary in wire paths.** The launcher and its test serve `/avodah/views/<id>`,
   the path the app's existing `avodah.routes.ts` and `offline-view.repository.ts` already
   use; the brand-vocabulary validator flagged it (advisory). Rename the route family with
   the app side in one change, or record why it stays.
7. The rescued `_gate-elohim-hub` recipe ran `elohim/rakia/work-composition/contract.test.mjs`,
   which does not exist at the current rakia pin. The recipe now runs only
   `elohim/sdk/schemas/tests/work-composition.test.mjs`. When rakia publishes the composition
   contract test, add it back to the recipe and to the hub manifest's inputs.

Retire this item when items 1, 2, 3 and 5 are recorded here with evidence.
