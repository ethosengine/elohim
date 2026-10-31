---
id: "backlog-chrome-trust-edge-deferred-egress-census-site"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "port/chrome-trust-edge is deferred: its signal-intake reach check is a reach decision site the egress census has not classified"
slug: "chrome-trust-edge-deferred-egress-census-site"
written: "2026-10-31"
author: "claude-opus-5-5"
status: "open"
priority: "high"
area: "elohim-storage · reach verifier · content chrome"
domain: "D5"
tags: [chrome-trust-edge, content-chrome, reach-verifier, egress-census, deferred]
---

# chrome-trust-edge is deferred, not dropped

Train 2 of the 2026-10-27..31 overnight integration merged `port/chrome-trust-edge`
(e2206344c; 95 files, the content-chrome facing route and signal intake behind
`ELOHIM_CONTENT_CHROME`, off by default). Push 12 redded in the elohim-storage gate on
`tests/reach/egress_census.rs::legacy_reach_predicates_are_counted_and_shrink_with_plane_status`:

```
new reach decision sites need an EgressPlane rule: ["api/epr.rs: unclassified reach_level_index( (2)"]
```

The site is `resolve_signal_subject` in `elohim/elohim-storage/src/api/epr.rs` (from ebe16126b):
it refuses a signal whose record is narrower than `content_signals::UNDECLARED_BEHAVIOUR_REACH`
(`community`). The census landed on dev (e64e47914, the one-reach-verifier slice) after the
branch was written, so the two lines met for the first time in this train. The merge was
reverted (6108c434b) so the rest of train 2 can land. The branch is intact.

Classifying the site means changing the census or adding a verifier rule. Both are judge-side
decisions, so the integrator did not make either one.

| Row | What | State |
|---|---|---|
| 1 | Decide the site's class: route the check through `ReachVerifier` under an `EgressPlane` rule, or, if it is judged non-egress, add an exact `pending: None` `LegacySite` for `api/epr.rs` `reach_level_index(` with its reason | open: reach-verifier owner or steward |
| 2 | Review the check order. The "Reach narrower than the signal" 403 names the record's reach before `content_reach_gate::reach_refusal` runs, so an anonymous caller learns that a private record exists and what its reach is. `a_record_narrower_than_the_observation_is_refused_whoever_asks` asserts this on purpose; confirm it is wanted, or run the read gate first | open |
| 3 | Re-land: merge the branch onto dev; it changes storage and doorway serving paths, so it needs the household receipt (`epr-app-deliverability`) and the storage gate (about 65 min at one job) | open |

## Done when

- The content chrome ships on dev with the egress census green and every reach decision site classified.
