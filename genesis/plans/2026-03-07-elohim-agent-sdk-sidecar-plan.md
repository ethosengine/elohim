# Elohim Agent SDK Sidecar Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Wire real Claude Haiku intelligence through the existing Elohim presence pipeline via a TypeScript Agent SDK sidecar, with a 10-call budget cap and one demo feature (discovery completion insight).

**Architecture:** New TypeScript service (`elohim/elohim-agent-sdk/`) uses `@anthropic-ai/sdk` to call Claude Haiku with constitutional prompts. Doorway proxies `/api/v1/elohim/invoke` to it. Angular's existing `NativeBackend` connects automatically once the route exists.

**Tech Stack:** TypeScript, Fastify, `@anthropic-ai/sdk`, Rust (doorway route addition)

---

### Task 1: Scaffold the sidecar package

**Files:**
- Create: `elohim/elohim-agent-sdk/package.json`
- Create: `elohim/elohim-agent-sdk/tsconfig.json`
- Modify: `pnpm-workspace.yaml`

**Step 1: Create package.json**

```json
{
  "name": "@elohim/agent-sdk",
  "version": "0.1.0",
  "private": true,
  "type": "module",
  "main": "dist/server.js",
  "scripts": {
    "build": "tsc",
    "start": "node dist/server.js",
    "dev": "tsx watch src/server.ts"
  },
  "dependencies": {
    "@anthropic-ai/sdk": "^0.39.0",
    "fastify": "^5.2.0"
  },
  "devDependencies": {
    "tsx": "^4.19.0",
    "typescript": "^5.7.0",
    "@types/node": "^22.0.0",
    "vitest": "^3.0.0"
  }
}
```

**Step 2: Create tsconfig.json**

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "NodeNext",
    "moduleResolution": "NodeNext",
    "outDir": "dist",
    "rootDir": "src",
    "strict": true,
    "esModuleInterop": true,
    "declaration": true,
    "skipLibCheck": true,
    "resolveJsonModule": true
  },
  "include": ["src"],
  "exclude": ["node_modules", "dist"]
}
```

**Step 3: Add to pnpm workspace**

Add `elohim/elohim-agent-sdk` to `pnpm-workspace.yaml`.

**Step 4: Install dependencies**

Run: `cd /projects/elohim && pnpm install`
Expected: Dependencies installed, no errors.

**Step 5: Commit**

```bash
git add elohim/elohim-agent-sdk/package.json elohim/elohim-agent-sdk/tsconfig.json pnpm-workspace.yaml pnpm-lock.yaml
git commit -m "feat(elohim-agent-sdk): scaffold sidecar package"
```

---

### Task 2: Implement types (request/response matching Angular models)

**Files:**
- Create: `elohim/elohim-agent-sdk/src/types.ts`

**Step 1: Write the types file**

The sidecar receives requests from the Angular `NativeBackend` (see `elohim-app/src/app/elohim/services/backends/native-backend.ts:56-66`) and must return the TypeScript `ElohimResponse` shape (see `elohim-app/src/app/elohim/models/elohim-agent.model.ts:330-357`).

The NativeBackend sends:
```typescript
{
  requestId: string,
  elohimId: string,
  capability: ElohimCapability,  // kebab-case string
  params: any,
  requesterId: string,
  priority: string,
  credentials?: { apiKey: string, backendId: string }
}
```

The Angular frontend expects `ElohimResponse`:
```typescript
{
  requestId: string,
  elohimId: string,
  status: 'fulfilled' | 'declined' | 'deferred' | 'escalated',
  constitutionalReasoning: {
    primaryPrinciple: string,
    interpretation: string,
    valuesWeighed: { value: string, weight: number, direction: 'for' | 'against' }[],
    confidence: number,
    precedents?: string[],
    newPrecedent?: boolean,
  },
  payload?: any,
  declineReason?: string,
  respondedAt: string,
  cost?: {
    tokensProcessed: number,
    timeMs: number,
    constitutionalChecks: number,
    precedentLookups: number,
  }
}
```

Create `src/types.ts` with these interfaces plus the `InvokeRequest` (what doorway forwards). Include `ElohimCapability` as a union type string matching the Angular model.

**Step 2: Commit**

```bash
git add elohim/elohim-agent-sdk/src/types.ts
git commit -m "feat(elohim-agent-sdk): add request/response type definitions"
```

---

### Task 3: Implement constitutional prompt assembly

**Files:**
- Create: `elohim/elohim-agent-sdk/src/constitutional.ts`
- Reference: `elohim/constitution/src/prompt.rs` (lines 18-72)

**Step 1: Port PromptAssembler from Rust to TypeScript**

Port the logic from `elohim/constitution/src/prompt.rs:18-72`. The system prompt has four sections:

1. `# CONSTITUTIONAL CONTEXT` — header establishing role
2. `## ACTIVE PRINCIPLES` — top 15 principles ordered by weight, with layer tags
3. `## INVIOLABLE BOUNDARIES` — with enforcement markers (`[HARD BLOCK]`, `[REQUIRES GOVERNANCE]`, `[SOFT LIMIT]`, `[WARNING]`)
4. `## INTERPRETIVE GUIDANCE` — 4 rules for decision-making
5. Stack hash for audit

Since we don't have DHT verification in the sidecar, hardcode the default principles from `elohim/constitution/src/layers/` (global: Human Dignity, Ecological Integrity, etc.). This is the same as `ConstitutionalStack::build_defaults()`.

Also add a capability-specific user prompt builder matching `elohim/elohim-agent/src/service.rs:352-379`. The prompt includes:
- Capability name and description
- Any content/contentId/query from params
- Instruction to respond in JSON matching the `ElohimResponse` shape

Add the JSON output instruction that tells Claude to return a JSON object with `constitutionalReasoning` and `payload` fields.

**Step 2: Write a test for prompt assembly**

Create `elohim/elohim-agent-sdk/src/constitutional.test.ts`:
- Test that `buildSystemPrompt()` returns a string containing "CONSTITUTIONAL CONTEXT", "ACTIVE PRINCIPLES", "INVIOLABLE BOUNDARIES"
- Test that `buildUserPrompt(capability, params)` includes the capability name

**Step 3: Run the test**

Run: `cd elohim/elohim-agent-sdk && pnpm exec vitest run src/constitutional.test.ts`
Expected: PASS

**Step 4: Commit**

```bash
git add elohim/elohim-agent-sdk/src/constitutional.ts elohim/elohim-agent-sdk/src/constitutional.test.ts
git commit -m "feat(elohim-agent-sdk): port constitutional prompt assembly from Rust"
```

---

### Task 4: Implement the invoke handler with budget enforcement

**Files:**
- Create: `elohim/elohim-agent-sdk/src/invoke.ts`

**Step 1: Write the failing test**

Create `elohim/elohim-agent-sdk/src/invoke.test.ts`:

Tests:
1. `budgetEnforcer starts at configured limit` — creates a BudgetEnforcer(10), checks remaining is 10
2. `budgetEnforcer decrements on use` — consume(), remaining is 9
3. `budgetEnforcer rejects after exhaustion` — consume 10 times, 11th returns false
4. `buildResponse constructs valid ElohimResponse` — verify response shape matches Angular model

**Step 2: Run test to verify it fails**

Run: `cd elohim/elohim-agent-sdk && pnpm exec vitest run src/invoke.test.ts`
Expected: FAIL (module not found)

**Step 3: Implement the invoke handler**

Create `src/invoke.ts` with:

- `BudgetEnforcer` class: in-memory counter, configurable limit (default 10 from `ELOHIM_BUDGET_LIMIT` env var), `consume()` returns boolean, `remaining()` getter
- `handleInvoke(request, sdk, budgetEnforcer)` async function:
  1. Check budget → if exhausted, return 429 with declined response
  2. Build system prompt via `buildSystemPrompt()`
  3. Build user prompt via `buildUserPrompt(request.capability, request.params)`
  4. Call `sdk.messages.create()` with model `claude-haiku-4-5-20251001`, system prompt, user message, max_tokens 2048
  5. Parse Claude's response text
  6. Build and return `ElohimResponse` with:
     - `status: 'fulfilled'`
     - `constitutionalReasoning` parsed from Claude's JSON output (or default if parsing fails)
     - `cost` with real token counts from `response.usage`
     - `respondedAt` as ISO string

**Step 4: Run test to verify it passes**

Run: `cd elohim/elohim-agent-sdk && pnpm exec vitest run src/invoke.test.ts`
Expected: PASS

**Step 5: Commit**

```bash
git add elohim/elohim-agent-sdk/src/invoke.ts elohim/elohim-agent-sdk/src/invoke.test.ts
git commit -m "feat(elohim-agent-sdk): invoke handler with budget enforcement"
```

---

### Task 5: Implement the Fastify HTTP server

**Files:**
- Create: `elohim/elohim-agent-sdk/src/server.ts`

**Step 1: Implement the server**

Create `src/server.ts`:

```typescript
import Fastify from 'fastify';
import Anthropic from '@anthropic-ai/sdk';
import { BudgetEnforcer, handleInvoke } from './invoke.js';
import type { InvokeRequest } from './types.js';

const PORT = parseInt(process.env.ELOHIM_AGENT_PORT || '8095');
const API_KEY = process.env.ANTHROPIC_API_KEY;
const BUDGET_LIMIT = parseInt(process.env.ELOHIM_BUDGET_LIMIT || '10');
const MODEL = 'claude-haiku-4-5-20251001';

if (!API_KEY) {
  console.error('ANTHROPIC_API_KEY is required');
  process.exit(1);
}

const app = Fastify({ logger: true });
const sdk = new Anthropic({ apiKey: API_KEY });
const budget = new BudgetEnforcer(BUDGET_LIMIT);

// Health endpoint
app.get('/health', async () => ({
  status: 'ok',
  budgetRemaining: budget.remaining(),
  model: MODEL,
}));

// Invoke endpoint
app.post<{ Body: InvokeRequest }>('/invoke', async (request, reply) => {
  const result = await handleInvoke(request.body, sdk, budget);
  reply.header('X-Elohim-Budget-Remaining', budget.remaining().toString());
  if (result.status === 'declined' && result.declineReason?.includes('Budget exhausted')) {
    reply.status(429);
  }
  return result;
});

app.listen({ port: PORT, host: '0.0.0.0' });
```

**Step 2: Verify build**

Run: `cd elohim/elohim-agent-sdk && pnpm build`
Expected: Compiles without errors

**Step 3: Commit**

```bash
git add elohim/elohim-agent-sdk/src/server.ts
git commit -m "feat(elohim-agent-sdk): Fastify HTTP server with health + invoke endpoints"
```

---

### Task 6: Add doorway proxy route

**Files:**
- Modify: `doorway/src/config.rs` (add `elohim_agent_url` field at ~line 131)
- Create: `doorway/src/routes/elohim_agent.rs`
- Modify: `doorway/src/routes/mod.rs` (add module + re-export)
- Modify: `doorway/src/server/http.rs` (add route match at ~line 1205)

**Step 1: Add config field**

In `doorway/src/config.rs`, after `storage_url` (line 131), add:

```rust
/// URL of elohim-agent-sdk sidecar for AI agent invocation
/// (e.g., "http://localhost:8095")
/// Doorway proxies /api/v1/elohim/invoke requests here
#[arg(long, env = "ELOHIM_AGENT_URL", default_value = "http://localhost:8095")]
pub elohim_agent_url: String,
```

**Step 2: Create the proxy route handler**

Create `doorway/src/routes/elohim_agent.rs` following the same pattern as `presence.rs`:

```rust
//! Elohim Agent invocation — proxy to TypeScript Agent SDK sidecar.

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::{Method, Request, Response, StatusCode};
use std::sync::Arc;
use tracing::{debug, warn};

use crate::server::AppState;

pub async fn handle_elohim_agent_request(
    req: Request<Incoming>,
    state: Arc<AppState>,
    path: &str,
) -> Response<Full<Bytes>> {
    let agent_url = &state.args.elohim_agent_url;
    forward_to_agent(req, agent_url, path).await
}
```

The `forward_to_agent` function follows the same pattern as `forward_to_storage` in `presence.rs`:
- Strip `/api/v1/elohim/invoke` prefix, map to sidecar path (`/invoke` or `/health`)
- Clone method, headers (Content-Type, Authorization), and body
- Forward request and return response with `X-Elohim-Budget-Remaining` header passthrough

Path mapping:
- `POST /api/v1/elohim/invoke` → `POST http://localhost:8095/invoke`
- `GET /api/v1/elohim/invoke/health` → `GET http://localhost:8095/health`

**Step 3: Wire into mod.rs**

In `doorway/src/routes/mod.rs`, add:
```rust
pub mod elohim_agent;
pub use elohim_agent::handle_elohim_agent_request;
```

**Step 4: Wire into http.rs router**

In `doorway/src/server/http.rs`, add a match arm before the presence route (~line 1205):

```rust
// Elohim Agent invocation
(_, p) if p.starts_with("/api/v1/elohim") => {
    return Ok(to_boxed(
        routes::handle_elohim_agent_request(req, Arc::clone(&state), p).await,
    ));
}
```

**Step 5: Verify Rust compilation**

Run: `cd /projects/elohim/doorway && RUSTFLAGS="" cargo build --release`
Expected: Compiles without errors

**Step 6: Run Rust tests**

Run: `cd /projects/elohim/doorway && RUSTFLAGS="" cargo test --lib --bins`
Expected: Tests pass

**Step 7: Commit**

```bash
git add doorway/src/config.rs doorway/src/routes/elohim_agent.rs doorway/src/routes/mod.rs doorway/src/server/http.rs
git commit -m "feat(doorway): add /api/v1/elohim/invoke proxy route to agent SDK sidecar"
```

---

### Task 7: Add Angular dev proxy for `/api/v1/elohim`

**Files:**
- Modify: `elohim-app/proxy.conf.mjs` (add `/api/v1/elohim` to context array)

**Step 1: Update proxy config**

In `elohim-app/proxy.conf.mjs`, add `'/api/v1/elohim'` to the context array:

```javascript
context: ['/api', '/db', '/blob', '/apps', '/epr-head', '/account', '/health'],
```

Note: `/api` already covers `/api/v1/elohim` — verify this is the case by checking the proxy matching behavior. If `/api` catches all `/api/*` paths (it does in Angular dev server proxy), no change is needed. Verify and skip if already covered.

**Step 2: Commit if changed**

```bash
git add elohim-app/proxy.conf.mjs
git commit -m "chore(elohim-app): ensure proxy covers /api/v1/elohim route"
```

---

### Task 8: Surface budget remaining in NativeBackend response

**Files:**
- Modify: `elohim-app/src/app/elohim/services/backends/native-backend.ts` (~line 69-83)

**Step 1: Read the `X-Elohim-Budget-Remaining` header from the response**

In `native-backend.ts`, in the `callDoorway` method (line 69-83), after `if (!response.ok)` check, extract the budget header:

```typescript
const budgetRemaining = parseInt(response.headers.get('X-Elohim-Budget-Remaining') || '-1');
```

Include it in the returned response's `cost` field. The `ElohimComputationCost` interface doesn't have a `budgetRemaining` field, so add it as optional metadata on the response. The simplest approach: if the response has a `cost` object, spread in `budgetRemaining` as an extra field (TypeScript won't complain since `ElohimComputationCost` is the minimum shape).

Actually — don't change the model. Just log it for now. The presence UI already shows cost. Budget remaining can be surfaced in a follow-up.

**Step 2: Commit**

```bash
git add elohim-app/src/app/elohim/services/backends/native-backend.ts
git commit -m "feat(elohim-app): log budget remaining from agent SDK response header"
```

---

### Task 9: Wire E2E step definitions for presence scenarios

**Files:**
- Create: `genesis/a2o/steps/ui/elohim-presence.steps.ts`
- Modify: `genesis/a2o/features/elohim/elohim-presence.feature` (remove `@wip` from tag line)

**Step 1: Check existing step definition patterns**

Read `genesis/a2o/CLAUDE.md` for conventions. Steps go in `steps/ui/` for browser steps. Use `E2EWorld`, `PlaywrightDevice`, `requirePlaywright(this)`.

**Step 2: Write step definitions for the 4 scenarios**

The scenarios test UI behavior, not LLM output quality. They work with both mock and real backends since the UI components are the same.

Key step implementations:
1. `Given the learner navigates to a discovery assessment` — navigate to a discovery content node
2. `When the learner completes the assessment` — interact with sophia-question component to complete
3. `Then an elohim insight section appears below the results` — check for presence banner
4. `And the insight contains a learning path recommendation message` — check banner content
5. `And the insight includes a "Constitutional Reasoning" expandable section` — check for expandable
6. `When the learner expands the reasoning details` — click expandable
7. `Then the primary constitutional principle is visible` — check content
8. `Then the insight shows tokens processed` — check cost display
9. `Given the learner navigates to "/doorway/elohim"` — navigate
10. `When the learner clicks the "Test Connection" button` — click button
11. `Then a banner notification appears confirming the connection` — check banner

**Step 3: Remove @wip tag**

In `elohim-presence.feature`, change line 1 from:
```
@e2e @elohim @browser-only @presence @wip
```
to:
```
@e2e @elohim @browser-only @presence
```

**Step 4: Commit**

```bash
git add genesis/a2o/steps/ui/elohim-presence.steps.ts genesis/a2o/features/elohim/elohim-presence.feature
git commit -m "feat(a2o): wire E2E step definitions for elohim-presence scenarios"
```

---

### Task 10: Integration test — end-to-end demo

**Files:**
- No new files. This is a manual verification task.

**Step 1: Start the sidecar**

```bash
cd elohim/elohim-agent-sdk
ANTHROPIC_API_KEY=<key> ELOHIM_BUDGET_LIMIT=10 pnpm dev
```

Expected: Server listening on port 8095

**Step 2: Test health endpoint**

```bash
curl http://localhost:8095/health
```

Expected: `{"status":"ok","budgetRemaining":10,"model":"claude-haiku-4-5-20251001"}`

**Step 3: Test invoke endpoint directly**

```bash
curl -X POST http://localhost:8095/invoke \
  -H "Content-Type: application/json" \
  -d '{
    "requestId": "test-1",
    "elohimId": "auto",
    "capability": "path-recommendation",
    "params": {"query": "I just completed a values hierarchy assessment. What learning path should I explore next?"},
    "requesterId": "test-user",
    "priority": "normal"
  }'
```

Expected: JSON response with `status: "fulfilled"`, `constitutionalReasoning` with real principle and interpretation, `cost` with real token counts from Claude Haiku.

**Step 4: Start doorway + Angular**

In separate terminals:
```bash
# Terminal 1: doorway (with agent URL configured)
cd doorway && RUSTFLAGS="" ELOHIM_AGENT_URL=http://localhost:8095 cargo run

# Terminal 2: Angular dev server
cd elohim-app && pnpm start
```

**Step 5: Verify through Angular**

Navigate to a discovery assessment in the Angular app. Complete it. Verify:
1. Banner notice appears with a real Claude-generated insight (not mock text)
2. Constitutional reasoning expandable shows real principle weighting
3. Computation cost shows real token counts
4. Budget counter decrements

**Step 6: Verify budget exhaustion**

After 10 calls, verify the 11th returns a declined response with "Budget exhausted".

---

### Task 11: Update alpha deployment manifest

**Files:**
- Modify: `genesis/orchestrator/manifests/edgenode/alpha.yaml`

**Step 1: Add ELOHIM_AGENT_URL env var**

Add the sidecar URL to the alpha deployment environment. The sidecar will need its own container or be deployed alongside doorway. For alpha, add the env var pointing to the sidecar's service URL.

**Step 2: Add ANTHROPIC_API_KEY secret reference**

Reference from a Kubernetes secret (not hardcoded in manifest):

```yaml
- name: ANTHROPIC_API_KEY
  valueFrom:
    secretKeyRef:
      name: elohim-secrets
      key: anthropic-api-key
- name: ELOHIM_AGENT_URL
  value: "http://localhost:8095"
- name: ELOHIM_BUDGET_LIMIT
  value: "10"
```

**Step 3: Commit**

```bash
git add genesis/orchestrator/manifests/edgenode/alpha.yaml
git commit -m "feat(deploy): add elohim-agent-sdk env vars to alpha manifest"
```
