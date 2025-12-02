# Lamad Data Models

This directory contains the TypeScript interfaces that define the **Static Structure** (Lamad) and **Traveler State** (Agent).

## Core Models (v1.0)

Refer to `LAMAD_API_SPECIFICATION_v1.0.md` Part 2 for detailed field definitions.

### 1. Territory
*   **`ContentNode`**: The atomic unit of knowledge.
    *   Replaces rigid `Epic` / `Feature` / `Scenario` types with a generic `contentType` field.
    *   Contains `metadata` for domain-specific fields.
    *   Rendered by the `RendererRegistry`.

### 2. Journey
*   **`LearningPath`**: A curated sequence of steps.
    *   First-class entity (can be created, forked, versioned).
*   **`PathStep`**: A specific stop on a journey.
    *   Links a `ContentNode` with a *narrative* context ("Why this matters here").

### 3. Traveler
*   **`Agent`**: The learner/user.
    *   Identity (will be `AgentPubKey` in Holochain).
*   **`AgentProgress`**: The state of a specific journey.
    *   Tracks `stepAffinity`, `completedStepIndices`.
*   **`Attestation`**: Proof of capacity.
    *   Unlocks content ("Fog of War").

## Legacy Models
*   `DocumentNode`, `EpicNode`, `FeatureNode`: These are being phased out or adapted into the generic `ContentNode` structure via `adapters/`.