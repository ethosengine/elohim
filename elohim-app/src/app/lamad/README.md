# Lamad Module

**Lamad** (לָמַד - Hebrew: "to learn/teach") is the graph-based learning and documentation platform for the Elohim Protocol.

## Intent

This module implements the **Meaning Map**, a navigation interface that guides users through the complexity of the Elohim Protocol (and potentially any other domain) using:

1.  **Affinity**: Tracking user engagement to tailor the journey.
2.  **Orientation**: Calculating the "next best step" toward a target subject.
3.  **Attestations**: Unlocking content based on proven capacity (Fog of War).

## Architectural Distinction

It is critical to distinguish **Lamad** from **Elohim**:

*   **Lamad** is the **Structure**: The nodes, edges, content, and metadata. It is the library.
*   **Elohim** are the **Agents**: The active intelligence that curates, protects, and guides users through the Lamad structure.

This module builds the **Structure** (Lamad) so the **Agents** (Elohim) have a place to work.

## Key Directories

*   `models/`: Defines the static shape of the graph (Nodes, Relationships, Types).
*   `parsers/`: Converts raw source files (Markdown, Gherkin) into Graph Nodes.
*   `components/`: Visualizes the Meaning Map and Content.
*   `services/`: Manages graph state and affinity tracking.
