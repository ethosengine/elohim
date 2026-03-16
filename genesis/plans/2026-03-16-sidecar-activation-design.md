# Sidecar Activation — Design

**Goal:** Activate the inference sidecar so the ElohimGate makes real Claude API calls during mutation evaluation.

---

## Current State

The entire sidecar stack is already built:

| Component | Status | Location |
|-----------|--------|----------|
| TypeScript Fastify sidecar | Complete | `elohim/elohim-agent/elohim-agent-sdk/` |
| Rust SidecarEngine (HTTP client) | Complete | `services/sidecar_engine.rs` |
| InferenceRouter | Complete | `services/inference_router.rs` |
| ElohimGate integration | Complete | `services/elohim_gate.rs` |
| Doorway proxy | Complete | `doorway-service/src/routes/elohim_agent.rs` |
| Angular NativeBackend | Complete | `services/backends/native-backend.ts` |
| Wire format | Locked | `types.ts` ↔ `sidecar_engine.rs` |

`Services::new()` already creates `SidecarEngine` → `InferenceRouter` → `ElohimGate::new(router)`. The gate will call the sidecar when it's running. If the sidecar is unavailable, the router returns an error and the gate falls back to PassThrough.

## What's Missing

1. **The sidecar isn't started** — `hc-start.sh` launches conductor, storage, and doorway but not the agent SDK
2. **The sidecar isn't built** — TypeScript needs `pnpm build` before `pnpm start`
3. **No verification** — no end-to-end test that the gate actually calls Claude

## Changes

### 1. Add sidecar to hc-start.sh

After storage starts (Step 2) and before doorway (Step 3), add a Step 2.5 that:
- Checks if `ANTHROPIC_API_KEY` is set (skip with warning if not — gate falls back to PassThrough)
- Builds the agent SDK if needed (`pnpm build` in the agent-sdk directory)
- Starts the sidecar in background (`pnpm start &`)
- Waits for health check at `http://localhost:8095/health`

### 2. Add sidecar to hc:stop

Kill the sidecar process alongside the other services.

### 3. Verification

Manually verify with curl that the sidecar responds to `/health` and `/invoke`.
