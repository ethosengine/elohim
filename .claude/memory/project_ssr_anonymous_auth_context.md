---
name: SSR fetch shim has anonymous-only auth context (higher-reach content renders empty)
description: Current AngularRenderer fetcher only sees V8 HttpClient headers, not the originating request's Cookie/Authorization; commons-reach renders fine, but regional-private/local/private content gets anonymous-equivalent denial in the rendered HTML
type: project
originSessionId: cf962313-d70a-459d-acb7-925c8f19e9e1
---
The doorway-ssr-deliver shift (2026-05-08) wired `ResolverFetcher::new(client, storage_url)` and the worker uses `with_full_shims(fetcher)`. The shim forwards Angular's HttpClient request headers to elohim-storage — but those are headers the SSR-running Angular code set, not the originating public request's `Authorization` / `Cookie`. Effective auth context for storage fetches during render is anonymous.

**Why:** Per `app/elohim-app/REACH.md`, commons content is allowed for anonymous, but regional-private/local/private require authenticated access. SSR currently returns empty/placeholder HTML for any logged-in user requesting higher-reach content. Not a security hole (no privilege escalation — SSR has anonymous-equivalent access, never more), but a feature gap: rendered HTML for authenticated users matches anonymous, not their actual reach.

**How to apply:**
- Don't extend SSR to new pillars expecting authenticated content to render correctly until auth-threading is solved.
- Higher-reach content delivered through SSR is currently a graceful-degradation path: substrate returns the public-only render and the client hydrates with the authenticated content via CSR.
- The fix is framework-agnostic auth propagation through the V8 boundary; do NOT lock it to Angular HttpClient interceptors. Brainstorm prompt: `.claude/shifts/doorway-ssr-deliver-2026-05-07T23-37/brainstorm-prompt-followup.md` (Q3).
- Plausible patterns: forwarded subset-of-headers (Cookie/Authorization), per-render scoped V8 OpState with `getAuthContext()` op, signed delegation token (OAuth on-behalf-of style). Last one is most framework-neutral and aligns with `project_socially_derived_security` delegation thinking.
- Couples to `project_reach_gate_is_elohim_mediated_matchmaking`: the SSR auth context is a special case of the same delegation pattern; design them together, not separately.
