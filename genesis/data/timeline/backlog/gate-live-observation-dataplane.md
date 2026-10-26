---
id: "backlog-gate-live-observation-dataplane"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Gate-live observations wait on signing authority and a second-person subscriber"
slug: "gate-live-observation-dataplane"
written: "2026-10-26"
author: "codex"
status: "open"
priority: "medium"
area: "gate / observations / transport"
relatedNodeIds:
  - "gate-live-failure-channel-design"
  - "observation-event-layer-design"
tags: [gate-live, observation, p2p, constraint]
shift_objective: >-
  Establish observer signing authority and a second-person subscriber, then prove
  the existing observation log's announce, receive, pull and verification path
  before carrying gate-live observations on it.
---

# A subscriber and signing authority precede transport

chain: gate-live observation delivery / between A: device-local gate diagnostics → C: a second person's subscribed observation channel / missing node B: an authorized observer signs an observation that an authorized second-person subscriber receives and verifies through the observation log; probe: publish one signed cursor, receive it, pull its segment and verify identity, signature and reach at that subscriber / current state: open; signing authority and the subscriber are not established, and the transport path is unwired.

The existing [observation module](../../../../elohim/elohim-storage/src/observation/mod.rs)
is the substrate to complete. A device-private gate journal is not already a signed observer log,
and an HTTP acknowledgement is not proof that another person received anything. The intended
reader must be a second person with an explicit subscription and reach, not merely another
process replaying the producer's local SQLite rows. Nothing here changes the gate's authority:
diagnostics remain observations and the brit attestation remains the verdict record.

The source inspection on 2026-10-26 found these concrete missing steps:

- [api/observations.rs:69](../../../../elohim/elohim-storage/src/api/observations.rs#L69) declares
  unsigned acknowledgements, and
  [line 339](../../../../elohim/elohim-storage/src/api/observations.rs#L339) constructs an empty signature.
- [publish_announcement](../../../../elohim/elohim-storage/src/p2p/observation_gossip.rs#L77)
  has no caller; the API's append path only logs that a cursor is announceable.
- The storage P2P source has a cursor type and topic subscription but no observation receive arm.
  An announcement counter therefore does not prove delivery.
- [IrohObservationBackend](../../../../elohim/elohim-storage/src/p2p_iroh/observation_backend.rs)
  is an in-memory segment backend. The observation ALPN constant exists, but no network pull path
  connects received cursors to segment fetching and remote projection.

Pickup must first decide who may sign for this observer and establish the subscriber's authority
to read. Reuse the per-observer log, cursor and remote projection seams; a parallel gate-only
transport would leave those missing nodes untouched. Keep excerpts device-only and resolve any
disclosure of loci and paths through the separate device-crossing item.

Completion requires an actual subscriber receipt for a valid observation, rejection of an invalid
signature or unauthorized reach, and recovery after a disconnected subscriber resumes from its
cursor. Those probes are future acceptance evidence, not tests run for this entry. The source
architecture is the [observation event layer](../../../docs/content/elohim-protocol/architecture/2026-05-11-observation-event-layer-design.md).
