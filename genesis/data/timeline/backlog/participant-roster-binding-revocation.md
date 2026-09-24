---
id: "backlog-participant-roster-binding-revocation"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Participant roster — revoke a lost device's binding"
slug: "participant-roster-binding-revocation"
written: "2026-09-24"
author: "agent:implementer@claude-opus-5-5 (Lane P review follow-up, finding M6)"
status: "backlog"
priority: "medium"
tags: [identity, participant-roster, imagodei, devices, revocation, lane-p]
relatedNodeIds:
  - acts-attributed-to-participants
cites:
  - genesis/a2o/reports/post-station-4-2026-09-25/rulings.md
  - genesis/docs/superpowers/plans/2026-09-25-post-station-4-participant-classifier-search-attention-sprint.md
  - elohim/epr-rea/src/participants.rs
  - elohim/eprfs/epr-cli/src/actor.rs
shift_objective: |
  Add a signed revocation to the participant roster so a human whose device is lost or
  compromised can end that device's membership from a device still in the lineage, and a
  contest by the lost device's key stops counting from then on. Done when the scenario below
  passes as a test in epr-rea (participants.rs) and through `epr actor device revoke`.
---

# Participant roster — revoke a lost device's binding (finding M6)

**The gap.** A handle's roster (`.eprfs/status/participants/<handle>.jsonl`) admits devices and
never removes one. After ruling R-P14 only a roster **member's** contest counts, so a device key
that leaves its owner stays a member for good. Whoever holds it can:

- contest the human's current standing record; the contest is admissible because the key is a
  member;
- counter-contest the human's own contests, so their standing flips back and forth;
- authorize a new device of its own (`device authorize`).

R-P14 accepted this cost on purpose and named revocation (M6) as backlog. The member who lost a
device also cannot contest from it until a replacement is bound.

**Scenario.**

```gherkin
Scenario: a human revokes a lost device and its later contests stop counting
  Given human:matthew's roster roots at device A and binds device B
  And device B is lost
  When matthew runs "epr actor device revoke --handle matthew --device <B>" on device A
  Then the roster gains a Revocation row signed by A naming B
  And B is no longer a member of matthew's roster
  And a Contest signed by B after the revocation is inadmissible
  And the contests B signed before the revocation keep the standing they had
  But a Revocation signed by a key that is not a member names nothing
  And the chain root cannot revoke itself while it is the only member
```

**Design notes to settle at pickup** (run the p2p-design-gate first: this adds a row kind).

- **Row kind.** Add `ParticipantRow::Revocation { handle, chain_root, revoked, by, basis,
  signature }`. Membership already follows append order, so `members()` can drop `revoked` at the
  revocation row, and a contest's admissibility can use membership as it stood at that contest's
  own row. Today `contested()` checks membership against the roster's current state; that changes
  with this work.
- **Who may revoke.** Any other current member, or only the chain root? Letting any member revoke
  is the mirror of the binding ceremony. Letting only the root revoke makes the root a
  single point of failure.
- **The network graduation.** `mishpat::bind_identity` (identity-head plan D2) inherits this
  roster. Revocation should keep the shape the DHT version will need, so check it against that
  design.
- **Root pins (R-P15)** are not affected: revoking a device removes a member and leaves the
  chain root unchanged.
