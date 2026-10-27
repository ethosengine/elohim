---
id: "backlog-brit-daily-driver-handoff-2026-10-27"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Next sprint: daily-drive brit — the social and evidence layer around commits, built on the steward-space landing plane"
slug: "brit-daily-driver-handoff-2026-10-27"
written: "2026-10-27"
author: "claude-opus-5-5 (steward-space design session), at the operator's direction"
status: "backlog"
priority: "high"
area: "scm / brit / rakia / steward space"
tags: [brit, scm, handoff, rakia, landing-plane, attestation, change-id, walkthrough, cas]
cites:
  - "steward-space-landing-plane-design | The steward space | sha256:6ef574355778bebd | path: genesis/docs/superpowers/specs/2026-10-27-steward-space-landing-plane-design.md"
  - genesis/data/timeline/backlog/gate-merge-gradient-reading-2026-10-25.md
  - genesis/data/timeline/backlog/brit-act-level-attribution-identity-chain.md
  - genesis/data/timeline/backlog/native-peer-dev-channel-and-collective-memory-seed.md
---

# Daily-drive brit: the social and evidence layer around commits

> Operator, 2026-10-27: *"I'd really like to daily drive brit and get it ready for those exciting wins."*

This is a handoff for the sprint after the steward-space landing plane
(`2026-10-27-steward-space-landing-plane-design`). It integrates with that spec's Arm A slices. It
does not fork them.

## Why brit, and what brit is for

The 2026-10-27 SCM horizon scans read the field in three camps:
- agent-scale centralisation: Anthropic Scion, GitLab next-gen SCM, Garden, code.storage;
- social objects replicated beside code: Radicle collaborative objects (COBs), git-bug, jj, Willow, Keyhive;
- agent provenance as a git sidecar: Entire, GitLab Governance for Agents.

The full reports are in the operator's plans folder:
- `/projects/.claude-config/plans/sunny-growing-quail-agent-a9e65fb1d1919cc24.md` (named systems);
- `/projects/.claude-config/plans/sunny-growing-quail-agent-ae574755886092133.md` (p2p SCM).

Their consistent message is that brit should not race to replace git transport. Its job is to own the
**social and evidence layer around commits**: proposals, reviews, attestations, walkthroughs.

- **Already built (pin 76ba95d8a):** a signed, content-addressed note store under
  `refs/notes/brit/{build,validate,deploy,reach,covenant,meta}`, which CI already trusts (`brit-attest.mjs`,
  `gate-evidence.mjs` reuse by inputs hash, `delivery-series.mjs`, `just measure`'s validate note).
- **Only planned:** `brit attest`, `Reviewed-By:`, `MergeProposalContentNode`, git over p2p, fork governance.
- **Not specified anywhere:** review, issue and discussion objects.

The SCM-side primitives from the scans mostly did not reach the spec's slices (stable change id, CAS
with expected-prior head, walkthrough records, sparse fetch, derived data). This handoff carries them.

## Precondition (do first)

- **Pin the brit checkout to its pin.** The working tree was 385 commits behind
  (`git submodule update elohim/brit`). Re-read anything cited from the off-pin checkout.
- **Daily-drive means brit is on every device's path.** A pinned `brit` and `brit-build-ref` must resolve
  through the bin store like `epr` (the session pin already reports a brit CID), on both ethosengine and shem.

## The wins, in order (each small, each on the landing plane's critical path)

| # | Win | Integrates with | Borrowed from | Done when |
|---|---|---|---|---|
| 1 | **Landing intents as brit notes**: `refs/notes/brit/proposal/<intent-cid>` carrying {base, tip oid, closure keys, evidence, actor, lane, state}. A working first form of `MergeProposalContentNode`: a pull request with no GitHub, written by agents and invisible to Jenkins | spec A9 (landing-intent queue), D3 (no forge branches) | Radicle COBs, git-appraise | two devices queue, see and land each other's intents with no forge PR; `brit` lists them |
| 2 | **Sibling verdict reuse as the build cache**: a green gate is a brit attestation whose subject is the closure key and whose `output_cid` is the artifact CID (rakia R12 c). A `binds-signer` roster row admits the sibling's signer for reuse, labelled `independence: sibling` and never counted in k-of-n | spec A8, A0 probe 1 (closure keys must match at 100 %) | in-toto Statements, Turborepo cache semantics | a closure gated on one device is reused on the other with a named attestation, not re-run |
| 3 | **Stable change id trailer** (`Change-Id:` or a brit-native key): claims, intents and attestations key on it, so a rebase no longer orphans them | spec A6, A9; `brit verify` trailer validation | jj change ids, Gerrit | an intent and its attestations survive a rebase onto a moved dev |
| 4 | **Walkthrough record per handoff**: {claims, proof kind + proof (attestation CID or build number), `unproven`, unaccounted residue} as a brit note. The fruit that crosses, never the transcript | spec A7 (offer), §5.7, "thoughts private, fruit judged" | Garden SCM walkthroughs, Anthropic's review tool Antique | the next device-to-device code handoff is checkable from its walkthrough alone |
| 5 | **CAS with expected-prior head on every shared head**: lease, lane, rakia channel head and the dev ref move each cite the head they expect; a stale writer is refused and named | spec A3, A9; the 2026-10-27 launcher stranding (two readers of "verified" disagreed about one head) | Anthropic Scion ref moves, code.storage `expectedHeadSha` | a stale writer is refused with the head it missed, never silently wins |

Each win also emits a ValueFlows event beside its record (produce, use, deliver-service), accounted to
the principal with agent provenance per the spec's A5. Each artifact carries its upstream contribution
edges (spec §5.7 guidestar).

## Later, not this sprint

- **Review and discussion objects** as op DAGs (Radicle COBs) at S2, held by the collective's members, never on the lamad `Content` valve.
- **Derived data as pure functions of CIDs** (Scion): blame, attribution and folds that any peer computes and verifies.
- **Want-based sparse fetch for agent sessions** (Scion, GitLab, Garden): the device keeps the full replica; sessions read through it.
- **Git over p2p** (brit phase 3): bundles over the eprfs tier (spec A7b) are the stepping stone.
- **Merge tallies and fork governance in brit:** wait for qahal's tally home and the mishpat `CouncilRoster`. brit reads consent requirements and never owns governance (brit spec, composition.md:66).

## Not building

- Session or transcript capture beside commits (Entire). A trailer may carry a hash commitment to a private session record; never the transcript.
- A brit-owned governance or tally engine.
- A forge-hosted queue, PR bot, or Jenkins branch per intent.

## Open question for the sprint's first decision agent

Should the proposal namespace live in brit (`refs/notes/brit/proposal`) or as typed epr records on the
steward space, with brit notes as the degraded transport? The scans favour one replication substrate,
social objects next to code. Decide at the sprint's first milestone (operator ruling 2026-10-27:
resolve forks in flight, behind a swappable interface).
