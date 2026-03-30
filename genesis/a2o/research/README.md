# Distributed Observation Protocol — Research

## What This Is

The a2o test harness is being researched as the seed of a **distributed observation protocol**. Scenarios become content-addressed behavioral claims. Execution mints economic events. Replication by independent agents builds trust. Invalidation is rewarded, not punished.

The organizing concept is **interpretability** — not mechanistic interpretability (looking inside the model) or chain-of-thought (asking the model to narrate), but **behavioral interpretability through peer-attested observation**: an agent produces a structured claim about what it did, and independent peers replicate or dispute it. The Gherkin grammar is already structured natural language — already halfway to an interpretability format. The fork completes that journey.

## Why Here

The existing `genesis/a2o/` directory already contains the infrastructure this protocol builds on:

- **30 feature files** with 100+ scenarios across 9 domain directories
- **24 step definition files** (API + browser/Playwright)
- **Coverage gap analysis** (`scripts/scan-coverage.ts`) and step skeleton generation
- **Close-loop workflow**: `dev-intent.jsonl` → scenario generation → implementation
- **Cucumber profiles** for multiple environments (alpha, local, browser, genesis, testnet)

Cucumber's Given/When/Then maps directly to **precondition/action/postcondition** — an observation grammar, not just a test grammar. The existing close-loop workflow already treats scenarios as development artifacts that connect intent to evidence. The protocol makes that connection formal, economic, and peer-verifiable.

## Connection to Existing Protocol Work

| Design | Path | Relationship |
|--------|------|--------------|
| **Observer Protocol** | `genesis/docs/content/elohim-protocol/observer-protocol.md` | Ephemeral witness architecture applied to behavioral claims — "being seen becomes sacred" extended from physical observation to any claim an agent makes about reality |
| **Feedback Information Flows** | `genesis/plans/2026-03-28-feedback-information-flows-design.md` | The approved three-layer model (claims + observations + obligation accumulation) that this protocol implements for scenario execution |
| **P2P Build System Roadmap** | `genesis/plans/2026-03-20-p2p-native-build-system-roadmap.md` | The Seed→Root→Canopy→Forest staging pattern this roadmap follows, and the precedent of artifacts-as-ContentNodes |
| **REA Economics** | `.claude/skills/rea-economics/SKILL.md` | The economic event pipeline — observations are EconomicEvents with `action="observe"`, flowing through the same infrastructure as all other value |
| **Signal Harness** | `app/elohim-app/src/app/lamad/services/signal-harness.service.ts` | The direct architectural precedent — renderer completion events translated to economic events via manifest coupling |
| **EPR Content Addressing** | `.claude/skills/epr-content-addressing/SKILL.md` | Content-addressed linking with three-leg coupling (knowledge + value + governance) applied to scenario nodes |

## Documents

1. **[vision.md](vision.md)** — The Distributed Observation Protocol: why fork Cucumber, what the protocol is, how it connects to observer/feedback/REA, the interpretability thesis, and the LLM verification implications
2. **[grammar-spec.md](grammar-spec.md)** — The Grammar Fork: what changes (tag vocabulary), what stays the same (everything humans write), how agents absorb protocol complexity, the dual-surface interpretability contract
3. **[execution-model.md](execution-model.md)** — From Test Runner to Observation Minter: component-by-component transformation, dual-mode executor (local/attestation), the signal harness parallel
4. **[replication-protocol.md](replication-protocol.md)** — Peer-Attested Observations: scientific replication model, freshness decay, diversity weighting, adversarial resilience, connection to Observer Protocol
5. **[economics.md](economics.md)** — Observation Royalties: bounty/delivery/residual/invalidation model, REA expression, commons default, self-sustainability analysis, shefa integration
6. **[sprints.md](sprints.md)** — Implementation Roadmap: six sprints following Seed→Root→Canopy→Forest arc, with deliverables, entry points, and upgrade paths

## Status

**Research phase.** No implementation changes to existing a2o infrastructure yet. These documents define the vision and roadmap; implementation begins when the research is validated and Sprint 0 is scheduled.
