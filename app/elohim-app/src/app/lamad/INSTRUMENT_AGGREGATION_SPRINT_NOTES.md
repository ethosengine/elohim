# Instrument Aggregation Sprint Notes

## Context
Sophia assessment engine is integrated for quiz rendering (mastery mode). This next sprint extends to discovery and reflection instrument aggregation — surfacing psychometric insights into the learner profile and self-knowledge map.

## 1. Storage
- `AssessmentService` currently stores results in localStorage (MVP)
- Key pattern: `lamad-assessment-{instrumentId}-{sessionId}`
- Next: Migrate to source chain entries via LocalSourceChainService
- Entry type: `assessment-result` on agent's private source chain

## 2. Psyche API Integration
- `aggregateReflections()` — aggregate open-ended reflection responses
- `getPrimarySubscale()` — identify dominant subscale from instrument results
- `interpretReflection()` — AI-assisted interpretation of reflection text
- These are exposed via sophia-element-loader's psyche-core bridge
- Key constraint: psyche-core must NEVER depend on perseus packages

## 3. Instrument → Attestation Flow
- `AssessmentResult` from sophia → check `AssessmentAttestationRequirement`
- If requirements met → mint `Attestation` with `AttestationJourney` proof
- Attestation types: `discovery-subscale`, `reflection-insight`, `psychometric-profile`
- Gating: certain attestations require minimum instrument validity scores

## 4. Self-Knowledge Map Integration
- `SelfKnowledgeMap` has `discoveredGifts`, `shadowAreas`, `insights` fields
- Instrument results contribute via `SelfKnowledgeLink` (links result → map node)
- Discovery instruments populate `discoveredGifts` with `GiftCategory` tags
- Reflection instruments populate `insights` with `SelfInsight` entries
- Longitudinal tracking via `LongitudinalChange` records

## 5. Dashboard Integration
- New `InstrumentInsightsComponent` sub-component for learner dashboard
- Shows: self-knowledge discoveries, subscale profiles, reflection summaries
- Visualization: radar chart for subscale profiles, timeline for discoveries
- Links to full self-knowledge map at `/lamad/map/self`

## Dependencies
- sophia-element UMD bundle with psyche-core bridge
- SelfKnowledgeMap model (already in knowledge-map.model.ts)
- AssessmentService (already in lamad/services)
- AttestationService (exists in elohim pillar)

## Open Questions
1. Should reflection interpretations be stored on-chain or kept ephemeral?
2. Privacy model for psychometric data — never DHT, source-chain only?
3. Minimum validity thresholds for attestation minting
4. How to handle instrument version upgrades and result comparability
