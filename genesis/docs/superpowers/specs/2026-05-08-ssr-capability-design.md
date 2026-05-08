# SSR as a Capability-Advertised, Framework-Agnostic Doorway Feature

**Status:** Draft for review
**Date:** 2026-05-08
**Origin:** P2P design-gate audit of the `doorway-ssr-deliver` shift (2026-05-07/08). Audit verdict: no anti-patterns introduced, three open seams need cohesive design treatment.
**Related memories:**
`project_three_layer_truth_model`,
`project_compute_and_model_independent_diversity_surfaces`,
`project_doorway_manifest_driven_routes`,
`project_doorway_single_target_no_fanout`,
`project_doorway_ssr_pod_resource_floor`,
`project_doorway_full_facilitator_sprint`,
`project_ssr_is_compute_capability_claim`,
`project_ssr_anonymous_auth_context`,
`project_dht_vs_libp2p_scoping`,
`project_peer_native_account_canonical_surface`,
`project_m5_reframe_auth_portal_convergence`.

## Problem

The `doorway-ssr-deliver` shift brought server-side rendering live on alpha. The substrate works end-to-end and the architecture audit found no anti-patterns. Three seams remain:

1. **No feature gate.** Every alpha doorway carries the SSR runtime regardless of whether the operator opted in. SSR is a real compute-shape claim — V8 + 51 MB Angular bundle + per-render CPU — but operators have no way to say "I don't run SSR" and no way to declare *which* bundles they carry.
2. **No capability advertisement.** The substrate doesn't know which peers can SSR which content. Peer A picking a peer B for an SSR-eligible request can't see whether B's doorway can render it.
3. **Anonymous-only auth context.** The current SSR fetch shim only forwards V8's HttpClient headers, never the originating user's session credentials. Higher-reach content renders empty for authenticated users — not a security hole, but a feature gap.

These three seams are intertwined. The auth pattern shapes what the capability claim must express, which shapes what the operator opts into. This design treats them as one mechanism.

## Goals

- **Auto-honest capability** — doorway derives the claim from on-disk bundles intersected with storage's manifest, so it can only advertise what it can actually serve. Operator can reduce the claim via override (cap concurrency, hide a bundle, restrict auth modes) but never inflate it.
- **Substrate-visible diversity** — peers can see which doorways carry which bundles, support which auth modes, and have what concurrency budget.
- **Framework-agnostic auth threading** — the originating user's auth context flows through the V8 boundary to outbound storage fetches, regardless of which framework is doing the rendering.
- **No anti-pattern introduction** — preserve the three-layer truth model, single-target dispatch, manifest-driven routing, CSR-fallback as the floor.

## Non-goals

- Substrate-side matchmaking on render capability (deferred — needs real-world bottleneck data).
- Cross-doorway P2P proxy of rendered HTML (deferred — pairs with substrate matchmaking).
- Bundle distribution via content-addressed substrate (deferred — bundles stay docker-baked or Harbor-pulled).
- Stage-3 elohim-defender enforcement of capability claims (deferred — tracks the broader bootstrap-to-elohim security gradient).
- The `doorway-as-full-web2-facilitator` sprint (independent design; this work is compatible with it).

## Decisions captured during brainstorm

| Decision | Choice |
|---|---|
| SSR audience | Both anonymous and authenticated, designed cohesively |
| Auth-propagation pattern | Header-forward via V8 fetch shim; doorway is the credential-normalization layer |
| Auth modes advertised | `anonymous`, `doorway-hosted`, `steward-presence` (see definitions below) |
| Capability-claim shape | Per-bundle + per-auth-mode + per-concurrency-budget |
| Capability-claim origin | Auto-derived at startup from disk + storage manifest; operator override may only reduce |
| Capability-claim role | Informational — CSR fallback is the floor; substrate routing is a later sprint |
| Capability-claim notarization | DHT-notarized via extension to existing peer-status entry |
| Capability-tier strategy | Ship Tier 1 (`renderCapability`) **and** Tier 2 (`extensions` map) together; Tier 3 (libp2p gossip) for runtime state |
| Doorway identity in SSR | SSR always renders as the user (or anonymous); doorway never elevates privilege |

### Auth-mode definitions

The three auth modes carried in `authModes` are not session backends themselves — they are claims about which session-establishment flows the doorway is wired to honor for SSR rendering. Per `project_peer_native_account_canonical_surface` and `project_m5_reframe_auth_portal_convergence`, doorway is the OAuth relying-party / portal layer regardless of where the credentials originate.

| Mode | What the doorway is asserting | Credential origin |
|---|---|---|
| `anonymous` | The doorway can serve SSR for unauthenticated requests with no session lookup. Always required (anonymous is the floor). | No credential — render proceeds with public-reach access only. |
| `doorway-hosted` | The doorway can look up a session it issued itself (classic JWT / cookie flow) and forward the user's credential to outbound storage fetches. | Doorway issued the credential at login time; pre-graduation accounts. |
| `steward-presence` | The doorway can look up a session backed by a steward attestation (peer-native OAuth from the user's home steward) and forward that attestation to outbound storage fetches. The doorway is the portal; the actual login UI was rendered by the user's peer. | User's home steward (peer-native); doorway holds the attestation as a session credential. |

The V8 fetch shim itself is mode-agnostic — it just attaches an opaque `UserCredential` blob the session layer hands it. The `authModes` list is what the doorway commits to *honoring*; the session layer is where mode-specific lookup actually lives.

## Architecture

```
                    ┌─────────────────────┐
                    │  storage manifest   │   ◄── source of truth: which routes
                    │  build_manifest()   │      are SSR-eligible + which bundle
                    │  (per app)          │      they need
                    └─────────┬───────────┘
                              │
                              ▼
       ┌──────────────────────────────────────────┐
       │              DOORWAY                      │
       │                                           │
       │  ┌──────────────────────────────────┐    │
       │  │ Capability deriver (startup)     │    │
       │  │ disk(bundles) ∩ manifest(routes) │    │
       │  │ + override.toml (reduce only)    │    │
       │  └────────────────┬─────────────────┘    │
       │                   │                       │
       │                   ▼                       │
       │  ┌──────────────────────────────────┐    │
       │  │ peer-status entry (DHT)          │────┼──► DHT (notarized)
       │  │   .renderCapability  (Tier 1)    │    │
       │  │   .extensions[…]     (Tier 2)    │    │
       │  └──────────────────────────────────┘    │
       │                                           │
       │  ┌──────────────────────────────────┐    │
       │  │ Request → SsrRoute disposition   │    │
       │  │   ├─ session lookup (normalize)  │    │
       │  │   ├─ V8 render w/ fetch shim     │    │
       │  │   └─ shim forwards user creds    │    │
       │  └──────────────────┬───────────────┘    │
       │                     │                     │
       │                     ▼                     │
       │           HTML │ CSR shell fallback      │
       └─────────────────────┬─────────────────────┘
                             │
                             ▼
                     Browser / SPA hydrate
```

Three changes layered onto the existing substrate:

1. **Capability derivation** — at doorway startup, scan `/bundles/*.bundle.mjs`, read each bundle's manifest header, intersect with storage's `build_manifest()` SSR-route declarations. The result is a structured claim: `{ bundles, authModes, maxConcurrentRenders, renderers }`. An optional `doorway-config.toml` may *reduce* the claim but never inflate it.

2. **Capability publication** — the derived claim populates a new `renderCapability` field on the existing peer-status DHT entry, sibling to `elohimCapability`. The same entry gains an `extensions` map for Tier-2 capabilities.

3. **Auth normalization** — doorway's session layer (already polymorphic over JWT and steward attestation) hands the V8 fetch shim an opaque user-credential blob per render. The shim attaches it as `Authorization` / `Cookie` to outbound storage fetches. Storage validates as if the user called direct.

What does *not* change:

- DHT entry-type count (no new entry types — `renderCapability` is a field on existing peer-status)
- Storage's peer-selection / matchmaking logic (claim is informational this sprint)
- CSR fallback path (still the floor)
- Render cache, dispatch logic, manifest declarations, ingress rules

## Capability tier strategy

To prevent the WHM-dashboard / nginx-config sprawl pattern (every new capability claiming its own typed first-class field forever), the design stratifies capability claims:

| Tier | Pattern | When to use | Examples |
|---|---|---|---|
| **1: Typed core** | Named field on peer-status with referenced profile schema | Validators or matchmakers will branch on the shape; protocol-defined; load-bearing for routing/accountability | `elohimCapability`, `renderCapability`, eventual `storageCapability` |
| **2: Registered extensions** | Generic `extensions: { [name]: ProfileRef }` map field. Each name is registered in a capability registry; profile schema is validated separately | App-specific or domain-specific capabilities with known shape, not core to substrate matchmaking | `transcodeCapability`, `captioningCapability`, `indexingCapability` |
| **3: Operational gossip** | Not on DHT at all — libp2p heartbeat only | Runtime state that changes second-by-second; doesn't need notarization | current render queue depth, recent latency, instantaneous memory pressure |

### Tier-2 → Tier-1 promotion criteria

A Tier-2 extension capability graduates to Tier-1 (typed field on peer-status) when **two or more** of these are true:

1. **Substrate-wide matchmaking** — at least one peer-selection algorithm needs to branch on the capability's shape, not just its presence.
2. **Validator enforcement** — integrity zome validators need to enforce structural constraints on the claim (not just "is an object").
3. **Multi-app consumption** — three or more independent app surfaces (lamad / qahal / shefa / etc.) read the claim and act on its contents.
4. **Cross-pillar accountability** — peers hold each other accountable for the claim's accuracy via observation/attestation flows.

**Default action: stay in Tier 2.** Promotion is a deliberate decision, not a default. A Tier-2 capability that lives there for years and works fine should *stay* there. Tier 1 is for capabilities the substrate genuinely cannot function without.

`renderCapability` ships as Tier 1 from day one because the substrate-routing sprint (deferred but planned) will branch on its shape, and authenticated-render accountability needs the typed `authModes` field for validators to reason about.

### Capability registry

`elohim/sdk/schemas/v1/registries/capability-registry.json` — a flat list of registered Tier-2 capability names, each with:

- `name`: kebab-case identifier
- `schemaRef`: pointer to the profile schema (e.g., `epr:schema:view:transcode-capability-profile`)
- `tier`: `2` (or `1` once promoted; the entry then becomes the migration record)
- `addedAt`: ISO timestamp
- `description`: one-line summary

Registration is a PR. Validators don't read the registry (they validate structurally). It's a documentation + collision-avoidance surface, not a runtime gate.

## Schema design

### `views/render-capability-profile.schema.json` (new — Tier 1)

```json
{
  "$id": "epr:schema:view:render-capability-profile",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "RenderCapabilityProfile",
  "description": "Source of truth: auto-derived at doorway startup from on-disk bundles intersected with elohim-storage's manifest of SSR-eligible routes (Operational, Category C). doorway-config.toml may reduce the claim but never inflate it. Notarized via the peer-status DHT entry that wraps it.",
  "type": "object",
  "required": ["bundles", "authModes", "maxConcurrentRenders", "renderers"],
  "properties": {
    "bundles": {
      "type": "array",
      "minItems": 1,
      "items": {
        "type": "object",
        "required": ["name", "version", "renderer"],
        "properties": {
          "name":     { "type": "string" },
          "version":  { "type": "string" },
          "renderer": { "$ref": "../enums/renderer-kind.schema.json" },
          "digest":   { "type": ["string", "null"] }
        },
        "additionalProperties": false
      }
    },
    "renderers": {
      "type": "array",
      "minItems": 1,
      "items": { "$ref": "../enums/renderer-kind.schema.json" }
    },
    "authModes": {
      "type": "array",
      "minItems": 1,
      "items": {
        "type": "string",
        "enum": ["anonymous", "doorway-hosted", "steward-presence"]
      }
    },
    "maxConcurrentRenders": {
      "type": "integer",
      "minimum": 0
    },
    "memoryBudgetMib": {
      "type": ["integer", "null"],
      "minimum": 0
    }
  },
  "additionalProperties": false
}
```

### `views/capability-extensions.schema.json` (new — Tier 2 hatch)

```json
{
  "$id": "epr:schema:view:capability-extensions",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "CapabilityExtensions",
  "description": "Tier-2 capability claims map. Each key is a kebab-case capability name registered in the protocol's capability registry. Each value carries a schemaRef pointer (so consumers can resolve the profile schema) and an opaque structured profile. The DHT validator checks structural well-formedness only — claim CONTENTS are interpreted by consumers who recognize the capability name.",
  "type": "object",
  "patternProperties": {
    "^[a-z][a-z0-9-]{2,30}$": {
      "type": "object",
      "required": ["schemaRef", "profile"],
      "properties": {
        "schemaRef": {
          "type": "string",
          "pattern": "^epr:schema:"
        },
        "profile": { "type": "object" }
      },
      "additionalProperties": false
    }
  },
  "additionalProperties": false
}
```

### `enums/renderer-kind.schema.json` (new)

```json
{
  "$id": "epr:schema:enum:renderer-kind",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "RendererKind",
  "type": "string",
  "enum": ["angular-ssr", "react-rsc", "vue-ssr", "svelte-ssr", "lit-ssr", "static-html"],
  "_tiers": {
    "core": {
      "values": ["angular-ssr"],
      "rationale": "angular-ssr is the only renderer the protocol ships today; others are reserved for future framework adapters and are valid claim values but not yet implemented in elohim-render"
    }
  }
}
```

### Extension to `views/peer-status-view.schema.json`

Two new optional fields, sibling to existing `elohimCapability`:

```json
"renderCapability": {
  "oneOf": [
    { "$ref": "render-capability-profile.schema.json" },
    { "type": "null" }
  ],
  "description": "If this peer runs a doorway that can server-render, advertises its render-capability profile. Null or absent = no SSR (storage-only peer, or operator opted out)."
},
"extensions": {
  "oneOf": [
    { "$ref": "capability-extensions.schema.json" },
    { "type": "null" }
  ],
  "description": "Tier-2 extension capabilities. Apps register kebab-case capability names; each entry has a schemaRef + structured profile. Validator checks structural well-formedness only. Promotion to Tier 1 (a typed sibling field) requires substrate-wide load-bearing use."
}
```

### Codegen distribution

Per `elohim/sdk/schemas/CLAUDE.md`, both new files added to `INTERFACE_FILES` in `codegen-ts.mjs`. Distributed to:

- `genesis/seeder/src/generated/`
- `app/elohim-app/src/app/generated/`
- `app/elohim-library/projects/elohim-service/src/generated/`

Rust side: ts-rs structs in `elohim-storage/src/views.rs` mirror these for the wire format. Schema contract test in `elohim/elohim-storage/tests/schema_contract.rs` catches drift.

### DHT integrity-validator scope (Stage 1, structural only)

Per `project_bootstrap_to_elohim_security_gradient` and `project_hdi_no_get_links_in_validators`, the integrity zome validator can only do *deterministic structural checks*:

- Shape compliance (already enforced by serde + ts-rs).
- "If `renderCapability` is non-null, `bundles` is non-empty and `authModes` includes `anonymous`."
- "If present, `maxConcurrentRenders` is a non-negative integer."
- *Cannot* enforce "the doorway actually has these bundles on disk" — that's Stage 3 (elohim defender observation), out of scope for this sprint.

## Doorway runtime

### Capability deriver — `doorway-service/src/render/capability.rs` (new)

Runs once at startup. Three inputs, one output.

**Inputs:**

- **Bundle directory scan** — `/bundles/*.bundle.mjs` files. Each bundle's manifest header yields `{ name, version, renderer }`. Compute sha256 digest at scan time (cached for restart).
- **Storage manifest fetch** — `GET ${STORAGE_URL}/admin/manifest`. Extract every route where `render` is set; produce `{ renderer → required-bundles set }`.
- **Operator override** (optional) — `doorway-config.toml [render]` section: `bundles_hidden`, `max_concurrent`, `auth_modes`, `memory_budget_mib`.

**Algorithm:**

1. For each bundle on disk, look up its renderer in the storage manifest.
2. If no manifest route uses that renderer, the bundle is unused — skip it.
3. Apply override: drop hidden bundles, cap concurrency, restrict `auth_modes`.
4. Validate: `authModes` must contain `anonymous`; `bundles` must be non-empty after override (else publish `renderCapability = null`).
5. Compute `renderers[]` = unique set of `bundles[].renderer`.

**Output:** `Option<RenderCapabilityProfile>`. `None` → publish `renderCapability = null`. `Some` → publish the profile.

**Override layering rule (one-way):** override can only *reduce* the derived claim. If override declares an auth mode the session layer can't actually serve, the deriver drops it from the final claim and warns. Override never inflates beyond what disk + session-layer + manifest agree on.

**Why "intersection with manifest" matters:** an operator who installed `qahal-app.bundle.mjs` but storage's manifest doesn't declare any qahal SSR routes shouldn't advertise qahal-SSR capability — they have the bundle but the substrate has nothing for them to render. Honest by construction.

### Capability publisher (extension to existing peer-status publisher)

Doorway already publishes peer-status to its local conductor (which DHT-notarizes). The publisher gains one extra step:

```rust
let render_cap = capability::derive(bundles_dir, manifest, override_path)?;
let extensions = capability::derive_extensions(plugin_dir)?;

let peer_status = PeerStatus {
    // existing fields...
    elohim_capability: existing_elohim_cap,
    render_capability: render_cap,
    extensions,
};

conductor.publish_peer_status(peer_status).await?;
```

**Republish on change:** capability derivation runs once at startup. If bundles change at runtime (typically a redeploy) doorway re-derives and republishes. Storage manifest changes also trigger re-derive — storage's manifest version is exposed via a header; doorway watches and re-runs the deriver on bump. (Initial sprint may ship startup-only derivation; runtime watching is a follow-up if churn proves frequent.)

### Concurrency limiter — extends existing `SsrRoute` dispatch

A tokio `Semaphore` sized at `maxConcurrentRenders`. Acquired before render, released after.

```
Request arrives → SsrRoute disposition
  semaphore.try_acquire() {
    Ok(_permit)  → render normally, release on drop
    Err(_)       → return CSR-fallback shell with `x-ssr-skipped: overflow`
                   client renders client-side as it does today
  }
```

No queueing. A fallback shell is always faster than waiting. The header lets observability layers count overflow events.

### Auth-mode runtime enforcement

When `SsrRoute` fires, doorway's session layer determines the request's auth posture (one of: `anonymous` / `doorway-hosted` / `steward-presence`). Cross-check against `renderCapability.authModes`:

| Request posture | Doorway claim includes posture? | Action |
|---|---|---|
| anonymous | always | Render. |
| doorway-hosted | yes | Forward session creds via fetch shim. Render. |
| doorway-hosted | no | CSR fallback shell. `x-ssr-skipped: auth-mode-not-supported`. Client hydrates with its own session. |
| steward-presence | yes | Forward steward attestation via fetch shim. Render. |
| steward-presence | no | CSR fallback shell. Same skip header. |

**Key property:** authenticated requests *never* downgrade silently to anonymous renders. Either the doorway honors the auth mode it claimed, or it punts to CSR. Anonymous-render-of-authenticated-content was the failure mode flagged in the audit; this design eliminates it.

### V8 fetch shim — `doorway-service/src/ssr.rs::ResolverFetcher`

The shim already takes the storage URL as a base. One change: it now also takes an `Option<UserCredential>`. The credential is opaque from the shim's perspective — it's just `{ header_name: String, header_value: String }`. Doorway's session layer constructs it; shim attaches it to every outbound fetch.

```rust
pub struct ResolverFetcher {
    client: reqwest::Client,
    storage_base: String,
    user_credential: Option<UserCredential>,
}

impl DataFetcher for ResolverFetcher {
    async fn fetch(&self, path: &str, headers: HashMap<String, String>) -> Result<Bytes, FetchError> {
        let mut req = self.client.get(format!("{}{}", self.storage_base, path));
        for (k, v) in headers { req = req.header(k, v); }
        if let Some(cred) = &self.user_credential {
            req = req.header(&cred.header_name, &cred.header_value);
        }
        req.send().await
    }
}
```

V8 doesn't see the credential. The shim doesn't decode it. The session layer (which knows the difference between JWT and steward attestation) constructs it once per render. That keeps the boundary truly framework-agnostic and credential-shape-agnostic.

## Substrate consumers and observability

### Who reads `renderCapability` today

Three consumers, all read-only / informational this sprint:

| Consumer | What it does with the claim | Where the code lives |
|---|---|---|
| Doorway operator dashboard | Renders the claim back to the operator (a "what does the network see about my doorway?" view). Confirms operator intent matches what was derived. | `doorway-app` admin UI, new tile under `/admin/capability` |
| `elohim-storage /admin/peers` | Shows each known peer's render-capability alongside its elohim-capability. Substrate-wide visibility for operators. | `elohim-storage/src/http.rs` admin handler, extended view |
| Future substrate matchmaker | Reads the claim to pick SSR-capable doorways for SSR-eligible routes. | **NOT THIS SPRINT** — deferred substrate-routing work |

No client-side code reads the claim. The capability is gossip-visible but not consumed by routing logic until the later sprint.

### Observability headers (returned by doorway on every SSR-eligible response)

| Header | Value | Meaning |
|---|---|---|
| `x-render-cache` | `MISS` / `HIT` | (existing) render cache lookup outcome |
| `x-ssr-rendered` | `1` / `0` | `1` if SSR fired; `0` if CSR fallback was returned |
| `x-ssr-skipped` | `bundle-not-loaded` / `auth-mode-not-supported` / `overflow` / absent | Reason for `x-ssr-rendered: 0`. Absent on success or when route isn't SSR-eligible |
| `x-ssr-renderer` | `angular-ssr` etc. | Which renderer kind handled this render (when SSR fired) |
| `x-ssr-bundle-version` | e.g. `lamad-app@1.0.3` | Which bundle was used (when SSR fired) |

These headers are the immediate substrate-debugging surface. The pre-push hook for `app/elohim-app` can grep for them in dev-server smoke tests.

## Test strategy

| Level | What we test | Where |
|---|---|---|
| Unit | capability deriver: bundle scan + manifest intersection + override layering, table-driven fixtures | `doorway-service/src/render/capability.rs` `#[cfg(test)]` |
| Unit | session layer → user-credential normalization (JWT path, steward-presence path, anonymous path) | `doorway-service/src/session/mod.rs` |
| Unit | `ResolverFetcher` attaches `user_credential` header when present, omits when `None` | `doorway-service/tests/render_fetcher.rs` |
| Schema contract | `peer-status-view` round-trip with new fields; `render-capability-profile` serde via ts-rs | `elohim-storage/tests/schema_contract.rs` |
| Integration | doorway boots, derives capability, publishes to local conductor; `/admin/capability` returns the published profile | new `doorway-service/tests/capability_publish.rs` |
| Integration | SsrRoute with auth-mode mismatch returns CSR fallback + `x-ssr-skipped: auth-mode-not-supported` | extend `doorway-service/tests/registry_render.rs` |
| Integration | concurrency limit: spawn `max_concurrent + 2` renders; assert two return `x-ssr-skipped: overflow` | new test |
| a2o | "When a doorway operator restricts SSR to anonymous-only, an authenticated user requesting an SSR-eligible route gets CSR fallback (not anonymous render of their authenticated content)" | new `genesis/a2o/features/content/ssr_capability.feature` |
| a2o | "When peer A inspects peer B's status, A can see B's render-capability profile and reason about which bundles B carries" | same feature file |

## Edge cases

- **Bundle digest mismatch on disk** — bundle file's actual sha256 doesn't match what was cached at last scan. Treat as bundle changed: re-derive, re-publish, log warning.
- **Manifest fetch failure at startup** — storage's `/admin/manifest` unreachable. Doorway publishes `renderCapability = null` (rather than guessing) and retries every 30s. CSR fallback handles all SSR-eligible routes meanwhile.
- **Override config malformed** — invalid TOML or unknown fields. Doorway logs error, ignores override entirely (publishes derived claim), surfaces error in `/admin/capability` view.
- **Auth-mode declared but session layer rejects** — operator override says `auth_modes: [steward-presence]` but session layer hasn't been wired for steward-presence. Deriver drops the unsupported mode, logs, publishes whatever's actually working.
- **Empty claim after override** — operator restricted everything to nothing. Publish `renderCapability = null` (cleaner than empty arrays).

## Out of scope (deferred)

| Deferred | Why deferred | When it lights up |
|---|---|---|
| Substrate-side matchmaking on `renderCapability` | Touches storage's peer-selection algorithms; needs real-world bottleneck data to design well | After 1–3 months of capability data flowing |
| Cross-doorway P2P proxy of rendered HTML | Novel transport; speculative without routing data | Pairs with substrate matchmaking |
| Bundle distribution via content-addressed substrate | Bundles stay docker-image-baked or Harbor-pulled for now | Tied to doorway-as-full-facilitator sprint |
| Tier-2 → Tier-1 promotion of `renderCapability` | Already Tier-1 from day one; the *promotion mechanism* is what's deferred | First time a Tier-2 capability genuinely needs promoting |
| Stage-3 elohim-defender enforcement | Requires defender observation infrastructure | Tracks bootstrap-to-elohim security gradient |
| Steward-presence auth wiring through fetch shim | M5 (auth-portal convergence) sprint owns the session-layer side; this design accommodates it but does not ship it | When M5 lands the steward-presence session shape |
| Auto-republish on storage manifest change | Initial sprint ships startup-only derivation; runtime watching adds complexity | If manifest churn proves frequent in practice |

## Implementation pointers

- Existing render-cache + dispatch code: `doorway/doorway-service/src/server/http.rs:1641-1727`
- Existing render fetcher: `doorway/doorway-service/src/ssr.rs`
- Renderer init site: `doorway/doorway-service/src/server/http.rs:244` (`init_renderer`)
- Existing peer-status schema: `elohim/sdk/schemas/v1/views/peer-status-view.schema.json`
- Existing `elohimCapability` precedent: `elohim/sdk/schemas/v1/views/elohim-capability-profile.schema.json`
- Storage manifest's SSR-route declaration site: `elohim/elohim-storage/src/http.rs:9463-9505`
- Codegen pipeline reference: `elohim/sdk/schemas/CLAUDE.md`
- Integrity-validator constraints: `project_hdi_no_get_links_in_validators` memory
- Audit findings: `.claude/shifts/doorway-ssr-deliver-2026-05-07T23-37/sprint-result.md` and `brainstorm-prompt-followup.md` in the same directory
