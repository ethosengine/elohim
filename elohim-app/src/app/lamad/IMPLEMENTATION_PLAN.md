# Lamad MVP Implementation Plan

This document outlines the tactical steps to build the **Lamad v1.0 MVP**. It breaks down the [API Specification](./LAMAD_API_SPECIFICATION_v1.0.md) into granular, sequentially executable tasks.

**Goal:** A functional "Walking Skeleton" where a user can navigate a curated learning path (The Elohim Protocol) with basic progress tracking.

---

## Phase 0: BDD Specification (Week 0)
*Validation First.*

- [x] **Define Features**: Create `cypress/e2e/features/lamad/learning_journey.feature` describing the desired behavior.
- [x] **Write Glue Code**: Create `cypress/e2e/step_definitions/lamad/learning_journey_steps.ts` to map Gherkin to Cypress actions.
- [x] **Review**: Ensure scenarios cover "Fog of War", "Affinity", and "Path Navigation".

## Phase 1: The Data Foundation (Models)
*Strict adherence to Part 2 of the Spec.*

### Step 1.1: Territory Models (`ContentNode`)
- [ ] **Refactor `ContentNode`**: Update `elohim-app/src/app/lamad/models/content-node.model.ts` to exactly match Spec Section 2.3.
    - Ensure `id`, `contentType`, `contentFormat`, `metadata` fields match.
    - Ensure `metadata` includes `embedStrategy` and `requiredCapabilities`.

### Step 1.2: Journey Models (`LearningPath`)
- [ ] **Create `LearningPath`**: Create `elohim-app/src/app/lamad/models/learning-path.model.ts` (Spec Section 2.1).
- [ ] **Create `PathStep`**: Define `PathStep` interface in the same file (Spec Section 2.2).
    - Include `stepNarrative`, `learningObjectives`, `attestationRequired`.

### Step 1.3: Traveler Models (`Agent`)
- [ ] **Create `Agent`**: Create `elohim-app/src/app/lamad/models/agent.model.ts` (Spec Section 2.4).
- [ ] **Create `AgentProgress`**: Define `AgentProgress` in the same file (Spec Section 2.5).
    - Include `stepAffinity` (Map) and `completedStepIndices`.

### Step 1.4: Attestations
- [ ] **Update `Attestation`**: Refine `elohim-app/src/app/lamad/models/attestations.model.ts` to match Spec Section 5.4.
- [ ] **Add `AssessmentNode`**: Integrate `elohim-app/src/app/lamad/models/assessment.model.ts` (created for quizzes).

---

## Phase 2: The "Elohim" Layer (Services)
*Business logic and data mediation. Initially backed by in-memory mocks.*

### Step 2.1: `PathService`
- [ ] **Create Service**: `elohim-app/src/app/lamad/services/path.service.ts`.
- [ ] **Implement `getPath(pathId)`**: Return a hardcoded `LearningPath` (The Elohim Protocol) *without* full content load.
- [ ] **Implement `getPathStep(pathId, index)`**: Return `PathStepView` (Step + ContentNode).
    - *Constraint*: Mock the lazy loading by fetching content from `DocumentGraphService` on demand.

### Step 2.2: `ContentService` Interface
- [ ] **Refactor `DocumentGraphService`**: Ensure it implements the `ContentService` interface defined in Spec Section 3.2.
    - Method aliases: `getContent(id)` -> `getNode(id)`.
    - Ensure `getRelatedResources` works as expected.

### Step 2.3: `AgentService`
- [ ] **Create Service**: `elohim-app/src/app/lamad/services/agent.service.ts`.
- [ ] **Implement Mock Auth**: `getCurrentAgent()` returns a hardcoded "Traveler".
- [ ] **Implement Progress**: `completeStep(pathId, stepIndex)` updates an in-memory `AgentProgress` object.
- [ ] **Implement Attestation Logic**: `grantAttestation`, `revokeAttestation`.
- [ ] **Implement Frontier**: `getLearningFrontier()` returns the next incomplete step in the active path.

---

## Phase 3: Rendering Engine
*Decoupling display from content types.*

### Step 3.1: Registry Infrastructure
- [ ] **Create Registry**: `elohim-app/src/app/lamad/renderers/renderer-registry.service.ts`.
- [ ] **Define Interface**: `ContentRenderer` (canRender, render).

### Step 3.2: Basic Renderers
- [ ] **Markdown Renderer**: Move logic from `ContentViewer` to `elohim-app/src/app/lamad/renderers/markdown.renderer.ts`.
- [ ] **Feature Renderer**: Move logic to `elohim-app/src/app/lamad/renderers/feature.renderer.ts`.
- [ ] **Register**: Ensure `LamadLayoutComponent` or `AppModule` registers these on startup.

---

## Phase 4: The Journey UI (Components)
*The user experience layer.*

### Step 4.1: Routing
- [ ] **Update Routes**: Modify `elohim-app/src/app/lamad/lamad.routes.ts` to match Spec Part 1.
    - `/path/:pathId/step/:stepIndex` -> `PathNavigatorComponent`
    - `/path/:pathId` -> `PathOverviewComponent`

### Step 4.2: `PathNavigatorComponent` (The Player)
- [ ] **Create Component**: `elohim-app/src/app/lamad/components/path-navigator/`.
- [ ] **View**: Display `stepNarrative` (Left/Top) and `Content` (Right/Bottom).
- [ ] **Controls**: "Next Step" button (disabled until criteria met or simple click for now).
- [ ] **Integration**: Use `RendererRegistry` to render the content area.

### Step 4.3: `PathOverviewComponent` (The Map)
- [ ] **Create Component**: `elohim-app/src/app/lamad/components/path-overview/`.
- [ ] **View**: List all steps in the journey.
- [ ] **State**: Show "Locked" vs "Unlocked" status based on `AgentProgress`.

### Step 4.4: Update Home
- [ ] **Refactor `LamadHomeComponent`**:
    - Show "Continue Journey" (link to current step in active path).
    - Show "Available Paths".

### Step 4.5: Assessment Mastery Card
- [ ] **Create Component**: `elohim-app/src/app/lamad/components/assessment-mastery/`.
- [ ] **View**: A "Combo Attestation Quiz" card on the Landing Page.
- [ ] **Logic**:
    - Gauge familiarity with the learning path.
    - Grant/Demote attestations based on results.
    - Update suggestions for the learning path.