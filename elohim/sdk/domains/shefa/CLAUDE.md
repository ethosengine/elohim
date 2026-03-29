# Shefa Domain

This directory is the **shefa protocol domain** — the economy pillar's vocabulary, metadata schemas, and coupling contracts. Shefa declares how value flows through the protocol: stewardship, economic events, resource accounting, and obligation tracking.

## Shefa Wraps Protocol Primitives

Unlike lamad (which owns many content types), shefa primarily wraps protocol-level REA primitives with economic coupling semantics. The protocol owns `EconomicEvent`, `Agreement`, `Commitment`, and `Resource` as wire types. Shefa declares:

- What **signals** each economic interaction produces
- What **metadata** shapes describe stewardship and exchange
- What **observations** the protocol should make about economic health
- How economic acts **couple** across the three legs (value + governance + feedback)

Shefa owns one content type: `stewardship-context` (Category A2, derived via link).

## Value Accounting, Not Currency

Shefa is value accounting — tracking what was produced, consumed, transferred, and by whom. It is not a currency system. Key mechanisms:

- **Demurrage**: Standing decays without active curation work. Prevents dormant capture.
- **Circulation rights**: What a steward can do with content — governs reach expansion.
- **Obligation tracking**: Agreements produce commitments; unfulfilled commitments accumulate as negative observations.
- **Resource nature**: Rivalry, excludability, depletability, fungibility, circularity dimensions from REA extension.

## Stewardship as Anti-Capture

The mastery gate + affinity lifecycle makes content self-governing:

1. **Mastery gate**: Can't build steward affinity without proving deep content understanding (lamad `apply` level)
2. **Affinity accrual**: Earned through sustained curation (edits, reviews, dispute resolution), NOT through attention or consumption
3. **Community resistance**: Other stewards have governance standing (qahal) to resist hostile changes
4. **Demurrage**: Standing decays without ongoing engagement — no dormant landlords

**Critical distinction**: Learner attention does NOT increase steward affinity. That would recreate the attention economy. Affinity only grows through active curation work.

## Cross-Pillar Coupling

The conversation between lamad (wisdom) and avodah (action) is where shefa value is created:

```
lamad (wisdom)  ←→  avodah (action)
      ↕                    ↕
   attestation gates    value creation
      ↕                    ↕
imagodei (identity) → shefa (value) → qahal (governance)
```

| Coupling | Trigger | Produces | Gate |
|----------|---------|----------|------|
| lamad → shefa | mastery-achieved | stewardship-eligibility | mastery-level >= apply |
| shefa → lamad | stewardship-allocated | steward-recognition | economic-event flows to steward |
| shefa → qahal | custodian-attestation | governance-standing | affinity-score > threshold |
| imagodei → shefa | identity-created | economic-agency | presence-established |

## Directory Structure

```
elohim/sdk/domains/shefa/
├── manifest.json               # Vocabulary: 1 content type, 4 protocol primitives,
│                                 4 relationships, 6 signals, 7 observations
│                                 Plus cross-pillar coupling declarations
├── schemas/
│   ├── stewardship-metadata.schema.json   # { allocationStrategy, affinityScore, custodianRole, demurrageRate, ... }
│   ├── exchange-metadata.schema.json      # { offerType, requestType, terms, resourceNature, ... }
│   └── agreement-metadata.schema.json     # { parties, obligations, fulfillmentCriteria, state }
├── scripts/
│   └── codegen.mjs             # Reads manifest + schemas → generates TypeScript
└── CLAUDE.md                   # This file
```

## Generated Output

`codegen.mjs` produces files to one location:

| Location | Consumer |
|----------|----------|
| `app/elohim-app/src/app/shefa/generated/` | Angular app (import via `@app/shefa/generated/`) |

Generated files:

| File | Contents |
|------|----------|
| `metadata-types.ts` | `StewardshipMetadata`, `ExchangeMetadata`, `AgreementMetadata` interfaces |
| `coupling-map.ts` | `SHEFA_COUPLING_MAP` — value flows and governance signals per content type |
| `manifest-types.ts` | Content type lists, signal map, relationship types |

## Commands

```bash
# Generate shefa domain types
pnpm run shefa:codegen

# Check if generated files are stale
pnpm run shefa:codegen:verify
```

## Rules

### Schema before code

Edit the schema first, then regenerate. Never hand-write types that a schema should own.

1. Protocol primitives (enums, wire types) → edit in `elohim/sdk/schemas/v1/`, run `pnpm run schema:codegen:ts`
2. Domain metadata shapes → edit in `elohim/sdk/domains/shefa/schemas/`, run `pnpm run shefa:codegen`
3. Vocabulary (signals, coupling) → edit `manifest.json`, run `pnpm run shefa:codegen`

### Stewardship standing is earned, not granted

Any design that allows stewardship standing to be acquired without demonstrated mastery and sustained curation work is wrong. The cost of building affinity IS the security model.

### Shefa is UX, not truth

The Angular services in `app/elohim-app/src/app/shefa/` are the experience layer. Protocol truth lives on the DHT. Shefa reads from storage (fast) but writes should go through the conductor (truthful). See `app/elohim-app/src/app/shefa/CLAUDE.md` for the Angular layer.

## Related Files

| Purpose | Path |
|---------|------|
| Protocol schemas | `elohim/sdk/schemas/v1/` |
| Angular shefa pillar | `app/elohim-app/src/app/shefa/` |
| Angular shefa CLAUDE.md | `app/elohim-app/src/app/shefa/CLAUDE.md` |
| REA economics skill | `.claude/skills/rea-economics/SKILL.md` |
| Steward affinity design | Memory: `project-steward-affinity-anti-capture.md` |
| Resource nature design | Memory: `project-resource-nature-circularity.md` |
| Feedback design | `genesis/plans/2026-03-28-feedback-information-flows-design.md` |
| Pillar topology | Memory: `project-pillar-topology-power-responsibility.md` |
