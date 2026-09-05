---
title: "Two-portals SSO consolidation — retire elohim-app's third portal; apps become relying parties of the doorway portal"
id: two-portals-sso-consolidation-plan
status: Draft
class: protocol-canonical
context-tier: disclosed
steward: angular-architect
domain: D2
habits: [hosted-human-lifecycle]
graduation-trigger: elohim-app renders no password field and posts no registration; `/identity/login` and `/identity/register` redirect to the discovered doorway portal; the app's imagodei suite is green; `@concern:oauth-authorization-code` and the hosted-human station-1 "registering because an application asked me" scenario pass on the household mesh
topic: [auth, portal, sso, oauth, imagodei, doorway, elohim-app]
informed-by:
  - genesis/docs/superpowers/specs/2026-09-05-two-portals-sso-consolidation-design.md (the decision)
  - genesis/docs/superpowers/plans/2026-09-04-hosted-human-lifecycle-e2e-plan.md (the primary-portal stories this serves)
cites:
  - "two-portals-sso-consolidation-design | Two portals, shared like SSO | sha256:38a88563ff60fb43 | path: genesis/docs/superpowers/specs/2026-09-05-two-portals-sso-consolidation-design.md"
  - app/elohim-app/src/app/imagodei/components/login/login.component.ts
  - app/elohim-app/src/app/imagodei/components/register/register.component.ts
  - app/elohim-app/src/app/imagodei/services/providers/password-auth.provider.ts
  - app/elohim-app/src/app/imagodei/services/providers/oauth-auth.provider.ts
  - app/elohim-app/src/app/imagodei/services/session-migration.service.ts
  - app/elohim-app/src/app/imagodei/services/identity.service.ts
  - app/elohim-app/src/app/imagodei/imagodei.routes.ts
  - doorway/doorway-service/src/routes/auth_routes.rs
  - genesis/a2o/features/auth/oauth-authorization-code.feature
  - genesis/a2o/features/auth/hosted-human/01-creating-an-account.feature
---

# Two-portals SSO consolidation — plan

**Base branch:** the 2026-09-04 auth-fix worktree branch (`worktree-agent-ab7156cef8ff673c8`), because it already carries the doorway-portal fixes this plan builds on. **Its commit `6270e7a8b` ("stop blocking in-app registration on a conductor socket") is reverted first**: under the two-portal rule the in-app registration page retires rather than works better.

**P2P design gate:** not applicable — no entity is created, no route serves data; one optional query parameter is added to an existing OAuth endpoint.

## Tasks

- [ ] **Task 0 — Revert fix six.** `git revert 6270e7a8b` on the branch. Evidence: the register component's socket gate is back exactly as it was on `dev` (it is deleted in Task 3 anyway).
- [ ] **Task 1 — Doorway `authorize` honours `prompt=create`.** Add `prompt: Option<String>` to `OAuthAuthorizeRequest`; in the unauthenticated arm of `handle_authorize`, when `prompt == "create"` redirect to `/threshold/register?…` with the same parameters (client_id, redirect_uri, response_type, state, scope, login_hint), otherwise `/threshold/login?…` as today. Unit-test the URL builder as a pure function. Register components already carry OAuth params through, so the human lands back in the app. Evidence: unit test; `cargo fmt --check`; the heavy gate when the cargo lease allows.
- [ ] **Task 2 — elohim-app `/identity/login` becomes a redirector with a doorway picker.** In `login.component.ts`: keep the resolver step; on `resolved`, call `OAuthAuthProvider.initiateLogin(doorwayUrl, callback, identifier)` immediately (login_hint = identifier). If a proven doorway is already selected (`DoorwayRegistryService.selectedUrl()` non-null on init) skip the resolver and redirect at once, preserving `returnUrl` via the provider's stored return URL. Remove the `<elohim-imagodei-login-card>` usage and the `allow-password` attribute; remove `onPasswordSubmit`. Keep `_prefetchAuthority` (fix four). Update `login.component.spec.ts`. Evidence: focused vitest green; the component never references `PasswordAuthProvider`.
- [ ] **Task 3 — elohim-app `/identity/register` becomes a redirector.** Replace `RegisterComponent` with a thin component that resolves the doorway the same way as Task 2 and calls a new `OAuthAuthProvider.initiateRegistration(doorwayUrl, callback)` (same as `initiateLogin` plus `prompt=create`). Delete the old form, its template, css, and spec (849 lines); add a small spec for the redirect. Remove the two `routerLink="/identity/register"` CTAs' expectations of an in-app form (they keep working — they hit the redirector). Evidence: focused vitest green; `pnpm exec vitest run --config vite.config.ts imagodei` green.
- [ ] **Task 4 — Visitor → hosted migration moves after the callback.** `SessionMigrationService.migrate` no longer calls `identityService.registerHuman`. New shape: `applySessionToProfile()` — after `AuthCallbackComponent` succeeds, if `SessionHumanService.hasSession()`, update the freshly authenticated human's profile (display name if the doorway-issued one is the identifier local-part, bio, interests → affinities) through the existing profile-update path in `IdentityService`, then link the session (`linkSession` / whatever `migrate` did after registration today), then clear the visitor session. `canMigrate` no longer requires a conductor socket. Keep the upgrade banner/modal copy honest ("Create an account at your doorway to keep this progress"). Evidence: session-migration spec rewritten; upgrade-modal spec green.
- [ ] **Task 5 — Retire `PasswordAuthProvider` from elohim-app.** Delete the provider, its spec, and its registration in `login.component` / `register.component` / `app.config`; delete `IdentityService.registerHuman` (hosted variant) and its `RegisterHumanRequest` email/password fields if nothing else uses them (`registerHumanNative` stays — it is the native path). `AuthService.register` and the `register` capability on the provider interface go if no provider implements them. Evidence: `grep -rn PasswordAuthProvider app/elohim-app/src` returns nothing; `just gate elohim-app` (AOT build + tests) green — this is the gate that catches strictTemplates errors the container's tsc misses.
- [ ] **Task 6 — a2o alignment.** (a) `genesis/a2o/features/auth/hosted-human/01-creating-an-account.feature` scenario "Registering because an application asked me…" — confirm its steps describe: app register link → doorway portal register (with the app's request) → back in the app signed in; adjust the narrative only if it assumed an in-app form. (b) grep `genesis/a2o/steps` for any step that drives `/identity/register` or types a password into the app (not the portal) and retarget it to the portal; the harness itself is unaffected. (c) `oauth-authorization-code.feature`: add one scenario "A signed-out human asking to create an account is sent to registration without losing the request" pinning Task 1. Evidence: `npx cucumber-js --dry-run --tags '@auth'` 0 new undefined steps; the new scenario passes on the household mesh when the lease allows. (d) `features/peer-oauth-portal/` was the third portal in story form: a unified portal projected at `/auth/portal` with a "Mode A" hosted login through an in-app login card. Disposition: Mode A (`hosted-login.feature`) is rewritten into the hosted-human series as `06-reaching-the-app.feature` (app resolver → doorway portal → back in the app); `peer-conductor-login.feature` (Mode B) stays as the native portal's story and opens the graduation series — its `/auth/portal` path is stale and is re-pointed when that series is written; `rp-consent.feature` (external-app consent) keeps its claims but the consent surface belongs to the doorway portal, not a third one — retarget when consent is built.
- [ ] **Task 7 — Docs and gospel.** Update `app/CLAUDE.md` (or the imagodei pillar note) and `doorway/CLAUDE.md`'s "Trust Model" with one paragraph: two portals, apps are relying parties, discovery not configuration. Managed surfaces: go through the cite tooling (`cite-gen.py --seal`), never hand-edit envelopes. Evidence: `python3 .claude/scripts/memory-kit/cite-gen.py --seal <file>` clean.

## Sequencing

Task 0 first. Tasks 1 and 2–5 are independent (Rust vs Angular) and can run in parallel on two workers; Task 6 after 2–5; Task 7 last. One worktree, one branch, sequential commits per task.

## Risks named

- `just gate elohim-app` is the only rail that catches AOT template errors; the container's `vitest` will pass a broken template. Run it before calling Task 5 done.
- The heavy doorway gate needs the cargo lease and memory headroom; if unavailable, say so — do not merge Task 1 on `fmt --check` alone.
- Deleting `RegisterHumanRequest` fields may ripple into `@elohim/identity` generated types; if it does, stop and report rather than editing generated files.
