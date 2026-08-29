---
title: "The Keep — one client control surface over one Custodian and N Witnesses"
id: keep-client-control-surface
tier: spec
status: Draft
created: 2026-08-29
maintainers: Matthew Dowell + Claude Opus 5
class: substrate
context-tier: disclosed
steward: angular-architect
graduation-trigger: slices 1-4 landed AND `can.accountReads` flips true (slice 5's storage fix), OR superseded-by-implementation
domain: client SDK seam x identity/custody plane x peer plurality x doorway projection
habits: [operator-runtime-surface]
topic: [keep, sdk, client-surface, identity, storage, keystore, recovery, peer-plurality, custodian, witnesses, discovery]
cites:
  - "elohim-seam-map-concern-routing | The Elohim Seam Map | sha256:fd5ced9f996ff5af | path: genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md"
  - "substrate-trust-contract-runbook | The Substrate Trust Contract | sha256:e47d962ca7259c79 | path: genesis/docs/content/elohim-protocol/architecture/2026-07-12-substrate-trust-contract-runbook.md"
  - genesis/data/timeline/backlog/security-client-doorway-origin-synthesis-credential-exfil.md
  - genesis/data/timeline/backlog/security-doorway-oauth-redirect-uri-interception.md
  - genesis/data/timeline/backlog/security-unsigned-gossip-peer-becomes-jwt-trust-anchor.md
---

# The Keep

## Why this exists

Five client libraries — `@elohim/identity`, `@elohim/storage-client`, `@elohim/service`,
`@elohim/rea-runtime`, `@elohim/epr` — and **none of them composes another**. Nothing gives an app
identity *and* storage from one place; the session token crosses between them through a hardcoded
`localStorage` key string. `StorageClient`'s only auth is a static `apiKey` captured at construction
with no setter, so a refreshed bearer structurally cannot reach it. Three independent classes are
named `StorageClient`. Keystore and recovery have no client primitive at all. Three of the five are
built by no pipeline.

The ask this answers: **one control surface for login / identity / storage / keystore / recovery, so
a client app carries near-zero configuration** — while underneath, those primitives are held by a
pluralism of peers, because social-compute and network solidarity are what sovereignty means here.

## The structural finding

A Keep is **not** a symmetric set of peers. It is **one Custodian + N Witnesses**, and the asymmetry
is enforced by *type*, not by runtime policy, because the substrate makes it non-negotiable:

- **The bearer does not cross peers.** Doorways mint HS256 by default (`auth/jwt.rs:330`, `:343`;
  the EdDSA opt-in `DOORWAY_JWT_SIGN_ALG` is in no deployment manifest), and the foreign-kid branch
  hard-refuses non-EdDSA (`jwt.rs:569-574`).
- **The private key is process-local to one doorway.** Minted at registration, password-encrypted in
  that doorway's MongoDB, decrypted into that process's `DashMap` keyed by `session_id`, TTL 1h.
- **`humanId` is minted at one doorway** (`auth_routes.rs:838`, `uuid::Uuid::new_v4()`).

So identity, keys and documents are singular and bearer-pinned; content and topology are plural,
credential-free and `Answer<T>`-wrapped. Pluralism is expressed exactly where the substrate supports
it, and the type system stops an app assuming more. A doorway is the degenerate one-element case of
the Witness set, never of the Custodian.

**The invariant, enforced by construction:** a credential minted by the Custodian is transmitted
ONLY to the Custodian. Witness transport is a separate fetch path that cannot attach an
`Authorization` header.

## Housing

`app/elohim-library/projects/elohim-service/src/keep/`, exposed as `@elohim/service/keep`.
**Not a sixth package.**

Verified reasons, in order of weight:

1. **It is gated on landing with zero pipeline work.** `_gate-elohim-library` (justfile:371-373) runs `pnpm exec eslint projects/elohim-service/src …` then `cd app/elohim-library/projects/elohim-service && pnpm exec tsc --noEmit && pnpm exec vitest run`. That project's `vitest.config.ts` sets `root:'./src'`, `include:['**/*.spec.ts']`, `exclude:['node_modules','dist','resilience/**','distribution/**']` — a new `src/keep/*.spec.ts` is picked up automatically. Registered via `gate.projects.elohim-library` in `app/elohim-app/build-manifest.json` with `sources:["app/elohim-library/**"]`, so `just gate` and pre-push detect it. By contrast `@elohim/identity` and `@elohim/rea-runtime` appear in **no** justfile recipe at all — the `elohim-library` gate fires on their path and tests neither, which is worse than no gate. A new package would be the sixth artifact and would need three pipeline edits (`pnpm-workspace.yaml`, a `_gate-*` recipe, a `build-manifest.json` gate project) before CI touched it — and `@elohim/epr` proves a gated package in `elohim/sdk/` can still have zero importers repo-wide.

2. **It already owns the seam being extended.** `DoorwayAddressResolver` / `DoorwayResolution` / `DoorwayEndpoint` / `ConfiguredDoorwayResolver` / `gatewayCandidates` live at `src/client/doorway-address-resolver.ts:10-79`, with `'registration' | 'pkarr'` reserved at :10 and the header at :5-7 stating "Config is the first adapter; a registration/pkarr adapter can replace it without changing request retry or stickiness semantics." Slice 1 *is* that adapter. It also owns the only N-host failover implementation (`elohim-client.ts:584-593`) and the read/write asymmetry note (:483-487).

3. **Reachable from all three consumer workspaces today.** `@elohim/service` has no `exports` field, so subpaths resolve through tsconfig `paths` that already exist: `app/elohim-app/tsconfig.json:31` and `doorway/doorway-app/tsconfig.json:9` both map `@elohim/service/*` → `…/elohim-service/src/*`. `@elohim/service/keep` needs no manifest change. (doorway-app maps *only* the wildcard, which is fine — Keep is a subpath.)

4. **`@elohim/storage-client` is already a peerDependency** (`workspace:*`) with tsconfig paths to its `dist`, and `node_modules/@elohim/storage-client` is symlinked into the project. `@elohim/identity` is **not** — adding it in slice 3 is an explicit, listed step (dependency + tsconfig path + `pnpm install`), not an assumption.

Two conditions attached: (a) `src/keep/` must be framework-free — no `@angular/*` import — with the boundary proven by a spec copying the pattern at `elohim-identity/src/core.boundary.spec.ts:110-113`; the Angular provider stays in the existing `src/angular/`. (b) The `tsc --noEmit` leg does **not** type-check specs (`tsconfig.json` ends `"exclude":["node_modules","dist","**/*.spec.ts"]`) — a `tsconfig.spec.json` already exists in that project, so add it as a second leg in `_gate-elohim-library`.

## Status — 2026-08-29

**Slice 1 has landed** (`689c383d0`), library half only. `FederationPeerResolver` implements the
existing `DoorwayAddressResolver` seam; `DoorwayResolutionSource` gained `federation-gossip` so an
unsigned row stops being labelled `config`. `just gate elohim-library`: eslint clean, both tsc legs,
34 files / 815 tests (16 new).

Both of the housing conditions are now met: `keep/` is asserted framework-free by
`keep-boundary.spec.ts` (with a detector control, plus a check that every file under `keep/` is
reachable from the barrel — an unreachable file is one the boundary walk never visits), and the
`tsconfig.spec.json` leg was added to `_gate-elohim-library`, which had never type-checked a single
spec in this project.

**Slice 2 has landed.** `Answer<T>` and `readAuthDiscovery` — the discovery document's first
production reader anywhere (its only reader was an a2o step). Measured against both live mesh
doorways: alpha and apex both `present`, 10 endpoints, `pathDrift` NONE, `portalUrl` resolving on
the serving origin, and an unowned base correctly `absent(not-found)` rather than unreachable.

One design decision was taken AGAINST this document's own recommendation, and the reason belongs
here. The design proposed a fourth `refused` state on `Answer<T>`. The shared Rust contract
(`crates/seam-contracts/src/answer.rs`) argues explicitly for three, because "keeping the state set
at three is what lets every seam share one vocabulary", and pairs `Answer` with a reason enum
instead. The objection the fourth state answers is real — collapsing "this peer rejected who I am"
into `unreachable` misinforms, since `unreachable` means absence is NOT established — but it is
answered just as well by making the reason REQUIRED on every non-present variant.
`{state:'unreachable', reason:'refused'}` carries exactly what a fourth variant would, cannot be
produced by accident, and keeps one state set across both languages. Requiring the pairing is
closer to the Rust design than adding a variant would be.

**Not yet landed from slice 1:** the provider flip in `app/elohim-app/src/app/app.config.ts` that
binds `DOORWAY_ADDRESS_RESOLVER` to the register. It is one line and it is what makes the register
live; it is held because a push touching `app/elohim-app` is refused by that project's 590
pre-existing lint errors. Until it lands, the register is built, gated and unused.

## What is real today

- GET /api/v1/federation/doorways is live and returns a real peer set with per-row signature material — DoorwaySummary{id, url, identity_root, signing_key, endpoints[], record_serial, record_signature, region, tier, capabilities, status} (doorway/doorway-service/src/routes/federation.rs:33-45, handler :73). Rows sourced from the infrastructure DHT carry signatures; rows merged from HTTP gossip carry None (:112-131), so the trust label is derivable at parse time with no new server work.
- The resolver seam it plugs into is real and consumed in production: DoorwayAddressResolver{resolve(identity)} (elohim-service/src/client/doorway-address-resolver.ts:28-30), gatewayCandidates (:69), ConfiguredDoorwayResolver (:40), DOORWAY_ADDRESS_RESOLVER token (client/angular-provider.ts:28, provided :81). The interceptor injects it at api-base-url.interceptor.ts:222, already handles an async resolve (:293-296), and already builds its candidate ladder from gatewayCandidates (:236).
- A working mapper already exists and can be moved rather than invented: federationDoorwayResolution (doorway/doorway-app/src/app/services/doorway-federation.service.ts:145-160) already derives source:'registration' from record_signature presence and already maps endpoints[]. Only its keying, its throw-on-miss (:88-93) and its never-called warm path (:77-85) need fixing.
- AuthDiscovery is already generated INTO elohim-service and is currently dead code: src/generated/auth-discovery.ts:11, describing itself as 'the unauthenticated document an app reads INSTEAD of carrying auth configuration'. The route is live (doorway/doorway-service/src/server/http.rs:5202 → routes/auth_discovery.rs:136) with 10 fixed endpoint paths (AuthEndpoints::current(), :87-99) and PORTAL_PATH = '/threshold/login' (:118). Its only reader repo-wide is genesis/a2o/steps/auth/auth-discovery.steps.ts:62.
- DoorwaySessionClient is a complete, framework-free session client with all eight auth calls implemented: login :213, register :220, logout :230, me :239, refresh :244, requestSessionToken :254, exchangeSessionToken :263, restoreSession :285, plus the SessionTokenStore seam :114-118 and InMemorySessionTokenStore :121. Keep composes it; it does not reimplement it.
- StorageClient's blob and Automerge surfaces are real and typed: getBlob :270, putBlob :228, blobExists :313, getManifest :345, listDocuments :124, getDocument :138, getHeads :148, getChangesSince :162, applyChanges :177, plus the public escape hatches getJson :368 / postJson :359 / getStatusDiscardingBody :382, and AutomergeSync load/save/sync/exists/forget (sync.ts:77,116,144,183,198).
- Three unauthenticated, doorway-reachable peer/custody routes are declared in the storage→doorway route manifest: GET /api/v1/peers/delivery (elohim/elohim-storage/src/http.rs:15487-15492, no .auth_required(), cache_ttl 10), GET /api/v1/blob/{hash}/distribution/details (:15776-15781, explicitly no auth so the public surface stays addressable), GET /p2p/status (:13768). These are what Witnesses can honestly answer with today.
- A production client already does peer-plural resolution through the origin: the Service Worker fetches /api/v1/peers/delivery, scores the set and tries peers in rank order before falling back to the doorway (app/elohim-app/src/apps-sw.ts:119, :73-83, :213-238, :239). It is real — with two caveats named below.
- The three-state answer vocabulary is a shared, tested Rust contract ready to port: Answer<T>{Present, Absent (observed, never assumed), Unreachable} at crates/seam-contracts/src/answer.rs:88-96, with the module note at :80-85 that refusal is a paired reason label rather than a fourth in-process variant.
- The gate is real and already fires on the housing path: _gate-elohim-library (justfile:371-373) runs eslint + tsc --noEmit + vitest run in projects/elohim-service, and that project's vitest.config.ts includes **/*.spec.ts under src/.

## What is declared absent — and says so to the caller

These are named by the surface and refused at the call site, rather than omitted and rediscovered at
runtime. The rule: a facet that cannot be honoured **throws without issuing a request**, and a
`can.*` flag lets an app read the posture.

- CROSS-PEER AUTH. `can.crossPeerAuth: false`. Doorways default to HS256 (JwtSignAlg::Hs256, doorway/doorway-service/src/auth/jwt.rs:330,:343; the EdDSA opt-in DOORWAY_JWT_SIGN_ALG is in no deployment manifest) and the foreign-kid branch hard-refuses any non-EdDSA algorithm as UnknownIssuer (:569-574). A session minted by peer A is cryptographically unusable at B and C. The surface signals this by making `custodian` singular by TYPE — there is no method that fans an authenticated call — and by `KeepPeer.acceptsMySession`, false for every non-custodian.
- ACCOUNT READS. `can.accountReads: false`; every AccountApi method throws KeepAbsentError WITHOUT issuing a request. This is not caution, it is correctness: extract_agent_key (elohim/elohim-storage/src/api/account.rs:954) falls back to the pod's active local session when X-Agent-Id is absent, its own doc at :946-949 says the generic middleware never sets that header, the doorway proxy injects only X-Agent-Cid (routes/storage_proxy.rs:456), and every hosted pod HAS a local session (services/genesis_self_heal.rs:162-163). Issuing the request returns the node operator's key history as yours, wrapped in an Attribution asserting it is yours.
- ACCOUNT WRITES. `can.accountWrites: false`. verify_caller_owns_cell compares the caller's key against the storage node's OWN cell key and otherwise returns 503 BROWSER_WRITE_PATH_PENDING (account.rs:522-530, body :532-546: 'Self-sovereign writes require a peer the human controls'). More peers does not unblock this; a peer the human controls does.
- SOCIAL RECOVERY. `can.socialRecovery: false`; no `recovery` facet is exposed at all. /auth/recover-custody, /auth/check-recovery-status and /auth/activate-recovery validate input then return 501 (doorway/doorway-service/src/routes/auth_routes.rs:3032-3040 and the two following handlers). The Angular client's ten /api/recovery/* calls hit routes served by nothing. The imagodei DNA carries create_recovery_request / commit_key_rotation / create_shamir_custody_setup / upsert_recovery_hint with no HTTP caller.
- KEYSTORE. `can.localKeys: false`. No browser key generation, import, unlock, rotation or signing exists anywhere. The hosted human's key is minted at the doorway, password-encrypted in that doorway's MongoDB (custodial_keys/mod.rs:8-11; db/schemas/user.rs:184-187) and decrypted into that process's DashMap keyed by session_id, TTL 1h (custodial_keys/cache.rs:39-44). NOT FOUND: any TypeScript type named KeyExportFormat — grep across app/, elohim/sdk/, doorway/doorway-app/, genesis/ returns zero hits. GET /auth/export-key exists (auth_routes.rs:2451) and is absent from AuthEndpoints::current(), i.e. owned-but-undiscoverable, so Keep will not call it from a guessed path with an invented return type.
- PEER-RECORD VERIFICATION. `can.verifiedPeerSet: false`; verifyPeerRecord() returns 'unverifiable' unconditionally. The canonical bytes are Rust-only: pkarr_bridge::canonical_record_bytes (bridges/pkarr/pkarr-bridge/src/lib.rs:55, domain tag :9, verify_record :87). @elohim/epr ships verifyEd25519 (elohim/sdk/epr-ts/src/index.ts:4) but no port of the encoding. The function ships anyway so the missing prerequisite is compile-time visible rather than a footnote.
- CONTENT VERIFICATION. `can.verifiedContentReads: false`; witnesses.readContent throws until slice 4. Attribution.verified is a TWO-value union ('unverified' | 'content-hash') — 'signature' is deliberately absent because the server-side slice check is base64-decodability plus a length range with no key and no verify (elohim/elohim-storage/src/services/federator.rs:178-186). 'content-hash' needs a raw SHA-256 verifier that does not exist in TypeScript: @elohim/epr's computeCid is CIDv1 dag-cbor over canonical EPR bytes (epr-ts/src/cid.ts:5-11) and cannot verify a BlobManifest.blob_hash (types.ts:114-116).
- QUORUM READS. `can.quorumReads: false`. No HTTP route returns per-peer signed slices — ViewSlice{peerId, viewKind, freshness, payload, signature} (elohim/sdk/storage-client-ts/src/generated/ViewSlice.ts:11) rides only the libp2p /elohim/view-federation/1.0.0 plane, and Federator::query's per-peer results (services/federator.rs:74-81) are folded into an anonymous aggregate before any HTTP client sees them.
- BROWSER→PEER DIRECT FETCH. The Service Worker 'existence proof' constructs http://{ip}:{httpPort} from the peer's multiaddr (app/elohim-app/src/apps-sw.ts:92-93), so on the deployed HTTPS origin every one of those fetches is blocked as mixed content and the loop falls through to the doorway on every request. It also dials /apps/{identifier}/{filePath} (:222), never /blob/{hash}. Keep does not inherit a blob capability from it, and `UnreachableReason` carries 'mixed-content-blocked' so the failure is nameable rather than silent.
- PLURAL DOC SYNC. AutomergeSync's watermark is `Map<docId, string[]>` keyed by docId alone (sync.ts:66) and overwritten from the answering server's new_heads (:132); the server drops unparseable have-hashes with a filter_map then calls save_after (elohim/elohim-storage/src/sync/mod.rs:122-141). Routing doc traffic to two peers corrupts sync state silently, so DocsApi lives on `custodian` by type until the watermark is keyed by (peer, docId).
- PLURAL WRITES. There is no write fan-out and no idempotency key at the protocol layer. The reason is already written down in-tree: retrying a non-idempotent write on another host risks a duplicate (elohim-client.ts:483-487; RETRIABLE_METHODS = GET|HEAD at api-base-url.interceptor.ts:32). writeContent (slice 4) returns the accepting peer in its Attribution so read-after-write can be pinned with `preferPeer` rather than fanned.
- SESSION HANDOFF. requestSessionToken/exchangeSessionToken are NOT on the surface. The transfer token is a bare UUID with a 60s TTL, no audience and no target identity in SessionTransferEntry, redeemed by an UNAUTHENTICATED GET with the secret in a query string (auth_routes.rs:4171-4213, :4219-4240) — it leaks to Referer, history and every proxy log. Exposing it as first-class ergonomics would route around /auth/authorize, the one flow with a redirect_uri matcher.
- OAUTH PKCE. `can.oauthPkce: false`, surfaced so an app can see the posture rather than assume it. A repo-wide grep for code_challenge|pkce|PKCE across doorway/**/*.rs returns zero hits, while get_registered_clients() hardcodes http://localhost:* and http://127.0.0.1:* with trusted:true (db/schemas/oauth_session.rs:140-148).
- GENERATED-TYPE REACH. Of the types a full Keep wants, only Freshness is in the generated barrel. AccountView, KeyRotationView, PeerTopologyView, PeerHouseholdEdge, MyClusterView, FreshnessState, ViewSlice, ReplicaPeer and DistributionDetails are all ABSENT from elohim/sdk/storage-client-ts/src/generated/index.ts (281 export lines over 458 files) AND from dist/generated/index.d.ts, and the exports map (package.json:8-28) has no ./generated/* subpath. Slices 1-3 import none of them; slice 4 lists the barrel fix as a prerequisite, not a detail.
- PEER-SET TYPE GENERATION. DeliveryPeer is #[derive(Debug, Clone, Serialize)] with NO ts-rs derive (elohim/elohim-storage/src/p2p/mod.rs:423-426, contrast PeerListView at :1179-1182), so the one peer-set type a browser consumes exists as two divergent hand-written TS copies (apps-sw.ts:52-59 and elohim-service/src/cache/content-resolver.ts:36-43). Keep declares FederationDoorwayRow once and does not mint a third copy.

## The surface

```typescript
// =============================================================================
// @elohim/service/keep  —  the Keep control surface
//
// Housing: app/elohim-library/projects/elohim-service/src/keep/
// Reachable as `@elohim/service/keep` from all three consumer workspaces
// through the tsconfig `paths` that already exist:
//   app/elohim-app/tsconfig.json:31       "@elohim/service/*"
//   doorway/doorway-app/tsconfig.json:9   "@elohim/service/*"  (wildcard only)
//
// Why here and not a sixth package: `_gate-elohim-library` (justfile:371-373)
// already runs `eslint … && pnpm exec tsc --noEmit && pnpm exec vitest run`
// inside projects/elohim-service, and that project's vitest.config.ts has
// `root:'./src', include:['**/*.spec.ts']`, so a new src/keep/**.spec.ts is
// gated on landing with ZERO pipeline edits. @elohim/identity and
// @elohim/rea-runtime appear in NO justfile recipe at all.
//
// THE ONE STRUCTURAL IDEA, and it is a correction of the winning design:
// a Keep is NOT a symmetric set of peers. It is
//     ONE Custodian  (holds my key material, my UserDoc, my session, my chain)
//   + N Witnesses    (may answer content and topology; hold none of the above)
// The asymmetry is enforced by TYPE, not by runtime policy, because the
// substrate makes it non-negotiable — see the three citations under Custodian.
// =============================================================================

export async function openKeep(opts?: KeepOptions): Promise<Keep>;

export interface KeepOptions {
  /** Defaults to globalThis.location.origin — the one fact a page always knows
   *  (doorway/doorway-service/src/routes/auth_discovery.rs:1-6). Off-browser
   *  (Tauri, Node, a2o) this is the ONLY field a caller sets. */
  origin?: string;
  /** Defaults to globalThis.fetch — mirrors doorway-session-client.ts:203. */
  fetchImpl?: typeof fetch;
  /** Defaults to InMemorySessionTokenStore
   *  (elohim-identity/src/lib/doorway-session-client.ts:121). */
  store?: SessionTokenStore;
}

export interface Keep {
  readonly custodian: Custodian;
  readonly witnesses: Witnesses;
  readonly can: KeepCapabilities;
}

// -----------------------------------------------------------------------------
// 1. CUSTODIAN — singular by type. No Answer<T>. No fan-out. Bearer pinned.
// -----------------------------------------------------------------------------
//
// Three verified facts make this singular, permanently:
//
// (a) THE BEARER DOES NOT CROSS PEERS. Doorways mint HS256 by default
//     (JwtSignAlg::Hs256 at doorway/doorway-service/src/auth/jwt.rs:330 and
//     :343; the EdDSA opt-in env var DOORWAY_JWT_SIGN_ALG appears in zero
//     deployment manifests). A token presented to a sibling takes the
//     foreign-kid branch, which hard-refuses non-EdDSA: jwt.rs:569-574
//     `if header.alg != Algorithm::EdDSA { … return Err(JwtError::UnknownIssuer) }`.
//
// (b) THE PRIVATE KEY IS PROCESS-LOCAL TO ONE DOORWAY. Generated at
//     registration, password-encrypted in that doorway's MongoDB
//     (custodial_keys/mod.rs:8-11; db/schemas/user.rs:184-187), decrypted into
//     that process's DashMap keyed by session_id, TTL 1h
//     (custodial_keys/cache.rs:39-44).
//
// (c) humanId IS MINTED AT ONE DOORWAY. auth_routes.rs:838
//     `uuid::Uuid::new_v4().to_string()`, with a divergent SHA-256 derivation
//     on the dev path (:953-963). Two doorways produce two ids for one person.
//
// A "keyed multi-session store" would let one browser hold three sessions with
// three humanIds for what the human believes is one self. Refused.

export interface Custodian {
  /** Fixed at openKeep() from `origin`. NEVER sourced from the peer set, never
   *  from a response body, never from a federated identifier's domain part. */
  readonly origin: string;

  /** Verbatim GET {origin}/.well-known/elohim-auth, or null when it 404s /
   *  fails assertOriginRelative. Type is already generated and currently dead:
   *  elohim-service/src/generated/auth-discovery.ts:11. This is its FIRST
   *  production reader — today the only fetch repo-wide is a test step
   *  (genesis/a2o/steps/auth/auth-discovery.steps.ts:62). */
  readonly discovery: AuthDiscovery | null;

  /** Non-fatal drift report. connect() does NOT throw at boot: it compares
   *  discovery.endpoints against the paths DoorwaySessionClient hardcodes
   *  ('/auth/login' at doorway-session-client.ts:214, '/auth/me' :240, …) and
   *  reports mismatches. The document is a CHECK, not yet the SOURCE, because
   *  DoorwaySessionClientOptions (:180-187) accepts only baseUrl/fetchImpl/
   *  tokenStore — there is no path map to override. Claiming "every path
   *  discovered" while composing that client would be false. */
  readonly pathDrift: readonly PathDrift[];

  readonly session: SessionApi;
  readonly docs: DocsApi;
  readonly account: AccountApi;
}

export interface PathDrift {
  endpoint: 'register' | 'login' | 'logout' | 'refresh' | 'me'
          | 'authorize' | 'token' | 'sessionToken' | 'exchangeSession' | 'portalHost';
  advertised: string;
  clientUses: string;
}

// --- session: plain returns, plain throws. No three-way branch to log in. ----
// Every member is one existing DoorwaySessionClient method.

export interface SessionApi {
  /** doorway-session-client.ts:208 */
  readonly token: string | null;
  /** :213 POST /auth/login  — throws DoorwaySessionError on failure */
  signIn(req: LoginRequest): Promise<StoredSession>;
  /** :220 POST /auth/register */
  signUp(req: RegisterRequest): Promise<StoredSession>;
  /** :230 POST /auth/logout — clears the local store even when the fetch fails */
  signOut(): Promise<void>;
  /** :244 POST /auth/refresh. Triggers the internal StorageClient rebuild —
   *  StorageClient.apiKey is constructor-captured with no setter
   *  (client.ts:45; five reads at :76,:234,:274,:317,:386). */
  refresh(): Promise<StoredSession>;
  /** :285 local, expiry-checked, no round trip */
  restore(): StoredSession | null;

  /** :239 GET /auth/me → MeResponse. The ONLY source of authority/trustMode/
   *  conductorEndpoint (elohim-identity/src/generated/me-response.ts:39-44).
   *  AuthResponse does NOT carry them (generated/auth-response.ts:8-49), so
   *  they are NOT on StoredSession and are NOT populated by signIn. Callers
   *  that need them pay this extra round trip explicitly. */
  me(): Promise<MeResponse>;

  /** Resolved from discovery.portal (auth_discovery.rs:112; PORTAL_PATH =
   *  "/threshold/login" at :118) against `origin`. Replaces the four inlined
   *  '/threshold/login' literals in doorway-app. Does NOT replace an app's own
   *  '/identity/login' route — that is app routing, and there are 14 of those.
   *  Returns null when discovery is null. */
  portalUrl(returnUrl?: string): string | null;
}

// --- docs: pinned to the custodian by TYPE, and this is a correctness fix ----
// AutomergeSync holds `private knownHeads: Map<docId, string[]>` (sync.ts:66)
// keyed by docId ALONE, overwritten from the server's reported new_heads on
// every save/sync (:132). Route doc traffic to two peers and peer A's watermark
// is sent as `have=` to peer B, whose get_changes_since drops unparseable
// hashes with a filter_map and calls save_after with heads it has never seen
// (elohim/elohim-storage/src/sync/mod.rs:122-141). Sync state corrupts silently.
// Until the watermark is keyed by (peer, docId), the doc plane is custodian-only.

export interface DocsApi {
  load<T>(docId: string): Promise<import('@automerge/automerge').Doc<T>>;  // sync.ts:77
  save<T>(docId: string, doc: import('@automerge/automerge').Doc<T>): Promise<string[]>; // :116
  sync<T>(docId: string, doc: import('@automerge/automerge').Doc<T>): Promise<SyncResult<T>>; // :144
  exists(docId: string): Promise<boolean>;   // :183
  forget(docId: string): void;               // :198
  list(options?: ListOptions): Promise<ListDocumentsResponse>; // client.ts:124
}

// --- account: DECLARED ABSENT, and this is the sharpest gap in the tree ------
// The four "obvious" reads — key history, revocations, pending recovery,
// AccountView — MUST NOT be shipped, because they do not fail: they return the
// STORAGE POD'S OWN HUMAN'S data and a naive client renders it as yours.
//
//   extract_agent_key (elohim/elohim-storage/src/api/account.rs:954) resolves
//   1. `X-Agent-Id` header — its own doc at :946-949 says this is set only by
//      doorway's bespoke portal-host handlers, "NOT by generic middleware"
//   2. active local session's agent_pub_key  ← the silent substitution
//   The generic proxy injects only `X-Agent-Cid`
//   (doorway/doorway-service/src/routes/storage_proxy.rs:456).
//   Every hosted pod HAS an active local session:
//   services/genesis_self_heal.rs:162-163
//     `if !db::local_sessions::has_any_session(conn)? { create_session(…) }`
//
// Cross-identity misattribution in the one design whose thesis is attribution.
// Refused until storage resolves the account caller through extract_agent_cid
// (account.rs:1000+) the way peer-topology and cluster already do.

export interface AccountApi {
  readonly status: Absent;
  /** All throw KeepAbsentError WITHOUT issuing a request. */
  get(): Promise<never>;
  keys(): Promise<never>;
  revocations(): Promise<never>;
  pendingRecovery(): Promise<never>;
  /** Writes are separately refused: verify_caller_owns_cell compares against
   *  the storage node's OWN cell key (account.rs:522-530) → 503
   *  BROWSER_WRITE_PATH_PENDING (body builder :532-546). */
  selfRevoke(): Promise<never>;
  voteOnRecovery(): Promise<never>;
}

// -----------------------------------------------------------------------------
// 2. WITNESSES — plural by type. Answer<T>. Credential-free. Hash-verified.
// -----------------------------------------------------------------------------
//
// THE INVARIANT, stated once and enforced by construction:
//   A credential minted by the Custodian is transmitted ONLY to the Custodian.
// Witness transport is a SEPARATE fetch path that cannot attach an
// Authorization header. This is not policy; it is why `witnesses` never
// receives the session client and never receives the credentialed
// StorageClient. A gossiped peer row must never see the human's bearer — the
// token is a plain replayable bearer verified by signature alone
// (auth_routes.rs:4166 `jwt.verify_token(token)`; foreign-kid path jwt.rs:576-585
// is pubkey-only, no audience), so one leak is account takeover at the real doorway.

export interface Witnesses {
  /** The peer register, with trust labelled per row. */
  list(): Promise<Answer<KeepPeer[]>>;

  /** Ask ONE named peer, deliberately. Credential-free by construction. */
  ask<T>(peer: KeepPeer, path: string): Promise<Answer<T>>;

  /** WHO HOLDS THIS. GET /api/v1/blob/{hash}/distribution/details — declared
   *  in the storage→doorway manifest WITHOUT .auth_required()
   *  (elohim/elohim-storage/src/http.rs:15776-15781), so it is the most legible
   *  pluralism affordance available today with no credential involved.
   *  Return typed `unknown` until DistributionDetails is added to the generated
   *  barrel — see whatIsDeclaredAbsent. */
  custodyOf(hashOrCid: string): Promise<Answer<unknown>>;

  /** GET /api/v1/network/posture — StorageClient.getNetworkPosture (client.ts:215) */
  posture(): Promise<Answer<NetworkPostureView>>;

  /** GET /p2p/status — DataplaneApi.getP2pStatus (api/dataplane.ts:155);
   *  route declared at http.rs:13768. */
  status(): Promise<Answer<unknown>>;

  /** Content read across witnesses. SLICE 4 — gated, and the gate is real:
   *  `can.verifiedContentReads` is false until a raw SHA-256 verifier is
   *  exported from @elohim/epr. @elohim/epr's computeCid is CIDv1 dag-cbor over
   *  canonical EPR bytes (elohim/sdk/epr-ts/src/cid.ts:5-11) and CANNOT verify a
   *  BlobManifest.blob_hash, which is a raw content hash (types.ts:114-116).
   *  Until then this throws KeepAbsentError rather than returning unverified
   *  bytes from an unsigned peer. */
  readContent(hashOrCid: string, opts?: { preferPeer?: KeepPeer }): Promise<Answer<Uint8Array>>;
}

/** NOT offered: witnesses.connectedPeers(). StorageClient.listConnectedPeers
 *  (client.ts:201) hits /p2p/peers, which is absent from the storage→doorway
 *  route manifest (verified by scanning build_manifest(), http.rs 13717-16350,
 *  while its sibling /p2p/status IS present at :13768). Node-direct only. */

export interface KeepPeer {
  /** Stable identity when the row is signed; the operator-chosen slug otherwise. */
  readonly id: string;
  readonly origin: string;
  /** From a signed row only. */
  readonly identityRoot?: string;
  readonly signingKey?: string;

  /** THE FIELD @elohim/origin's PeerRef had and the winning design dropped.
   *  Derived at parse time, never inferred from absence:
   *    'dht-notarized'   → record_signature && signing_key && record_serial present
   *    'unsigned-gossip' → everything else
   *  Half of GET /api/v1/federation/doorways is HTTP gossip: fetch_single_peer
   *  (doorway/doorway-service/src/services/federation.rs:1039-1058) deserializes
   *  only {id,url,region,capabilities,status} from ANY configured peer's
   *  response — no signature parsed, no allowlist — and the doorway re-serves
   *  those rows with `identity_root: None, signing_key: None,
   *  record_signature: None, tier: "Federated"` (routes/federation.rs:112-131).
   *  Those rows are UNSIGNABLE BY CONSTRUCTION: there is nothing for a future
   *  verifier to check. */
  readonly trust: 'dht-notarized' | 'unsigned-gossip';

  /** Always false except for the Custodian. Encodes fact (a) above: a session
   *  minted by peer A is cryptographically refused at peers B and C today. */
  readonly acceptsMySession: boolean;

  readonly source: 'discovery' | 'federation-signed' | 'federation-gossip' | 'config';
  readonly endpoints: readonly DoorwayEndpoint[];  // elohim-service/src/client/doorway-address-resolver.ts:12-20
}

/** Verify a signed row against its own signing key.
 *  RETURNS 'unverifiable' UNCONDITIONALLY TODAY, and shipping it that way is
 *  the point: the missing prerequisite becomes a compile-time visible fact.
 *  The canonical bytes are Rust-only — pkarr_bridge::canonical_record_bytes
 *  (bridges/pkarr/pkarr-bridge/src/lib.rs:55, domain tag at :9, verify_record
 *  at :87), which is what the DNA validator itself runs. @elohim/epr ships
 *  verifyEd25519 (elohim/sdk/epr-ts/src/index.ts:4) — the signature half — but
 *  no port of the canonical encoding. Returning 'verified' would be a lie. */
export function verifyPeerRecord(peer: KeepPeer): 'verified' | 'unverifiable';

// -----------------------------------------------------------------------------
// 3. Answer<T> — FOUR states, not three, and the fourth is load-bearing
// -----------------------------------------------------------------------------
// Ported from crates/seam-contracts/src/answer.rs:88-96 (Present / observed
// Absent / Unreachable). That module's own note at :80-85 says refusal is a
// REASON paired with the Answer, not a fourth variant — but it is describing an
// in-process Rust seam. Over HTTP, "the peer answered and rejected who I am" is
// the single most likely three-peer disagreement in this substrate (fact (a)
// above), and collapsing it into `unreachable` is an active lie: `unreachable`
// is specified as "absence NOT established", so a client reading 2-of-3
// unreachable concludes the network is degraded when identity is simply
// non-portable. `refused` is that fourth state, carrying the reason label the
// Rust contract asks for.

export type Answer<T> =
  | { state: 'present';     value: T;             from: Attribution }
  | { state: 'absent';      reason: AbsentReason; from: Attribution }
  | { state: 'refused';     reason: RefusedReason; status: number; from: Attribution }
  | { state: 'unreachable'; reason: UnreachableReason; tried: readonly Attribution[] };

export type AbsentReason      = 'not-found' | 'empty-projection';
export type RefusedReason     = 'unknown-issuer' | 'expired' | 'forbidden'
                              | 'identity-mismatch' | 'not-implemented' | 'other';
export type UnreachableReason = 'network' | 'timeout' | 'mixed-content-blocked'
                              | 'no-candidates';

export interface Attribution {
  /** The origin actually fetched. Degenerate (constant) with one witness. */
  readonly origin: string;
  /** libp2p PeerId when the answering route names one. Undefined today: NO
   *  HTTP route self-identifies. ViewSlice{peerId,…,signature}
   *  (elohim/sdk/storage-client-ts/src/generated/ViewSlice.ts:11) rides only the
   *  libp2p /elohim/view-federation/1.0.0 plane, and every HTTP route returns
   *  the folded aggregate. */
  readonly peerId?: string;
  readonly trust: 'dht-notarized' | 'unsigned-gossip';

  /** WHICH IDENTITY THE PEER RESOLVED. This is the axis on which three peers
   *  actually disagree, and no other design carries it. `auth_required` in the
   *  doorway route registry is declared and never read on the dispatch path
   *  (doorway/doorway-service/src/routes/catching_up.rs:168-171 names the trap;
   *  backlog genesis/data/timeline/backlog/doorway-auth-required-metadata-unenforced.md).
   *  So an unauthenticated peer answers 200 as somebody else, not 401.
   *  Keep MUST downgrade any identity-scoped answer whose resolvedIdentity !==
   *  session.humanId to {state:'refused', reason:'identity-mismatch'}. */
  readonly resolvedIdentity: string | 'anonymous' | 'unknown';

  /** Did WE check these bytes, or are we trusting the answerer?
   *  Two values today, not three. 'signature' is deliberately absent: the
   *  server-side "signed slice" check is base64-decodability plus a length
   *  range, with no key and no verify
   *  (elohim/elohim-storage/src/services/federator.rs:178-186), so there is
   *  nothing a client could inherit. 'content-hash' is producer-only — set by
   *  Keep after computing the digest itself, never copied from a peer claim. */
  readonly verified: 'unverified' | 'content-hash';
}

/** DELIBERATELY NOT ON Attribution: `freshness`. The federator simply
 *  propagates the freshness the answering peer reported about ITSELF
 *  (federator.rs:126-131), and it is absent entirely from every route plurality
 *  touches first. A peer's self-report about its own trustworthiness must not
 *  sit in a field named like an adjudication. */

// -----------------------------------------------------------------------------
// 4. Capabilities — flat booleans an app branches on, plus evidence
// -----------------------------------------------------------------------------

export interface KeepCapabilities {
  /** true — GET /api/v1/peers/delivery is unauthenticated and doorway-reachable
   *  (elohim/elohim-storage/src/http.rs:15487-15492, no .auth_required()). */
  readonly peerSet: boolean;
  /** true — /api/v1/blob/{hash}/distribution/details, no auth (http.rs:15776-15781) */
  readonly custodyPerCid: boolean;
  /** false — see verifyPeerRecord */
  readonly verifiedPeerSet: false;
  /** false — no raw-SHA-256 blob verifier exists in TypeScript */
  readonly verifiedContentReads: false;
  /** false — no HTTP route returns per-peer signed slices */
  readonly quorumReads: false;
  /** false — the bearer does not cross peers (jwt.rs:569-574) */
  readonly crossPeerAuth: false;
  /** false — extract_agent_key cannot see a browser caller (account.rs:946-954) */
  readonly accountReads: false;
  /** false — verify_caller_owns_cell (account.rs:522-530) */
  readonly accountWrites: false;
  /** false — no browser key generation exists anywhere */
  readonly localKeys: false;
  /** false — the three doorway recovery routes 501 after validating input
   *  (doorway/doorway-service/src/routes/auth_routes.rs:3032-3040 and the two
   *  following handlers) */
  readonly socialRecovery: false;
  /** false — /auth/authorize + /auth/token have no PKCE anywhere
   *  (repo-wide grep for code_challenge|pkce over doorway/**.rs: zero hits) */
  readonly oauthPkce: false;
}

export interface Absent { readonly available: false; readonly reason: string; readonly evidence: string; }
export declare class KeepAbsentError extends Error {
  constructor(capability: string, evidence: string);
  readonly capability: string;
  readonly evidence: string;
}

// -----------------------------------------------------------------------------
// 5. The resolver — SLICE 1, and the only part that ships first
// -----------------------------------------------------------------------------

/** Federation-backed peer register. Implements the EXISTING seam
 *  (elohim-service/src/client/doorway-address-resolver.ts:28-30) so the
 *  interceptor consumes it with zero call-site changes:
 *  api-base-url.interceptor.ts:222 injects DOORWAY_ADDRESS_RESOLVER, :293-296
 *  handles a Promise return, :236 calls gatewayCandidates(resolution).
 *
 *  THREE HARDENINGS over the doorway-app original this replaces
 *  (doorway/doorway-app/src/app/services/doorway-federation.service.ts:72-160):
 *
 *  1. MULTI-ALIAS INDEX. The original keys only by
 *     `identity_root ?? signing_key ?? id` (:145). The interceptor asks with
 *     `doorwayIdentity ?? doorwayUrl ?? effectivePrimary` (:215-216) — and
 *     doorwayIdentity is set in NO environment file (declared optional at
 *     environment.types.ts:59, referenced only at app.config.ts:98 and the
 *     interceptor), so the interceptor always asks with a URL. Every row is
 *     therefore indexed under identityRoot, signingKey, id, `url`, AND every
 *     endpoint url — normalized via normalizeDoorwayUrl (:82).
 *
 *  2. NEVER THROWS. The original throws on a miss (:88-93) and the interceptor
 *     has no fallback (:293-300 catches and rethrows), so a miss kills every
 *     request on ['/api/','/db/','/blob/','/apps/','/health'] (:26). This falls
 *     back to ConfiguredDoorwayResolver (:40), i.e. to today's exact behaviour.
 *
 *  3. WARMS ITSELF. The original's map is only populated by loadDoorways()
 *     (:77-85), which nothing on the interceptor path ever calls — so it is
 *     empty on the first request regardless of keying. resolve() returns a
 *     Promise on cold start (the interceptor already handles that) and caches. */
export class FederationPeerResolver implements DoorwayAddressResolver {
  constructor(opts: {
    /** Same-origin GET /api/v1/federation/doorways
     *  (doorway/doorway-service/src/routes/federation.rs:73). */
    fetchImpl?: typeof fetch;
    /** Used verbatim on a miss and on a fetch failure. */
    fallback: ConfiguredDoorwayResolver;
    /** Re-fetch interval; 0 disables. Default 300_000. */
    ttlMs?: number;
  });
  resolve(identity: string): DoorwayResolution | Promise<DoorwayResolution>;
  /** The register, trust-labelled. Feeds Witnesses.list() in slice 4. */
  peers(): Promise<readonly KeepPeer[]>;
}

/** Pure mapper, moved from doorway-app and given the trust flag.
 *  The original already derives `source: record_signature?.length ?
 *  'registration' : 'config'` (doorway-federation.service.ts:147) — this keeps
 *  that and adds the explicit trust label plus the alias list. */
export function federationRowToPeer(row: FederationDoorwayRow): KeepPeer;
export function peerAliases(peer: KeepPeer): string[];

/** Wire shape of one row of GET /api/v1/federation/doorways. Hand-declared
 *  because DoorwaySummary (doorway/doorway-service/src/routes/federation.rs:33-45)
 *  is `#[derive(Serialize)]` with NO ts-rs export. Declared here once rather
 *  than as a third copy — doorway-app's FederationDoorway becomes an alias. */
export interface FederationDoorwayRow {
  id: string;
  url: string;
  identity_root?: string | null;
  signing_key?: string | null;
  endpoints?: { service: string; url: string; priority: number; ttl_secs?: number }[] | null;
  record_serial?: number | null;
  record_signature?: number[] | null;
  region?: string | null;
  tier: string;
  capabilities: string[];
  status: string;
}
```

## Slices

### Slice 1 — The peer register — federation-backed resolver with trust labels, behind the DI token the interceptor already injects

**Delivers.** FederationPeerResolver implements the existing DoorwayAddressResolver seam (doorway-address-resolver.ts:28-30), fetches GET /api/v1/federation/doorways (federation.rs:73), and emits DoorwayResolution plus a trust-labelled KeepPeer[] — trust:'dht-notarized' only when record_signature && signing_key && record_serial are present, 'unsigned-gossip' otherwise (the merge path sets all three to None at federation.rs:112-131). Provider flip in app.config.ts binds DOORWAY_ADDRESS_RESOLVER (angular-provider.ts:28) to it, so ALL traffic on ['/api/','/db/','/blob/','/apps/','/health'] (api-base-url.interceptor.ts:26) resolves against the DHT-known doorway set instead of the one-element hardcoded environment.prod.ts:29 array — with ZERO changes to any service, component or call site, because :236 already calls gatewayCandidates(resolution) and :293-296 already handles an async resolve. Fixes all three refuted defects in the original slice-1 proposal: (1) multi-alias index over identityRoot/signingKey/id/url/every endpoint url, because the interceptor asks with a URL (identity = doorwayIdentity ?? doorwayUrl ?? effectivePrimary at :215-216, and doorwayIdentity is set in no environment file); (2) NEVER throws — falls back to ConfiguredDoorwayResolver (:40) on a miss or a fetch failure, so a resolution failure degrades to today's behaviour instead of killing every request (the original throws at doorway-federation.service.ts:88-93 and the interceptor rethrows at :293-300); (3) self-warming and cached, since the original's map was only filled by loadDoorways() which nothing on the interceptor path calls. doorway-app's DoorwayFederationService becomes a thin delegate — a copy DELETED, not added. Also adds the tsconfig.spec.json leg to _gate-elohim-library so specs are type-checked, not merely run.

**Proven by.** app/elohim-library/projects/elohim-service/src/keep/peer-register.spec.ts, run by `just gate elohim-library` (_gate-elohim-library, justfile:371-373). Four assertions, each pinned to a refuted defect: (a) an unknown identity returns the configured primary and NEVER throws; (b) resolving by raw URL, by id, and by identity_root all return the same row; (c) a row with record_signature:null yields trust:'unsigned-gossip' and source:'federation-gossip'; (d) a failing fetch degrades to the ConfiguredDoorwayResolver result. Plus keep-boundary.spec.ts, copying elohim-identity/src/core.boundary.spec.ts:110-113, asserting zero @angular/* in src/keep's transitive import closure.

**Files.**
  - `app/elohim-library/projects/elohim-service/src/keep/peer-register.ts`
  - `app/elohim-library/projects/elohim-service/src/keep/peer-register.spec.ts`
  - `app/elohim-library/projects/elohim-service/src/keep/keep-boundary.spec.ts`
  - `app/elohim-library/projects/elohim-service/src/keep/index.ts`
  - `app/elohim-library/projects/elohim-service/src/index.ts`
  - `app/elohim-app/src/app/app.config.ts`
  - `doorway/doorway-app/src/app/services/doorway-federation.service.ts`
  - `justfile`

### Slice 2 — Answer<T> and the first production reader of /.well-known/elohim-auth

**Delivers.** Answer<T> with four states (present / absent / refused / unreachable) and reason labels, ported from crates/seam-contracts/src/answer.rs:88-96 with 'refused' added because over HTTP a peer that rejects your identity is neither absent nor unreachable. Attribution{origin, peerId?, trust, resolvedIdentity, verified} — note resolvedIdentity, the axis on which three peers actually disagree (auth_required is declared and never enforced on the dispatch path: doorway/doorway-service/src/routes/catching_up.rs:168-171), and note the deliberate ABSENCE of a freshness field, since the federator merely propagates the answering peer's self-report (services/federator.rs:126-131). Then readAuthDiscovery(origin) — the first production reader of GET /.well-known/elohim-auth (server/http.rs:5202 → routes/auth_discovery.rs:136), typed against the already-generated, currently-dead elohim-service/src/generated/auth-discovery.ts:11 — with assertOriginRelative (the client-side mirror of the Rust walker's '//host/x' bypass case) and an allowlist that rejects any advertised path outside /auth/ or /threshold/, so a hostile document can never point a password at /apps/{id}/… (a service prefix serving third-party ZIP content at server/http.rs:2438 and :5820). Plus portalUrl() from AuthDiscovery.portal (auth_discovery.rs:112, PORTAL_PATH :118), replacing the four inlined '/threshold/login' literals in doorway-app.

**Proven by.** discovery.spec.ts asserting a hostile document is rejected — '//evil.tld/login' and 'https://evil.tld/login' and '/apps/evil/login.html' all refused, mirroring the Rust detector-control at auth_discovery.rs:247-267 — and a 404 yields {state:'absent'} while a network throw yields {state:'unreachable'}, never the reverse. Plus one scenario added to the existing genesis/a2o/features/auth/auth-discovery.feature under its existing @concern:auth-discovery tag (line 1), reusing the step 'the auth discovery document is fetched from doorway {string}' (genesis/a2o/steps/auth/auth-discovery.steps.ts:60-64): a client library reads the document and reports drift instead of following an unowned path.

**Files.**
  - `app/elohim-library/projects/elohim-service/src/keep/answer.ts`
  - `app/elohim-library/projects/elohim-service/src/keep/discovery.ts`
  - `app/elohim-library/projects/elohim-service/src/keep/discovery.spec.ts`
  - `app/elohim-library/projects/elohim-service/src/keep/answer.spec.ts`
  - `genesis/a2o/features/auth/auth-discovery.feature`

### Slice 3 — openKeep + Custodian — one token path, one origin, and the localStorage literal deleted

**Delivers.** openKeep({origin, store, fetchImpl}) returning Keep{custodian, witnesses, can}. Custodian composes DoorwaySessionClient (doorway-session-client.ts:196) and a StorageClient constructed with apiKey = the session token from the SAME SessionTokenStore — one token path, which works because the doorway proxy forwards Authorization verbatim. custodian.origin is FIXED from location.origin at construction and is never sourced from the peer set or from a response body, which is what closes the password-to-synthesized-origin path (resolveGatewayToDoorwayUrl returns 'https://doorway-' + whatever follows the last '@' at federated-identifier.ts:134, fed to selectDoorwayByUrl by login.component.ts:167-170, returned FIRST by PasswordAuthProvider.getAuthBaseUrl() at password-auth.provider.ts:60-63 and used at :124). pathDrift reports discovery/client mismatch as data — it does NOT throw at boot, because DoorwaySessionClientOptions (:180-187) has no path map to override, so the document is honestly a CHECK not a SOURCE. session.signIn/signUp/refresh return StoredSession or throw — no Answer<T> on the login path. authority/trustMode/conductorEndpoint are NOT on StoredSession because AuthResponse does not carry them (generated/auth-response.ts:8-49); callers pay an explicit me() round trip. AccountApi ships as pure KeepAbsentError with the account.rs:946-954 evidence line. Converts auth.service.ts:563-571's destroy-and-rebuild memo, and DELETES the raw literal at holochain-client.service.ts:138 — one of only three non-dist occurrences of 'elohim-auth-token' in the repo. Adds @elohim/identity as a dependency plus its tsconfig path (it is currently in neither elohim-service's peerDependencies nor its paths, and node_modules/@elohim/ contains only storage-client), and runs pnpm install.

**Proven by.** custodian.spec.ts with an injected fetchImpl, asserting: (a) after signIn, a docs.list() carries the same bearer the session holds; (b) NO request to any origin other than custodian.origin ever carries an Authorization header — the invariant that keeps a gossiped peer from seeing the human's token; (c) a document advertising '/apps/x/login' produces pathDrift and openKeep still resolves rather than throwing; (d) every AccountApi member rejects with KeepAbsentError and issues zero fetches. Run by `just gate elohim-library`.

**Files.**
  - `app/elohim-library/projects/elohim-service/src/keep/custodian.ts`
  - `app/elohim-library/projects/elohim-service/src/keep/open-keep.ts`
  - `app/elohim-library/projects/elohim-service/src/keep/custodian.spec.ts`
  - `app/elohim-library/projects/elohim-service/package.json`
  - `app/elohim-library/projects/elohim-service/tsconfig.json`
  - `app/elohim-app/src/app/imagodei/services/auth.service.ts`
  - `app/elohim-app/src/app/elohim/services/holochain-client.service.ts`

### Slice 4 — Witnesses — credential-free plural reads, gated on a verifier that must land first

**Delivers.** witnesses.list() / ask(peer, path) / custodyOf(hash) / posture() / status() over a SEPARATE transport that structurally cannot attach an Authorization header, plus readContent with hash verification. Three prerequisites land in the same slice because the surface would otherwise lie: (1) export sha256BlobHash / verifyBlobHash from @elohim/epr beside verifyEd25519 (index.ts:4) — the sha256 import is already present transitively at epr-ts/src/cid.ts:2, and computeCid is CIDv1 dag-cbor so it CANNOT verify a raw blob_hash; this is what lets Attribution.verified read 'content-hash' instead of 'unverified'. (2) Add #[derive(TS)] + #[ts(export, export_to="../../sdk/storage-client-ts/src/generated/")] to DeliveryPeer (elohim/elohim-storage/src/p2p/mod.rs:423-426), collapsing the two divergent hand-copies at apps-sw.ts:52-59 and cache/content-resolver.ts:36-43. (3) Add the missing export * lines for DistributionDetails / ReplicaPeer / PeerTopologyView / FreshnessState to the generated barrel (currently 281 exports over 458 files, and dist mirrors the gap). readContent takes only same-scheme https candidates, never plain-http LAN IPs from a multiaddr (apps-sw.ts:92-93, blocked as mixed content on the deployed origin), returns {state:'unreachable', reason:'mixed-content-blocked'} rather than silently exhausting, and DISCARDS bytes whose digest does not match — mirroring what the Rust race_fetch already does (p2p/blob_fetch.rs:9-10) and the SW does not (apps-sw.ts:222-226 caches unverified).

**Proven by.** witnesses.spec.ts asserting: (a) a tampered body is discarded and yields {state:'unreachable'} rather than {state:'present', verified:'unverified'}; (b) no witness request carries an Authorization header, ever; (c) an http:// candidate on an https page yields reason:'mixed-content-blocked'; (d) a 401/403 from a witness yields {state:'refused'} with a status, never {state:'absent'}. Plus `cargo test export_bindings` in elohim/elohim-storage proving DeliveryPeer.ts is generated, and `_gate-epr-ts` (justfile:388-389) covering the new blob-hash export.

**Files.**
  - `app/elohim-library/projects/elohim-service/src/keep/witnesses.ts`
  - `app/elohim-library/projects/elohim-service/src/keep/witnesses.spec.ts`
  - `elohim/sdk/epr-ts/src/blob-hash.ts`
  - `elohim/sdk/epr-ts/src/index.ts`
  - `elohim/elohim-storage/src/p2p/mod.rs`
  - `elohim/sdk/storage-client-ts/src/generated/index.ts`

### Slice 5 — Account and recovery — blocked on Rust, listed so the gap has an owner

**Delivers.** Not client work. The one-line unblock is in storage: make the account routes resolve their caller through extract_agent_cid (account.rs:1000+, reading the X-Agent-Cid header the doorway proxy actually injects at storage_proxy.rs:456) the way peer_topology.rs:66 and cluster.rs:73 already do, instead of extract_agent_key's ambient-local-session fallback (:954). Only then does can.accountReads flip true and AccountApi stop throwing. Recovery needs strictly more: the three doorway endpoints return 501 after validating input (auth_routes.rs:3032-3040 and the two following handlers), /auth/elohim-verify/* scores against a hardcoded 'Test User' mock profile (:3169-3172, :3187), and the imagodei DNA's create_recovery_request (zomes/imagodei/src/lib.rs:2719) has no HTTP caller at all. Listed here so the gap is owned rather than discovered by an app author at runtime.

**Proven by.** A new Rust unit test in elohim/elohim-storage/src/api/account.rs asserting that a request bearing only X-Agent-Cid (no X-Agent-Id) and a populated local_sessions table resolves the HEADER's identity, not the session's — the exact substitution that makes today's reads wrong. Run by _gate-epr-storage (justfile:404-406).

**Files.**
  - `elohim/elohim-storage/src/api/account.rs`
  - `doorway/doorway-service/src/routes/auth_routes.rs`
  - `app/elohim-library/projects/elohim-service/src/keep/custodian.ts`


## Open decisions

**Does slice 1's provider flip ship enabled, or behind an environment flag? Wiring DOORWAY_ADDRESS_RESOLVER to FederationPeerResolver changes where every /api/, /db/, /blob/, /apps/, /health request in elohim-app can go — and the refuters proved the naive version of this change takes the app down.**

> Ship ENABLED, without a flag, but only because the three hardenings make it strictly-additive: the resolver falls back to ConfiguredDoorwayResolver on a miss, on a fetch failure, and on an empty federation list, so its worst case is byte-identical to today's ConfiguredDoorwayResolver behaviour. A flag would leave the old path alive and untested. The spec asserting 'unknown identity never throws' is what makes this safe; do not land the flip without it.

**Should Keep expose unsigned-gossip peers at all? Half of GET /api/v1/federation/doorways is transitively injectable — fetch_single_peer (services/federation.rs:1039-1058) accepts {id,url,region,capabilities} from any configured peer with no signature and no allowlist — and that same peer cache feeds the JWKS trust-anchor refresh (:840-861), so a URL landing in it becomes a token issuer this doorway accepts.**

> Expose them, labelled trust:'unsigned-gossip', and make the label load-bearing: never credentialed, never used for identity, never used as a content candidate once verified reads exist. Hiding them would make the client blind to a set the server is already acting on. Separately — and this is the more urgent finding, outside Keep's scope — file the JWKS coupling as a security backlog item: a peer should enter the JWT trust set only from a signature-verified DoorwaySummary, and PeerJwksCache::insert_positive should refuse to overwrite an existing kid with a different pubkey rather than last-writer-wins.

**Is `refused` a legitimate fourth Answer state, or does it violate the shared contract? crates/seam-contracts/src/answer.rs:80-85 argues explicitly that refusal is a paired ReasonLabel, not a fourth variant, 'because keeping the state set at three is what lets every seam share one vocabulary.'**

> Add it, and say why in the port's doc comment. That Rust note describes an in-process seam where the caller holds both values; over HTTP, collapsing 'this peer rejected who I am' (the single most likely three-peer disagreement, given the HS256 kid refusal at jwt.rs:569-574) into 'unreachable' actively misinforms — unreachable is specified as 'absence NOT established', so a client reading 2-of-3 unreachable concludes the network is degraded. If the operator prefers strict contract parity, the alternative is Answer<T> with three states plus a required `reason` on every non-present variant, where 'unreachable' + reason:'unknown-issuer' carries the same information. Either works; four states reads better at call sites.

**Should the discovery document gain `hAppId`? It is the last field a non-browser caller must supply, because StorageClient bakes it into /sync/v1/{hAppId}/docs (client.ts:131) and it is absent from AuthDiscovery.**

> Yes, and it is the cheapest possible schema addition: it names no location, so the origin-relative invariant (auth_discovery.rs:17-23) is structurally untouched. But it is a six-file Rust+schema change (AuthEndpoints' fixed [&str;10] at :70-83, AUTH_OWNED_PATHS, the symmetry test, the schema's additionalProperties:false, then schema:codegen:ts rewriting six generated directories with the known Prettier oscillation). Do it as its own commit after slice 2 proves the document has a reader. Do NOT add storage or peer paths to the document in the same pass — those would need a second symmetry guard against the storage route manifest, since the existing guard covers only auth-owned paths.

**Should slice 3 delete the federated-identifier origin-synthesis path (resolveGatewayToDoorwayUrl → selectDoorwayByUrl → getAuthBaseUrl), or only stop using it?**

> Delete it in the same commit. Pinning custodian.origin makes Keep safe, but the old chain stays live in PasswordAuthProvider and will keep POSTing {identifier, password} to https://doorway-{whatever-followed-the-@} for as long as it exists. Make resolveGatewayToDoorwayUrl's conventions resolve only against a known-doorway match and return {ok:false} otherwise, and make selectDoorwayByUrl refuse a URL absent from the federation list. If that is judged too large for slice 3, split it into its own commit immediately after — but do not leave it indefinitely, because Keep's arrival makes it look solved when it is not.


## Provenance

Designed by a 16-agent workflow: six parallel grounding readers (170 evidenced claims, every one
carrying a path:line), three independent designs held to the peer-pluralism constraint, a
three-lens judge panel (pluralism / honesty / burden), and three adversarial refuters
(invented-API, doorway-bake-in, security). Ten fatal defects were found and fixed before this
document existed — including a first-slice proposal that would have taken the app down by throwing
on an unknown identity. The refuters' standing instruction was that a plausible-sounding API which
does not exist is the worst possible output; an earlier pass had invented `elohim.data.fetch()`,
`listDocuments()` and `putBlob()` and had to be discarded.
