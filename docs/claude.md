# Elohim Documentation Graph - Data Model & Storage Architecture

## Overview

This document describes the graph-based data model used across the Elohim Protocol for organizing, storing, and relating content nodes. The model is **domain-agnostic** and designed to be easily serializable for future graph database migration while remaining functional with simple JSON storage today.

The graph structure is implemented in the **Lamad** learning platform (`elohim-app/src/app/lamad`) but the principles apply to any content domain within the Elohim ecosystem.

## Core Philosophy

**Think Graph-First, Store Simply**

The data model is designed as a true graph from the ground up, with:
- **Nodes** representing content entities
- **Edges** (relationships) representing semantic connections
- **Indices** for efficient querying and traversal
- **Metadata** for extensibility without schema changes

However, storage is kept simple using JSON files and Angular services until scale demands a true graph database (Neo4j, OrientDB, etc.).

## Graph Structure

### Three-Layer Architecture

```
┌─────────────────────────────────────────────────┐
│           DocumentGraph (Container)              │
├─────────────────────────────────────────────────┤
│                                                  │
│  ┌────────────────────────────────────────┐    │
│  │         Nodes (Content)                 │    │
│  │  - All nodes keyed by ID               │    │
│  │  - Type-specific indices               │    │
│  │  - Tag indices                         │    │
│  │  - Category indices                    │    │
│  └────────────────────────────────────────┘    │
│                                                  │
│  ┌────────────────────────────────────────┐    │
│  │      Relationships (Edges)              │    │
│  │  - Semantic connections                │    │
│  │  - Typed relationships                 │    │
│  │  - Bidirectional where appropriate     │    │
│  └────────────────────────────────────────┘    │
│                                                  │
│  ┌────────────────────────────────────────┐    │
│  │    Traversal Structures                 │    │
│  │  - Adjacency lists (forward)           │    │
│  │  - Reverse adjacency (backward)        │    │
│  │  - Path caching (future)               │    │
│  └────────────────────────────────────────┘    │
│                                                  │
└─────────────────────────────────────────────────┘
```

## Data Models

### 1. Base Node Structure

All content in the system inherits from `DocumentNode`:

```typescript
interface DocumentNode {
  id: string;                    // Unique identifier (e.g., "epic-social-medium")
  type: NodeType;                // 'epic' | 'feature' | 'scenario'
  title: string;                 // Display title
  description: string;           // Brief summary
  tags: string[];                // For categorization and search
  sourcePath: string;            // File path to source document
  content: string;               // Full content body
  relatedNodeIds: string[];      // Direct relationships
  metadata: Record<string, any>; // Flexible extensions
  createdAt?: Date;
  updatedAt?: Date;
}
```

**Design Rationale:**
- Generic base allows extension to any content type
- `metadata` field prevents schema brittleness
- `relatedNodeIds` are denormalized for quick access (also stored as formal relationships)

### 2. Node Type Hierarchy

#### Epic Node (Narrative Documentation)
Source: `docs/*.md` files

```typescript
interface EpicNode extends DocumentNode {
  type: NodeType.EPIC;
  authors?: string[];            // Content creators
  version?: string;              // Version tracking
  category?: string;             // Thematic grouping
  featureIds: string[];          // Features implementing this epic
  relatedEpicIds: string[];      // Cross-references to other epics
  markdownContent: string;       // Original markdown
  sections: EpicSection[];       // Parsed structure
}

interface EpicSection {
  title: string;
  level: number;                 // Heading depth (1-6)
  anchor: string;                // URL fragment
  content: string;
  embeddedReferences: EmbeddedReference[];
}
```

**Storage Pattern:**
- One epic node per markdown file in `/docs`
- Sections extracted during parse for quick navigation
- Embedded references create relationships automatically

#### Feature Node (Executable Specifications)
Source: `cypress/e2e/features/**/*.feature` files

```typescript
interface FeatureNode extends DocumentNode {
  type: NodeType.FEATURE;
  category: string;              // Based on directory structure
  epicIds: string[];             // Epics this implements
  scenarioIds: string[];         // Scenarios within this feature
  featureDescription: string;    // Gherkin description
  background?: GherkinBackground;
  testStatus?: TestStatus;       // CI/CD integration
  gherkinContent: string;        // Original .feature file
}
```

**Storage Pattern:**
- One feature node per `.feature` file
- Category derived from directory path
- Test status updated from CI/CD pipeline

#### Scenario Node (Individual Test Cases)
Extracted from feature files

```typescript
interface ScenarioNode extends DocumentNode {
  type: NodeType.SCENARIO;
  featureId: string;             // Parent feature
  epicIds: string[];             // Related epics (via tags)
  scenarioType: 'scenario' | 'scenario_outline' | 'example';
  steps: GherkinStep[];
  examples?: ScenarioExamples[];
  testStatus?: TestStatus;
  stepResults?: StepResult[];
}
```

**Storage Pattern:**
- Multiple scenario nodes per `.feature` file
- Automatically linked to parent feature
- Test execution results tracked per-step

### 3. Generic Content Node (Future)

For extensibility beyond documentation:

```typescript
interface ContentNode {
  id: string;
  contentType: string;           // Domain-specific: 'tutorial', 'article', etc.
  title: string;
  description: string;
  content: string;
  contentFormat: ContentFormat;  // 'markdown' | 'gherkin' | 'html' | 'plaintext'
  tags: string[];
  sourcePath?: string;
  relatedNodeIds: string[];
  metadata: ContentMetadata;     // Completely flexible
  createdAt?: Date;
  updatedAt?: Date;
}
```

**Migration Path:**
- Adapters bridge `DocumentNode` ↔ `ContentNode`
- See `elohim-app/src/app/lamad/adapters/document-node.adapter.ts`

### 4. Relationships (Edges)

Relationships are **first-class entities**, not just properties:

```typescript
interface NodeRelationship {
  id: string;                    // Unique relationship ID
  type: RelationshipType;        // Semantic relationship type
  sourceId: string;              // From node
  targetId: string;              // To node
  weight?: number;               // Relationship strength (0-1)
  description?: string;          // Human-readable explanation
  metadata?: Record<string, any>;
  bidirectional: boolean;        // Whether this is symmetric
}
```

#### Relationship Types

```typescript
enum RelationshipType {
  DESCRIBES = 'describes',       // Epic → Feature
  IMPLEMENTS = 'implements',     // Feature → Epic (reverse)
  BELONGS_TO = 'belongs_to',     // Scenario → Feature
  VALIDATES = 'validates',       // Scenario → Epic
  RELATES_TO = 'relates_to',     // Generic relation
  REFERENCES = 'references',     // Epic → Epic
  DEPENDS_ON = 'depends_on'      // Dependency
}
```

**Bidirectional Relationships:**

Some relationships automatically create reverse edges:

```typescript
// Epic DESCRIBES Feature also creates Feature IMPLEMENTS Epic
createBidirectionalRelationship(
  RelationshipType.DESCRIBES,
  'epic-social-medium',
  'feature-reach-earned'
)
// Creates TWO relationship entities:
// 1. epic-social-medium --DESCRIBES--> feature-reach-earned
// 2. feature-reach-earned --IMPLEMENTS--> epic-social-medium
```

**Design Rationale:**
- Relationships as entities enable rich metadata
- Bidirectional relationships simplify traversal
- Type system enforces semantic correctness
- Weights enable future ranking/relevance algorithms

## Graph Container

The `DocumentGraph` interface wraps everything together:

```typescript
interface DocumentGraph {
  // Core storage
  nodes: Map<string, DocumentNode>;
  relationships: Map<string, NodeRelationship>;

  // Indexed views for efficient queries
  nodesByType: {
    epics: Map<string, EpicNode>;
    features: Map<string, FeatureNode>;
    scenarios: Map<string, ScenarioNode>;
  };
  nodesByTag: Map<string, Set<string>>;       // tag → Set<nodeId>
  nodesByCategory: Map<string, Set<string>>;  // category → Set<nodeId>

  // Traversal structures
  adjacency: Map<string, Set<string>>;        // nodeId → Set<connectedNodeIds>
  reverseAdjacency: Map<string, Set<string>>; // nodeId → Set<incomingNodeIds>

  // Metadata
  metadata: GraphMetadata;
}
```

### Index Strategy

**Multi-Index Pattern:**

Every node is stored once but indexed multiple ways:

```typescript
// Example: Adding a feature node
function addNodeToGraph(graph: DocumentGraph, node: FeatureNode) {
  // 1. Primary storage
  graph.nodes.set(node.id, node);

  // 2. Type index
  graph.nodesByType.features.set(node.id, node);

  // 3. Tag indices
  node.tags.forEach(tag => {
    if (!graph.nodesByTag.has(tag)) {
      graph.nodesByTag.set(tag, new Set());
    }
    graph.nodesByTag.get(tag)!.add(node.id);
  });

  // 4. Category index
  if (!graph.nodesByCategory.has(node.category)) {
    graph.nodesByCategory.set(node.category, new Set());
  }
  graph.nodesByCategory.get(node.category)!.add(node.id);

  // 5. Adjacency initialization
  graph.adjacency.set(node.id, new Set());
  graph.reverseAdjacency.set(node.id, new Set());
}
```

**Query Performance:**

- Get all epics: `O(1)` via `nodesByType.epics`
- Get nodes by tag: `O(1)` via `nodesByTag.get(tag)`
- Get related nodes: `O(1)` via `adjacency.get(nodeId)`
- Full-text search: `O(n)` but parallelizable

### Adjacency Lists

Two adjacency structures enable bidirectional traversal:

```typescript
// Forward adjacency: "What does this node point to?"
adjacency: Map<string, Set<string>>
// Example: adjacency.get('epic-social-medium')
//   → Set(['feature-reach-earned', 'feature-attention-sacred'])

// Reverse adjacency: "What points to this node?"
reverseAdjacency: Map<string, Set<string>>
// Example: reverseAdjacency.get('feature-reach-earned')
//   → Set(['epic-social-medium', 'scenario-emma-bus-advocacy'])
```

**Use Cases:**
- Forward: "Show all features implementing this epic"
- Reverse: "Show all epics related to this scenario"
- Both: "Find shortest path between two nodes"

## Storage Formats

### Current: In-Memory + JSON Manifests

**Build-Time:**
1. Markdown parser scans `/docs` → Epic nodes
2. Gherkin parser scans `/cypress/e2e/features` → Feature/Scenario nodes
3. Relationship builder connects nodes based on:
   - Epic `@feature` tags → Feature IDs
   - Feature `@epic` tags → Epic IDs
   - Scenario tags → Both
4. Graph serialized to JSON and placed in `assets/`

**Runtime:**
```typescript
// Service loads manifests
http.get('assets/docs/manifest.json')
http.get('assets/features/manifest.json')

// Builds graph in-memory
graph = {
  nodes: new Map(),
  relationships: new Map(),
  // ... indices ...
}

// Cached in BehaviorSubject
graphSubject.next(graph)
```

**File Structure:**
```
elohim-app/src/assets/
├── docs/
│   ├── manifest.json          # List of epic files
│   ├── social-medium.md       # Epic content
│   ├── governance.md
│   └── ...
└── features/
    ├── manifest.json          # List of feature files + categories
    ├── social-medium/
    │   └── reach-earned.feature
    └── governance/
        └── voting.feature
```

**Manifest Format:**
```json
// assets/docs/manifest.json
{
  "files": [
    "social-medium.md",
    "governance.md"
  ]
}

// assets/features/manifest.json
{
  "files": [
    { "path": "social-medium/reach-earned.feature", "category": "social-medium" },
    { "path": "governance/voting.feature", "category": "governance" }
  ]
}
```

### Future: Graph Database

**Serializable Export Format:**

```typescript
interface SerializableGraph {
  nodes: DocumentNode[];         // Array instead of Map
  relationships: NodeRelationship[];
  metadata: GraphMetadata;
  version: string;               // Schema version
}
```

**Export to JSON:**
```typescript
function serializeGraph(graph: DocumentGraph): SerializableGraph {
  return {
    nodes: Array.from(graph.nodes.values()),
    relationships: Array.from(graph.relationships.values()),
    metadata: graph.metadata,
    version: '1.0.0'
  };
}
```

**Import to Neo4j (Example):**
```cypher
// Create nodes
UNWIND $nodes AS node
CREATE (n:DocumentNode {
  id: node.id,
  type: node.type,
  title: node.title,
  description: node.description,
  tags: node.tags,
  content: node.content,
  metadata: node.metadata
})

// Create relationships
UNWIND $relationships AS rel
MATCH (source:DocumentNode {id: rel.sourceId})
MATCH (target:DocumentNode {id: rel.targetId})
CREATE (source)-[r:RELATES {
  type: rel.type,
  weight: rel.weight,
  description: rel.description
}]->(target)
```

**JSON-LD for Semantic Web (Future):**

```typescript
interface GraphJSONLD {
  '@context': {
    '@vocab': 'https://elohim.love/schema#',
    epic: 'https://elohim.love/schema/epic',
    feature: 'https://elohim.love/schema/feature',
    describes: 'https://elohim.love/schema/describes',
    // ...
  };
  '@graph': Array<{
    '@id': string;
    '@type': string;
    [key: string]: any;
  }>;
}
```

## User Affinity Layer

**Separate from Content Graph:**

User engagement is tracked independently and **joined at query time**:

```typescript
interface UserAffinity {
  userId: string;
  affinity: { [nodeId: string]: number };  // 0.0 to 1.0
  lastUpdated: Date;
}
```

**Storage:**
- Current: `localStorage` with key `user-affinity-${userId}`
- Future: Backend database with per-user records

**Why Separate?**
- Content graph is **shared** across all users
- Affinity is **per-user** state
- Separation enables:
  - Static content CDN hosting
  - Collaborative filtering algorithms
  - Privacy controls
  - Multi-tenancy

**Query Pattern:**
```typescript
// Get nodes with user affinity
const nodes = getNodesByType('epic');
const withAffinity = nodes.map(node => ({
  ...node,
  userAffinity: affinityService.getAffinity(node.id)
}));
```

## Attestations Layer (Future)

Similar separation for achievement tracking:

```typescript
interface UserAttestations {
  userId: string;
  attestations: Attestation[];
  lastUpdated: Date;
}

interface Attestation {
  id: string;
  name: string;
  type: AttestationType;
  journey: AttestationJourney;   // Proof of capacity
  earnedAt: Date;
  metadata?: Record<string, any>;
}
```

**Storage:**
- Future: Backend with cryptographic verification
- Journey includes content nodes visited, affinity progression, contributions

**Integration with Graph:**
- Content nodes can specify `accessRequirements: string[]` (attestation IDs)
- Graph queries filter based on user's earned attestations
- Progressive revelation UI shows locked content

## Graph Queries & Traversal

### Basic Queries

```typescript
// Get node by ID
getNode(id: string): DocumentNode | undefined

// Get all nodes of type
getNodesByType(type: 'epic' | 'feature' | 'scenario'): DocumentNode[]

// Get related nodes
getRelatedNodes(nodeId: string): DocumentNode[]

// Search by text
searchNodes(query: string): DocumentNode[]
```

### Advanced Queries (Future)

```typescript
interface GraphQuery {
  startNodeId?: string;
  nodeTypes?: string[];
  tags?: string[];              // AND logic
  categories?: string[];
  maxDepth?: number;
  relationshipTypes?: string[];
  searchText?: string;
}

interface GraphTraversalResult {
  nodes: DocumentNode[];
  relationships: NodeRelationship[];
  paths: Map<string, string[]>;    // nodeId → path
  depths: Map<string, number>;     // nodeId → depth
}
```

**Example: Breadth-First Traversal**

```typescript
function traverseGraph(
  graph: DocumentGraph,
  query: GraphQuery
): GraphTraversalResult {
  const visited = new Set<string>();
  const queue: Array<{ nodeId: string; depth: number; path: string[] }> = [];

  // Initialize
  if (query.startNodeId) {
    queue.push({
      nodeId: query.startNodeId,
      depth: 0,
      path: [query.startNodeId]
    });
  }

  const results: GraphTraversalResult = {
    nodes: [],
    relationships: [],
    paths: new Map(),
    depths: new Map()
  };

  // BFS
  while (queue.length > 0) {
    const { nodeId, depth, path } = queue.shift()!;

    if (visited.has(nodeId)) continue;
    if (query.maxDepth && depth > query.maxDepth) continue;

    visited.add(nodeId);
    const node = graph.nodes.get(nodeId);
    if (!node) continue;

    // Apply filters
    if (query.nodeTypes && !query.nodeTypes.includes(node.type)) continue;
    if (query.tags && !query.tags.every(tag => node.tags.includes(tag))) continue;

    // Add to results
    results.nodes.push(node);
    results.paths.set(nodeId, path);
    results.depths.set(nodeId, depth);

    // Enqueue neighbors
    const neighbors = graph.adjacency.get(nodeId) || new Set();
    neighbors.forEach(neighborId => {
      queue.push({
        nodeId: neighborId,
        depth: depth + 1,
        path: [...path, neighborId]
      });
    });
  }

  return results;
}
```

## Build Process

### 1. Source Scanning

```bash
# Epic markdown files
docs/
├── social-medium.md
├── governance.md
└── ...

# Feature Gherkin files
elohim-app/cypress/e2e/features/
├── social-medium/
│   └── reach-earned.feature
├── governance/
│   └── voting.feature
└── ...
```

### 2. Parsing

**Markdown Parser:**
- Extracts front matter (metadata)
- Parses headings into sections
- Identifies `@feature` and `@epic` tags
- Generates epic node

**Gherkin Parser:**
- Parses feature/scenario structure
- Extracts tags (including `@epic` references)
- Generates feature + scenario nodes

### 3. Relationship Building

```typescript
function buildRelationships(graph: DocumentGraph) {
  // Epic → Feature (DESCRIBES)
  graph.nodesByType.epics.forEach(epic => {
    epic.featureIds.forEach(featureId => {
      addRelationship(graph, {
        id: `${epic.id}_${featureId}_describes`,
        type: RelationshipType.DESCRIBES,
        sourceId: epic.id,
        targetId: featureId,
        bidirectional: true
      });
    });
  });

  // Feature → Scenario (CONTAINS)
  graph.nodesByType.features.forEach(feature => {
    feature.scenarioIds.forEach(scenarioId => {
      addRelationship(graph, {
        id: `${feature.id}_${scenarioId}_contains`,
        type: RelationshipType.BELONGS_TO,
        sourceId: scenarioId,
        targetId: feature.id,
        bidirectional: false
      });
    });
  });

  // Scenario → Epic (VALIDATES via tags)
  graph.nodesByType.scenarios.forEach(scenario => {
    scenario.epicIds.forEach(epicId => {
      addRelationship(graph, {
        id: `${scenario.id}_${epicId}_validates`,
        type: RelationshipType.VALIDATES,
        sourceId: scenario.id,
        targetId: epicId,
        bidirectional: false
      });
    });
  });
}
```

### 4. Index Building

All indices built automatically during node addition (see Index Strategy above).

### 5. Export

```typescript
// Generate manifests
writeFile('assets/docs/manifest.json', {
  files: epicFiles.map(f => f.name)
});

writeFile('assets/features/manifest.json', {
  files: featureFiles.map(f => ({
    path: f.path,
    category: f.category
  }))
});

// Copy source files to assets
epicFiles.forEach(f => copy(f, 'assets/docs/'));
featureFiles.forEach(f => copy(f, 'assets/features/'));
```

## Design Principles

### 1. Graph-First Thinking

Always model as nodes and relationships, not nested hierarchies:

**Bad:**
```typescript
interface Epic {
  features: Feature[];  // Nested
}
```

**Good:**
```typescript
interface Epic {
  featureIds: string[];  // References
}
// Separate relationship entities track Epic → Feature edges
```

### 2. Denormalize for Performance

Store redundant data when it improves query speed:

```typescript
interface DocumentNode {
  relatedNodeIds: string[];  // Denormalized from relationships
}
```

Relationships are source of truth, but `relatedNodeIds` provides O(1) access.

### 3. Extensibility Through Metadata

Never modify core interfaces for domain-specific needs:

```typescript
// Bad: Adding domain-specific field
interface EpicNode {
  testCoverage: number;  // Too specific
}

// Good: Using metadata
interface EpicNode {
  metadata: {
    testCoverage?: number;
    // Any future fields
  }
}
```

### 4. Bidirectional Relationships Where Semantic

Epic DESCRIBES Feature semantically implies Feature IMPLEMENTS Epic:

```typescript
createBidirectionalRelationship(
  RelationshipType.DESCRIBES,
  epicId,
  featureId
)
```

But Scenario BELONGS_TO Feature is unidirectional (reverse is just "contains").

### 5. Separate Concerns: Content vs User State

- **Content graph:** Shared, static(ish), versioned
- **User affinity:** Private, dynamic, per-user
- **Attestations:** Verified, portable, cryptographically signed (future)

Never merge these into a single storage model.

## Migration Path

### Phase 1: Current (In-Memory JSON)
✅ Simple deployment
✅ Fast reads
✅ No backend required
❌ No persistence
❌ Limited to ~10k nodes

### Phase 2: Client-Side Storage
⬜ IndexedDB for offline support
⬜ Service Worker caching
⬜ Incremental updates

### Phase 3: Backend + Graph DB
⬜ Neo4j or OrientDB
⬜ GraphQL API
⬜ Real-time sync
⬜ Advanced queries (Cypher)

### Phase 4: Federated Graph
⬜ Multiple graph instances
⬜ Cross-domain relationships
⬜ Decentralized storage (IPFS?)
⬜ Blockchain attestations

## Key Files Reference

All model files located in `elohim-app/src/app/lamad/models/`:

- `document-graph.model.ts` - Graph container, metadata, query interfaces
- `document-node.model.ts` - Base node, node types enum
- `node-relationship.model.ts` - Relationship types, bidirectional helpers
- `epic-node.model.ts` - Epic-specific extensions
- `feature-node.model.ts` - Feature-specific extensions
- `scenario-node.model.ts` - Scenario-specific extensions
- `content-node.model.ts` - Generic future-proof model
- `user-affinity.model.ts` - User engagement tracking
- `attestations.model.ts` - Achievement/credential system (future)

Service:
- `elohim-app/src/app/lamad/services/document-graph.service.ts`

Parsers:
- `elohim-app/src/app/lamad/parsers/markdown-parser.ts`
- `elohim-app/src/app/lamad/parsers/gherkin-parser.ts`

## For Future Agents

When working with this graph model:

1. **Think in nodes and edges, not trees** - Everything is a graph
2. **Index everything you query frequently** - O(1) lookups are free
3. **Keep user state separate** - Content graph is shared
4. **Use metadata for extensions** - Don't modify base schemas
5. **Relationships are entities** - Not just properties
6. **Bidirectional where semantic** - But be explicit
7. **Design for graph DB migration** - Use serializable formats
8. **Test with real data** - 100+ nodes minimum

## Questions?

If unsure about storage decisions, ask:

- **Is this content or user state?** → Separate storage
- **Will this work in a graph database?** → Ensure serializability
- **Can I denormalize for speed?** → Yes, but track source of truth
- **Should this be indexed?** → If you query it frequently
- **Is this relationship semantic?** → Model as first-class edge
- **Can this go in metadata?** → Prefer metadata over schema changes

The graph model is the foundation for all content discovery, learning paths, attestations, and earned access in the Elohim ecosystem. Design with care.
