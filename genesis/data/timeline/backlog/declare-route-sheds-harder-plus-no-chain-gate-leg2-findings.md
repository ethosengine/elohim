---
id: "backlog-declare-route-sheds-harder-plus-no-chain-gate-leg2-findings"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Leg-2 one-id declare validation findings — canonical-head route sheds harder than PATCH; zome no-chain gate blocks declare on re-keyed conductors; adam write-pool oscillation"
slug: "declare-route-sheds-harder-plus-no-chain-gate-leg2-findings"
written: "2026-07-28"
author: "heads-converge-truthful-resilience shift"
status: "open"
priority: "high"
tags: [canonical-head, declare, adam, admission, notary-authority, ch06]
cites:
  - genesis/data/timeline/backlog/content-divergence-unhealable-without-canonical-heads.md
  - genesis/data/timeline/backlog/self-heal-adam-projection-catchup-exhaustion-full-arc.md
---

# One-id declare validation (2026-07-28 overnight) — three sharpened findings

The handoff's leg-2 falsifiable-prediction run (declare A's head for
`elohim-host-landing` onto elohim.host/adam, ~7h of paced attempts across
three deploy cycles). The `refused_stale` tripwire NEVER fired — the
ordering-proof gate is NOT the blocker. What actually blocks, in order:

1. **adam's write-admission pool starves then oscillates.** Post-restart it
   sheds `503 catching-up` nearly flat for hours (write permits held by zome
   calls timing out against the overwhelmed full-arc conductor), later
   oscillating open every ~2-6 min. Root: the same catch-up exhaustion as
   `self-heal-adam-projection-catchup-exhaustion-full-arc.md` (ceiling: adam
   restart / arc-factor decision).
2. **The canonical-head route sheds HARDER than the generic content route** —
   observed repeatedly: `PATCH /db/content/{id}` returns 200 in the same
   seconds `POST /db/content/{id}/canonical-head` returns 503. The declare
   path's extra shed-eligibility means the propagation leg loses precisely the
   windows a plain write wins. Worth a look: should the declare route share
   the heal-exemption posture (it IS the convergence path — "gate growth,
   never convergence"), or at least the PATCH route's admission class?
3. **The zome no-chain gate blocks declare on re-keyed conductors** — full
   error captured in-window: `declare_canonical_head: no content found for id`
   (content_store lib.rs:3362). A conductor whose incarnation never authored
   the id cannot accept a declaration even WITH a carried record. The shipped
   ghost sweep (b91ee0f95) authors the missing chain in Preserve mode, but on
   adam its zome calls hit the same timeouts as (1) — so all three findings
   collapse into one unblock: adam conductor health.

Also proven this night: empty `PATCH {}` returns 200 without authoring a
conductor chain (no-op short-circuit) — an empty-patch "touch" is NOT a
chain-authoring lever over HTTP; and an empty-commit `[build:edge]` retrigger
does NOT restart the genesis pair (STS-unchanged skip) — only an
image-changing commit or operator kubectl does.

Status: OPEN. Items 2 and 3 are code-shaped candidates (admission class of the
declare route; whether declare-with-carried-record should be exempt from the
no-chain gate since the record IS the verifiable content); item 1 is the
standing operator ceiling.
