---
name: Placement/verification signals are shefa economic inputs
description: Gaps, breaches, and recovery needs are structured signals for shefa economic planning — not just operational errors
type: project
originSessionId: 17546f03-3ee8-4704-bdf9-18d0d64baf9b
---
Placement gaps, verification breaches, reconstruction events, and over-extended commitments are not operational warnings — they are **structured economic signals** that flow up into shefa to drive planning: where new peer support is needed (more nodes), where subsidies should flow, who needs recovery/repair, who is over-extended.

**Why:** User framing (2026-04-19, self-healing dataplane spec): "these signals flow up into Shefa, it helps elohim plan and prioritize peer support (more nodes), where subsidies go, who needs recovery, repair, or who's over extended." The dataplane's imperfect reality is the shefa layer's input surface — resilience degradation is how elohim learns where to act economically.

**How to apply:**
- Every dataplane anomaly (placement gap, missed verification, reconstruction event, commitment breach) should be a structured, queryable record — not just a log line or toast.
- Signal surfaces belong in shefa views (`/shefa/dashboard`, `/shefa/resources/*`, operator-facing tabs) — not hidden in elohim-storage logs.
- Economic planning loops (subsidies, stewardship allocation, peer recruitment) must be able to subscribe/query these signals.
- When designing a dataplane flag/warning, ask: "what shefa decision would this inform?" If none, it's probably noise. If some, it deserves structure.
- This is the positive case of sense-and-respond: the mesh's breathing (not just its crashes) is data for the guardian loop (L7) once that lands.
