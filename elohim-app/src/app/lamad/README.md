# Lamad Module

**Lamad** (לָמַד - Hebrew: "to learn/teach") is the path-centric learning infrastructure for the Elohim Protocol.

## ⚠️ Definitive Specification

**The Source of Truth for this module is:**  
[**`LAMAD_API_SPECIFICATION_v1.0.md`**](./LAMAD_API_SPECIFICATION_v1.0.md)

All development, refactoring, and architectural decisions must align with this document.

## Core Philosophy

Lamad operates on three fundamental separations of concern:

1.  **Territory (`ContentNode`)**: The immutable units of knowledge (videos, docs, simulations).
2.  **Journey (`LearningPath`)**: The curated paths that add narrative meaning and sequence to the Territory.
3.  **Traveler (`Agent`)**: The sovereign agents whose progress, affinity, and attestations shape their experience.

## Directory Structure

*   `models/`: TypeScript interfaces defining the Territory, Journey, and Traveler states.
*   `services/`: Business logic (the "Elohim" agents) for navigation, graph exploration, and progress tracking.
*   `components/`: Angular UI components for the "Meaning Map", "Path Navigator", and dashboards.
*   `renderers/`: Extensible rendering engine for different content types (Markdown, Video, Interactive).
*   `parsers/`: Tools to ingest source content (Markdown, Gherkin) into the Graph.
*   `adapters/`: Legacy adapters for migration.

## Development Status

We are currently migrating from the "Prototype" architecture to the **v1.0 Architecture**.
See [**`IMPLEMENTATION_PLAN.md`**](./IMPLEMENTATION_PLAN.md) for the active roadmap.