---
title: hc-start.sh documents STORAGE_DIR as overridable but unconditionally clobbers it
created: 2026-06-10
domain: process-meta (build-and-test; dev stack)
source: arc plan Task 2.1 in-flight simplification sweep (2026-06-10)
severity: low
---

`app/elohim-app/scripts/hc-start.sh` header documents `STORAGE_DIR` (default
`/tmp/elohim-storage`) as an env override, but ~line 256 unconditionally
reassigns it to the repo crate path — the documented override is dead. Either
honor the env (`: "${STORAGE_DIR:=…}"`) or delete the doc line. Sibling
accretions noted same sweep (candidates for one cleanup pass): five
near-identical DNA build stanzas (loop candidate), socat/pty keep-alive hack
fragility, ELOHIM_AGENT_PORT default computed twice, app CLAUDE.md "three
deployment modes" prose listing four.
