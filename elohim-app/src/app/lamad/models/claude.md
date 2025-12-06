# Lamad Models

Data models for content, paths, mastery, and maps.

## Files

| File | Key Types |
|------|-----------|
| `content-node.model.ts` | ContentNode, ContentType, ContentFormat, ContentReach |
| `learning-path.model.ts` | LearningPath, PathStep, PathStepView |
| `content-mastery.model.ts` | ContentMastery, mastery helpers |
| `knowledge-map.model.ts` | KnowledgeMap (domain, self, person, collective) |
| `exploration.model.ts` | ExplorationQuery, ExplorationResult |
| `search.model.ts` | SearchQuery, SearchResult, SearchFacets |
| `path-extension.model.ts` | Learner path mutations |
| `content-access.model.ts` | AccessLevel, ContentAccessMetadata |
| `feedback-profile.model.ts` | FeedbackProfile, FeedbackMechanism |

## Conventions

- All timestamps use ISO 8601 strings (not Date objects)
- Use "human" not "user"
- Use "contributor" not "creator"

## Attestation Models (3 distinct types)

| Model | Purpose |
|-------|---------|
| Agent Attestations (`@app/imagodei`) | Credentials earned BY humans |
| Content Attestations | Trust granted TO content |
| Content Access | Access tier requirements |

## Barrel Exports

`index.ts` re-exports from other pillars for backward compatibility.
Prefer direct imports from pillar when possible.
