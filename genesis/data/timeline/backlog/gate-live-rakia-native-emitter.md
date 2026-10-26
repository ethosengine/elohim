---
id: "backlog-gate-live-rakia-native-emitter"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Gate-live native emission after TaskRunner reaches provider acceptance and completion"
slug: "gate-live-rakia-native-emitter"
written: "2026-10-26"
author: "codex"
status: "open"
priority: "medium"
area: "gate / placement / rakia"
relatedNodeIds:
  - "gate-live-failure-channel-design"
  - "habit:gate-failures-arrive-live"
tags: [gate-live, rakia, placement, constraint]
shift_objective: >-
  Once TaskRunner-to-provider acceptance and completion are observable, project
  gate-live events from that native lifecycle using its existing completion event
  and receipt. Keep the SDK schema authoritative and introduce no second recognizer.
---

# Native emission waits on the provider lifecycle

chain: gate Task lifecycle / between A: device-local Task execution → C: native rakia gate-live emission / missing node B: TaskRunner reaches provider acceptance and completion, and its outcome resolves to the existing completion event and receipt; probe: execute one accepted provider Task and resolve its task identity through acceptance, outcome, completion event and receipt / current state: open; the provider bridge must be demonstrated before native emission is taken up.

The device's own gate is the floor. Running it locally and projecting its diagnostics does not
prove that the same Task has crossed a provider's acceptance boundary or settled there. This item
waits for those lifecycle facts instead of treating a new emitter as proof of distributed execution.
Its scope is the native producer that becomes available after that bridge exists; it does not
request a rakia pin move or a fleet rollout as a prerequisite for the local channel.

[TaskOutcome](../../../../elohim/ark/core/src/task.rs#L915) explicitly names the existing
provider-authored `rakia-compute-complete-v1` EconomicEvent and rejects reusable derivations for
failed execution. That is the settlement seam to preserve, together with the provider's receipt.
The local completion work happening alongside this backlog does not discharge the provider
acceptance requirement. Re-read the current implementation at pickup because these crates are
being changed concurrently.

The projection should translate lifecycle events already produced by the native runner. It must
not parse the runner's output with a second recognizer, invent another completion record, or use
diagnostic counts to decide success. The current
[recognizer](../../../orchestrator/gate-failure-lines.mjs) remains the single interpretation of
tool output until the native stream supplies the corresponding facts. The canonical event schema
stays in [the SDK](../../../../elohim/sdk/schemas/v1/gate-live-event.schema.json); adapters consume it.

The acceptance probe must include a clean run, a failing run and an interrupted run, recording
which completion or Unanswered answer each produces. A repeated admissible derivation must run
nothing and retain the same meaning as today's brit-backed reuse. These are proposed checks,
not results claimed by this entry. Ownership of provider acceptance, completion and receipt
resolution must be clear before implementation begins.

The source scope is the
[gate-live design](../../../docs/superpowers/specs/2026-10-26-gate-live-failure-channel-design.md)
and sections 5–7 of the operator's 2026-10-26 native-device-floor plan
(`/projects/.claude-config/plans/yeah-except-it-should-streamed-whisper.md`).
