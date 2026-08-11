---
id: "backlog-a2o-acquisition-pins-story-coherence"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "acquisition-pins.feature carries three unstated promises and puts load-bearing meaning in comments — blind-reader REVISE, deferred at the 2026-08-11 integration"
slug: "a2o-acquisition-pins-story-coherence"
written: "2026-08-11"
author: "claude (integrator, 2026-08-11 ghost-decay wave)"
status: "proposed"
priority: "medium"
area: "genesis/a2o"
recurrence: 1
domain: "docs"
cites:
  - genesis/a2o/features/delivery/acquisition-pins.feature
tags: [genesis, a2o, cucumber, blind-reader, story-coherence, docs-domain, deferred-review]
---

# acquisition-pins.feature: a REVISE verdict the integration deliberately deferred

## Why this exists

`genesis/a2o/.epr-meta` routes every authoring pass on this feature through a
context-isolated blind-reader, and the loop must run "until the story is READY
or the operator explicitly defers named findings." The 2026-08-11 wave added one
scenario (`Pull status distinguishes an unmeasured boot from an observed empty
pin set`) and ran the loop. The reader returned **REVISE**.

**The deferral is deliberate and this entry is what makes it explicit rather
than silent.** Every finding is against the FILE AS A WHOLE — pre-existing
structure across nine scenarios — not against the scenario this wave added. The
reader in fact named the new scenario the most self-contained one in the file
("its Given/When/Then makes its own point without needing its trailing comment,
and the point itself — unreadable is not the same as zero — is a clear,
valuable, human-relevant honesty guarantee"). Fixing the whole file inside a
deploy-carrying integration wave would have been unbounded scope on a surface
that is largely `@wip` and excluded from every CI run (`not @wip` gates all a2o
invocations), so nothing here blocked the push.

## Named findings (the deferral is scoped to exactly these)

**BLOCKER — scenarios 7 and 8 are unreadable without their comments.** The
causal claim ("peer choice was `batch_position % peer_count`", "only the success
arm removed the entry from `pending_acquisition_fetches`") lives in `#` comments
below the Gherkin, outside the executable story. A reader cannot tell what "a
stable position in the acquisition batch" asserts or why 25 is the significant
number. A comment also discloses these are fixture-pending and not currently
runnable — evidentiary status a reader can only learn from prose. Repair: move
the claim into Given/Then language; signal not-yet-executable in-scenario.

**MAJOR — the feature promise covers a third of its scenarios.** The header
promises the device pin and byte-arrival-not-inventory-arrival; roughly six of
nine scenarios serve the provide/peer-serving pin ("slice 2b, rung 4") or the
acquisition-loop regression trio. Repair: split per topic, or restate all three
promises up front. `slice 1` / `slice 2b` / `rung 4` are undefined roadmap
coordinates pointing at a document the reader cannot see.

**MAJOR — scenario 1's title outruns its steps.** "creatable and durable with no
network at all" is proved by a POST then a GET: nothing establishes an offline
topology and nothing checks persistence across a restart. This is the flagship
proof of the feature's central claim.

**MAJOR — `@requires:doorway` names a dependency the prose says is not
exercised** ("the pin API is not proxied through the doorway — it is always
own-node"). The tag is apparently standing in for "needs a running
elohim-storage." Tags are the fastest way to read scope, so this actively
misleads. Repair: rename to the real dependency.

**MAJOR — undefined load-bearing vocabulary.** "cluster pin" and "closure
resolver" carry a whole scenario with no introduction; pin `kind` is never
established. "pull status" is asserted from `/p2p/status` and from
`/pins/{id}/pull` with no stated relationship, so a reader cannot check the
scenarios against each other.

**MAJOR — the provide-pin rationale lives only in a comment**, and scenario 4
asserts merely "a pull rollup" where scenario 3 checks specific values.

**MINOR** — unexpanded `(R-A)`; `spec §1.1`/`§11` cited with no title or link;
`epr:strawberry-guide` vs bare `strawberry-guide` in a URL path; "active pin"
implies unnamed sibling states; opaque incident framing in the regression
preamble.

## Graduation target

One bounded authoring pass that splits the file along its three real topics (or
restates the promise), lifts the comment-borne claims into Gherkin, and re-runs
the blind-reader loop to READY. Worth pairing with whichever shift next touches
acquisition pins so the story and the code move together.

Sibling note: this is the sixth standalone `a2o-*` entry. `CLUSTERS.md` already
names the `a2o-*` family a candidate for future clustering — this is another row
for that cluster when it is minted.
