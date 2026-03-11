# Elohim Agent SDK Sidecar — Design Document

_2026-03-07. Validated through brainstorming session._

## Problem

The Elohim Agent infrastructure is built on both sides — Rust (`elohim-agent` crate with `LlmBackend` trait, `AnthropicBackend`, constitutional stack) and TypeScript (`ElohimAgentService`, `ElohimPresenceService`, `NativeBackend` targeting `POST /api/v1/elohim/invoke`). But the pipeline is disconnected: doorway has no `/api/v1/elohim/invoke` route, so `NativeBackend` health checks return false, and the system falls back to `MockBackend` (simulated responses).

The goal is to make real Claude intelligence flow through the existing presence system, replacing mock responses with actual constitutional reasoning from Claude Haiku via the Anthropic Agent SDK.

## Decision: TypeScript Agent SDK Sidecar

### Why a sidecar (not embedded Rust, not frontend-direct)

- The Anthropic Agent SDK is TypeScript/Python only — no Rust SDK exists
- Doorway already proxies to elohim-storage; adding another proxy target is a known pattern
- Keeps LLM intelligence in its own process — clean separation, easy to iterate on prompts
- Alpha uses a hardcoded server-side API key — can't expose in browser
- Future BYOK: API keys pass through from frontend via doorway, same architecture

### Alternatives rejected

- **Embedded V8/Deno in Rust**: Massive complexity for alpha. Over-engineered.
- **Frontend-direct Agent SDK**: Exposes hardcoded alpha API key in browser. No server-side audit.
- **Keep Rust AnthropicBackend**: Works but misses Agent SDK features (tool use, multi-turn). Replacing with SDK aligns with production direction.

## Architecture

```
┌──────────────┐     ┌──────────────┐     ┌─────────────────────┐     ┌───────────────┐
│  elohim-app  │────▶│   doorway    │────▶│  elohim-agent-sdk   │────▶│ Anthropic API │
│  (Angular)   │     │   (Rust)     │     │  (TS sidecar)       │     │ (Claude Haiku) │
│              │     │              │     │                     │     └───────────────┘
│ NativeBackend│     │ /api/v1/     │     │ - @anthropic-ai/sdk │
│ → POST invoke│     │  elohim/     │     │ - Constitutional    │
│              │     │  invoke      │     │   prompt assembly   │
│ ElohimPresence     │  (proxy)     │     │ - 10-call budget    │
│ Service      │     │              │     │ - Hardcoded Haiku   │
└──────────────┘     └──────────────┘     └─────────────────────┘
```

## Components

### 1. `elohim/elohim-agent-sdk/` — New pnpm workspace package

Located inside the `elohim/` directory alongside the existing Rust `elohim-agent` crate.

**Runtime:** Node.js + TypeScript
**HTTP:** Fastify (lightweight, fast, TypeScript-first)
**SDK:** `@anthropic-ai/sdk`
**Port:** 8095 (configurable via `ELOHIM_AGENT_PORT`)
**API Key:** `ANTHROPIC_API_KEY` env var
**Model:** `claude-haiku-4-5-20251001` (hardcoded for alpha)

**Endpoint:**

```
POST /invoke
Content-Type: application/json

Request: ElohimRequest (same shape as Angular NativeBackend sends)
Response: ElohimResponse (same shape Angular already expects)
```

**Budget enforcement:**
- In-memory counter, starts at 0, caps at 10 (configurable via `ELOHIM_BUDGET_LIMIT`)
- `X-Elohim-Budget-Remaining` response header
- After exhaustion: `429 { status: "declined", declineReason: "Budget exhausted" }`
- Resets on process restart (intentional for alpha)

**Health endpoint:**

```
GET /health
Response: { status: "ok", budgetRemaining: N, model: "claude-haiku-4-5-20251001" }
```

**Constitutional prompt assembly:**
- Port `PromptAssembler::build_system_prompt()` from Rust to TypeScript
- Constitutional principles, inviolable boundaries, interpretive guidance
- Capability-specific user prompt templates
- JSON output mode for structured ElohimResponse

### 2. Doorway route — `/api/v1/elohim/invoke`

**Config addition:**
```rust
#[arg(long, env = "ELOHIM_AGENT_URL", default_value = "http://localhost:8095")]
pub elohim_agent_url: String,
```

**Route handler:** Same proxy pattern as `presence.rs`
- Proxies `POST /api/v1/elohim/invoke` → sidecar `POST /invoke`
- Proxies `GET /api/v1/elohim/invoke/health` → sidecar `GET /health`
- Passes through Authorization header (for future BYOK)
- Passes through Content-Type and body

### 3. Angular changes — minimal

The `NativeBackend` already targets the right endpoint. Once doorway serves the route:
- `NativeBackend.isAvailable()` returns true (health check succeeds)
- `ElohimBackendCatalog` selects it over `MockBackend`
- Real Claude responses flow through existing presence pipeline

**One addition:** Surface `X-Elohim-Budget-Remaining` from response headers into the `ElohimComputationCost` model so the UI can display remaining budget.

### 4. Demo feature — discovery completion insight

The demo: complete a discovery assessment → see a real Claude-generated learning path recommendation in the existing banner notice UI.

Flow:
1. Learner completes discovery → `ElohimPresenceService.onDiscoveryCompleted()` fires
2. Invokes elohim with `capability: 'path-recommendation'`
3. Real Claude Haiku responds with constitutional reasoning + path recommendation
4. Banner notice appears with Claude's actual insight (not mock data)
5. Constitutional reasoning expandable shows real principle weighting and confidence

### 5. E2E scenarios

Wire step definitions for the 4 `@wip` scenarios in `elohim-presence.feature`:
1. Learner sees elohim insight after discovery completion
2. Constitutional reasoning transparency (expandable details)
3. Computation cost display (real token counts from Claude)
4. Test connection banner notification

These validate UI behavior and work with both mock and real backends.

## Constraints

- **10-call budget**: In-memory, resets on restart. Alpha safety net.
- **Haiku only**: Cheapest model, hardcoded. No model selection UI.
- **No streaming**: Full response, not SSE. Keeps sidecar simple.
- **No multi-turn**: Single completion per invoke. Agent SDK multi-turn is future work.
- **No tool use**: Simple prompt → completion. Tool definitions are future work.
- **No BYOK passthrough**: Alpha uses server-side key only. BYOK architecture is designed but not wired.
- **No UI redesign**: Banner notices stay. Transcendent presence UI is a separate sprint.
- **No persistent state**: Sidecar is stateless (except in-memory budget counter).

## File Inventory

| File | Action | Purpose |
|------|--------|---------|
| `elohim/elohim-agent-sdk/package.json` | Create | New workspace package |
| `elohim/elohim-agent-sdk/tsconfig.json` | Create | TypeScript config |
| `elohim/elohim-agent-sdk/src/server.ts` | Create | Fastify HTTP server |
| `elohim/elohim-agent-sdk/src/invoke.ts` | Create | Invoke handler + budget |
| `elohim/elohim-agent-sdk/src/constitutional.ts` | Create | Prompt assembly (ported from Rust) |
| `elohim/elohim-agent-sdk/src/types.ts` | Create | Request/response types |
| `pnpm-workspace.yaml` | Edit | Add elohim/elohim-agent-sdk |
| `doorway/src/config.rs` | Edit | Add `elohim_agent_url` |
| `doorway/src/routes/elohim_agent.rs` | Create | Proxy handler |
| `doorway/src/routes/mod.rs` | Edit | Export new handler |
| `doorway/src/server/http.rs` | Edit | Wire route |
| `elohim-app/src/proxy.conf.mjs` | Edit | Add dev proxy for `/api/v1/elohim` |
| `elohim-app/.../native-backend.ts` | Edit | Surface budget header |
| `genesis/a2o/features/elohim/elohim-presence.feature` | Edit | Remove @wip tags |
| `genesis/a2o/step_definitions/elohim/` | Create | E2E step definitions |
