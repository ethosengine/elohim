---
title: join-alpha still runs the full local DNA WASM build it will never install
created: 2026-06-10
domain: process-meta (build-and-test; dev stack)
source: arc plan Task 2.2 in-flight simplification sweep (2026-06-10)
severity: low
---

`hc-start.sh` Step 1 builds all 5 WASM DNAs when `HAPP_PATH` is missing even
under `NETWORK_PROFILE=join-alpha` without `FORCE_LOCAL_HAPP` — minutes of
cargo on a fresh clone for a bundle the generate wrapper won't install (it
installs `deployed-bundles/elohim.happ`). Bounded fix: skip Step 1 when
`join-alpha && !FORCE_LOCAL_HAPP`. Sibling accretions same sweep: oras install
snippets duplicated across dna/Jenkinsfile (~:903) and edge Jenkinsfile
(~:108) — a shared `scripts/ci/` helper fits the heredoc-free rule; and the
DNA pipeline's version-metadata `sed` into happ.yaml (dna/Jenkinsfile:638)
doesn't survive `hc app unpack`, so bundle provenance isn't recoverable from
the artifact (the fetcher's `.src` sidecar partially fills the gap).

RESOLVED 2026-06-10 (papercut sweep commit ce3d7f801, reviewed ✅): Step-1 build
skipped under join-alpha && !FORCE_LOCAL_HAPP. Residual INFO from review: the
`--build`/-b flag (and `hc:build:all`) is now silently ignored in join-alpha
mode — semantics arguably correct (artifact never installs; FORCE_LOCAL_HAPP=1
restores build+install) but the help text doesn't say so and the skip echo
doesn't acknowledge an ignored --build. One-line echo + help amendment when
hc-start is next touched. Sibling accretions (oras dedupe, happ.yaml sed
provenance) remain open above.
