# Elohim Protocol - App Architecture

Angular 18+ standalone app implementing the Elohim Protocol for human-centered learning.

## Pillar Structure

```
src/app/
├── elohim/     # Protocol core - data, agents, trust
├── imagodei/   # Identity - session, profile, attestations
├── lamad/      # Learning - content, paths, mastery, maps
├── qahal/      # Community - consent, governance, affinity
├── shefa/      # Economy - REA, recognition, value flows
└── components/ # Shared app components (home, not-found)
```

## Routes

| Path | Pillar | Component |
|------|--------|-----------|
| `/` | app | HomeComponent |
| `/lamad/*` | lamad | Learning content and paths |
| `/community/*` | qahal | Community and governance |
| `/shefa/*` | shefa | Economy (placeholder) |

## Key Concepts

- **Human** not "user" - dignity-first terminology
- **Contributor** not "creator" - for content authors
- **Presence** - placeholder identity for external contributors
- **Attestation** - earned credential/capability proof
- **Reach** - content visibility scope (private → commons)
- **Mastery** - Bloom's Taxonomy progression (not_started → create)

## TypeScript Path Aliases

```json
"@app/*": ["src/app/*"]
"@app/elohim/*": ["src/app/elohim/*"]
"@app/imagodei/*": ["src/app/imagodei/*"]
"@app/lamad/*": ["src/app/lamad/*"]
"@app/qahal/*": ["src/app/qahal/*"]
"@app/shefa/*": ["src/app/shefa/*"]
```

## Import Patterns

```typescript
// Direct pillar imports (preferred)
import { DataLoaderService } from '@app/elohim/services';
import { SessionHumanService } from '@app/imagodei/services';
import { ContentNode } from '@app/lamad/models';

// Barrel re-exports (backward compatible)
import { ContentNode, PathService } from '@app/lamad/models';
```

## Code Quality

- SonarQube compliance required
- Use `??` not `||` for nullish defaults
- Mark constructor dependencies `readonly`
- Remove unused imports
- Use `.substring()` not `.substr()`

See pillar-specific `claude.md` files for domain details.
