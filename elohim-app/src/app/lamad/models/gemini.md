# Lamad Models Implementation Guide

Ref: `LAMAD_API_SPECIFICATION_v1.0.md` - Part 2: Data Models

## Objective
Implement the rigorous Typescript interfaces that define the "Static Structure" (Lamad) and "Traveler" (Agent) states. These models must be Holochain-ready (serializable, ID-based references).

## Tasks

### 1. Refine `ContentNode`
- [ ] Update `models/content-node.model.ts` to match spec Section 2.3 exactly.
- [ ] Ensure `metadata` is typed but extensible.
- [ ] **Critical**: Ensure `id` is treated as an opaque string (ready for content hash).

### 2. Create `LearningPath` & `PathStep`
- [ ] Create `models/learning-path.model.ts`.
- [ ] Implement `LearningPath` interface (Section 2.1).
  - Note `prerequisitePaths` and `attestationsGranted`.
- [ ] Implement `PathStep` interface (Section 2.2).
  - Note `stepNarrative` vs the resource's own description.
  - Note `attestationRequired`.

### 3. Create `Agent` & `AgentProgress`
- [ ] Create `models/agent.model.ts`.
- [ ] Implement `Agent` interface (Section 2.4).
  - Handle `profileVisibility` privacy settings.
- [ ] Implement `AgentProgress` interface (Section 2.5).
  - **Key**: This tracks the *state* of a journey.
  - Include `stepAffinity` (Map) and `completedStepIndices`.

### 4. Create `Attestation`
- [ ] Update/Refine `models/attestations.model.ts`.
- [ ] Match spec Section 5.4 data model.
- [ ] Ensure `verificationType` and `revocable` fields are present.

## Constraints
- Do not import Service logic into Models. Models should be pure data structures.
- Use `string` for IDs, not numbers.
- Prepare for `AgentPubKey` types in future (alias `type AgentID = string` for now).
