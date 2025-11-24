# Lamad: Vision & Context

*Note: This document provides the high-level vision and conceptual inspirations. For technical implementation details, strictly refer to `LAMAD_API_SPECIFICATION_v1.0.md`.*

## The Vision

**Lamad** is a graph-based learning platform where:
*   **Attention is Sacred**: Content is presented with context and purpose, not as an endless feed.
*   **Reach is Earned**: Contribution rights are unlocked through proven capacity.
*   **Visibility is Earned** ("Fog of War"): Advanced or sensitive content is revealed only when the learner has the necessary foundation (Attestations).

### Conceptual Inspirations

1.  **Khan Academy's "World of Math"**:
    *   **Target Subject**: "The Elohim Protocol" (mastery goal).
    *   **Orientation**: You are here, the goal is there.
    *   **Mastery**: Progress is measured by demonstrated understanding, not just time spent.

2.  **Gottman's Love Maps**:
    *   **Affinity**: Learning is about building a relationship with the subject. We track *affinity* (0.0 - 1.0) as a measure of this relationship depth.
    *   **Knowing**: To know the protocol is to love it; to understand its "inner world."

3.  **Zelda: Breath of the Wild**:
    *   **Fog of War**: You can see the "Sheikah Towers" (goals) in the distance, but the map details are hidden until you make the journey.
    *   **Earned Access**: You must climb the tower (prove capacity) to download the map data.

## Architecture: Territory, Journey, Traveler

To support this vision, we distinguish three layers:

*   **Territory (The Graph)**: The immutable nodes of content. A video is a video; it doesn't know about your learning path.
*   **Journey (The Path)**: The narrative overlay. "Watch this video *because* it explains X, which you need for Y." The same territory can be part of multiple journeys.
*   **Traveler (The Agent)**: You. Your history, your affinity scores, your earned attestations. In the future, this data lives on your sovereign Holochain source chain.

## Terminology: Elohim vs. Lamad

*   **Lamad** is the **Structure**: The library, the map, the roads. It is passive.
*   **Elohim** are the **Agents**: The active intelligence (software services or AI) that guides you. "You seem stuck on this concept; try this path instead."

## Migration Note

We are transitioning from a "Document Graph" prototype (where structure was inferred from file folders) to a "Path Centric" v1.0 architecture (where paths are first-class entities).

**Key Changes in v1.0:**
*   **Lazy Loading**: We do not load the whole graph. We load the Path, and fetch steps as needed.
*   **Renderer Registry**: We don't hardcode views for Markdown/Gherkin. We use a registry to support any content type (VR, Audio, etc.).
*   **Attestations**: We are moving from simple "view tracking" to "proven capacity" (Quizzes, manual sign-offs, etc.).