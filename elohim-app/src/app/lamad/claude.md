# Lamad Pillar - Learning & Content

Content graph, learning paths, mastery tracking, knowledge maps.

## Architecture

```
Territory (Content) → Journey (Paths) → Traveler (Progress) → Maps (Knowledge)
```

## Models

| Model | Purpose |
|-------|---------|
| `content-node.model.ts` | ContentNode, ContentType, ContentReach |
| `learning-path.model.ts` | LearningPath, PathStep |
| `content-mastery.model.ts` | ContentMastery, mastery progression |
| `knowledge-map.model.ts` | Four map types (domain, self, person, collective) |
| `exploration.model.ts` | Graph traversal queries |
| `search.model.ts` | SearchQuery, SearchResult, facets |

## Services

| Service | Purpose |
|---------|---------|
| `PathService` | Path & step navigation |
| `ContentService` | Content access with reach checking |
| `ContentMasteryService` | Bloom's Taxonomy progression |
| `KnowledgeMapService` | Four-dimensional knowledge maps |
| `ExplorationService` | Graph traversal, pathfinding |
| `SearchService` | Enhanced search with scoring |
| `PathExtensionService` | Learner path customization |

## Routes

```typescript
{ path: '', component: LamadHomeComponent },
{ path: 'path/:pathId', component: PathOverviewComponent },
{ path: 'path/:pathId/step/:stepIndex', component: PathNavigatorComponent },
{ path: 'resource/:resourceId', component: ContentViewerComponent },
{ path: 'explore', component: GraphExplorerComponent },
{ path: 'me', component: ProfilePageComponent },
```

## Key Constraints

1. **Lazy Loading** - Never load "all paths" or "all content"
2. **Fog of War** - Humans access: completed, current, or next step only
3. **Territory vs Journey** - Content is reusable; paths add narrative context

## Content Types

```typescript
type ContentType =
  | 'epic' | 'feature' | 'scenario' | 'concept'
  | 'simulation' | 'video' | 'assessment'
  | 'organization' | 'book-chapter' | 'tool' | 'role';

type ContentFormat =
  | 'markdown' | 'html5-app' | 'video-embed'
  | 'quiz-json' | 'gherkin' | 'html';
```

## Knowledge Map Types

| Type | Question | Inspiration |
|------|----------|-------------|
| Domain | What do I know? | Khan Academy |
| Self | Who am I? | "Know thyself" |
| Person | Who do I know? | Gottman Love Maps |
| Collective | What do we know? | Org knowledge mgmt |

## Barrel Re-exports

`lamad/models/index.ts` and `lamad/services/index.ts` re-export from other pillars for backward compatibility. Prefer direct imports:

```typescript
// Preferred
import { DataLoaderService } from '@app/elohim/services';

// Also works (re-export)
import { DataLoaderService } from '@app/lamad/services';
```

See subdirectory `claude.md` files for component/renderer details.
