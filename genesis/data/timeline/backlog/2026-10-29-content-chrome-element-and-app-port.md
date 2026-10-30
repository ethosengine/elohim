---
id: "backlog-content-chrome-element-and-app-port"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Port the rest of content-chrome layer 2: the doorway apps origin and CSP relay, the page-info element, storage's peer-native /apps shell, the app changes and the trust-edge story"
slug: "content-chrome-element-and-app-port"
written: "2026-10-29"
author: "claude (rust-architect, porting chrome-trust-edge layer 2 to port/chrome-trust-edge)"
status: "backlog"
priority: "high"
relatedNodeIds:
  - "elohim/elohim-chrome-asset/.epr-meta/content-chrome-trust-edge.habit.md"
  - "genesis/docs/superpowers/specs/2026-10-14-content-apps-wear-chrome-design.md"
  - "genesis/data/timeline/backlog/2026-10-29-record-witness-coordinator-extern.md"
tags: ["chrome", "content-chrome-trust-edge", "doorway", "serving-path", "deferred-port", "a2o"]
shift_objective: |
  The content chrome's story (content-chrome-trust-edge.feature, nine scenarios) runs on the
  household with the chrome switched on and passes, while a household run with it switched off
  keeps epr-app-deliverability and served-shell-boots green with byte-identical /apps answers.
  Done when the parts below are on dev behind one default-off switch per runtime and a
  serving-receipt lane has measured both settings.
---

## What landed and what did not

The source is the snapshot `47ba8da76` (branch `feat/chrome-trust-edge`, also
`refs/held/feat/chrome-trust-edge`), preserved 2026-10-23 from `.worktrees/wt-sw-preload` on
base `db4f7634a`. The 2026-10-29 port on `port/chrome-trust-edge` landed, dormant by default:

- the SDK schemas and codegen (frame-capability, page-info-section, the undeclared-behaviour
  signal, chrome-context, content-chrome-facing, the html5-app body), lamad-types'
  `Html5AppBody` parser and generated vocabulary, the elohim-views facing structs and their
  ts-rs types, and body declarations on five seeds (legacy `metadata.securityPolicy.sandbox`
  and `requiredCapabilities` kept, because the current renderers still read them);
- the elohim-facings `content_chrome` fold;
- in storage, behind `ELOHIM_CONTENT_CHROME` (off unless `1|true|yes|on`):
  `GET /api/v1/epr/{id}/chrome`, `POST /api/v1/epr/{id}/signals` (64 KiB body cap; 30 reports
  per address and 10 per source per minute; at most 4096 tracked keys), both declared in
  `/manifest` only when on, and the witness probe registered only when on;
- in elohim-chrome-asset, the v1 context types (`ContentIsland`, `DeliveryHop`, `Viewer`, …),
  `policy.rs` and `render_content_shell`. These are Rust additions only: every v1 field is
  `None` by default, so a pre-v1 island stays byte-identical (`pre_v1_contexts_stay_byte_stable`).

Everything below is NOT ported. Each part changes bytes on an existing serving route, or only
works together with a part that does. A switch cannot make any of them dormant without keeping
the old path alive beside the new one.

## Exact plan, in landing order

### 1. The element (elohim-chrome-asset)

- Port `src/omni-element.js` (+1803 lines: page info, the frame built from
  `data-frame-src`, grants under `elohim-grants:<contentAddress>`, the generated vocabulary)
  and `build.rs`, which splices `src/generated/chrome-vocabulary.json` (already committed and
  kept fresh by `pnpm run schema:codegen:ts --verify`) into the `/*__CHROME_VOCABULARY__*/null`
  placeholder. `ELEMENT_JS` then reads `concat!(env!("OUT_DIR"), "/omni-element.js")`.
- Port `tests/frame-attributes.test.mjs`, and the JS-asserting `lib.rs` tests that the 10-29
  port left out (`source_declares_the_page_info_hooks_and_row_order`,
  `source_declares_a_frame_with_no_sandbox_escape`,
  `element_carries_the_spliced_generated_vocabulary`,
  `frame_attribute_function_passes_its_node_tests`, the user-agent preference test, and the
  rewritten claims tests).
- Port the `escape_json_for_script` fix (every `<`, `>`, `&`, U+2028 and U+2029 becomes a
  `\u` escape). The old `<\!--` rewrite is not a valid JSON escape and corrupts a title
  containing `<!--`. This changes island bytes for titles containing those characters, so it
  ships with the element, with `hostile_title_round_trips_through_the_island`.
- Why this cannot land dormant: the element is content-addressed and served to every page that
  mounts the chrome today, and changing it changes `element_js_hash` and what every medium
  loads. Gate it as a SECOND served asset (a v1 element at its own hash), chosen by the
  producer when its switch is on. Do not replace `ELEMENT_JS` in place.
- `elohim-render`: the snapshot deleted `src/chrome/omnibar.rs` (1188 lines, the Rust SSR
  omnibar) and re-exported the v1 types. Grep its callers on dev first. Delete it only when no
  SSR path renders it.

### 2. Storage's peer-native /apps shell

- Restore `src/apps_chrome.rs` from the snapshot. The 10-29 port also kept a copy, with an
  env-switch helper added, at `genesis/local-dev/port-chrome-trust-edge/apps_chrome.rs.deferred`
  (gitignored). Then restore `handle_apps_get`, `resolve_app_record` and `render_app_shell` in
  `src/http.rs`, the `/_elohim/shell/apps/` arm, the `content_checks` map (hash the ZIP before
  extraction, recorded but not enforced), and `manifest_with_peer_identity` (the
  `peer: {agentKey, transportId}` on `/manifest`).
- Gate every one of these on the existing `HttpServer::content_chrome` field. With it off,
  `/apps` must call `handle_app_request` unchanged, nothing may be hashed or recorded, and
  `/manifest` must stay exactly `build_manifest()`. The snapshot's chrome tests drive a server
  built with `.with_content_chrome(true)`. Keep the five pre-existing reach tests
  (`apps_slug_of_a_restricted_row_is_refused_anonymously` …) on the default server.
- Lands with step 1: the shell has no iframe, and only the v1 element builds one.

### 3. The doorway (apps origin, content shell, CSP relay)

- `routes/apps.rs` (+2876/−… lines; unchanged on dev since `db4f7634a`, so the snapshot hunk
  applies cleanly). `handle_app_request` gains `ctx: &AppServeContext`. The shell becomes
  `content_shell_response` (the v1 shell) in place of `chrome_shell_response`. Path refusal no
  longer bypasses to storage under the admin cache bypass. The `apps.<name>` pairing,
  `police_raw_html`, `boot_response` and the Report-Only policy are added.
- Dormancy: pairing is already opt-in (`DOORWAY_APPS_NAMES` empty, or a membership document
  without `"apps": true`). The UNPAIRED path still changes, though: the v1 shell replaces the
  current one, and the bypass and path rules change. Add `DOORWAY_CONTENT_CHROME` (default
  off) that keeps `chrome_shell_response`, `fetch_holders`, `html_escape` and the old path
  checks exactly as they are on dev today, and takes the new path only when on. Test that the
  off answer is byte-identical for an entry navigation, a frame load and a raw asset.
- `routes/csp_report.rs` (622 lines, new): `POST /_elohim/csp-report/<address>` relays to
  storage's `POST /api/v1/epr/{id}/signals` with `X-Elohim-Relayed-By`,
  `X-Elohim-Observed-Origin` and `X-Elohim-Document-Match`. It is already bounded: a 64 KiB
  body (`CSP_REPORT_MAX_BYTES`), 20 reports per client per 60 s, 4096 tracked keys. Register
  it only when `DOORWAY_CONTENT_CHROME` is on, in BOTH the main-listener match arm and
  `is_service_path` (otherwise the EPR router shadows it), with an `is_service_path` unit
  test.
- Reconcile with dev, which has moved since the snapshot: `main.rs` (+210/−30) and
  `server/http.rs` (+348) carry hosted provisioning and the breaker, `config.rs` (+30) and
  `seam-registry.yaml` (+879) have changed, and `routes/mod.rs` gained four modules. Re-derive
  the snapshot's `http.rs` hunk (apps-host routing by `request_edge`) against the new request
  path; do not 3-way it blind. Also: `auth_discovery.rs` (the `apps: Option<bool>` field, only
  ever `Some(true)`; the schema already admits it), `route_registry.rs`,
  `name_routing/standing.rs`, `identity.rs`, `auth/operator.rs`, `cache/app_file_cache.rs`,
  `blob.rs`, `tests/schema_contract.rs` (`auth_discovery_with_apps_flag_matches_schema`),
  `Cargo.toml` (adds `elohim-chrome-asset` and `lamad-types` with `json`), and the
  Dockerfile's `COPY` and `sed` lines for `elohim/sdk/domains/lamad/types` (a new path-dep
  needs both, or the edge build breaks).
- `app/elohim-app/scripts/hc-mesh.sh`: `MESH_APPS_ORIGINS` / `MESH_APPS_PUBLIC_NAME`
  (`elohim.localhost`) and the doorway's `DOORWAY_APPS_NAMES`. Default it OFF in the port,
  so the household's existing lanes are unchanged until the steward turns it on.

### 4. The apps worker and the navigation preload

- `app/elohim-app/src/apps-sw.ts` (+36/−…). The returning visitor whose apps worker is
  installed still lands in the chrome. Station 5 of `epr-app-deliverability.feature` proves
  it. Add `app/elohim-app/src/apps-sw\.ts$` to `SERVING_RE` in
  `genesis/orchestrator/scripts/t2-receipt.sh`, with the `serving-receipt.test.mjs` case.
  Run the serving receipt on the household before landing.

### 5. The Angular surfaces

- lamad: `apps-origin.service.ts` (new, with spec), `iframe-renderer.component.ts`,
  `html5-app-format.plugin.ts` (+245) and `content-viewer` (the record page frames the app
  from the apps origin; the wrapper, `presents: true`, means no second chrome). The renderer
  must keep reading `metadata.securityPolicy.sandbox` until it reads `contentBody.capabilities`.
  Only then remove the legacy sandbox and `requiredCapabilities` metadata from the five seeds
  (the snapshot removed them; the 10-29 port kept them).
- elohim-app: `epr-home` (the legs, the model and the page sections that page info's cards
  link to with `#<section>`), the resiliency badge in elohim-elements, and its Storybook story.
- Run vitest for each touched app file.

### 6. The story

- Port `genesis/a2o/features/protocol/content-chrome-trust-edge.feature`,
  `steps/protocol/content-chrome-trust-edge.steps.ts`, `scripts/lib/chrome-shell.ts` and
  `scripts/lib/chrome-testids.ts`, plus the snapshot's edits to `dataplane.steps.ts`,
  `epr-app-deliverability.{feature,steps.ts}` (+281 in the feature), `sut.ts` and the other
  step files.
- The snapshot also renamed the step vocabulary `peer "<name>"` to `federated peer "<name>"`
  across about 35 feature files. That rename is separate from the chrome (the "three peer
  relationships" distinction). Land it as its own commit, with the step definitions accepting
  both phrasings for a release, and not mixed into the chrome commits.
- Every new or edited `.feature` needs a context-isolated blind-reader pass until it is READY.
  The atom records ten READY rounds in `wt-sw-preload`. Any prose changed since then needs a
  fresh pass. The 10-29 port had no subagent tool to run the pass, so it did not land the
  story.

### 7. Afterwards

- Once steps 1–6 pass on the household with both switches on, write the measure as a DELTA in
  `content-chrome-trust-edge.habit.md` and re-project with `.claude/scripts/habits-project.py`.
- Update the elohim-hub README debt row: the snapshot's row names `apps_chrome.rs`, which
  lands in step 2.
