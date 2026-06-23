---
id: "backlog-integration-prepush-preexisting-gate-debt"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "dev pre-push gate is red on PRE-EXISTING debt (doorway clippy + a2o prettier) — not the integrating change's fault; owned by active tracks"
slug: "integration-prepush-preexisting-gate-debt"
written: "2026-06-18"
status: "open"
priority: "medium"
ci_status: blocked
tags: [pre-push-gate, clippy, prettier, doorway, genesis-a2o, integration, items-after-test-module]
cites:
  - doorway/doorway-service/src/routes/health.rs
  - genesis/a2o/steps/ui/ssr-trace.steps.ts
  - genesis/a2o/steps/resilience.steps.ts
  - genesis/a2o/scripts/look-resilience-panel.ts
  - .husky/pre-push
---

# dev pre-push gate red on pre-existing debt (surfaced during the 2026-06-18 feat→dev integration)

Discovered while integrating `feat/frontend-eyes-sprint` → `origin/dev` (serve-blob fix + Fix#4 + smaps
leak-localizer + doorway /metrics). The dev-targeting pre-push gate failed on **`genesis-a2o doorway
elohim-storage`** — but **none of it was the integrating change**. Verified each; the integration content is
clean (my files fmt+9/9 tests+compile; doorway /metrics clippy-clean; net diff does not touch the failing
files). The push proceeded with `--no-verify` per [[hook-bypass-integration-shakeout]]. These are the
pre-existing/environmental causes, owned by the tracks that authored them:

## Durable (committed on dev — will keep failing the gate until the owning track fixes)

1. **doorway clippy `items_after_test_module`** — `doorway/doorway-service/src/routes/health.rs:378`
   `pub async fn startup_check` sits AFTER `#[cfg(test)] mod tests` (line 294-295). Added by `82edc611e`
   ("GET /admin/self-healing unified read model", Plan C) — the **self-healing-control-plane** track's landed
   work. Under `cargo clippy --all-targets -- -D warnings` this is an error. **Fix (trivial, owner-gated):**
   move `startup_check` (and any other post-module items) to BEFORE the `#[cfg(test)] mod tests` block. Not
   fixed during integration to avoid stepping on that track's active doorway work. (CI evidently does NOT
   enforce this exact lint at -D, since 82edc611e landed — so it's a local-gate strictness gap, not a CI red.)

2. **genesis-a2o prettier drift** — 3 committed TS files fail `format:check`:
   `genesis/a2o/steps/ui/ssr-trace.steps.ts`, `genesis/a2o/steps/resilience.steps.ts`,
   `genesis/a2o/scripts/look-resilience-panel.ts` (SSR/resilience tracks' files). **Fix:** `prettier --write`
   those three + commit. Owner-gated for the same reason.

## Environmental (local-only; CI's clean env is unaffected)

3. **`eslint-plugin-sonarjs@3.0.7` module-resolution failure** in `genesis/a2o` lint (`S6759/rule.js` not
   resolvable) — a broken/partial local `node_modules`, not a code issue. **Fix:** refresh the local install
   (`pnpm install`) when convenient.

4. **elohim-storage ambient WIP dirt** — the shared worktree carried uncommitted `recursion.rs`,
   `services/household_resilience.rs`, `tests/chaos_dataplane.rs`, `tests/household_resilience.rs` from other
   sessions; the gate ran clippy against that dirty tree. Transient (resolves when those sessions commit/clean).

**Net:** the dev pre-push gate gives a confusing "your push failed 3 projects" to ANY integrator whose change
touches doorway/storage/a2o, even with clean content, because it runs the full project gate against
pre-existing committed debt + the dirty shared worktree. Two cheap durable fixes (#1, #2) would clear the
recurring false signal.
