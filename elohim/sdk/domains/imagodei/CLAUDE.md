# Imagodei Domain

This directory is the **imagodei protocol domain** — the identity pillar's vocabulary, metadata schemas, and coupling contracts. Imagodei grounds the protocol in demonstrated capability and community trust.

## Two-Layer Type Architecture

```
Protocol SDK (elohim/sdk/schemas/)          This Domain (elohim/sdk/domains/imagodei/)
├─ Wire types: ContentView,                 ├─ Domain types: HumanMetadata,
│  CreateContentInput,                      │  PresenceMetadata
│  CreateAttestationInput                   ├─ Coupling map: value flows + signals
├─ Enums: ContentType, Reach,              │  per content type
│  SubstrateSignal                          ├─ Type guards: isHumanNode(),
├─ Manifest schema: validates              │  isContributorNode()
│  three-leg coupling structure             └─ manifest.json: vocabulary + coupling
└─ Generic metadata: {}
```

The protocol owns the **envelope** (wire shape, field names, generic metadata bag). This package owns the **payload** (what metadata means for identity entities, what signals each interaction produces, how identity couples to the other pillars).

## How Identity Differs from Other Domains

Identity content types (human, role, contributor) are **embedded in the app shell**, not content-rendered. There are no imagodei-specific renderers — profiles, presence indicators, and attestation UIs are Angular components in `app/elohim-app/src/app/imagodei/components/`, not content format renderers.

The manifest declares coupling and signals, not rendering. Identity is the **ground layer** that all other domains reference: lamad mastery attaches to a human, shefa value flows to a contributor, qahal governance weight derives from attestations.

## Directory Structure

```
elohim/sdk/domains/imagodei/
├── manifest.json               # Vocabulary: 3 content types, 8 relationships, 6 signals
│                                 Each content type declares three-leg coupling:
│                                 knowledge (graph edges) + value (REA flows) + governance (reach + model)
│                                 Plus claims (feedback: what outcomes it asserts, validity horizon)
├── schemas/                    # Metadata schemas per content type
│   ├── human-metadata.schema.json       # { displayName, bio, agencyStage, affinities, ... }
│   └── presence-metadata.schema.json    # { presenceState, affinityTotal, citationCount, ... }
└── scripts/
    └── codegen.mjs             # Reads manifest + schemas → generates TypeScript
```

## Content Types

| Type | Category | Description |
|------|----------|-------------|
| `human` | B (agent-scoped) | A person in the network. One per agent. |
| `role` | A2 (derived via link) | A functional role (steward, elder, reviewer). Gates capabilities. |
| `contributor` | A (notarized) | Contributor presence in the content graph. Bridge to stewardship. |

## Cross-Pillar Coupling

Imagodei is the ground layer. Its content types couple to every other pillar:

- **lamad**: Contributors STEWARD learning content. Mastery attestations from lamad gate role assignments.
- **shefa**: Contribution produces stewardship-standing (value). Attestation accuracy feeds back as economic signal.
- **qahal**: Roles are SCOPED_TO collectives. Agency stage gates governance participation weight.
- **avodah**: Work capability is gated by attestations — demonstrated competence, not institutional credentials.

The attestation gate between lamad (wisdom) and avodah (action) is where the protocol prevents rent extraction. Universities and licensing boards capture this gate today; the protocol makes it transparent, community-governed, and coupled to demonstrated capability.

## Generated Output

`codegen.mjs` produces files to one location:

| Location | Consumer |
|----------|----------|
| `app/elohim-app/src/app/imagodei/generated/` | Angular app (import via `@app/imagodei/generated/`) |

Generated files:

| File | Contents |
|------|----------|
| `metadata-types.ts` | `HumanMetadata`, `PresenceMetadata` interfaces |
| `content-node-types.ts` | `TypedIdentityNode` discriminated union, `isHumanNode()` / `isContributorNode()` type guards |
| `coupling-map.ts` | `IMAGODEI_COUPLING_MAP` — value flows and governance signals per content type |
| `manifest-types.ts` | Content type lists, relationship types, signal map |

No seeder output — identity types are not seeded from JSON files. Humans and contributors are created through protocol interactions.

## Commands

```bash
# Generate imagodei domain types
pnpm run imagodei:codegen

# Verify generated files are up to date
pnpm run imagodei:codegen:verify

# Validate manifest against protocol schema
pnpm run schema:test
```

## Rules

### Schema before code

Edit the schema first, then regenerate. Never hand-write types that a schema should own.

1. Protocol primitives (enums, wire types) → edit in `elohim/sdk/schemas/v1/`, run `pnpm run schema:codegen:ts`
2. Domain metadata shapes → edit in `elohim/sdk/domains/imagodei/schemas/`, run `pnpm run imagodei:codegen`
3. Vocabulary (content types, signals, coupling) → edit `manifest.json`, run `pnpm run imagodei:codegen`

### Typed metadata, not string keys

```typescript
// WRONG — untyped metadata access
const name = (node.metadata as Record<string, unknown>)['displayName'];

// RIGHT — use type guard to narrow, then access typed metadata
if (isHumanNode(node)) {
  const name = node.metadata.displayName; // HumanMetadata — typed
}
```

### Attestation accuracy has negative feedback

The manifest requires `attestation-inaccuracy` as a negative observation. If attested claims don't hold up in downstream work, the system must detect and signal this. Positive-only attestation is not allowed — it would recreate credential inflation.

### Affinity accrues from curation, not attention

`PresenceMetadata.affinityTotal` grows through active curation work (edits, reviews, dispute resolution), NOT through learner engagement/attention. Learner attention flows as reciprocal value (tokens/energy) through shefa, but does not inflate governance standing.

## Key Files in Angular Pillar

| Purpose | Path |
|---------|------|
| Auth service | `app/elohim-app/src/app/imagodei/services/auth.service.ts` |
| Identity service | `app/elohim-app/src/app/imagodei/services/identity.service.ts` |
| Agency service | `app/elohim-app/src/app/imagodei/services/agency.service.ts` |
| Profile component | `app/elohim-app/src/app/imagodei/components/profile/` |
| Presence components | `app/elohim-app/src/app/imagodei/components/presence-list/` |
| Agency badge | `app/elohim-app/src/app/imagodei/components/agency-badge/` |
| Models | `app/elohim-app/src/app/imagodei/models/` |

## Related Files

| Purpose | Path |
|---------|------|
| Protocol schemas | `elohim/sdk/schemas/v1/` |
| Protocol codegen | `elohim/sdk/schemas/scripts/codegen-ts.mjs` |
| Manifest schema | `elohim/sdk/schemas/v1/manifest/app-manifest.schema.json` |
| Lamad domain (reference) | `elohim/sdk/domains/lamad/` |
| Design doc | `genesis/plans/2026-03-29-domain-manifests-sdk-boundary-design.md` |
| Sprint plan | `genesis/plans/2026-03-29-sprint-B-imagodei-domain-manifest.md` |
