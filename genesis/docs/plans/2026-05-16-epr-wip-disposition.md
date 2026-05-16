# EPR @wip Disposition — Foundation Closure Walk

**Date:** 2026-05-16
**Audited against:** dev @ `a3f5018cd3c67c88147c0e33edd6bfb346476c4b`
**Walker:** Task-1 sub-agent, Claude Opus 4.7 (1M context)
**Source files:**
- `genesis/a2o/features/content/epr-content-addressing.feature` (4 @wip)
- `genesis/a2o/features/federation/epr-cross-peer-resolution.feature` (8 @wip)

## Walk method

For each scenario I (1) read the prose + step verbs, (2) grepped
`genesis/a2o/steps/` for matching Given/When/Then definitions, (3) read the
existing `epr-content.steps.ts` end-to-end, (4) confirmed the federation
step-def gap (no step-defs anywhere match the federation @wip verbs:
`peer "X" has content Y stewarded by Z`, `the EPR protocol "..." is active
between peers`, `the content is resolved via EPR protocol from peer "X"`,
etc.), (5) traced the substrate path that *would* satisfy the scenario, and
(6) classified the body-fetch path for the D4 question.

Key substrate findings that influenced multiple rows:

- `elohim/elohim-storage/src/epr_service.rs:86–189` (`handle_resolve`) already
  enforces reach-tier check, policy-ceiling check, AND attestation
  prerequisite check at the libp2p EPR boundary. Reach-gated, policy-gated,
  and attestation-gated scenarios are substrate-ready; what's missing is the
  BDD glue.
- `EprRequest::GetDocument` is a shape-only stub: `handle_get_document` at
  `epr_service.rs:208–211` returns
  `EprResponse::Error("GetDocument not yet implemented")`. Document-tier
  body reads today go HTTP-via-doorway through `/api/v1/content/{cid}` and
  blob reads through `/blob/{cid}`.
- The federation feature file's own inline comment block (lines 70–110)
  acknowledges that the 5 "verified landed" foundational scenarios above the
  @wip section are running undefined-silently — they have no step-defs
  either, but were lifted from @wip on substrate-readiness alone. This is a
  pre-existing accuracy gap noted but not addressed by this walk.

## Per-scenario dispositions

| # | Scenario | Line | Step-def state | Disposition | Backlog destination | Rationale |
|---|---|---|---|---|---|---|
| 1 | EPR popover surfaces all three pillars when present | epr-content-addressing.feature:96 | SUBSYSTEM-MISSING | defer-with-evidence | graph-native | Existing popover steps cover title/type-badge/reach (lines 266–307 of `epr-content.steps.ts`), but `the popover shows the shefa stewardship summary` has no step-def AND requires shefa-context populated on fixtures + a `shefa stewardship summary` DOM slot in the popover renderer. Both are graph-native scope (the EPR popover surface is a graph-native UX deliverable). |
| 2 | Following an EPR link transfers reading context to the destination | epr-content-addressing.feature:113 | SUBSYSTEM-MISSING | defer-with-evidence | graph-native | `the destination ... renders with origin context "manifesto"` and `the destination shows a back-affordance to the originating manifesto step` are not in step-defs and have no DOM hooks today. Cross-path navigation works (scenario 1 in file), but origin-context propagation through the renderer is the experience-story/experience-moment territory of graph-native. |
| 3 | EPR link to a versioned-since-authored CID degrades gracefully | epr-content-addressing.feature:128 | SUBSYSTEM-MISSING | defer-with-evidence | graph-native | The EPR codec carries supersedence references but the renderer does not yet display a "this version was superseded" notice. Requires both content-side supersedence wiring (which graph-native owns under the experience-story model) and a UI affordance pass. No step-defs match `the historical content body renders with a "this version was superseded" notice`. |
| 4 | EPR Head signature is verifiable end-to-end | epr-content-addressing.feature:145 | SUBSYSTEM-MISSING | defer-with-evidence | a2o-tooling (standalone) | The substrate signs (Phase 2B/EPR Head signing is live) — what's missing is the test-side DAG-CBOR decoder + Ed25519 verify wired into step-defs. No protocol gate; this is pure tooling work that can land independently of graph-native. Today's existing step (`epr-content.steps.ts:255–262`) only asserts content-type header is `dag-cbor`; it does not decode or verify. |
| 5 | Community-reach guide accessible only to consented collective members | epr-cross-peer-resolution.feature:113 | SUBSYSTEM-MISSING | defer-with-evidence | doorway-full-facilitator | Substrate is ready: `handle_resolve` enforces reach tier with `agent_pubkey`. Gap is BDD federation glue + a `consented member of collective "X"` fixture helper (collective membership seeding). The 403-with-reason assertion shape matches what `EprResponse::AccessDenied` carries, but no step-def implements the `requests content ... from peer "X"` shape. Doorway-full-facilitator sprint will land the cross-peer HTTP edge that drives these. |
| 6 | Trusted-reach content requires standing relationship with steward | epr-cross-peer-resolution.feature:129 | SUBSYSTEM-MISSING | defer-with-evidence | doorway-full-facilitator | Same shape as #5. Substrate gating works (`handle_resolve` reach check is trust-aware); gap is `human "X" has a "trusted" relationship with human "Y"` fixture helper + the federation step-def layer. |
| 7 | Attestation-gated content requires prerequisite mastery | epr-cross-peer-resolution.feature:143 | SUBSYSTEM-MISSING | defer-with-evidence | doorway-full-facilitator | Substrate is ready: `handle_resolve` at `epr_service.rs:133–177` queries `content_attestations`, filters `prerequisite-mastery`, looks up `content_mastery`, and returns `AccessDenied` with reason "Prerequisite mastery required" — exactly matching the scenario's Then. Gap is BDD glue + mastery-fixture seeding + the `content "X" requires prerequisite mastery of "Y"` Given. |
| 8 | Recognition distributes proportionally to stewards on P2P delivery | epr-cross-peer-resolution.feature:159 | SUBSYSTEM-MISSING | defer-with-evidence | doorway-full-facilitator | Requires cross-peer recognition-event tracking from the HTTP-edge delivery point + a `recognition events are created for steward "X" and steward "Y"` step that introspects the recognition feed. Stewardship-share fixtures (`stewarded by "Pete" at 60% and "Terrance" at 40%`) also new. Doorway-full-facilitator is where cross-peer recognition lands. |
| 9 | Policy ceiling blocks content above the device's reach level max | epr-cross-peer-resolution.feature:171 | SUBSYSTEM-MISSING | defer-with-evidence | doorway-full-facilitator | Substrate-ready: `handle_resolve` at `epr_service.rs:109–131` invokes `policy_enforcement.can_serve` and returns `AccessDenied` with the policy reason; reach-level-max ceiling is already enforced. Gap is BDD glue + `device policy with reach_level_max of N` fixture (device-policy seeding helper). |
| 10 | Steward sees recognition land for content delivered cross-peer | epr-cross-peer-resolution.feature:185 | SUBSYSTEM-MISSING | defer-with-evidence | doorway-full-facilitator | Recognition-feed observability + cross-peer recognition propagation. Pete viewing his feed implies the recognition-feed UI surface + a `views his recognition feed` step + a cross-peer recognition event with matching timestamp. Pair-bundles with #8. |
| 11 | Cross-peer fetch surfaces transient peer-offline as a soft state | epr-cross-peer-resolution.feature:197 | SUBSYSTEM-MISSING | defer-with-evidence | doorway-full-facilitator | Requires a cross-peer disconnect simulator + multi-steward failover logic at the HTTP edge + a `fetching from another steward` renderer affordance. The federation feature file's own inline note (line 104) explicitly calls this out as new work for the implementer. Doorway-full-facilitator is the natural home (cross-peer disconnect handling at the doorway HTTP boundary). |
| 12 | Identity binding allows cross-peer fetches to attribute reach correctly | epr-cross-peer-resolution.feature:210 | SUBSYSTEM-MISSING | defer-with-evidence | iroh-phase-12-followon | The crux of D4 (see below). `handle_resolve` accepts `agent_pubkey` and gates accordingly; the scenario asserts `peer "shem-pete" resolves the requesting peer to agent "agent-matthew" via PeerIdentityMap` — i.e., the PeerIdentityMap → agent_pubkey lookup at the receiving peer (Phase 2B identity binding). Body-fetch path is the open architectural question for D4. |

## D4 — GetDocument escalation answer

**Verdict:** **DEFER-TO-GRAPH-NATIVE**

**Evidence:**
- Scenarios classified **HTTP-VIA-DOORWAY**: 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11 (eleven scenarios — all of the content-addressing four plus seven of the eight federation scenarios).
- Scenarios classified **EPR-ATOM-PROTOCOL**: none.
- Scenarios classified **AMBIGUOUS**: 12 (the only candidate; resolved as HTTP-VIA-DOORWAY by reading the substrate as it stands today — see reasoning below).

**Reasoning:**

The 11 HTTP-VIA-DOORWAY classifications are mechanical. Scenarios 1–4 are
all browser-renderer interactions whose body fetch is the existing
`<content-renderer>` blob/markdown loader hitting `/api/v1/content/{cid}` or
`/blob/{cid}` through doorway. Scenarios 5–11 are federation HTTP-edge
scenarios — the step verbs say `requests content "X" from peer "alpha"`,
which is an HTTP request that doorway routes; the federation flow does an
EPR Resolve via libp2p to find a custodian peer for the head, but the body
itself comes back through the doorway HTTP path. None of these need a
`GetDocument` variant.

Scenario 12 is the case the plan's preliminary read flagged. Its step verb
literally says `fetches "private-journal" via the EPR-atom protocol`, which
sounds like it requires `GetDocument`. But reading the substrate as it
stands today, the EPR-atom protocol's role in this scenario is the
**Resolve** — peer `household-matthew-desktop` issues an `EprRequest::Resolve`
carrying the requester's `agent_pubkey`, and peer `shem-pete` uses
PeerIdentityMap to translate `requesting peer → agent_matthew` and gates the
reach decision accordingly. The body, once the Head resolves, can still be
fetched via the standard HTTP path with the same identity binding carried
through (the HTTP edge knows the agent context from the same auth path).
The phrasing `fetches ... via the EPR-atom protocol` is the *discovery and
authorization* over EPR-atom; the body transport is orthogonal.

Therefore: zero scenarios in the 12-scenario walk strictly require
`EprRequest::GetDocument`. D4 is satisfied by deferring `GetDocument` to
graph-native as originally planned. The current stub returning
`EprResponse::Error("GetDocument not yet implemented")` (`epr_service.rs:210`)
is the correct placeholder; graph-native may revisit if its experience-story
renderer architecture wants a libp2p-direct document fetch path, but no
*currently-authored* scenario forces that decision now.

## Backlog destinations summary

- **graph-native sprint:** 3 scenarios — 1 (popover three-pillar fan-out), 2 (origin-context transfer), 3 (superseded-version graceful degrade).
- **doorway-full-facilitator sprint:** 7 scenarios — 5, 6, 7, 8, 9, 10, 11 (reach/trust/attestation gates + cross-peer recognition events + multi-steward failover at the HTTP edge).
- **iroh-phase-12-followon:** 1 scenario — 12 (PeerIdentityMap-mediated cross-peer fetch attribution; depends on Phase 2B identity binding being end-to-end through the iroh path).
- **a2o-tooling (standalone):** 1 scenario — 4 (DAG-CBOR + Ed25519 verify harness in step-defs; no protocol dependency).
- **Lifted in this sprint (no backlog):** 0 scenarios. Every one of the 12 has at least one step verb with no matching definition AND/OR a substantive renderer/fixture/tooling subsystem missing.

## Sanity-check counts

- Total @wip at sprint start: 12 (4 content + 8 federation)
- Lifted by this walk: 0
- Retained @wip with new backlog citations: 12
- D4 decision: **DEFER-TO-GRAPH-NATIVE** — zero scenarios require `GetDocument`; current stub stays as placeholder.

## Implementer handoff notes

For Task 2 (`epr-content-addressing.feature`): replace each `@wip` block's
existing inline comment with the rationale + backlog-destination cell from
the row above. All four scenarios remain `@wip`; no scenario is lifted in
the content-addressing file.

For Task 3 (`epr-cross-peer-resolution.feature`): the existing inline block
at lines 70–110 already documents the unblock path well; update it to cite
this disposition file and the per-row backlog destinations (7 →
doorway-full-facilitator, 1 → iroh-phase-12-followon). All eight scenarios
remain `@wip`. Optionally tighten the pre-existing accuracy concern in the
inline block (lines 84–88) noting that the 5 "verified landed" foundational
scenarios above run undefined-silently — that's outside Task 3's scope but
worth flagging for follow-up.

For Task 4 (iroh Phase 12): scenario 12 is the only scenario gated on
Phase 12 substrate. If Phase 12 lands before doorway-full-facilitator picks
up scenarios 5–11, the conditional arm is: keep scenario 12 `@wip` until
the PeerIdentityMap → agent_pubkey translation is wired end-to-end on the
iroh path; lift only when the federation step-def layer also exists. Don't
lift scenario 12 ahead of the doorway-full-facilitator step-def layer
landing — the verb shape `peer "shem-pete" resolves the requesting peer to
agent "agent-matthew"` requires the same federation glue as 5–11.

For Task 5 (sprint-result memory): record verdict
`D4 → DEFER-TO-GRAPH-NATIVE` with reasoning "zero of 12 @wip scenarios
strictly require an EPR-atom-protocol-direct document body fetch; all 12
body-fetch paths are HTTP-via-doorway under the current substrate; scenario
12's `via the EPR-atom protocol` verb is satisfied by Resolve + identity
binding, not GetDocument." Note that all 12 scenarios stay `@wip` post-walk
with cleaner backlog destinations attached.
