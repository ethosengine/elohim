---
id: "backlog-genesis-seeders-double-rooted-projection-commitment"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "One projection commitment on alpha has two root Creates and can never change state"
slug: "genesis-seeders-double-rooted-projection-commitment"
written: "2026-10-06"
author: "claude-fable-5-1 (settling the device-consent landing shake-out, 2026-10-06)"
status: "backlog"
priority: "medium"
tags: [genesis, seeder, rea-commitment, project-epr, standing, authority, alpha]
cites:
  - genesis/seeder/src/seed-projections.ts
  - genesis/seeder/src/seed-commitments.ts
  - elohim/holochain/dna/elohim/zomes/content_store/src/commitment_observation.rs
---

# One projection commitment on alpha has two root Creates

## Symptom (measured, elohim-genesis #1623 to #1625)

`Seed REA Commitments` ended UNSTABLE on every build with one line:

```
[X] elohim-host-landing @ / on alpha-elohim-host: HTTP 503 …
    content_store::commitment_observation:21: Guest("commitment observation unavailable:
    multiple root Creates for ID")
```

The id is `project-epr-98f0d59051751497`. The grant itself is in force:
`GET /db/rea_commitments?action=project-epr&doorwayId=alpha-elohim-host` on the alpha doorway
returns three active projections (`elohim-host-landing`, `lamad-spa`, `imagodei-portal`), and the
landing page serves 200 (read 2026-10-06).

## Cause (read from source)

The id is derived from the grant's own terms (`peerId|project-epr|doorway|epr[|host]`), not from
the cell that writes it. The seeder posted through the doorway's shared name, which reaches more
than one peer; two peers' cells each wrote a root Create for the same id. The zome refuses to
observe an id with more than one root, and a state change requires the root author, so the id can
be read where it was projected and can never be updated, superseded or revoked.

## What changed 2026-10-06

- The cause is closed for new ids: `seed-projections.ts` writes each `project-epr` commitment
  through the steward's own storage peer, as the custody seeder already does.
- The seeder no longer re-posts into the refusal. A 503 naming `multiple root Creates for ID`
  takes the same compare-then-decide path as a conflict; when the active row matches the declared
  grant it prints `[=] … (in force on this peer; its DHT id has two root Creates …)` and counts it
  as already existing. A drifted grant still attempts the supersession and fails loudly if the
  zome refuses.
- The seeder accepts an optional `idGeneration` on a spec. It is not declared on this row: a new
  generation would mint a second active grant for `/` beside one that cannot be withdrawn.

## What is still true

The record on alpha is unchanged: one id, two roots, no state change possible. That is a scar,
not a fault in service. It becomes a fault the day the landing grant must change terms or be
withdrawn.

## The cure, and why it is not a seeder change

"Only the root author may change it" is authority read from a key. The operator's ruling of
2026-10-04 is that authority over a subject comes from standing the network recognises, never from
who authored the root. Under that rule an id with two roots is an ordinary case: whoever holds
standing over the doorway's routes elects one record or supersedes both. The cure belongs with the
standing-based authorization design
(`genesis/docs/superpowers/specs/2026-09-30-claim-backed-credentials-and-contextual-authority-design.md`),
not with another id input.

## Probe

`Seed REA Commitments` in an `elohim-genesis` build prints no `[X]` line. For the scar itself: the
landing grant on alpha is superseded once through the ordinary ceremony and both doorways serve the
successor.
