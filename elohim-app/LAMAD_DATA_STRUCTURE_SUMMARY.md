# Lamad Content Graph - Data Structure Summary

## Overview

Successfully extracted and structured **430 content nodes** from the `/docs` directory into a systematic node graph database ready for import into the Lamad learning platform.

## What Was Created

### 1. Extraction Script
**Location**: `scripts/extract-docs-to-nodes.js`

A Node.js script that:
- Scans all `.md` and `.feature` files in `/docs`
- Extracts titles, descriptions, and metadata
- Determines content types (epic, persona, scenario, etc.)
- Builds automatic relationships between nodes
- Generates taxonomy and navigation structure
- Creates suggested learning paths
- Outputs structured JSON for database init

### 2. Generated Data File
**Location**: `src/app/lamad/data/content-nodes.json` (4.2 MB)

Complete content graph with:
- 430 nodes with full content, metadata, and relationships
- 6 epics (major narrative arcs)
- 61 personas (user roles/perspectives)
- 260 scenarios (concrete use cases)
- 102 reference materials (books, videos, organizations, etc.)
- 3 suggested learning paths
- Complete taxonomy for navigation

### 3. TypeScript Interfaces
**Location**: `src/app/lamad/models/content-graph-import.model.ts`

Type-safe interfaces including:
- `ContentGraphImport` - Main data structure
- `ContentGraphStats` - Statistics
- `ContentTaxonomy` - Navigation structure
- `SuggestedLearningPath` - Curated journeys
- `ContentGraphImportHelper` - 20+ utility methods

### 4. Documentation
**Location**: `src/app/lamad/CONTENT_GRAPH_IMPORT.md`

Comprehensive guide covering:
- Data structure details
- Extraction process
- Usage examples
- Integration instructions
- Extension guidelines
- Troubleshooting

## Data Structure Hierarchy

```
Content Graph
├── Epics (6)
│   ├── Autonomous Entity (35 nodes)
│   │   ├── Personas: worker, manager, customer, etc. (8)
│   │   └── Scenarios + Organizations
│   ├── Governance (92 nodes)
│   │   ├── Personas: appellant, researcher, policy_maker, etc. (7)
│   │   └── Scenarios + Books + Videos + Organizations
│   ├── Governance Layers (8 nodes)
│   │   ├── Geographic/Political: individual → global (11 layers)
│   │   └── Functional: ecological, educational, workplace, etc. (6 layers)
│   ├── Public Observer (72 nodes)
│   │   ├── Personas: journalist, citizen, activist, etc. (10)
│   │   └── Scenarios + Organizations
│   ├── Social Medium (34 nodes)
│   │   ├── Personas: child, elder, activist, content_creator (7)
│   │   └── Scenarios + Organizations
│   └── Value Scanner (183 nodes)
│       ├── Personas: child, teen, adult, elderly, etc. (20)
│       └── Scenarios + Organizations
│
├── Root Documents (6)
│   ├── manifesto.md - Core vision
│   ├── hardware-spec.md - Technical specifications
│   ├── governance-layers-architecture.md
│   ├── observer-protocol.md
│   ├── global-orchestra.md
│   └── ORGANIZATIONS_DECOMPOSITION.md
│
└── Suggested Learning Paths (3)
    ├── Understanding the Elohim Protocol
    ├── Governance Deep Dive
    └── Social Medium Foundations
```

## Statistics

### Content Distribution

| Type | Count | Percentage |
|------|-------|------------|
| Scenarios | 260 | 60.5% |
| Organizations | 91 | 21.2% |
| Personas | 61 | 14.2% |
| Root Documents | 6 | 1.4% |
| Books | 4 | 0.9% |
| Videos | 4 | 0.9% |
| Audio | 2 | 0.5% |
| Articles | 1 | 0.2% |
| Documents | 1 | 0.2% |

### Epic Distribution

| Epic | Nodes | Percentage |
|------|-------|------------|
| Value Scanner | 183 | 42.6% |
| Governance | 92 | 21.4% |
| Public Observer | 72 | 16.7% |
| Autonomous Entity | 35 | 8.1% |
| Social Medium | 34 | 7.9% |
| Governance Layers | 8 | 1.9% |
| Root | 6 | 1.4% |

## Node Relationships

Each node includes:
- **Hierarchical relationships**: Child → Parent (scenario → persona → epic)
- **Sibling relationships**: Other scenarios in same persona
- **Tag-based relationships**: Nodes sharing common tags
- **Path relationships**: Nodes in same learning path

Example relationship chain:
```
manifesto.md (root-document)
  ↓ related to
social_medium/README.md (epic overview)
  ↓ related to
social_medium/activist/README.md (persona)
  ↓ related to
social_medium/activist/scenarios/movement_building.feature (scenario)
```

## Sample Nodes

### Root Document Example
```json
{
  "id": "manifesto-abc123",
  "contentType": "root-document",
  "title": "Elohim Protocol - Digital Infrastructure for Human Flourishing",
  "description": "A manifesto for love-centered technology...",
  "contentFormat": "markdown",
  "category": "General",
  "tags": [],
  "metadata": {
    "priority": 100,
    "sourcePath": "manifesto.md"
  },
  "relatedNodeIds": ["social-medium-readme-xyz", "governance-readme-xyz"]
}
```

### Persona Example
```json
{
  "id": "social-medium-activist-def456",
  "contentType": "persona",
  "title": "Activist - Social Medium",
  "description": "Movement building and organizing...",
  "contentFormat": "markdown",
  "category": "Social Medium",
  "tags": ["social_medium", "activist"],
  "metadata": {
    "epic": "social_medium",
    "persona": "activist",
    "priority": 70
  },
  "relatedNodeIds": [...]
}
```

### Scenario Example
```json
{
  "id": "movement-building-affinity-networks-ghi789",
  "contentType": "scenario",
  "title": "Movement Building Through Affinity Networks for Activist",
  "description": "As an activist, I want to build movements...",
  "contentFormat": "gherkin",
  "category": "Social Medium",
  "tags": ["social_medium", "activist", "movement", "organizing"],
  "metadata": {
    "epic": "social_medium",
    "persona": "activist",
    "priority": 50
  },
  "relatedNodeIds": [...]
}
```

### Organization Reference Example
```json
{
  "id": "khan-academy-jkl012",
  "contentType": "organization",
  "title": "Khan Academy - Free Online Courses, Lessons, Practice",
  "description": "Educational organization providing...",
  "contentFormat": "markdown",
  "category": "Governance",
  "tags": ["governance", "organizations"],
  "metadata": {
    "epic": "governance",
    "referenceType": "organization",
    "priority": 40
  },
  "relatedNodeIds": [...]
}
```

## Suggested Learning Paths

### 1. Understanding the Elohim Protocol
**Target**: Core concepts and vision  
**Path**: manifesto.md → social_medium/README.md → governance/README.md → value_scanner/README.md → ...

### 2. Governance Deep Dive
**Target**: Multi-layered governance architecture  
**Path**: governance/README.md → governance_layers nodes → specific layer scenarios → ...

### 3. Social Medium Foundations
**Target**: Social Medium design principles  
**Path**: social_medium/README.md → personas → key scenarios → ...

## Taxonomy Structure

```json
{
  "epics": {
    "social_medium": {
      "name": "Social Medium",
      "count": 34,
      "personas": ["activist", "child", "elder", ...],
      "contentTypes": ["persona", "scenario", "organization"]
    },
    ...
  },
  "layers": {
    "geographic_political": {
      "individual": 3,
      "family": 5,
      "community": 7,
      ...
    },
    "functional": {
      "ecological_bioregional": 5,
      "educational": 6,
      ...
    }
  },
  "contentTypes": {
    "scenario": 260,
    "organization": 91,
    "persona": 61,
    ...
  },
  "personas": {
    "activist": {
      "count": 12,
      "epics": ["social_medium", "public_observer"]
    },
    ...
  }
}
```

## How to Use

### 1. Load the Data
```typescript
import { ContentGraphImportHelper } from './models/content-graph-import.model';

const graph = await ContentGraphImportHelper.loadFromJson(
  'assets/data/content-nodes.json'
);
```

### 2. Query by Epic
```typescript
const socialMediumNodes = ContentGraphImportHelper.getNodesForEpic(
  graph,
  EpicIdentifier.SOCIAL_MEDIUM
);
```

### 3. Navigate a Path
```typescript
const pathNodes = ContentGraphImportHelper.getNodesInPath(
  graph,
  'understanding-elohim-protocol'
);
```

### 4. Search
```typescript
const results = ContentGraphImportHelper.searchNodes(graph, 'democratic');
```

### 5. Get Related Content
```typescript
const related = ContentGraphImportHelper.getRelatedNodes(graph, nodeId);
```

## Next Steps for Database Integration

1. **Load on App Init**: Import JSON in Angular app initialization
2. **Enrich with Affinity**: Combine with user affinity tracking data
3. **Build UI Components**: Display in Meaning Map, Content Viewer
4. **Add Search**: Full-text search across nodes
5. **Implement Paths**: Display and track suggested learning paths
6. **Add Attestations**: Layer on access requirements and achievements
7. **Calculate Orientation**: Determine next best nodes toward target subjects

## Regeneration

To update the data after modifying `/docs`:

```bash
cd elohim-app
node scripts/extract-docs-to-nodes.js
```

This will:
- Re-scan all documentation
- Preserve node IDs (deterministic based on file path)
- Update content, relationships, and metadata
- Regenerate `content-nodes.json`

## File Locations

```
elohim-app/
├── scripts/
│   └── extract-docs-to-nodes.js          # Extraction script
├── src/app/lamad/
│   ├── models/
│   │   └── content-graph-import.model.ts # TypeScript interfaces
│   ├── data/
│   │   ├── content-nodes.json            # Generated data (4.2 MB)
│   │   └── README.md                     # Data directory README
│   ├── CONTENT_GRAPH_IMPORT.md           # Full documentation
│   └── claude.md                         # Lamad vision document
└── LAMAD_DATA_STRUCTURE_SUMMARY.md       # This file
```

## Key Design Principles

From the Lamad vision (claude.md):

1. **Domain-agnostic**: Generic ContentNode model works for any content type
2. **Scalar affinity**: 0.0-1.0 relationship strength (not discrete states)
3. **Reading first**: Excellent reading experience before graph visualization
4. **Extensible metadata**: Add domain-specific fields without code changes
5. **Progressive revelation**: Support for attestation-gated content (future)
6. **Orientation-based**: Guide users toward target subjects (future)

## Future Enhancements

### Planned
- [ ] Attestation requirements for locked content
- [ ] Difficulty level classification
- [ ] Time estimates for completion
- [ ] Explicit prerequisite relationships
- [ ] Dynamic path generation based on user affinity
- [ ] Orientation scoring toward target subjects
- [ ] Multi-language support
- [ ] Media extraction (images, embedded videos)
- [ ] Interactive exercise parsing

### Attestation Example (Future)
```json
{
  "nodeId": "advanced-governance-xyz",
  "accessRequirements": {
    "attestations": ["civic-engagement-level-2"],
    "affinity": [
      { "nodeId": "manifesto-abc", "minLevel": 0.5 }
    ]
  },
  "fogOfWar": {
    "visibleTitle": true,
    "visibleDescription": false,
    "unlockJourney": ["basic-governance-path"]
  }
}
```

## Success Metrics

✅ **430 nodes** systematically extracted and labeled  
✅ **100% of `/docs` directory** processed  
✅ **Hierarchical relationships** automatically built  
✅ **Tag-based connections** created  
✅ **3 learning paths** curated  
✅ **Complete taxonomy** for navigation  
✅ **Type-safe interfaces** for TypeScript integration  
✅ **20+ helper methods** for common queries  
✅ **Comprehensive documentation** for usage and extension  
✅ **Deterministic IDs** for stable references  
✅ **4.2 MB JSON file** ready for web delivery  

## Summary

The `/docs` directory has been successfully transformed into a structured, labeled, interconnected node graph ready for systematic import into the Lamad learning platform. The data is:

- **Organized**: Clear hierarchy from epics → personas → scenarios
- **Connected**: Automatic relationship building
- **Navigable**: Complete taxonomy and suggested paths
- **Extensible**: Generic model supports any content type
- **Type-safe**: Full TypeScript interfaces
- **Well-documented**: Comprehensive guides and examples
- **Ready to import**: Single JSON file with all content and metadata

This foundation enables the Lamad platform to provide a domain-agnostic, affinity-driven, orientation-based learning experience where users can explore the Elohim Protocol through curated paths while building their own unique journey.
