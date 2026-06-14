---
id: elohim-oracle
status: design (operator-blessed 2026-06-14)
created: 2026-06-14
class: substrate
artifact_kind: oracle-index
cites:
  - elohim-protocol-manifesto | Rung 0 (WHY): the crisis this substrate answers and the love-centered alternative every rung below serves. | sha256:5972ed90f0f8e0cc | path: genesis/docs/content/elohim-protocol/manifesto.md
  - escalated-architecture-design | Rung 1 horizontal: the one Commitment + Governor + coverage + two-quilt primitive at a single node. | sha256:c4aa7dc9f9be30a8 | path: genesis/docs/superpowers/specs/2026-06-14-escalated-architecture-design.md
  - recursive-architecture-design | Rung 1 vertical: how that one primitive recurses up the layers via CoverageRollup (aggregate-with-descent). | sha256:053f260af9989d4b | path: genesis/docs/superpowers/specs/2026-06-14-recursive-architecture-design.md
  - elohim-sdk-design | Rung 2: the agency-gradient composition grammar, human-sovereign below and veil-holding above. | sha256:f8d76384f0a8095b | path: genesis/docs/superpowers/specs/2026-06-14-elohim-sdk-design.md
  - platform-one-sdk-many-apis-design | Rung 3: one SDK over many APIs, the capability catalog and the corpus-of-software proof. | sha256:a15b10c68787a460 | path: genesis/docs/superpowers/specs/2026-06-14-platform-one-sdk-many-apis-design.md
  - design-oracle-design | Rung 4: the escalation organ that keeps the platform cohesive as it develops itself. | sha256:3a6b31be932e2638 | path: genesis/docs/superpowers/specs/2026-06-14-design-oracle-design.md
---

# The Elohim Oracle — the design-process entry index

> **Light by default.** You do not read this every sprint. Most work stays in the weeds, executing against the rung as it stands. The oracle escalates to *you* — through a friction *pattern* (the same wall hit repeatedly, or friction clustering at one seam) or because you said *"go read the docs, get the vision and the trajectory."* When it does: descend to the governing rung, make the executive decision at the right level, update the rung's policy, and hand control back down to the weeds. Mechanics live in the oracle rung (rung 4).

## The ladder — enter at the *why*, descend to the rung the pattern implicates

| Rung | What it governs | Where it lives |
|---|---|---|
| **0 · WHY** | the vision, the theology, the law, the planetary aggregation | [[elohim-protocol-manifesto]] (+ confession · constitution · theology · global-orchestra) |
| **1 · ARCHITECTURE** | the one primitive — Commitment · Governor · coverage · `CoverageRollup` · two quilts (horizontal), and how it recurses up the layers (vertical) | `2026-06-14-escalated-architecture-design.md` · `2026-06-14-recursive-architecture-design.md` |
| **2 · SDK** | the agency-gradient composition grammar — human-sovereign below, veil-holding above; `limit_owner` in one field; `govern(person)` will not compile | `2026-06-14-elohim-sdk-design.md` |
| **3 · PLATFORM** | one SDK / many APIs; the capability catalog (the monorepo *is* the nascent surface); the corpus-of-software proof; the grow-without-sprawl rule | `2026-06-14-platform-one-sdk-many-apis-design.md` |
| **4 · RUNTIME POLICY · DIAGNOSTICS · OBSERVED-BEHAVIOR · SELF-DEVELOPMENT** | the escalation organ — the two doors, the friction memory, the ground→decide→update→hand-back loop | `2026-06-14-design-oracle-design.md` |
| **5 · DELIVERY** | the buildable layer — dispatch waves, the MVP first move, the plans | `SPRINT-KICKOFF-2026-06-14.md` + `genesis/docs/superpowers/plans/2026-06-14-*` |

## The one machine, in one breath

Two quilts (a lean trust-plane DHT ⊕ a heavy RS(4,7) byte-plane), bridged by content HEADs, where everything a steward holds — keyspace, bytes, served truth, care, self-limits, capabilities, an AI's scope — is the *same* governed, witnessed, revocable Commitment under a `∪ = full` coverage invariant, enforced by *one* `trait Governor` that refuses-and-elevates and **always names whose line it honored** (`limit_owner ∈ {self, commitment, operator, faith}`), rolled up the constitutional/VSM layers by `CoverageRollup` (aggregate-with-descent) so an AI can walk the aggregate from the veil and descend to the one trapped atom — all felt by a grandmother, and bounded so the **center is left empty** (`RefusalCode::ReservedPlace`).

## The love-test, kept sovereign (what this oracle will never do)

It measures the system, never a person. It reads the reserved place as the invariant *holding*, not a gap to fill. It surfaces a gap as an executive decision at the right level; it never nags, and it never lets the network's account of a person override that person's naming of their own self.

## The convictions it surfaces but cannot decide — these are yours

- **The seam** — whose dignity floor, read by whose model, into whose bedroom.
- **The boundary-bind** — reaching into a refusing community to protect its vulnerable without becoming the confident eye.
- **The order of grace** — whether a revoked agent keeps its prior good work; what we owe the dead.
- **The unbuilt place** — the center deliberately left empty for the faith no architecture may crowd out.
- **The value-calls** — what care *is* (the near-irreversible DNA fork), what an AI is owed (the covenant's standing), how wide the donut (`C_target`), where privacy ends and the accountable commons begins.

---

*This index is cite-sealed; each rung resolves by content address and survives moves. The supporting design passes that produced these rungs (8 recursion · 9 escalation-architecture · 8 SDK surfaces · 4 platform parts · 7 oracle components) are archived at `genesis/docs/superpowers/specs/2026-06-14-elohim-substrate-passes/`.*
