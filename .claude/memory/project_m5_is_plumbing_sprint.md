---
name: M5 is the plumbing/SDK convergence sprint, not a UX polish sprint
description: M5's deliverable is a clearly-defined SDK/API/graph surface; UI is minimum scaffold to verify wiring, polished presentation is a SEPARATE Playwright-driven sprint
type: project
originSessionId: 4d20bf7b-4639-43d8-ad10-fccb514a7f0a
---
**2026-04-25 user direction on Cut 3:** the goal is **wiring up** existing primitives end-to-end. Many patterns already exist — M5's job is convergence, not invention. **Emphasis on foundational plumbing** — explicitly note old patterns, gaps, and disconnects, and **resolve them in this sprint, definitely this sprint**.

The success metric: **the SDK/API/graph surface is clearly defined** so a follow-on sprint can do "design session on the graph/REST/SDK surface with Playwright to clean up/streamline the presentation layer." That's NOT this sprint.

**What this means for design:**

- **Audit-first.** Inventory what already exists across doorway auth, identity-handshake, AgentPeerBinding, peer_identity_bindings, ReconcileController, KeyRotation/Revocation primitives, elohim-app auth state, storage-client SDK, elohim-agent service scaffolding. Find the gaps and disconnects.
- **Define the SDK/API surface as deliverable.** Wire types, HTTP routes, gossipsub topics, libp2p protocols. Schema-first per `feedback_schema_first_ioc`. The Security & sign-in pane is a **verification harness** for that surface, not the deliverable itself.
- **Resolve disconnects in this sprint.** If there's an old auth pattern that doesn't compose with the new peer-binding flow, fix it in M5 — don't leave it as "to be reconciled later."
- **UI is scaffold.** Enough to exercise the wiring, not enough to ship to humans. Pixel polish, design system, accessibility are explicitly out of scope and belong to the follow-on sprint.

**How to apply:**

- Spec sections must include an explicit "Existing primitives" inventory and "Gaps to resolve in M5" subsection.
- Plan tasks are dominated by SDK/API/wire-type/schema work, with UI tasks framed as "scaffold for verification."
- a2o features verify wiring (e.g., "given a hosted human with steward presence, when they hit doorway/account, they are redirected to elohim-app with handoff token verifiable via identity-handshake") rather than UX delight.
- If the brainstorm finds a gap in the plumbing that needs new primitive design, that's the work — don't punt.
