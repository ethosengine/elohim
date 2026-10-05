---
id: "backlog-coordinator-acceptance-tightening-contract"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "A coordinator release that tightens what it accepts is a network-visible change shipped as a local one: declare it, name the refusal, and make staleness a reading"
slug: "coordinator-acceptance-tightening-contract"
written: "2026-10-04"
author: "session 0aa04d40 (investigation of the all-apps coordinator sweep), at the operator's direction"
status: "open"
priority: "medium"
jobs: [elohim-edge, elohim-holochain]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "backlog-upgrade-propagation-p2p-design-arc"
  - "backlog-hosted-app-coordinator-coverage-gaps"
  - "habit:runtime-upgrade-propagation"
tags: [upgrade, coordinator-hot-swap, hosted-apps, delegation, plane-authority, trustful-self, friction-blind, unit-agent, phase-transition, lane-borrowed]
---

Design ruled 2026-10-04 (see Rulings). Read from `origin/dev` at `598abcc1c`; nothing here was
measured on the fleet.

## The gap

Campaign 1.4 leg 2 was refused on 2026-10-04 with `head delegation: legacy grant cannot authorize
a new publication` (`content_store/src/head_delegation.rs`, `verify_head_delegation`). The wire
discipline held: `issuance_action_hash` is optional, a legacy grant still decodes, and the signed
statement domains are already versioned (`elohim:accepted-content-head:v2|v3|v4`). What changed
was the acceptance rule for a new act, and three things about that change were unstated.

1. **Nothing declares it.** The upgrade arc's mixed-version discipline covers additive wire
   fields. It does not cover a release that stops accepting what an older coordinator still
   issues. The pipeline driver (`scripts/ci/fleet-coordswap.sh`) pushes a raw bundle with no
   manifest; the release manifest (`release_adoption::ReleaseManifest`) has no field for it.
2. **The refusal does not say who is behind.** "Legacy grant" is true and gives the person no
   next step. The cause was a stale issuer, an app the sweep had never reached.
3. **Staleness was not a reading.** Nine lamad coordinators on one conductor was learned from a
   failed ceremony. The sweep's dry-run report now produces the number; it runs only when
   dispatched.

Delegation verification lives in the coordinator; the integrity zome has no delegation
validation. So the rule is each verifier's own, takes effect on a verifier when that app is
swapped, and has no network-wide moment.

## Ruling the design rests on

A rule that must bind every peer belongs in the integrity zome and crosses under the lineage
commitment. A rule in a coordinator is the verifier's own diligence: it is in force where it
runs. A stale verifier is exposed, not exempt (the arc's 2026-09-01 course-set: silent staleness
is a harm class). The design therefore does not gate anything on fleet-wide completeness. It makes
the change declared, the refusal legible and the staleness visible.

## Proposed shape (coordinator-only, DNA-hash-neutral, no new entry or link type)

**1. The coordinator states its own statement contract.** One constant table in the coordinator
crate, exposed by one extern: for each signed-statement domain, whether this coordinator issues
it, accepts it for a new act, and honors it on a historical read. Storage reads it once per
distinct coordinator wasm hash, never once per app. The artifact speaks for itself, the same rule
`Provenance.build_info` follows.

**2. A contract lockfile pins the change.** The table is committed as a snapshot beside the zome;
a unit test fails when a domain moves from accepted-for-new-acts to refused unless the change is
written into the snapshot with a reason. A tightening is then a reviewed line in a diff. The
release manifest carries the same rows in an additive optional field for the elected path.

**3. Expand before contract, with a declared exception.** Default order: release N issues the new
domain and accepts both; release N+1 refuses the old one for new acts. A one-step tightening is
allowed when the snapshot marks it `immediate` with a reason (a security fix). The 10-01 refresh
was an undeclared one-step.

**4. The refusal names the cure.** The refusal carries a reason code (`issuer-behind`), the domain
presented and the lowest domain accepted. The person is told the issuing app runs an older
version, and that re-issuing from a current app is the cure. The check itself is unchanged.

**5. Staleness is a standing reading.** Per conductor: roles pending a hot-swap and distinct
coordinator hashes per role, on node-local diagnostics and as an aggregate gauge. App ids stay on
the node-local report; which people a conductor hosts is not published.

**6. The node's own app goes first on every path.** The boot path already does this. The route
and the driver sweep in `list_apps` order. Own app first, its contract read, then the hosted apps,
gives each conductor a one-app canary before the swap reaches the people it hosts.

Items 5 and 6 also want gap 1 of `hosted-app-coordinator-coverage-gaps` closed on the storage
side: one `list_apps` call on a slow ticker, sweeping only app ids not seen before.

## Rulings (operator: carry on, leave nothing open, 2026-10-04)

- The reviewed lockfile line is the approval for a one-step tightening. No second signature while
  one steward builds and reviews; revisit when a second steward holds standing over releases.
- Expand-before-contract applies to authority statements (delegations, grants, acceptances), not
  to every coordinator release.
- This entry stands alone beside `hosted-app-coordinator-coverage-gaps`, one incident and one
  bounded task.

## Probe

Household: apply a variant bundle that retires a domain to one peer's own app only; issue a grant
from a hosted app on the old coordinator; present it to the swapped app. Expect `issuer-behind`
with both domains named, the conductor's pending reading non-zero, and the lockfile test red on
the variant without its snapshot line.

## Ruled out

- Accepting legacy grants for a window on new publications: weakens the issuance check.
- Blocking a release until every app on the fleet is swept: there is no such moment in a
  network that adopts at its own pace, and the pipeline is the interim vehicle.
- A new DHT entry for coordinator versions: the reading is rebuilt from the conductor on demand.
