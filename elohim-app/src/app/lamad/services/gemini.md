# Lamad Services Implementation Guide

Ref: `LAMAD_API_SPECIFICATION_v1.0.md` - Part 3: Service Layer Contracts

## Objective
Implement the business logic layer. These services act as the "Elohim" (active agents) that mediate access to the "Lamad" (static structure).

## Tasks

### 1. `PathService`
- [ ] Create/Refactor `services/path.service.ts`.
- [ ] Implement `getPath(pathId)`: Returns path metadata *without* full content (Lazy Loading!).
- [ ] Implement `getPathStep(pathId, stepIndex)`: The core "Fog of War" method.
  - Fetches *only* the specific step and its resource.
  - **Constraint**: Do not preload >2 steps ahead.
- [ ] Implement `listPaths(filters)` for the discovery UI.

### 2. `ContentService`
- [ ] Refactor `services/document-graph.service.ts` into `ContentService` (or alias it).
- [ ] Implement `getContent(resourceId)`: Direct access to territory nodes.
- [ ] Implement `getRelatedResources(resourceId)`: Graph traversal (1 hop).
- [ ] Ensure separation of concerns: ContentService returns *Nodes*, PathService returns *Journeys*.

### 3. `AgentService`
- [ ] Create `services/agent.service.ts`.
- [ ] Implement `getCurrentAgent()`: Auth context resolution.
- [ ] Implement `getAgentProgress(agentId, pathId)`.
- [ ] Implement `completeStep(pathId, stepIndex)`:
  - Updates progress.
  - Triggers `grantAttestation` if applicable.
  - Updates `LearningFrontier`.
- [ ] Implement `getLearningFrontier()`: The "What's next?" logic.

### 4. `ExplorationService`
- [ ] Create `services/exploration.service.ts`.
- [ ] Implement `exploreNeighborhood(focus, depth)`:
  - **Check Attestations**: Depth > 1 requires 'researcher' attestation.
  - **Check Rate Limits**: Graph traversal is expensive.
- [ ] Implement `estimateCost(operation)`: For "Fog of War" computational costs.

## Mocking Strategy
- Initially, mock these services with in-memory data or local JSON files.
- Use Angular's dependency injection to swap real implementations later (e.g., `useClass: HolochainPathService`).
