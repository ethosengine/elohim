# Peer OAuth Portal — Substrate Audit

**Date:** 2026-05-25
**Spec:** `genesis/docs/superpowers/specs/2026-05-25-peer-oauth-portal-design.md` §8
**Plan:** `genesis/docs/superpowers/plans/2026-05-25-peer-oauth-portal-plan.md` Task A1
**Branch:** `design/peer-oauth-portal`
**Purpose:** Audit the five open questions before implementation; document any gaps and their dispositions.

The spec's Appendix B is explicit: NO new DHT entry types are introduced by the peer OAuth portal design. Any backend additions identified by this audit are operational projection-layer adjustments (new HTTP routes, response-shape extensions) — never substrate primitives. The architectural constraint from `doorway/CLAUDE.md` also applies: doorway is manifest-driven and free of per-domain proxy files, so new HTTP routes belong in `elohim-storage`'s `build_manifest()` and reach doorway through the registry. The exceptions are auth-specific surfaces that already live in doorway-service (the `/auth/*` family in `auth_routes.rs`) because they need JWT validation and other doorway-specific gating.

---

## 1. `/auth/me` response shape

**Current state.** `auth_routes.rs:1719-1767` defines `GET /auth/me` and `auth_routes.rs:216-229` defines the response type `MeResponse`. The handler verifies the Bearer JWT and returns exactly six fields from the JWT claims:

```rust
MeResponse {
    human_id,
    agent_pub_key,
    identifier,
    permission_level,        // hosted-visitor | hosted-steward | admin ...
    doorway_id,
    doorway_url,
}
```

The JWT `Claims` struct (`auth/jwt.rs:73-114`) already carries the two booleans the spec needs to derive `trustMode`: `is_steward` (line 104) and `has_local_conductor` (line 107). Neither is currently surfaced on `MeResponse`, even though the same two fields ARE surfaced on `AuthResponse` (the login/register response — see `auth_routes.rs:182-198`, where both `is_steward` and `portal_host_url` were already added during the M5 sprint).

**Gap analysis.** The spec wants `<elohim-imagodei-portal-shell>` to read `/auth/me` once on mount and resolve four things:
- `authenticated: boolean` (currently inferable only from HTTP 200 vs 401)
- `trustMode: 'doorway-host' | 'peer-conductor'` (currently NOT exposed)
- `authority: string` (a label/url for the trust-indicator chrome — currently NOT exposed)
- For Mode B browser→doorway-routed→peer-conductor: `conductorEndpoint` (currently NOT exposed)

Mapping the existing claims to the wire shape the spec sketches at §3.2 line 288:
- `trustMode = has_local_conductor ? 'peer-conductor' : 'doorway-host'`
- `authority` = derived label, falling back to `doorway_url` for Mode A and `conductor_id` / configured conductor URL for Mode B
- `conductorEndpoint` = the URL the bundle should call directly (Tauri: `http://localhost:8090`; doorway-routed Mode B: empty because doorway transparently proxies)

`authenticated` doesn't need its own bool — the current handler already returns 401 when the token is invalid/missing, so the bundle can rely on response status. But surfacing `authenticated: true` explicitly in the 200 body removes ambiguity (e.g., a session-expired-but-not-yet-refreshed window where the cookie is present but rejected).

**Recommended disposition.** Extend `MeResponse` with three optional fields (`trust_mode`, `authority`, `conductor_endpoint`) plus `authenticated: bool`. Keep all four fields on the same shape so the bundle has one wire contract regardless of trustMode. This is purely additive and breaks no existing clients.

**Proposed Phase A.N task:** YES — Task **A2: extend MeResponse with trustMode/authority** (~25 lines + 1 test in `doorway/doorway-service/src/routes/auth_routes.rs`).

Concrete shape:
```rust
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeResponse {
    pub authenticated: bool,
    pub human_id: String,
    pub agent_pub_key: String,
    pub identifier: String,
    pub permission_level: String,
    #[serde(skip_serializing_if = "Option::is_none")] pub doorway_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub doorway_url: Option<String>,
    /// "doorway-host" or "peer-conductor"; derived from claims.has_local_conductor
    pub trust_mode: String,
    /// Display label for the trust-indicator chrome (doorway URL or conductor identifier)
    pub authority: String,
    /// For Mode B Tauri path; the conductor URL the bundle should call directly
    #[serde(skip_serializing_if = "Option::is_none")] pub conductor_endpoint: Option<String>,
}
```

---

## 2. `/.well-known/elohim-doorway` endpoint

**Current state.** No such route exists.

- `grep -rn "well-known\|well_known" /projects/elohim/doorway/doorway-service/src/ /projects/elohim/elohim/elohim-storage/src/` returns only:
  - `doorway-service/src/routes/identity.rs:229` — `/.well-known/did.json`
  - `doorway-service/src/routes/federation.rs:5,143` — `/.well-known/doorway-keys` (JWKS)
  - `doorway-service/src/server/http.rs:1178,1188` — the two match arms above
- `elohim-storage`'s `build_manifest()` (`http.rs:8160`) declares zero `/.well-known/*` routes.
- The existing `DoorwayRegistryService` in the Angular pillar (`app/elohim-app/src/app/imagodei/services/doorway-registry.service.ts:388-413`) resolves a gateway domain to a doorway URL through a three-step convention:
  1. `BOOTSTRAP_DOORWAYS` hardcoded list (`models/doorway.model.ts:316-334`)
  2. Convention `alpha.elohim.host → doorway-alpha.elohim.host`
  3. Direct fetch of `https://{domain}/registry/doorways` or `/api/v1/federation/doorways`

So today, the Angular pillar discovers doorways via `/registry/doorways` or `/api/v1/federation/doorways` (the latter is real — `doorway-service/src/routes/federation.rs:4` and routed at `server/http.rs:1193`). The convention-based mapping is enough when domain naming follows the `doorway-*` pattern; the `.well-known` lookup the spec mentions is a NEW concept the spec proposes for the standalone-bundle path.

**Gap analysis.** The standalone EPR bundle in `app/imagodei-portal/` will be loaded under `<base href="/auth/portal/">` (often through a doorway). The bundle must NOT inherit Angular DI; it needs a fetch-based federated resolver. Two paths exist:

- **Reuse `/api/v1/federation/doorways`** — already implemented, already routed. The standalone resolver fetches `https://{gatewayDomain}/api/v1/federation/doorways`, walks the response, and picks the entry matching the user's gateway. Works today, no backend change.
- **Add `/.well-known/elohim-doorway`** — a smaller, single-doorway descriptor (URL + DID + portal-host URL + features) at a conventional path. Cleaner from the federated-identifier-resolution semantics POV but a new route.

For MVP, the standalone resolver can use the existing `/api/v1/federation/doorways` endpoint (or `parseFederatedIdentifier` + convention from `doorway.model.ts:316-334`). The `.well-known` path is a polish item — it makes domain-only doorways (no `doorway-` subdomain prefix) discoverable without a federation table lookup, which matters when a household runs its own doorway under a vanity domain like `family.example`.

**Recommended disposition.** For MVP, the standalone resolver uses `parseFederatedIdentifier` + `resolveGatewayToDoorwayUrl` from `doorway.model.ts` (which can be exported as a stand-alone helper) plus a single `GET {doorwayUrl}/health` (already routed at `routes/health.rs:235`) to verify reachability. Defer `.well-known/elohim-doorway` to a follow-up shift — the federated-resolver Lit primitive (Task B4) takes a `resolverFn` prop so the implementation can swap from convention-only to `.well-known`-aware later without touching the primitive.

**Proposed Phase A.N task:** YES — Task **A3: extract `parseFederatedIdentifier` and `resolveGatewayToDoorwayUrl` to a framework-agnostic module the standalone bundle can import** (~30 lines moving the existing functions from `app/elohim-app/src/app/imagodei/models/doorway.model.ts` into a leaf module that has no Angular dependencies — possibly `app/elohim-elements/elohim-imagodei/src/lib/federated-identifier.ts`). This unblocks the standalone resolver in Task B4 without a backend change.

A separate, smaller follow-up that the implementer can opt into later — add `GET /.well-known/elohim-doorway` to `elohim-storage`'s `build_manifest()` returning the doorway's own identity descriptor — should NOT be inside the peer-OAuth-portal sprint. Track it on the M5/M6 boundary as a portability cleanup.

---

## 3. Tauri local conductor `/auth/me`

**Current state.** The Tauri sidecar (elohim-storage on `localhost:8090`) does NOT expose `/auth/me`. It exposes a different surface:

- `GET /session` (`elohim/elohim-storage/src/http.rs:6218-6234`) — returns the active `LocalSessionView` for the device (humanId, identifier, agentPubKey, doorwayUrl, displayName, ...) or 404 if no session.
- `POST /session`, `DELETE /session` (`http.rs:6240, 6272`) — session lifecycle for the OAuth handoff.

The Angular Tauri integration uses `GET /session` not `/auth/me`. `TauriAuthService.getActiveSession()` (`app/elohim-app/src/app/imagodei/services/tauri-auth.service.ts:299-326`) calls `fetch(\`${storageUrl}/session\`)` and returns the `LocalSession` or null. Auth state in Tauri is also reflected through Tauri IPC commands (`doorway_status`, `doorway_unlock`, etc. — see `steward/device/src-tauri/src/lib.rs:426-440`); these are the device-scoped truth source for "are we authenticated?" because the IPC layer holds the encrypted key bundle.

The Holochain conductor itself (`localhost:4444` admin, `4445` app) has zero HTTP `/auth/me`; it's a WebSocket-only surface and is not directly addressed by the browser bundle.

**Gap analysis.** The spec's §3.2 Transport β says "Tauri → localhost conductor (no doorway). Tauri webview loads the same bundle from local storage. On mount, /auth/me hits localhost conductor directly." Two readings:

- **Strict reading:** The bundle calls `/auth/me` against `localhost:8090`. That endpoint doesn't exist today. We'd need to add `GET /auth/me` to `elohim-storage`'s `build_manifest()` that wraps the existing `/session` logic in the `MeResponse` envelope (with `trustMode: 'peer-conductor'`, `authority: 'Your conductor on this device'`).
- **Pragmatic reading:** The bundle is allowed to know it's in Tauri (via `window.__TAURI__` detection — already used by `tauri-auth.service.ts:160-163`) and call `/session` instead. The Lit `portal-shell` primitive takes an `authorityEndpoint` prop (spec §2 line 119) which defaults to `/auth/me`; Tauri mounts can override it to `/session` and provide a tiny adapter that maps `LocalSession → MeResponse`.

The strict reading keeps the bundle transport-agnostic (same fetch call, same response shape, only the base URL changes) and matches the spec's "Invariant" at line 313: "the bundle does not know which transport is in play." The pragmatic reading is faster to land but couples the standalone bundle to "if Tauri, do X different" logic that the spec explicitly rejects.

**Recommended disposition.** Add `GET /auth/me` to `elohim-storage`'s manifest. The handler is thin: read active `LocalSession`, project to a `MeResponse` shape with `trustMode: 'peer-conductor'`, `authority: displayName ? "${displayName}'s conductor" : "Your conductor on this device"`, `conductorEndpoint: "http://localhost:8090"`. Returns 401 with `{authenticated: false}` envelope when no session. Doorway-routed and Tauri-direct paths then share one fetch call, one response shape, one set of trust-mode resolution logic. This is operational projection, not new substrate.

**Proposed Phase A.N task:** YES — Task **A4: add `GET /auth/me` to elohim-storage's build_manifest projecting LocalSession → MeResponse** (~40 lines + 1 unit test in `elohim/elohim-storage/src/http.rs` and a route declaration in `build_manifest()`). The `MeResponse` shape lives in elohim-views (so both doorway-service and elohim-storage can serialize it) — but in practice this is a small struct that can be locally re-derived if cross-crate sharing is awkward.

---

## 4. OAuth-client registration surface

**Current state.** `doorway-service/src/db/schemas/oauth_session.rs:133-160` defines `pub fn get_registered_clients() -> Vec<OAuthClient>` which returns a hardcoded `Vec` of exactly two entries:

- `elohim-app` — redirects `http://localhost:*`, `http://127.0.0.1:*`, `https://*.elohim.host/*`, `https://elohim.host/*`, `https://*.ethosengine.com/*`; `trusted: true` (skips consent)
- `doorway-app` — redirects `/threshold/*`; `trusted: true`

The accompanying type (lines 116-128) anticipates dynamic registration with a `trusted` boolean and a `redirect_uri_patterns` list, but the SQLite-backed table for this exists only in shape — the function is hardcoded.

**Gap analysis.** Spec §6.2 and §8 question 4 both note that dynamic OAuth-client registration is a follow-up. The portal's consent flow (Task B6, F3) operates against the registered-clients table for the OAuth-RP consent UI. For MVP — only the first-party `elohim-app` and `doorway-app` clients exist — the hardcoded surface is sufficient. The §4.3 error surface ("alpha.elohim.host doesn't recognize this app. If you're the developer, register it at `/admin/oauth-clients`") references an admin UI that doesn't exist yet, but the error itself works against the hardcoded table (an `unauthorized_client` response is correct for unregistered RPs).

**Recommended disposition.** Keep `get_registered_clients()` hardcoded for MVP. Don't add a dynamic-registration UI in this sprint. The error copy for `unauthorized_client` can link to a "coming soon" admin page; the spec already treats `/admin/oauth-clients` as deferred.

**Proposed Phase A.N task:** NO — out of MVP scope. Track as a follow-up for a future shift (likely under a "doorway operator UX" milestone that also covers `/admin/oauth-clients`, `/admin/portal-hosts`, etc.).

---

## 5. PortalHost lookup

**Current state.** The lookup surface ALREADY EXISTS, end to end:

- **Substrate entry** — `elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/portal_host.rs:40-50` defines the `PortalHost` Category-A entry (anchored on Human ActionHash, M5 ships `Trusted` reach only).
- **Storage projection** — `elohim/elohim-storage/src/http.rs:9867-9881` registers three manifest routes:
  - `GET /api/v1/account/portal-hosts` — list registered portal hosts for the authenticated human (handler `get_portal_hosts`)
  - `POST /api/v1/account/portal-hosts` — register a new portal host (handler `post_portal_host`)
  - `DELETE /api/v1/account/portal-hosts/{url_b64}` — revoke (handler `delete_portal_host`)
- **Reconcile dispatch** — `elohim/elohim-storage/src/reconcile/controller.rs:273-281, 642-664` handles `PortalHostCreated` / `PortalHostRemoved` DNA signals, projecting the DHT-notarized entries into the `portal_hosts` SQLite table.
- **Doorway exposure** — `doorway-service/src/routes/auth_routes.rs:3510-3630` implements `GET /auth/portal-host` and the `PortalHostResponse` shape (`doorway-service/src/auth/portal_host.rs:11-27`):
  ```rust
  pub struct PortalHostResponse {
      pub reachable: bool,
      pub host_url: Option<String>,    // first reachable host
      pub all_hosts: Vec<String>,       // ALL registered hosts, ordered
  }
  ```
- **Login-time probe** — `auth_routes.rs:3636-3650` (`probe_first_portal_host`) is called inline during login and session-exchange so the `AuthResponse.portal_host_url` field is populated without a separate round-trip.

The `all_hosts: Vec<String>` field is EXACTLY what spec §4.5 needs for the "list other portals you've authorized, if any" surface. The list comes from the user's primary doorway via this endpoint; no new substrate work is required.

**Gap analysis.** The spec at §4.5 describes the error case "User enters matthew@beta.elohim.host on alpha.elohim.host's portal; beta returns 404." In this scenario, the portal-shell is on alpha but the user hasn't authenticated yet — alpha doesn't have a JWT for matthew, so it CAN'T call `/auth/portal-host` against matthew's account. The list "other portals you've authorized" can only come from the doorway that has matthew's session, which is none right now.

Two readings:
- **Strict reading:** alpha's portal needs an UNAUTHENTICATED lookup of "what are the PortalHosts registered for the imagodei matching this federated identifier?". This is a new endpoint (`GET /auth/portal-host/lookup?identifier=matthew@beta.elohim.host`) and a substrate-exposure question (do we leak which doorways a human has authorized to an anonymous query? Likely yes — these are already commons-reach PortalHost entries — but it's worth thinking through reach gating).
- **Pragmatic reading:** The spec's §4.5 error already has a fallback line: "List comes from best-effort lookup at user's primary doorway." Best-effort means: if the user has a session somewhere accessible (e.g., a recent cookie on alpha from a prior login), use it; otherwise just show the single host the user typed and "If you've lost access to all your portals → start recovery." MVP can show a minimal error WITHOUT the bulleted list.

**Recommended disposition.** For MVP, show the minimal error copy at §4.5 WITHOUT the cross-doorway portal list. The error message becomes:

> "matthew@beta.elohim.host hasn't authorized alpha.elohim.host to render their sign-in. Try signing in at beta.elohim.host (your registered portal). If you've lost access → start recovery."

This is correct: when the federated-resolver gets a 404 from beta, it already knows the canonical home doorway (beta). Listing OTHER portals requires data the anonymous query path can't access. Cross-doorway anonymous PortalHost lookup is a separate substrate-policy decision; it shouldn't gate this sprint.

**Proposed Phase A.N task:** NO — the existing surfaces are sufficient. Track the optional enhancement (anonymous-lookup endpoint for the §4.5 cross-portal list) as a follow-up. The peer-OAuth-portal spec's Definition of Done (§7) doesn't require the bulleted list — only the canonical-portal hint and the recovery link.

---

## Disposition summary

| Question | Status | Adds task? | Notes |
|---|---|---|---|
| /auth/me response shape | needs extension | YES — **A2** | Add `authenticated`, `trustMode`, `authority`, `conductorEndpoint` to `MeResponse` (~25 lines + test) |
| /.well-known/elohim-doorway | absent; existing `/api/v1/federation/doorways` covers MVP | YES — **A3** | Extract `parseFederatedIdentifier` + `resolveGatewayToDoorwayUrl` to framework-agnostic helper (~30 lines, no backend change) |
| Tauri /auth/me | missing on storage; `/session` exists with different shape | YES — **A4** | Add `GET /auth/me` to storage's `build_manifest()` projecting `LocalSession → MeResponse` (~40 lines + test) |
| OAuth client registration | hardcoded `elohim-app` + `doorway-app`; sufficient for MVP | NO | Deferred — dynamic registration UI is a separate operator-UX sprint |
| PortalHost lookup | fully exists — substrate entry, storage projection, doorway `/auth/portal-host` with `allHosts` array | NO | §4.5 error copy can omit cross-portal list; existing canonical-portal hint is enough |

---

## Recommended plan amendments

The implementation plan should insert three lightweight Phase A tasks between Task A1 (this audit) and Phase B (Lit primitives). All three are operational projection-layer adjustments — zero new DHT entry types, zero per-domain proxy files added to doorway-service.

### Task A2: Extend `MeResponse` with trustMode + authority
- **File:** `doorway/doorway-service/src/routes/auth_routes.rs`
- **Lines:** ~25 added (response struct extension + handler logic to derive `trust_mode` from `claims.has_local_conductor`, `authority` from the doorway/conductor identifier) + 1 unit test
- **Rationale:** Unblocks B3 (`<elohim-imagodei-portal-shell>`) trust-mode discovery
- **Risk:** Trivial — additive, no breaking changes
- **Verification:** `curl -H "Authorization: Bearer ..." /auth/me` returns the new fields; existing clients ignore unknown fields

### Task A3: Extract federated-identifier helpers to framework-agnostic module
- **File:** new — `app/elohim-elements/elohim-imagodei/src/lib/federated-identifier.ts` (preferred) OR a `@elohim/federated-identifier` workspace package
- **Lines:** ~30 (move existing `parseFederatedIdentifier` and `resolveGatewayToDoorwayUrl` from `app/elohim-app/src/app/imagodei/models/doorway.model.ts:295-334` into a leaf module with zero Angular dependencies)
- **Rationale:** Lets Task B4 (`<elohim-imagodei-federated-resolver>`) and the standalone bundle (Phase D) share the resolver without dragging Angular in
- **Risk:** Low — the Angular pillar re-exports from the new location; existing imports keep working
- **Verification:** Existing `app/elohim-app/src/app/imagodei/models/doorway.model.spec.ts` continues to pass; new bundle's resolver imports the leaf module

### Task A4: Add `GET /auth/me` to elohim-storage's manifest
- **File:** `elohim/elohim-storage/src/http.rs`
- **Lines:** ~40 (handler that reads active `LocalSession`, projects to `MeResponse` shape with `trustMode: 'peer-conductor'`, returns 401 envelope on no-session) + 1 unit test + route declaration in `build_manifest()`
- **Rationale:** Makes Tauri's transport-β invariant true (same `/auth/me` call, same response shape, only base URL changes); also makes Mode B browser→doorway-routed→conductor work without doorway needing to translate `/session` → `/auth/me`
- **Risk:** Low — read-only projection of existing SQLite session data
- **Verification:** `curl http://localhost:8090/auth/me` against a running Tauri sidecar returns the expected `MeResponse`; no-session case returns 401 with `{authenticated: false}`

### Plan-level sequencing

All three Phase A tasks are independent and can run in parallel before Phase B begins. The plan's Task B3 (portal-shell) consumes A2 + A4 (server-side surfaces) and Task B4 (federated-resolver) consumes A3 (client-side helper). None of these tasks gates the others structurally; whichever finishes first lands first.

---

## Out-of-scope follow-ups (NOT included in this sprint)

- `/.well-known/elohim-doorway` endpoint on storage — portability polish; M5/M6 boundary work
- Anonymous PortalHost-lookup endpoint for §4.5's cross-portal list — substrate-policy question first
- Dynamic OAuth-client registration UI at `/admin/oauth-clients` — separate operator-UX sprint
- Recovery launcher as Lit primitive — explicitly deferred by the spec (§6)
- PortalHost authorize UI — explicitly deferred by the spec (§6)
