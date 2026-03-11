# pnpm Workspace Normalization

> **Status:** Captured for later. Not yet brainstormed or planned.

**Goal:** Eliminate fragile relative paths between workspace packages by normalizing package names under `@ethosengine/*` and using pnpm `workspace:*` protocol for all cross-project references. Prepares packages for eventual npm publishing.

## Problem

- Several packages use bare names (`holochain-seeder`, `elohim-app`, `doorway-app`) instead of scoped names
- Cross-project references use filesystem paths (e.g. `../../../elohim-app/src/app/generated/`) that break on restructure
- Code generation (`generate-schema-types.ts`) writes directly into another package's source tree
- Only 2 of ~11 workspace packages use `workspace:*` protocol today
- Sophia and elohim-ui-playground stuck on older version of storybook, because newer versions aren't published to the npm registry. (developer note: if we established our own npm sonatype registry (community edition), could we self-host, and provide artifacts for github site deployments and our own build resiliency in dev and jenkins)

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

## Submodule CI/CD Problem

Jenkins cannot read Jenkinsfiles from inside git submodules — the GitHub Contents API returns a gitlink object instead of directory contents, and Jenkins' lightweight checkout (used to read the Jenkinsfile before the build starts) doesn't initialize submodules. This forces us to maintain duplicate "shim" Jenkinsfiles at root level outside the submodule boundary, disconnected from the code they build.

If sophia were published to a registry (npm or self-hosted Sonatype), the elohim-app pipeline could consume it as a versioned dependency instead of building it inline from source via submodule. The sophia repo would have its own standalone CI (GitHub Actions already does this for storybook). The parent repo would just pull the artifact. This eliminates:
- The Jenkinsfile-in-submodule discovery problem entirely
- The pnpm workspace boundary violations (sophia would be a registry dep, not a workspace member)
- The UMD bundle copy step in the app Jenkinsfile
- The submodule pointer as a fragile coupling mechanism

This pattern extends to any future submodules or extractable packages — publish to registry, consume as dependency, build independently.

## Anti-pattern to fix

`genesis/seeder/src/generate-schema-types.ts` writes directly into `app/elohim-app/src/app/generated/`. Instead, the generated types should live in `@ethosengine/elohim-service` and both seeder and app should consume them via `workspace:*`.

## Notes

- Sophia stays `@ethosengine/sophia-*` / `@khanacademy/*` (separate workspace, git submodule)
- Consider adding `publishConfig` to packages that should eventually be on npm
- This is a devops cleanup, not a feature — no user-facing changes


