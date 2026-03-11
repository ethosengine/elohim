# Lamad Services

Service layer for content, paths, and learning.

## Services

| Service | Purpose |
|---------|---------|
| `PathService` | Path & step navigation |
| `ContentService` | Content access with reach checking, back-links |
| `ContentMasteryService` | Bloom's Taxonomy mastery tracking |
| `ExplorationService` | Graph BFS traversal, pathfinding |
| `KnowledgeMapService` | Four-dimensional knowledge maps |
| `SearchService` | Enhanced search with scoring, facets |
| `PathExtensionService` | Learner path customization |
| `AssessmentService` | Psychometric instrument handling |
| `PathNegotiationService` | Collaborative path creation |

## Key Constraints

1. **Lazy Loading** - NEVER create `getAllPaths()` or `getAllContent()`
2. **DataLoaderService** - Only service that knows about data source
3. **Observable patterns** - Use `shareReplay(1)`, `switchMap`, `forkJoin`

## Code Quality

```typescript
// Use ?? not || for nullish defaults
const id = value ?? 'default';

// Mark dependencies readonly
constructor(private readonly dataLoader: DataLoaderService) {}

// Remove unused imports
// Use .substring() not .substr()
```

## Barrel Exports

`index.ts` re-exports from other pillars (`@app/elohim`, `@app/imagodei`, `@app/qahal`).
Prefer direct imports when possible.
