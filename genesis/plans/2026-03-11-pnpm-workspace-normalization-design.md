# pnpm Workspace Normalization

> **Status:** Captured for later. Not yet brainstormed or planned.

**Goal:** Eliminate fragile relative paths between workspace packages by normalizing package names under `@ethosengine/*` and using pnpm `workspace:*` protocol for all cross-project references. Prepares packages for eventual npm publishing.

## Problem

- Several packages use bare names (`holochain-seeder`, `elohim-app`, `doorway-app`) instead of scoped names
- Cross-project references use filesystem paths (e.g. `../../../elohim-app/src/app/generated/`) that break on restructure
- Code generation (`generate-schema-types.ts`) writes directly into another package's source tree
- Only 2 of ~11 workspace packages use `workspace:*` protocol today

## Current State

| Package | Current Name | Proposed Name |
|---------|-------------|---------------|
| app/elohim-app | `elohim-app` | `@ethosengine/elohim-app` |
| app/elohim-library | `elohim-library` | `@ethosengine/elohim-library` |
| app/elohim-library/projects/elohim-service | `@elohim/service` | `@ethosengine/elohim-service` |
| app/elohim-library/projects/lamad-ui | `lamad-ui` | `@ethosengine/lamad-ui` |
| doorway/doorway-app | `doorway-app` | `@ethosengine/doorway-app` |
| genesis/seeder | `holochain-seeder` | `@ethosengine/elohim-seeder` |
| genesis/a2o | `@elohim/a2o` | `@ethosengine/elohim-a2o` |
| genesis/orchestrator | `elohim-orchestrator` | `@ethosengine/elohim-orchestrator` |
| steward/device | `elohim-steward` | `@ethosengine/elohim-steward` |
| elohim/sdk/storage-client-ts | `@elohim/storage-client` | `@ethosengine/storage-client` |
| elohim/elohim-agent/elohim-agent-sdk | `@elohim/agent-sdk` | `@ethosengine/elohim-agent-sdk` |
| elohim/elohim-agent/mcp-servers/elohim-content | `elohim-content-mcp` | `@ethosengine/elohim-content-mcp` |

## Key Changes (sketch)

1. **Rename packages** to `@ethosengine/*` scope in all `package.json` files
2. **Wire `workspace:*` deps** where packages consume each other (replace `file:` and relative path imports)
3. **Move `schema-enums.ts`** generation target into `@ethosengine/elohim-service` (consumed by both seeder and app) instead of seeder writing into app's source tree
4. **Update all import references** in consuming code
5. **Update pnpm-lock.yaml**
6. **Update CI** if any pipeline references package names

## Anti-pattern to fix

`genesis/seeder/src/generate-schema-types.ts` writes directly into `app/elohim-app/src/app/generated/`. Instead, the generated types should live in `@ethosengine/elohim-service` and both seeder and app should consume them via `workspace:*`.

## Notes

- Sophia stays `@ethosengine/sophia-*` / `@khanacademy/*` (separate workspace, git submodule)
- Consider adding `publishConfig` to packages that should eventually be on npm
- This is a devops cleanup, not a feature — no user-facing changes
