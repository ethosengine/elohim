# Lamad Implementation Archive

*This document preserves deprecated concepts, deleted components, and lessons learned from the Lamad implementation journey. Historical context for future reference.*

---

## Archived: November 2024

### Deprecated Model Architecture

#### DocumentNode Abstraction (Removed)

**What it was:**
A base class/interface that `EpicNode`, `FeatureNode`, and `ScenarioNode` extended. Attempted to create a unified node type hierarchy for the content graph.

**Files deleted:**
- `models/document-node.model.ts`
- `models/document-node.model.spec.ts`
- `models/document-graph.model.ts`
- `models/document-graph.model.spec.ts`
- `models/epic-node.model.ts`
- `models/epic-node.model.spec.ts`
- `models/feature-node.model.ts`
- `models/feature-node.model.spec.ts`
- `models/scenario-node.model.ts`
- `models/scenario-node.model.spec.ts`
- `models/node-relationship.model.ts`
- `models/node-relationship.model.spec.ts`
- `models/lamad-node-types.ts`
- `adapters/document-node.adapter.ts`
- `adapters/document-node.adapter.spec.ts`

**Why it was removed:**
1. Over-engineered class hierarchy added complexity without value
2. Spec requires a flat `ContentNode` model with `contentType` discriminator
3. Path-centric navigation doesn't need hierarchical epic→feature→scenario drilling
4. Holochain entries are flat by nature; inheritance doesn't translate

**Lessons learned:**
- **Composition over inheritance**: The `ContentNode` with `contentType: string` and `metadata: Record<string, any>` is more flexible than type-specific classes
- **YAGNI**: We built epic/feature/scenario drilling before confirming it was needed; it wasn't
- **Data-first development**: Generate mock data matching interfaces BEFORE building services; validates assumptions early
- **Spec alignment**: Read the spec twice before coding once; our original architecture diverged significantly

---

#### Graph-Explorer Paradigm Components (Removed)

**What they were:**
A set of components implementing a three-pane drill-down UI for exploring content hierarchically (epic → feature → scenario).

**Files deleted:**
- `components/epic-viewer/` (4 files)
- `components/feature-viewer/` (2 files)
- `components/scenario-detail/` (2 files)
- `components/epic-content-panes/` (4 files)
- `components/graph-visualizer/` (2 files) - placeholder "coming soon"
- `components/module-viewer/` (4 files)

**Why they were removed:**
1. Built for deprecated DocumentNode hierarchy
2. Spec requires path-centric navigation, not graph exploration as primary UX
3. Components were never routed in production routes
4. Maintained code that was never used

**Lessons learned:**
- **Route-first development**: If it's not in `lamad.routes.ts`, question whether it should exist
- **Delete early, delete often**: Orphaned code accumulates tech debt; we held onto these for months
- **Feature flags vs dead code**: These weren't feature-flagged, they were just... there
- **User journey focus**: Build for the user flow (path navigation), not the data structure (graph)

---

#### Assessment Model (Removed)

**What it was:**
`models/assessment.model.ts` - Standalone assessment/quiz model not integrated with anything.

**Why it was removed:**
- Never imported by any service or component
- Quiz functionality lives in `quiz-renderer` component with inline types
- Premature abstraction before the mastery system (Phase 7) was designed

**Lessons learned:**
- **Don't abstract prematurely**: Wait until you have 2-3 concrete use cases before creating shared models
- **Check imports**: `grep 'from.*assessment\.model'` returned nothing; dead giveaway

---

### Deprecated Architectural Patterns

#### Eager Graph Loading

**What it was:**
`DocumentGraphService.buildGraph()` loaded ALL content nodes into memory on app init.

**Why it was deprecated:**
- Violated fog-of-war principle (users could inspect network/memory to see locked content)
- Didn't scale beyond ~100 nodes
- Wrong mental model for Holochain (can't "load all entries")

**What replaced it:**
- `DataLoaderService` with lazy `loadContentNode(id)` calls
- `PathService.getPath()` loads path metadata without step content
- `PathService.getStep()` loads individual step content on navigation

**Lessons learned:**
- **Lazy loading is a feature, not an optimization**: It enforces correct data access patterns
- **Backend shapes frontend**: Design data access as if you already have the production backend

---

#### Catch-All Route Pattern

**What it was:**
```typescript
{ path: '**', component: GraphExplorerComponent }
```
Used to handle hierarchical URLs like `/lamad/epic/feature/scenario`.

**Why it was deprecated:**
- Made routes unpredictable
- Couldn't distinguish between path navigation and direct resource access
- Spec requires explicit route patterns

**What replaced it:**
```typescript
{ path: 'path/:pathId/step/:stepIndex', component: PathNavigatorComponent }
{ path: 'resource/:resourceId', component: ContentViewerComponent }
```

**Lessons learned:**
- **Explicit routes are documentation**: Route file should read like a URL spec
- **Avoid dynamic path segments**: `/epic/:epicId/feature/:featureId` creates coupling

---

### Historical Context: Khan Academy Inspiration

#### World of Math Mission (2012-2020)

The original Lamad design was heavily inspired by Khan Academy's "World of Math" knowledge map, which was deprecated in 2020.

**What we took from it:**
- Star/constellation visualization of knowledge nodes
- Color-coded proficiency states (unseen, in-progress, mastered)
- Fog-of-war for undiscovered content
- Gamification through visual progress

**What we learned Khan got wrong (from their deprecation):**
- Graph visualization doesn't scale beyond ~200 nodes
- Users found it overwhelming, not empowering
- Path-based learning had better completion rates
- Mobile killed the "big canvas" visualization paradigm

**Our adaptation:**
- Graph explorer as SECONDARY interface (Phase 5)
- Path navigation as PRIMARY interface (Phase 4)
- Hierarchical lazy loading (Epic → Feature zoom levels)
- "Nodes visible, content gated" principle

---

### Migration Notes

#### From DocumentNode to ContentNode

```typescript
// OLD (deleted)
interface DocumentNode {
  id: string;
  type: NodeType;  // enum: EPIC | FEATURE | SCENARIO
  title: string;
  content: string;
  // ... type-specific fields via inheritance
}

// NEW (current)
interface ContentNode {
  id: string;
  contentType: ContentType;  // string union, extensible
  contentFormat: ContentFormat;  // how to render
  title: string;
  content: string | object;  // flexible payload
  metadata: Record<string, any>;  // extensible
}
```

**Migration path:**
1. Replace `NodeType` enum with `contentType` string
2. Move type-specific fields to `metadata`
3. Update parsers to output `ContentNode`
4. Delete old models and specs

---

### Archived Checklists

These checklists tracked work that is now complete or no longer relevant.

#### Model Cleanup (Complete Nov 2024)
- [x] Delete `document-node.model.ts` and spec
- [x] Delete `document-graph.model.ts` and spec
- [x] Delete `epic-node.model.ts` and spec
- [x] Delete `feature-node.model.ts` and spec
- [x] Delete `scenario-node.model.ts` and spec
- [x] Delete `node-relationship.model.ts` and spec
- [x] Delete `lamad-node-types.ts`
- [x] Update parsers to use inline types
- [x] Fix `content: string | object` handling

#### Component Cleanup (Complete Nov 2024)
- [x] Delete `epic-viewer/`
- [x] Delete `feature-viewer/`
- [x] Delete `scenario-detail/`
- [x] Delete `epic-content-panes/`
- [x] Delete `graph-visualizer/`
- [x] Delete `module-viewer/`
- [x] Update `components/claude.md` documentation

---

## Summary Statistics

| Category | Items Archived | Lessons Documented |
|----------|---------------|-------------------|
| Model files | 14 | 4 |
| Component directories | 6 | 3 |
| Architectural patterns | 2 | 4 |
| Historical context | 1 | 3 |

**Total lines of code removed:** ~3,500 (estimated)
**Tech debt reduced:** Significant - no more orphaned imports or dead code paths

---

*Last updated: November 2024*
