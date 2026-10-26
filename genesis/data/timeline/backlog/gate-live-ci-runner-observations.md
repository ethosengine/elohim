---
id: "backlog-gate-live-ci-runner-observations"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "CI gate observations from a sponsored service agent within its reach ceiling"
slug: "gate-live-ci-runner-observations"
written: "2026-10-26"
author: "codex"
status: "open"
priority: "medium"
area: "gate / CI / sponsored agents"
relatedNodeIds:
  - "backlog-gate-live-observation-dataplane"
  - "backlog-ci-as-steward-channel-participant"
  - "ci-runners-are-sponsored-service-agents"
tags: [gate-live, ci, jenkins, observation, constraint]
shift_objective: >-
  After the observation dataplane has an authorized signer and second-person
  subscriber, bind a sponsored CI runner's observations to its declared reach
  ceiling and bounded grants, with no device binding or roster membership.
---

# CI speaks through bounded service authority

chain: independent CI observation / between A: authorized observation delivery to a second person → C: subscribed gate results from a CI runner / missing node B: the sponsored runner has explicit observation authority and a reach ceiling that the publication path enforces; probe: deliver one in-scope result and refuse an out-of-ceiling or expired-grant result without adding any device or roster membership / current state: open; depends on gate-live-observation-dataplane and the runner's reach ruling.

The operator's 2026-10-19 ruling resolves the identity question: Jenkins is a **sponsored service
agent**, never a device and never a roster row. The source memory,
`ci-runners-are-sponsored-service-agents`, names `agent:ci-runner-jenkins-matthew`, one agent key,
a sponsor-hosted cell, a G1 `hosted-cell` grant and provider-issued G2 `measure-stage` grants.
It explicitly excludes both an eprfs roster row and Mishpat DeviceBinding. Its source file is
`/projects/.claude-config/projects/-projects-elohim/memory/ci-runners-are-sponsored-service-agents.md`.

The repository's [placement-contract design](../../../docs/superpowers/specs/2026-10-20-what-every-app-inherits-placement-contract-design.md#L179)
restates the sponsored-service-agent rule and points to the CI runner story. A bound human device
can speak for its person and approve other devices; CI must not acquire those powers through
convenient channel wiring. Sponsorship remains plural: this is not an exclusive Jenkins identity
owned by the first sponsor, nor permission to prevent other peers sponsoring runners.

The older [CI channel item](ci-as-steward-channel-participant.md) still asks whether a runner
belongs in the roster. This entry records the ruling without editing that existing file. The
remaining question is the runner's bounded authority to publish a particular observation at a
particular reach. A compute grant alone must not silently be treated as a disclosure ruling for
failure loci, paths or excerpts.

The [observation dataplane item](gate-live-observation-dataplane.md) is a prerequisite: an unsigned
row or local acknowledgement cannot make CI an independent subscribed witness. Once that path
works, carry the same SDK event vocabulary through it within the runner's declared ceiling.
Excerpts remain device-only. Validate grant expiry, refusal above the ceiling, correct runner
identity at the subscriber, and unchanged local operation when CI is absent. These are acceptance
probes to run after the blockers are resolved; no delivery or review is claimed here.
