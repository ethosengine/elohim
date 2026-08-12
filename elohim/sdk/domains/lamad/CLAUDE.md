---
id: lamad-domain-gospel
---

# Lamad Domain

This directory is the **lamad protocol domain** — the learning domain's vocabulary, metadata schemas, coupling contracts, and wire types. The protocol validates structure; lamad owns semantic meaning.

Three surfaces carry the name and must not be confused:

| Surface | Owns | Gate |
|---|---|---|
| `elohim/sdk/domains/lamad/` (here) | Vocabulary, schemas, codegen, wire types | `pnpm run lamad:codegen`, `pnpm run schema:test` |
| `elohim/sdk/domains/lamad/types/` | Rust ↔ MessagePack coordinator wire types | `cd elohim/sdk/domains/lamad/types && RUSTFLAGS="" cargo test --features ts` |
| `app/lamad/` | The reference client's view layer — consumes this vocabulary, never redefines it (`app/lamad/CLAUDE.md`) | its own workspace — `cd app/lamad && pnpm test` |

**`app/lamad/` is not purely the learning pillar, and this is a live tension.** It houses two things at once: the learning domain (paths, mastery, quiz-engine, knowledge maps) and the **cross-pillar content substrate** (`models/content-node.model`, `content-io/`, `renderers/`, `parsers/`). The second half is not pillar-local — `@app/lamad/models/content-node.model` is imported by the elohim, avodah, shefa, qahal, and imagodei pillars, and elohim-app's own `elohim/services/content.service.ts` (the cross-pillar core service) imports `ContentNode`/`ContentType`/`ContentReach` from it. That dependency points core → pillar. Note also that **two** ContentServices exist — `app/lamad/src/app/services/content.service.ts` (learning-flavored) and `app/elohim-app/src/app/elohim/services/content.service.ts` (transport/blob-flavored) — and they are separate implementations, not a re-export. Know which one you are editing; do not assume a change in one reaches the other.

## Two-Layer Type Architecture

The canonical protocol/domain split diagram lives in `elohim/sdk/CLAUDE.md` ("Two-Layer Type System") — read it there, not here. The protocol owns the **envelope** (wire shape, field names, generic metadata bag). This domain owns the **payload**: what metadata means per content type, what the body contains per format, what signals each interaction produces, and what each interaction owes the economy.

What is specific to lamad is the third leg — the coupling map is compiled *out of the manifest* and read at runtime, so vocabulary edits change behavior without a code change:

```
manifest.json (vocabulary + coupling)
    → codegen.mjs → coupling-map.ts (LAMAD_COUPLING_MAP)
    → SignalHarnessService reads it per contentType at render-completion time
```

## Directory Structure

```
elohim/sdk/domains/lamad/
├── manifest.json               # $ref shell — identity + one $ref per concern block
├── manifest/                   # the concern blocks (content types, formats, rendering,
│                               #   gates, attestations, graph, signals, observations)
├── schemas/                    # metadata + body schemas per content type / format
├── scripts/codegen.mjs         # manifest + schemas → TypeScript
└── types/                      # Rust wire-types crate (see `elohim/sdk/domains/CLAUDE.md`)
```

Enumerating the manifest blocks or the schema files here is how this file went stale before. Open `manifest.json` — it is a short `$ref` index and is the read-canon for what exists.

## Content formats have TWO tiers — get this wrong and content renders as raw JSON

This is the most expensive mistake available in this domain.

- **Core protocol formats** (`elohim/sdk/schemas/v1/`, DNA-notarized): broad and stable — `markdown`, `interactive`, `epr-composite`, `video`, `external`, …
- **Domain formats** (`manifest/content-formats.json`, extensible): specific and renderer-bound — `sophia-quiz-json`, `html5-app`, `gherkin`, `spa-bundle`, …

**The operative rule is claimed-vs-unclaimed, not core-vs-domain.** `manifest/rendering.json` is the sole claim surface: `codegen.mjs` builds `LAMAD_RENDERER_MAP` by iterating `rendering[*].formats[]` and never reads `content-formats.json`'s own `renderer` field, which is a documentary mirror with no enforced cross-check. A format that no renderer claims has no landing spot, so the content **falls through to the raw-JSON fallback** — it renders, it just renders as a JSON blob, which is why this failure reads as a styling bug rather than a vocabulary bug.

Tier is a strong correlate, not the mechanism: core formats are mostly unclaimed, which is why reaching for one usually lands you in the fallback. But `markdown-renderer` claims `markdown`, `html`, and `plaintext` — so a core format can be perfectly well rendered. Check `rendering.json`, not the tier.

| Content | `contentFormat` | Wrong |
|---|---|---|
| Sophia quiz / discovery assessment | `sophia-quiz-json` | `interactive` |
| HTML5 simulation | `html5-app` | `interactive` |

Seed data uses **domain** formats. Adding one takes three edits, and skipping any of them fails in a different place:

1. `elohim/sdk/schemas/v1/enums/content-format.schema.json` — add the format to the flat `enum` array. `create-content-input.schema.json` `$ref`s this enum and `pnpm run schema:validate` checks seeds against it, so skipping this rejects every seed using the format. Add it to `_tiers.extensible` in the same file too: `_tiers` drives the generated core/extensible constants (`codegen-ts.mjs`, `codegen-rs.mjs`, `check-dna.mjs`), not seed validation — which is why a format can validate fine while sitting outside `_tiers` (`spa-bundle` does exactly that today, and it is a gap, not a pattern to copy).
2. `manifest/content-formats.json` — the format's own declaration.
3. `manifest/rendering.json` — claim it in a renderer's `formats[]`. Skip this and you get the silent raw-JSON fallback.

**A new renderer takes a fourth edit that no schema enforces:** `RENDERER_COMPONENTS` in `app/lamad/src/app/renderers/renderer-initializer.service.ts` is a hand-maintained map from component *name* to component *class*. The initializer skips any manifest-declared renderer whose name is missing from it — no warning, no throw. A renderer can be fully declared, generated into `LAMAD_RENDERER_MAP`, and still never register. (`elohim-element-registry` is declared in `rendering.json` and absent from that map today.)

## Generated Output

Four independent codegen lanes write into this domain's consumers. Each lane's distribution list lives in code; read it there rather than trusting a table:

| Lane | Command | Distribution list (read-canon) |
|---|---|---|
| Domain types | `pnpm run lamad:codegen` | `OUTPUT_DIRS` in `scripts/codegen.mjs` |
| Protocol types | `pnpm run schema:codegen:ts` | `GENERATED_OUTPUT_DIRS` + `INTERFACE_FILES` in `elohim/sdk/schemas/scripts/codegen-ts.mjs` |
| Route claims | `pnpm run route-claims:codegen` | `BUNDLE_TARGETS` in `elohim/sdk/schemas/scripts/codegen-route-claims.mjs` |
| Coordinator wire types | `cd types && RUSTFLAGS="" cargo test --features ts`, then `pnpm run wire-types:generate` | `@elohim/storage-client/wire-types/lamad` |

**`route-claims.ts` is NOT a `lamad:codegen` product** — `scripts/codegen.mjs` never touches it. Editing `routeClaims` in the manifest and running only `lamad:codegen` leaves the generated file unchanged, which reads as "my manifest edit didn't take" and invites hand-editing a generated file. Run `route-claims:codegen`.

Order matters when a change crosses lanes: protocol enum (then `schema:check-dna`, since format/type enums carry `_dna` constants) → `schema:codegen:ts` → metadata schema + manifest → `lamad:codegen` → `route-claims:codegen` → `wire-types:generate` only if coordinator I/O changed.

What `lamad:codegen` itself emits (`codegen.mjs` is authoritative for the list; `route-claims.ts` is deliberately absent — it belongs to the lane above):

| File | Contents |
|---|---|
| `metadata-types.ts` | One interface per content type's `metadataSchema` |
| `body-types.ts` | `EprCompositeBody`, `Section`, `Item` |
| `content-node-types.ts` | `TypedContentNode` discriminated union + `isPathNode()` / `isConceptNode()` / `isAssessmentNode()` guards |
| `coupling-map.ts` | `LAMAD_COUPLING_MAP` — value flows and governance signals per content type |
| `manifest-types.ts` | Content type lists, renderer map, signal map |

`schema:codegen:ts` has **no idempotent fixed point** — union line-wraps flip between runs. A regenerate diff that is nothing but re-wrapped unions is cosmetic, not schema drift [[feedback_codegen_prettier_oscillation]].

## Commands

```bash
pnpm run lamad:codegen          # domain types (manifest + schemas → TS)
pnpm run schema:codegen:ts      # protocol types (enums, wire types)
pnpm run wire-types:generate    # coordinator wire types → @elohim/storage-client
pnpm run schema:test            # validate manifest against the protocol schema
pnpm run schema:validate        # validate seed data against content schemas
```

## Rules

### Schema before code — and backend before frontend

Edit the schema first, then regenerate. Never hand-write a type a schema should own, and never patch generated TS or the manifest to satisfy a renderer's convenience — that inverts the truth layer [[feedback-backend-authoritative-frontend-senses]]. A UI need is a reason to amend the manifest, not to edit its output.

1. Protocol primitives (enums, wire types) → `elohim/sdk/schemas/v1/`, then `pnpm run schema:codegen:ts`
2. Domain metadata/body shapes → `schemas/`, then `pnpm run lamad:codegen`
3. Vocabulary (content types, **formats**, signals, coupling) → `manifest/`, then `pnpm run lamad:codegen` — and see the two-tier format trap above, which bites only on this third path
4. Coordinator I/O shapes → `types/src/lib.rs`, then `pnpm run wire-types:generate`

### Typed metadata, not string keys

```typescript
// WRONG — untyped metadata access
const thumbUrl = (node.metadata as Record<string, unknown>)['thumbnailUrl'];

// RIGHT — narrow with the type guard, then read typed metadata
if (isPathNode(node)) {
  const thumbUrl = node.metadata.thumbnailUrl; // PathMetadata — typed
}
```

### Seeder and app share identical types

Domain codegen writes the same files to every target. If a field exists in the app, it exists in the seeder. The seeder seeds what the app renders — no guesswork.

### Signal harness reads coupling from the manifest

Renderers never call economic-event APIs directly. They emit `RendererCompletionEvent`:

```
Renderer → RendererCompletionEvent
    ↓ SignalHarnessService reads LAMAD_COUPLING_MAP for contentType
CreateEconomicEventInput { action, resourceConformsTo, ... }
    ↓ EconomicEventsApiService.createEconomicEvent()
```

### Three-leg coupling is required

`app-manifest.schema.json` rejects a content type missing its `value` and `governance` legs. Claims (feedback) are required too — every content type declares what outcomes it asserts and what would contradict them. This is not boilerplate: without evidence there is nothing to grade, and without governance there is no grader, so such a type could never earn reach.

## Manifest Structure

`manifest.json` is a short shell: identity fields plus, for most concerns, one `$ref` per block into `manifest/` (`routeClaims` is the exception — it is inline literal data, with no `manifest/route-claims.json`). Adding a concern means adding a block file and a `$ref` — the shell stays small by design (the modular-manifest pattern in `elohim/sdk/CLAUDE.md`). What each block *means*:

- **`vocabulary`** — `contentTypes` (each with `description`, `metadataSchema` `$ref`, `coupling.{knowledge,value,governance,claims}`), `contentFormats`, `relationships`, `signals`, and `observations` (polarity + archetype feedback evidence).
- **`observation_kinds`** — a *separate* top-level block from `vocabulary.observations`, despite the near-identical name: DHT observation-event schemas carrying `retention_class`, `reach`, and the `graduates_to` / `graduation_policy` declarations. When you mean one, check which file you are in — `manifest/observations.json` and `manifest/observation-kinds.json` are different concerns.
- **`rendering`** — renderer id → `{ component, formats[] }`. The claim surface for the format tiers above.
- **`routeClaims`** — canonical URL template per content type (e.g. `path` → `path/{id}`, with a `step` fragment), generated into `route-claims.ts` so consumers construct and parse content routes without hardcoding path shape.
- **`gates`** — governance step-processes, each declaring `handlesEvents`, `governanceReach`, and a **`closure`** with a written `closureRationale`. `closure: "closed"` means **silence is refusal** — refusal to mint, to widen, to publish — never a provisional pass. A gate that cannot evaluate declines; "we could not tell" and "it is fine" must not be the same output. Authority: `elohim/elohim-agent/spec/2026-04-18-gate-interface.md`.
- **`attestations`** — the lamad-scoped attestation-kind registry. **Only one DNA entry type was actually retired:** `attestation:content-quality` replaced `ContentAttestation` (removal comments in `content_store_integrity/src/lib.rs`). `ContentSuccession` and `CustodianCommitment` are **still live `EntryTypes` variants with active callers** — the source marks them `KEPT`, deliberately out of the attestation-consolidation. So `attestation:content-succession` and `attestation:custodian-commitment` are manifest-side declarations coexisting with live DHT entry types, not replacements for them; treating either entry type as retired breaks real call sites. `attestation:mastery` is declared to graduate from private `ContentMastery` progress (Category B2) via `observation-kinds.json`'s `graduates_to` / `graduation_policy` — but **no runtime reads those fields today**; the one graduation executor implements a different policy. Treat mastery minting as designed, not wired.
- **`graph`** — declared node/edge types, indexes, and Datalog rules for the content graph. **The Datalog is declarative only: no runtime executes it.** The real graph is computed natively in Rust via `ContentGraphResolver`; Cozo/Kuzu/Apollo were considered and rejected. Extend the resolver trait — do not wire a Datalog engine to "finish" this block [[project_content_graph_native_rust_not_cozo_apollo]]. Note its lamad-scoped `SUPERSEDES` edge is distinct from manifest-level supersedence.

The gate-family content types are one coupled step-graph system, not five stubs: `gate-process-declaration` is the DAG a `RelationalImpactEvent` walks; `gate-rules-declaration`, `aggregation-spec`, and `escalation-target-spec` are the parameter artifacts its step types (`mechanical-ruleset`, `aggregate-attestations`, `escalate-to-review`) consume; `universal-band-declaration` is the structurally-identical protocol-root wrapper, governed by `commons` / `protocol-ratification` instead of the domain.

## Content Pipeline (end to end)

```
genesis/docs/*.md → elohim-import CLI → genesis/data/lamad/content/*.json
    ↓ seed-sqlite.ts (uses ConceptMetadata, PathMetadata from generated types)
POST /db/content/bulk (CreateContentInput — protocol wire type)
    ↓ elohim-storage (Rust: camelCase API → snake_case DB → camelCase response)
GET /db/content/{id} (ContentView — protocol wire type)
    ↓ ContentService.transformContent() → TypedContentNode
    ↓ isPathNode() → parsePathView() uses EprCompositeBody/Section/Item
    ↓ RendererRegistryService looks up contentFormat in the manifest renderer map
      └─ DOMAIN format claimed by a renderer → MarkdownRenderer / SophiaRenderer / …
      └─ core format, or unclaimed → raw-JSON fallback (the silent failure above)
    ↓ RendererCompletionEvent → SignalHarnessService → CreateEconomicEventInput
    ↓ POST /db/events/bulk
```

**Unverified hop:** seed relationships — `children[]`, `relatedNodeIds[]`, `contributors[].presenceId` — are bare slugs with no verify step anywhere in this pipeline. They drift silently and the sealing fix is deferred by design [[project_epr_link_first_class_seed_authoring]]. Regenerating seed data off this diagram can ship broken relationships that validate clean.

## Related Files

| Purpose | Path |
|---|---|
| Protocol schemas | `elohim/sdk/schemas/v1/` (see `elohim/sdk/schemas/CLAUDE.md`) |
| Manifest schema | `elohim/sdk/schemas/v1/manifest/app-manifest.schema.json` |
| Wire-types pattern | `elohim/sdk/domains/CLAUDE.md` |
| Gate interface | `elohim/elohim-agent/spec/2026-04-18-gate-interface.md` |
| Lamad app (own gate) | `app/lamad/CLAUDE.md` |
| Content service (core: transport/blob) | `app/elohim-app/src/app/elohim/services/content.service.ts` |
| Content service (learning-domain) | `app/lamad/src/app/services/content.service.ts` — separate implementation, not a re-export |
| Cross-pillar content model | `app/lamad/src/app/models/content-node.model.ts` — imported by five pillars; see the core/pillar note above |
| Path model | `app/lamad/src/app/models/learning-path.model.ts` |
| Signal harness | `app/lamad/src/app/services/signal-harness.service.ts` |
| Renderer registry | `app/lamad/src/app/renderers/renderer-registry.service.ts` |
| Design docs | `genesis/plans/2026-03-27-typed-content-pipeline-design.md`, `genesis/plans/2026-03-28-feedback-information-flows-design.md`, `genesis/plans/2026-03-27-sprint-{1-5}-*.md` |

Editing this file: it is a gospel surface in the cite graph — citations go in with `cite-gen --seal` / `cite-describe`, never hand-written slugs [[feedback_managed_surface_edit_discipline]] — and it describes stable architecture, never where-we-are [[feedback_agent_prompts_no_process_status]].
