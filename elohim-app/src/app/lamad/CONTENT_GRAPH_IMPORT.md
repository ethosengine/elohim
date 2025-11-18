# Content Graph Import - Database Initialization Guide

## Overview

This document describes the structured data extraction and import process for initializing the Lamad learning platform's node graph database from the `/docs` directory.

The goal is to transform the documentation directory structure into a systematic, labeled, and interconnected graph of learning content that supports:

- **Epics**: Major narrative arcs (Autonomous Entity, Governance, Social Medium, etc.)
- **Personas**: User roles within each epic
- **Scenarios**: Concrete use cases and stories (Gherkin .feature files)
- **Reference Materials**: Books, videos, articles, organizations
- **Governance Layers**: Geographic/political and functional layers
- **Suggested Learning Paths**: Curated journeys through the content

## Directory Structure

```
/docs
├── manifesto.md                    # Root document
├── autonomous_entity/              # Epic
│   ├── worker/                     # Persona
│   │   ├── README.md
│   │   └── scenarios/              # Scenarios (.feature files)
│   ├── organizations/              # Reference materials
│   └── ...
├── governance/
│   ├── appellant/
│   ├── books/
│   ├── video/
│   ├── audio/
│   └── organizations/
├── governance_layers/
│   ├── geographic_political/
│   │   ├── individual/
│   │   ├── family/
│   │   ├── community/
│   │   └── ...
│   └── functional/
│       ├── ecological_bioregional/
│       ├── educational/
│       └── ...
├── public_observer/
├── social_medium/
└── value_scanner/
```

## Extraction Process

### Step 1: Run the Extraction Script

```bash
cd elohim-app
node scripts/extract-docs-to-nodes.js
```

This will:
1. Scan all `.md` and `.feature` files in `/docs`
2. Extract metadata (title, description, tags)
3. Determine content type (epic, persona, scenario, etc.)
4. Build relationships between nodes
5. Generate taxonomy structure
6. Create suggested learning paths
7. Output to `src/app/lamad/data/content-nodes.json`

### Step 2: Review Generated Output

The script generates a JSON file with the following structure:

```json
{
  "version": "1.0.0",
  "generatedAt": "2025-11-18T22:51:45.993Z",
  "stats": {
    "totalNodes": 430,
    "byType": {
      "persona": 61,
      "scenario": 260,
      "organization": 91,
      "root-document": 6,
      ...
    },
    "byEpic": {
      "autonomous_entity": 35,
      "governance": 92,
      "value_scanner": 183,
      ...
    }
  },
  "taxonomy": { ... },
  "suggestedPaths": [ ... ],
  "nodes": [ ... ]
}
```

## Data Structure

### ContentNode

Each node in the graph represents a piece of content:

```typescript
interface ContentNode {
  id: string;                    // Unique identifier
  contentType: string;           // 'epic', 'persona', 'scenario', etc.
  title: string;                 // Extracted from content or filename
  description: string;           // First paragraph or summary
  content: string;               // Full markdown or Gherkin content
  contentFormat: string;         // 'markdown' | 'gherkin'
  tags: string[];                // Auto-extracted from path and @tags
  sourcePath: string;            // Relative path from /docs
  relatedNodeIds: string[];      // IDs of related nodes
  category: string;              // Epic name for categorization
  metadata: {
    epic?: string;               // Epic identifier
    persona?: string;            // Persona identifier
    referenceType?: string;      // Type if reference material
    layer?: string;              // Governance layer name
    layerType?: string;          // 'geographic_political' | 'functional'
    priority: number;            // 0-100, higher = more important
    sourcePath: string;          // Relative path
  };
  createdAt: string;             // ISO timestamp
  updatedAt: string;             // ISO timestamp
}
```

### Content Types

| Type | Description | Example |
|------|-------------|---------|
| `root-document` | Top-level documents | manifesto.md, hardware-spec.md |
| `epic` | Major narrative arcs | (Currently represented as personas with epic.md) |
| `persona` | User roles/perspectives | worker, citizen, child |
| `scenario` | Concrete use cases | Democratic workplace governance (.feature) |
| `organization` | Reference organizations | Khan Academy, Polis, etc. |
| `book` | Book references | Finite and Infinite Games |
| `video` | Video references | Climate Town, Not Just Bikes |
| `audio` | Audio/podcast references | Daniel Schmachtenberger talks |
| `article` | Article references | New Public articles |
| `document` | Other documents | Policy papers, specifications |

### Taxonomy Structure

The taxonomy provides navigation and categorization:

```typescript
interface ContentTaxonomy {
  epics: {
    [epicId: string]: {
      name: string;              // "Autonomous Entity"
      count: number;             // Total nodes in epic
      personas: string[];        // Persona identifiers
      contentTypes: string[];    // Types present in epic
    }
  };
  layers: {
    [layerType: string]: {
      [layer: string]: number;   // Node count per layer
    }
  };
  contentTypes: {
    [type: string]: number;      // Total count per type
  };
  personas: {
    [personaId: string]: {
      count: number;             // Total nodes for persona
      epics: string[];           // Epics this persona appears in
    }
  };
  total: number;                 // Total node count
}
```

### Suggested Learning Paths

Pre-curated journeys through the content:

```typescript
interface SuggestedLearningPath {
  id: string;                    // 'understanding-elohim-protocol'
  title: string;                 // 'Understanding the Elohim Protocol'
  description: string;           // Journey description
  targetSubject: string;         // Learning goal
  path: string[];                // Ordered array of node IDs
}
```

Current paths:
- **understanding-elohim-protocol**: Core concepts and vision
- **governance-deep-dive**: Multi-layered governance architecture
- **social-medium-foundations**: Social Medium design principles

## Node Relationships

Relationships are automatically built based on:

1. **Hierarchy**: Scenarios → Persona README → Epic README
2. **Same Persona**: All scenarios within a persona are related
3. **Shared Tags**: Nodes with common tags (limited to avoid over-connection)
4. **Path Membership**: Nodes in the same suggested learning path

Example:
```
manifesto.md (root-document)
  ↓ related to
social_medium/README.md (persona/epic overview)
  ↓ related to
social_medium/activist/README.md (persona)
  ↓ related to
social_medium/activist/scenarios/movement_building.feature (scenario)
```

## Usage Examples

### Loading the Content Graph

```typescript
import { ContentGraphImportHelper } from './models/content-graph-import.model';

// Load the data
const contentGraph = await ContentGraphImportHelper.loadFromJson(
  'assets/data/content-nodes.json'
);

console.log(`Loaded ${contentGraph.stats.totalNodes} nodes`);
```

### Filtering by Epic

```typescript
import { EpicIdentifier } from './models/content-graph-import.model';

// Get all nodes for Social Medium epic
const socialMediumNodes = ContentGraphImportHelper.getNodesForEpic(
  contentGraph,
  EpicIdentifier.SOCIAL_MEDIUM
);

console.log(`Social Medium: ${socialMediumNodes.length} nodes`);
```

### Getting Scenarios for a Persona

```typescript
// Get all scenarios for the 'activist' persona in Social Medium epic
const activistScenarios = ContentGraphImportHelper.getNodesForPersona(
  contentGraph,
  EpicIdentifier.SOCIAL_MEDIUM,
  'activist'
);

console.log(`Activist scenarios: ${activistScenarios.length}`);
```

### Exploring a Learning Path

```typescript
// Get the nodes in the 'Understanding Elohim Protocol' path
const pathNodes = ContentGraphImportHelper.getNodesInPath(
  contentGraph,
  'understanding-elohim-protocol'
);

console.log('Learning path:');
pathNodes.forEach((node, index) => {
  console.log(`${index + 1}. ${node.title} (${node.contentType})`);
});
```

### Searching Content

```typescript
// Search for nodes containing "democratic"
const results = ContentGraphImportHelper.searchNodes(
  contentGraph,
  'democratic'
);

console.log(`Found ${results.length} nodes about democracy`);
```

### Getting Reference Materials

```typescript
// Get all books, videos, articles, etc.
const references = ContentGraphImportHelper.getReferenceMaterials(contentGraph);

const books = references.filter(n => n.contentType === 'book');
const videos = references.filter(n => n.contentType === 'video');

console.log(`References: ${books.length} books, ${videos.length} videos`);
```

### Governance Layer Navigation

```typescript
// Get all content for the 'family' governance layer
const familyNodes = ContentGraphImportHelper.getNodesByLayer(
  contentGraph,
  'geographic_political',
  'family'
);

console.log(`Family layer: ${familyNodes.length} nodes`);
```

### Grouping by Category

```typescript
// Group all nodes by their category (epic name)
const grouped = ContentGraphImportHelper.groupByCategory(contentGraph);

for (const [category, nodes] of Object.entries(grouped)) {
  console.log(`${category}: ${nodes.length} nodes`);
}
```

## Integration with Lamad Platform

### 1. Service Integration

Create a service to load and manage the content graph:

```typescript
// src/app/lamad/services/content-graph.service.ts
import { Injectable } from '@angular/core';
import { BehaviorSubject, Observable } from 'rxjs';
import { ContentGraphImport, ContentGraphImportHelper } from '../models/content-graph-import.model';
import { ContentNode } from '../models/content-node.model';

@Injectable({
  providedIn: 'root'
})
export class ContentGraphService {
  private contentGraph$ = new BehaviorSubject<ContentGraphImport | null>(null);

  async loadContentGraph(jsonPath: string = 'assets/data/content-nodes.json'): Promise<void> {
    const graph = await ContentGraphImportHelper.loadFromJson(jsonPath);
    this.contentGraph$.next(graph);
  }

  getContentGraph(): Observable<ContentGraphImport | null> {
    return this.contentGraph$.asObservable();
  }

  // Convenience methods
  getAllNodes(): ContentNode[] {
    const graph = this.contentGraph$.value;
    return graph ? graph.nodes : [];
  }

  getNodeById(id: string): ContentNode | undefined {
    return this.getAllNodes().find(n => n.id === id);
  }

  // ... add more methods as needed
}
```

### 2. Initialize on App Start

```typescript
// src/app/app.component.ts
export class AppComponent implements OnInit {
  constructor(private contentGraph: ContentGraphService) {}

  async ngOnInit() {
    await this.contentGraph.loadContentGraph();
  }
}
```

### 3. Use in Components

```typescript
// src/app/lamad/components/meaning-map/meaning-map.component.ts
export class MeaningMapComponent implements OnInit {
  nodes: ContentNode[] = [];

  constructor(
    private contentGraph: ContentGraphService,
    private affinity: AffinityTrackingService
  ) {}

  ngOnInit() {
    this.contentGraph.getContentGraph().subscribe(graph => {
      if (graph) {
        this.nodes = ContentGraphImportHelper.sortByPriority(graph.nodes);
        this.enrichWithAffinity();
      }
    });
  }

  enrichWithAffinity() {
    // Add user affinity data to nodes for display
    this.nodes = this.nodes.map(node => ({
      ...node,
      userAffinity: this.affinity.getAffinity(node.id)
    }));
  }
}
```

## Current Statistics

Based on the latest extraction (2025-11-18):

### Total Nodes: 430

### By Type:
- **Scenarios**: 260 (60.5%)
- **Organizations**: 91 (21.2%)
- **Personas**: 61 (14.2%)
- **Root Documents**: 6 (1.4%)
- **Books**: 4
- **Videos**: 4
- **Audio**: 2
- **Articles**: 1
- **Documents**: 1

### By Epic:
- **Value Scanner**: 183 nodes (42.6%)
- **Governance**: 92 nodes (21.4%)
- **Public Observer**: 72 nodes (16.7%)
- **Autonomous Entity**: 35 nodes (8.1%)
- **Social Medium**: 34 nodes (7.9%)
- **Governance Layers**: 8 nodes (1.9%)
- **Root**: 6 nodes (1.4%)

## Extending the System

### Adding New Content Types

1. Add to `NODE_TYPES` in `extract-docs-to-nodes.js`
2. Add to `ContentNodeType` enum in `content-graph-import.model.ts`
3. Update directory recognition in `REFERENCE_TYPES`
4. Re-run extraction script

### Customizing Relationships

Edit the `buildRelationships()` function in `extract-docs-to-nodes.js` to add custom relationship logic.

### Adding More Learning Paths

Edit the `generateSuggestedPaths()` function to create additional curated journeys.

### Metadata Extensions

Add custom metadata fields in the `extractMetadata()` function. The `metadata` object supports arbitrary key-value pairs.

## Future Enhancements

### Planned Features

1. **Attestation Requirements**: Add required attestations to access certain nodes
2. **Difficulty Levels**: Classify nodes by complexity/maturity requirements
3. **Time Estimates**: Add estimated reading/completion times
4. **Prerequisites**: Explicit prerequisite relationships
5. **Dynamic Paths**: Generate personalized paths based on user affinity and attestations
6. **Orientation Scoring**: Calculate orientation values toward target subjects
7. **Content Updates**: Track which nodes have been updated since last read
8. **Multi-language Support**: Extract and link translated content
9. **Media Integration**: Extract and link embedded images, videos, diagrams
10. **Interactive Exercises**: Parse and structure practice/exercise nodes

### Attestation Integration

Future structure for locked content:

```typescript
interface ContentAccessRequirement {
  nodeId: string;
  requirements: {
    attestations?: string[];        // Required achievement badges
    affinity?: {                    // Required affinity with other nodes
      nodeId: string;
      minLevel: number;             // 0.0 - 1.0
    }[];
    community?: {                   // Community endorsements
      count: number;
      fromAttestation?: string;     // Endorsers must have this attestation
    };
    timeGated?: {                   // Must wait X days after joining
      days: number;
    };
  };
  steward?: string;                 // Who manages access
  revocable?: boolean;              // Can access be revoked?
}
```

### Orientation Calculation

Future algorithm for guiding users toward target subjects:

```typescript
interface OrientationScore {
  nodeId: string;
  targetSubject: string;
  score: number;                    // 0.0 - 1.0
  factors: {
    inSuggestedPath: boolean;       // Is this node in the curated path?
    pathDistance: number;           // How many steps to target?
    affinityGap: number;            // User's current affinity gap
    attestationsNeeded: string[];   // Missing attestations
    relatedAffinityStrength: number; // Average affinity with related nodes
  };
}
```

## Troubleshooting

### Script Fails to Run

**Issue**: `Cannot find module 'fs'`
**Solution**: Ensure you're running with Node.js, not in browser

**Issue**: `ENOENT: no such file or directory`
**Solution**: Check that `/docs` directory exists and paths are correct

### Missing Nodes

**Issue**: Some files aren't being extracted
**Solution**: Check file extensions (must be `.md` or `.feature`)

### Incorrect Relationships

**Issue**: Nodes aren't related as expected
**Solution**: Review `buildRelationships()` logic and ensure README files exist

### Large File Size

**Issue**: `content-nodes.json` is too large (>10MB)
**Solution**: Consider splitting into multiple files by epic or using database storage

## References

- **Lamad Vision**: `elohim-app/src/app/lamad/claude.md`
- **Content Node Model**: `elohim-app/src/app/lamad/models/content-node.model.ts`
- **Extraction Script**: `elohim-app/scripts/extract-docs-to-nodes.js`
- **TypeScript Interfaces**: `elohim-app/src/app/lamad/models/content-graph-import.model.ts`

## Questions?

Refer back to the Lamad vision principles:
- **Would this work for non-software documentation?** (WordPress test)
- **Does this impose domain-specific semantics?** (Generic test)
- **Does this improve the reading experience?** (UX test)
- **Can this be added through metadata?** (Extension test)
- **Does it help the user progress toward their target subject?** (Orientation test)

Remember: **Affinity deepens, attestations prove, orientation guides**
