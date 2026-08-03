---
id: "backlog-resolve-canonical-election-get-links-deadline"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "resolve_canonical_election dies on get_links 'deadline has elapsed' — B2 recurrence at the election-READ site; obey path 0% survivable until cured"
slug: "resolve-canonical-election-get-links-deadline"
written: "2026-08-03"
author: "pipeline-landing shift (integrator)"
status: "backlog"
priority: "high"
tags: [dataplane, holochain, coordinator-zome, get-links, deadline, obey-path, convergence, b2-class, concern-c6a, concern-c4]
cites:
  - elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs
  - elohim/elohim-storage/src/services/head_adoption.rs
  - genesis/data/timeline/backlog/content-gap-limit-cycle-blocks-convergence.md
---

# `resolve_canonical_election` get_links deadline — the obey path's true wall

## Evidence (2026-08-03, quoted from ribosome ERROR logs, wasm_hash = live bundle)

`RuntimeError { source: … WasmError { file: "…/host_fn/get_links.rs", line: 76,
error: Host("deadline has elapsed") } } zome=content_store
fn_name=resolve_canonical_election` — 34 occurrences 01:30–02:49Z on
adam/eve/gertrude/susan (the conductor-missing-side pods the obey path exists
to serve). The coordinator hot-swap is PROVEN applied (7/7 pods, both rebake
windows, hash-chain corroborated) — this is not a missing extern.

Consequence: every obey probe (~900/hr fleet-wide) dies at the election-read
step; `election_obeyed_total` and `obey_failed_total` never got a first
increment. Until the obey-probe counter landed (same shift), the failure was
invisible: Err → `tracing::debug!` in a deployment that drops debug entirely.

## The class

The saga's **defect B2** — a `GetStrategy`-network-dependent gather hanging on
cold arcs — was cured at the DECLARE guard (`496a4aba8`, newest_canonical_link
Network→Local). The wave-4 election-READ extern (`resolve_canonical_election`,
`da8975176`) shipped with the same hazard uncured, despite its own doc saying
"gather/select WITHOUT target retrieval, Local links." Known class, second
site. Forecast row fp `836a69cc043c` (confirmed-live) carries it; scored
miss-of-ranking at the next saga-rung calibration.

## Fix direction (bounded, coordinator-only, hot-swappable — no DNA event)

Make the link gather inside `resolve_canonical_election` strictly local
(`GetOptions::local()` / the local link-query strategy the declare-guard cure
used), so the election read never awaits the network on a hot sweep path. On a
full-arc fleet a local link miss means gossip hasn't delivered — return the
honest no-election answer (C4: Unreachable-shaped, never a hang). Verify with
the new `elohim_content_election_obey_probe_total{outcome}` counter:
resolve_error should collapse and no_election / attempted should become the
live discriminator. Ships via `update_coordinators` (ALLOW_COORDINATOR_UPDATE
non-prod default true); needs one DNA build + edge rebake cycle.

Sibling observation (same evidence run): susan's conductor carries an 18–90×
1m–8m allocation-bucket anomaly (851MB, decelerating growth) with gossip
timeouts every ~5–6s — she poisons the carried-record supply fleet-wide
(responder-budget hash-only answers → fetch_none). Track under
susan-conductor-ws-dead-heal-pacing-blind-to-instant-errors.md; if the bucket
growth doesn't plateau, that entry graduates to a conductor-memory
investigation.

Status: open, unowned — the named next-session lever for saga convergence.
