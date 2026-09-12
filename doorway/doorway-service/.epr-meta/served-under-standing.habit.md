---
epr-habit-version: 1
id: served-under-standing
invariant: >
  A doorway serves a public name only under a standing it re-asks at serve time — a live
  projection contract for that name, the EPR's declared reach admitting THIS requester, the
  requester's standing as their own conductor states it, and the doorway's own liveness. The
  four terms are folded from the DHT, never from a doorway-local allowlist, and the fold is
  never cached as permission: bytes may be held warm, eligibility may not. A community holon's
  ruling on an EPR's reach therefore reaches every doorway's fold on its next reconcile —
  anonymous serving drops everywhere, authenticated standing keeps what the reach names. And
  the chrome around whatever is served carries the governance mark, the fair-trade receipt,
  the redress entry and the owed response, in the friend's voice, so a visitor who knows
  nothing of the protocol can see the standing the doorway holds on the commons' behalf and
  challenge it. Serving is a commons privilege, not the doorway's property.
status: red
active: false
checks:
  - "a2o @concern:served-under-standing (genesis/a2o/features/dataplane/served-under-standing.feature — @act:i @requires:owned-substrate, so its authority is the household lane: `just test mesh features/dataplane/served-under-standing.feature` against the built binaries, run-identified report under genesis/a2o/reports/. BORN @wip with no step definitions: a run reporting undefined, pending, skipped or WIP-filtered scenarios discharges NOTHING — every scenario and every step must have a real definition and pass on the committed tree. The eligibility fold, the ruling propagation and the anonymous-after-narrowing clause are all in this file.)"
  - "a2o @concern:served-under-standing (genesis/a2o/features/federation/name-routing.feature — @act:i @requires:owned-substrate, household lane: `just test mesh features/federation/name-routing.feature`. BORN @wip on the same terms. This file measures the routing half only: registry fold, one-hop forward to the nearest live holder, the holder's origin handed back for stickiness, next-holder on a shed, and the one-hop loop budget. Routing green never certifies eligibility; the two files are separate halves of one invariant and neither substitutes for the other.)"
  - "chrome affordances: the five commons affordances of the EPR chrome are declared as epr-atom-home's NEXT commons checks (app/elohim-app/.epr-meta/epr-atom-home.habit.md — governance mark, fair-trade receipt, redress entry, feedback with an owed response, interpretability surface; its three @wip commons scenarios in genesis/a2o/features/content/epr-atom-home.feature). This habit's chrome clause is discharged THERE, by that register's own evidence, and is deliberately not re-registered here — one home per check."
refs:
  - "genesis/docs/superpowers/plans/2026-07-31-doorway-federation-failover-sprint-plan.md (WS3 — the 2026-09-12 operator ruling, passes three and four: serving eligibility as a four-term fold, the governance lever as a holon's ruling rather than a config flag, the named cache gap `cache is bytes, never permission`, and standing-not-property with the public challenge path shipped as part of the feature)"
  - ".claude/memory/feedback_doorway_projection_is_commons_privilege.md (the standing is afforded by the network and publicly challengeable)"
  - ".claude/memory/feedback_epr_chrome_is_the_trust_signal.md (the five chrome affordances, and the friend's voice — protocol vocabulary one request away)"
  - "doorway/doorway-service/.epr-meta/doorway-failover.habit.md RULING 2026-09-12 (operator, option 1): doorways are the federated load balancer at the WAN addresses of the apex; shared membership is the one authority and every sink projects it. This habit is the serving-eligibility half of that ruling — failover decides WHICH doorway answers, this decides WHETHER it may."
retire-when: >
  when a person's client resolves head, reach and standing from the mesh directly — folding the
  same four terms in their own conductor — so no doorway sits between them and the record. At
  that point "served only under standing" is a statement about a component that is no longer in
  the path, and the fold has graduated from a projection surface to the person's own runtime.
---
BORN red 2026-09-12 (no glue, no code — the stories are the whole of it today). The invariant is
declared from the operator's 2026-09-12 ruling (WS3 passes three and four) and both features are
written @wip with zero step definitions, so nothing here has been measured and nothing can be:
a WIP-filtered or undefined-step run is not evidence. What the FIRST measure needs, in order:
(1) a household fixture that stages an EPR with a declared reach and a collective able to rule on
it, plus one root hosted by exactly one of the two household doorways, so "a doorway that does not
host this name" and "a requester the reach no longer admits" are real states rather than mocks;
(2) step definitions that read the four fold terms from their own homes — the projection contract
and hosting commitments from the DHT registry, the reach from the EPR's declared row, the
requester's standing from their own conductor, liveness from the doorway's own probe — never from
a doorway-local fixture, because a fold assembled by the test is the exact defect this habit
exists to refuse; (3) the named gap measured as a red before it is cured: the warm shell and
`app_file_cache` serve the last reconciled bundle by slug+hash and blob reads route by content
address, so a cached projection keeps answering anonymously after a narrowing until eviction —
the anonymous-after-narrowing scenario must FAIL on today's binaries, or it is not measuring the
gap it was written for. Until all three exist the honest status is red with two unmeasured checks,
not `unwired`: the checks are written, they simply have not run.
DELTA 2026-09-12 (first measure; RED preserved, one clause green): name-routing.feature "A doorway that does not host a name serves it through the nearest live holder" PASSED on the household mesh (run 20260912T214110Z, doorway 983c73a22 on both doorways: registry fold seeded from the DHT-registered sibling — "Federation peer discovery started static_peers=0 dht_seeded=true" — plus specificity-aware dispatch, so beta's "/" catch-all no longer swallows a path alpha holds more specifically; alpha's staged root served through beta in one hop with x-elohim-served-by naming alpha). Three sibling scenarios red in their own staging Given: a second staging within one run answers 200 without the archive's marker on alpha (glue id reuse vs a doorway cache surviving a row's delete-and-recreate — under diagnosis). Two product findings on the way, both fixed: the registry's only writer was gated on a static peer list (e2b360779) and local catch-all mounts beat federated specific ones (983c73a22). served-under-standing.feature has no glue yet.
