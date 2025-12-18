# Finish Holochain Zome Implementation

Complete the Holochain zome implementations to replace JSON prototype data.

## Context

- `data/lamad/` contains reference JSON showing data structures for each domain
- `docs/content/elohim-protocol/` contains raw markdown source content
- `holochain/dnas/lamad/zomes/content_store/` has existing zome code
- `elohim-app/src/app/elohim/services/data-loader.service.ts` has the wiring

## Already Wired (Completed)

These are connected to Holochain via `HolochainContentService`:
- `getAgent()` / `getAgentIndex()` - Agent profiles
- `getAgentAttestations()` - Agent credentials/achievements
- `getGraph()` - Content graph via `get_content_graph`
- `getAssessmentIndex()` - Queries content by type='assessment'
- Content, Paths, Steps - Fully migrated

## Priority 1: Create New Entry Types

These need new Holochain entry types:

### Content Attestations
- Reference: `data/lamad/attestations/`
- Different from Agent Attestations - these are trust claims about content
- Create: `ContentAttestation` entry type
- Wire: `DataLoaderService.getAttestations()` (currently returns empty)

### Knowledge Maps
- Reference: `data/lamad/knowledge-maps/`
- Create: `KnowledgeMap` entry type with index queries
- Wire: `DataLoaderService.getKnowledgeMapIndex()` / `getKnowledgeMap()`

### Path Extensions
- Reference: `data/lamad/extensions/`
- Create: `PathExtension` entry type linking to base paths
- Wire: `DataLoaderService.getPathExtensionIndex()` / `getPathExtension()`

### Governance
- Reference: `data/lamad/governance/`
- Create entry types for:
  - `Challenge` - content challenges
  - `Proposal` - governance proposals
  - `Precedent` - decision precedents
  - `Discussion` - threaded discussions
  - `GovernanceState` - entity status tracking
- Wire: All governance methods in DataLoaderService

## Implementation Pattern

1. Read JSON structure from `data/lamad/{domain}/`
2. Create entry type in `holochain/dnas/lamad/zomes/content_store/src/`
3. Add zome functions for CRUD + queries
4. Add method to `HolochainContentService`
5. Wire to `DataLoaderService` replacing the stub
6. Test with seeder data from `docs/content/`
