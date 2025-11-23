# Lamad Models

This directory contains the data structure definitions for the Lamad learning graph.

## Core Philosophy: Structural Agnosticism

The models defined here represent the **static structure** (the "bones") of the content graph, distinct from the **active agents** (the "Elohim") that operate upon it.

### Key Models

#### `ContentNode`
The generic container for all content. It replaces rigid type hierarchies with a flexible `contentType` string and a metadata object.

#### `LamadNodeType` (Enum)
Defines the specific content types supported by the Lamad graph.
*   **Why `Lamad` and not `Elohim`?**
    *   **Elohim** refers to real-time agents (actors).
    *   **Lamad** refers to the learning graph (artifacts).
    *   A `LamadNodeType.USER_TYPE` defines the *archetype* (e.g., "Community Investor"), whereas a `community_investor_elohim` would be an active agent instance.

#### `UserAffinity`
Tracks the relationship between a user and a specific node. This is the "footprint" left by the user as they traverse the graph.

#### `Attestation` (Planned)
Represents proof of capacity or achievement. These are the "keys" that unlock protected content in the graph.

## Relationship to Elohim Agents

*   **Lamad Models** define the map.
*   **Elohim Agents** read the map, guide the user, and verify attestations.

For example:
1.  **Lamad** defines a node "Advanced Civic Organizing" with metadata `requires: 'civic-trust-level-2'`.
2.  The **User** interacts with the platform.
3.  The **Elohim Agent** verifies if the user has the `civic-trust-level-2` attestation and grants or denies access.
