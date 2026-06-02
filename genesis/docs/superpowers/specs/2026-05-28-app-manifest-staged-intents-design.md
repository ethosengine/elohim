---
status: Substrate LANDED (schema + codegen + tests on disk) — feature HELD on the unbuilt session-bridge consumer
---

# App-Manifest Staged-Intents & Graduation Vocabulary — Design

> **STATUS (compacted-in-place 2026-06-02):** The **substrate LANDED** — the schema extension, the
> `SessionLifecycleState` enum, the per-pillar codegen, and the validation tests are on disk and validate clean
> (the truth now lives in the source files, not in this spec; the now-landed how-to sections §6/§7/§11/§12 were
> stripped — recover from git history if needed).
>
> **The FEATURE is HELD, not verified-stable:** **zero pillar manifests declare a `stagedIntent`**, and the
> **session-bridge consumer crate does not exist** (`crates/session-bridge/` is genuinely live, NOT landed — see
> `2026-05-28-session-bridge-design.md` + implementation plan). Assert only "substrate landed"; do not claim the
> staged-intents vocabulary is exercised end-to-end. This spec is **NOT yet an architecture seed** — the consuming
> primitive is unbuilt, so the vocabulary is not stable enough to graduate to `tier: architecture`.
>
> **Silent-failure trap to remember:** ceremony IDs are validated only as **non-empty strings** — a typo passes
> manifest validation and fails later at graduation-time registry lookup.
>
> **Residual open thread (plant the pointer THERE, not here):** the four `SessionLifecycleState` values landing as
> protocol-core Standings via a §5.1 patch to the Capability-Profile element-contract spec
> (`2026-05-20-capability-profile-element-contract-design.md`) — verify if landed and plant the confirmation in that spec.
>
> **Scope:** This spec covers ONLY the app-manifest substrate extension. The session-bridge primitive itself is specified separately (`genesis/docs/superpowers/specs/2026-05-28-session-bridge-design.md`). The runtime `CeremonyRegistry` pattern lives in the bridge crate, not here. The wisdom-invocation batch extension that powers negotiated graduation is a separate spec (deferred).
>
> **Origin:** Refinement work on the session-bridge implementation plan (`genesis/docs/superpowers/plans/2026-05-28-session-bridge-implementation.md` §1.7 + §1.8) surfaced this as a discrete substrate layer. The bridge needs manifest-driven extensibility so third-party pillars can contribute onboarding experiences without modifying the bridge crate, the doorway routes, or the elohim-storage runtime. This spec names that contract.

---

## §0 — Why this exists

The app-manifest substrate is the protocol's extensibility surface. Pillars declare their `contentTypes`, `signalKinds`, `attestations`, `projections`, `observations`, `graph` extensions, and three-leg coupling there. The substrate validates the SHAPE of the declarations; apps validate their own vocabulary.

The session-bridge primitive (`2026-05-28-session-bridge-design.md` §2) introduces a class of vocabulary the existing manifest schema has no home for: **pre-canonical staged-intent shapes**. These are intent payloads a participant expresses during anonymous / oauth-identified / peer-native-sampling lifecycles — held by the bridge until graduation, then replayed into canonical entries.

Without a manifest substrate for this vocabulary, the bridge crate would have to hardcode the pillar discriminator enum + the per-pillar intent shapes + the per-pillar graduation policy. Adding a new pillar's onboarding experience would require recompiling the bridge. The substrate's stewardship principle ([stewardship-over-sovereignty](epr:stewardship-over-sovereignty) §3) and the SDK boundary canon (`genesis/docs/architecture/elohim-sdk.md` §1) both reject this shape — pillar vocabulary must be declared by the pillar's manifest, not embedded in shared substrate.

This spec proposes two additive sections to the app-manifest schema:

1. **`vocabulary.stagedIntents`** — per-pillar declarations of the staged-intent shapes the pillar supports.
2. **`graduation`** — per-pillar declarations of how the bridge should compose the graduation ceremony for that pillar's intents.

Both sections are optional. Existing manifests continue to validate clean. Pillars adopt the substrate when they're ready to contribute an onboarding experience.

---

## §1 — Background and canon basis

### §1.1 — The app-manifest substrate

The protocol-level schema lives at `elohim/sdk/schemas/v1/manifest/app-manifest.schema.json`. The canonical per-pillar implementation lives at `elohim/sdk/domains/lamad/manifest.json` + the modular split pattern at `elohim/sdk/domains/lamad/manifest/`. The other pillars (`elohim`, `imagodei`, `qahal`, `shefa`, `mishpat`, `avodah`, `infrastructure`) follow the same shape.

Today the manifest's `vocabulary` carries:
- `contentTypes` — per-type metadata schema + three-leg coupling (knowledge + value + governance) + claims
- `contentFormats` — declared renderer mappings
- `relationships` — knowledge-graph relationship types
- `signals` — substrate-signal mappings per content type
- `observations` — substrate-known observation kinds

The top-level `rendering`, `gates`, `attestations`, `graph`, `projections`, `writeThrough`, `observation_kinds`, `signalKinds` round out the manifest. The schema-before-code IoC discipline (`project_schema_first_ioc`) means the schema is the wire contract; Rust and TypeScript conform to it.

### §1.2 — The session-bridge primitive's contract on manifests

Per the session-bridge spec §2 + §3:
- Each pillar that participates in tentative participation declares one or more `StagedIntent` shapes.
- Each shape names the canonical entry it graduates to, the lifecycle states it's actionable from, and how it resolves at graduation (deterministic vs negotiated).
- Each pillar has a per-pillar graduation policy (which ceremony runs deterministic resolutions; which ceremony runs negotiated ones; which elohim role appraises; whether the appraisal is notarized).

Today none of this has a manifest home. This spec fills the gap.

### §1.3 — What the bridge expects from the manifest at runtime

The session-bridge crate (per the implementation plan §3 B-CRATE) consumes:
- The list of declared `stagedIntents` per pillar — to know which intent shapes to deserialize per pillar discriminator
- The `actionableFrom` array per intent — to enforce `is_actionable()` predicates without per-impl hardcoding
- The `resolutionMode` per intent — to route to deterministic-only vs negotiated paths
- The per-pillar `graduation.deterministicCeremony` + `graduation.negotiatedCeremony` ceremony IDs — to look up `GraduationCeremony` impls in the runtime `CeremonyRegistry`
- The per-pillar `graduation.framingCid` — to compose the wisdom-invocation framing per pillar (the substrate-level Q7 deferral resolves through this)
- The per-pillar `graduation.appraisalAgent` + `graduation.notarizeAppraisal` — to configure the appraisal flow

The manifest is the declaration; the runtime registry resolves declarations into impls; the bridge composes runtime behavior from both.

---

## §2 — The `vocabulary.stagedIntents` section

### §2.1 — Shape

Mirrors the existing `vocabulary.contentTypes` pattern. Each entry has the same architectural discipline: a metadata schema reference + a three-leg coupling declaration. The coupling discipline is preserved deliberately — staged intent is still vocabulary the substrate stewards, and substrate vocabulary always carries its relational signature.

```jsonc
"vocabulary": {
  "stagedIntents": {
    "staged-mastery-intent": {
      "description": "Tentative mastery attestation expressed during pre-member lifecycle; graduates to a canonical ContentMastery entry on the agent's source chain at incarnation.",
      "intentSchema": { "$ref": "./schemas/staged-mastery-intent.schema.json" },
      "graduatesTo": "ContentMastery",
      "actionableFrom": ["OauthIdentified", "PeerNativeSampling", "PeerNativeMember"],
      "resolutionMode": "deterministic",
      "coupling": {
        "knowledge": {
          "relationships": {
            "REFERENCES": ["concept", "lesson", "assessment"]
          }
        },
        "value": {
          "onGraduate": {
            "action": "produce",
            "resourceConformsTo": "mastery-record",
            "recognition": "mastery-record"
          }
        },
        "governance": {
          "defaultReach": "self",
          "minimumReach": "self",
          "governanceModel": "self-sovereign",
          "signalTypes": ["mastery-recorded"]
        }
      }
    }
  }
}
```

### §2.2 — Required fields per entry

| Field | Type | Constraint | Purpose |
|---|---|---|---|
| `description` | string | non-empty | Human-readable. What this intent represents pre-canonically. Used in operator-facing tools + Sub-project #4 of Capability Profile spec (deferred appStandings substrate). |
| `intentSchema` | `$ref` | resolves to a JSON Schema in the pillar's `schemas/` directory | The payload shape. Cross-references protocol enums (`Reach`, `SubstrateSignal`, `SessionLifecycleState`, etc.) per `elohim/sdk/schemas/CLAUDE.md` Adding-a-new-input-schema pattern. |
| `graduatesTo` | string | names a canonical entry type the protocol already supports | What the staged intent replays into at graduation. Validation does NOT verify this entry type exists — runtime resolution does. The string is documentation + a runtime lookup key. |
| `actionableFrom` | array of `SessionLifecycleState` enum values | non-empty | Which lifecycle states the intent is actionable from. The bridge's `is_actionable()` predicate reads this directly. |
| `resolutionMode` | enum | `"deterministic"` \| `"negotiated"` \| `"either"` | Whether graduation is 1:1 (deterministic — no appraisal needed) or requires wisdom-invocation appraisal (negotiated). `"either"` means the bridge tries deterministic first; if the predicate fails, falls back to negotiated. |
| `coupling` | object | three-leg discipline | Knowledge / value / governance coupling per the existing `contentTypes` shape. See §2.3. |

### §2.3 — Three-leg coupling discipline (why intents declare coupling)

The same architectural reason `contentTypes` carry three-leg coupling applies to `stagedIntents`: staged intent is vocabulary, and substrate vocabulary's relational signature is its contract with the rest of the protocol. Specifically:

- **Knowledge leg** — names the EPR relationship types the graduated entry will participate in (the intent shape itself is operational, not notarized, but the canonical entry it graduates to lives in the knowledge graph). Example: `staged-mastery-intent` REFERENCES `concept | lesson | assessment` — at graduation, the canonical `ContentMastery` carries that relationship type.
- **Value leg** — names the REA event that the graduation ceremony emits (or that the substrate composes from the staged intent). For most staged intents, `onGraduate` is the relevant value-leg action — produced when the intent crosses from tentative to canonical. Differs from `contentTypes`' `onConsume` / `onContribute` / `onComplete` triplet because the lifecycle event is "graduation," not "consumption."
- **Governance leg** — names the reach/standing surface the graduated entry will be authored under, plus the signal types emitted at graduation. The `defaultReach` and `minimumReach` here describe the CANONICAL entry's reach, not the intent's. Visitor and OauthIdentified states have no reach surface; the intent is pre-canonical; its eventual entry will be authored at the declared reach when the participant has standing to author it.

### §2.4 — Validation discipline

The schema validator enforces:
1. Each `stagedIntents` entry has all required fields (§2.2).
2. The `intentSchema` `$ref` resolves to a file the validator can load (the same `resolveRefs` helper at codegen + validation time, per `elohim/sdk/CLAUDE.md` modular manifest pattern).
3. Each `actionableFrom` array entry validates against the canonical `SessionLifecycleState` enum (per §4 below).
4. `resolutionMode` is one of the three enum values.
5. `coupling` validates against the existing three-leg coupling sub-schema reused from `contentTypes`.

The validator does NOT enforce:
- `graduatesTo` references an entry type that actually exists (string is a runtime lookup key).
- `intentSchema` is materially different from other staged intents in the same pillar (drift would be caught at runtime by the bridge's lookup, not at manifest validation).

### §2.5 — Anti-patterns the section prevents

| Anti-pattern | Why it fails | This section's fix |
|---|---|---|
| Hardcoded pillar discriminator enum in the bridge crate | Adding a new pillar requires recompiling the bridge | Pillar discriminator is manifest-driven; new pillars register their manifests + ceremony impls; bridge discovers at runtime |
| Staged intents without coupling declarations | The graduated canonical entry's reach/signal surface is invisible to manifest auditors | Three-leg coupling discipline is required, mirroring `contentTypes` |
| Anonymous lifecycle staging consent / mastery / membership intents | Consent without an identifiable consenter is meaningless; mastery without an agent is no-op; membership without an applicant is invalid | `actionableFrom` array gates this at the schema level — anonymous-from intents are explicitly excluded for shapes that require identity |
| Staged intents that don't name a canonical entry to graduate to | Tentative intent without canonical destination has no protocol meaning | `graduatesTo` is required |
| Bridge-shaped logic leaking into element library code | Lifecycle-aware rendering should consult the Capability Profile primitive, not the bridge directly | The pre-member lifecycle values become protocol-core Standings (per session-bridge implementation plan §6 Q15); elements consult the Profile via the existing `<elohim-standing-refused>` slot pattern |

---

## §3 — The `graduation` top-level section

### §3.1 — Shape

```jsonc
"graduation": {
  "deterministicCeremony": "lamad::DeterministicMasteryAndPathCeremony",
  "negotiatedCeremony": "lamad::NegotiatedReflectionCeremony",
  "framingCid": "epr:lamad:graduation-framing-v1",
  "appraisalAgent": "home-elohim",
  "notarizeAppraisal": "on-request"
}
```

### §3.2 — Required fields per pillar (when the section exists)

| Field | Type | Constraint | Purpose |
|---|---|---|---|
| `deterministicCeremony` | string | non-empty; convention: `<pillar>::<CeremonyName>` | Runtime registry lookup key for the deterministic `GraduationCeremony` impl. The convention is namespacing by pillar to prevent cross-pillar collision. Validation does NOT check the impl exists — runtime composition does. |
| `negotiatedCeremony` | optional string | same convention | Runtime registry lookup key for the negotiated ceremony. If absent, the pillar's staged intents may only resolve deterministically (negotiated `resolutionMode` entries become rejected with a "no negotiated ceremony registered" reason). |
| `framingCid` | optional EPR CID | resolves to a published EPR | The wisdom-invocation framing the bridge composes when invoking appraisal. Per the session-bridge implementation plan §6 Q7, this is where the per-pillar elohim-role declaration lives — the framing CID dictates which elohim runs the appraisal + with what context + under what constitution. Q7 is deferred at the operator level but enabled at the schema level by this field. |
| `appraisalAgent` | optional enum | `"home-elohim"` \| `"commons-elohim"` \| `"neutral-counsel"` \| `"custom"` | Convenience hint for runtime composition when `framingCid` doesn't fully specify. The default per the implementation plan §6 Q7 v1 is `"home-elohim"`. `"custom"` means the framing CID is fully responsible. |
| `notarizeAppraisal` | optional enum | `"always"` \| `"on-request"` \| `"never"` | Default: `"on-request"`. Whether the bridge authors a `Manifest{kind: "graduation-record"}` entry per graduation. Implementation plan §6 Q8 default. |

### §3.3 — Validation discipline

- All fields are optional EXCEPT `deterministicCeremony` (when the section exists at all). A pillar that declares `stagedIntents` MUST declare at least how deterministic resolution composes.
- Ceremony strings validate as non-empty; namespace convention is checked at codegen time (warning, not error).
- `framingCid` validates as a CIDv1 if present (per the existing EPR CID validator).
- `appraisalAgent` + `notarizeAppraisal` enum values are validated against the canonical lists.

---

## §4 — Companion enum: `SessionLifecycleState`

A new protocol-level enum at `elohim/sdk/schemas/v1/enums/session-lifecycle-state.schema.json`. Follows the existing enum-codegen pattern from `elohim/sdk/schemas/CLAUDE.md`.

### §4.1 — Schema

```jsonc
{
  "$id": "epr:schema:enum:session-lifecycle-state",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "SessionLifecycleState",
  "type": "string",
  "enum": ["Anonymous", "OauthIdentified", "PeerNativeSampling", "PeerNativeMember"],
  "_tiers": {
    "core": {
      "values": ["Anonymous", "OauthIdentified", "PeerNativeSampling", "PeerNativeMember"],
      "rationale": "All four lifecycle states are protocol-core; no app-tier extensions."
    }
  },
  "_dna": {
    "constant": "SESSION_LIFECYCLE_STATES",
    "zome": null,
    "rationale": "Operational vocabulary — bridge state, not DHT-notarized state. No zome owns these as entries."
  }
}
```

### §4.2 — Generated artifacts

`pnpm run schema:codegen:ts` produces `CORE_SESSION_LIFECYCLE_STATES`, `ALL_SESSION_LIFECYCLE_STATES`, the `SessionLifecycleState` type alias, and the backward-compat alias per the existing enum-codegen pattern. Distributed to the standard four locations (`genesis/seeder/src/generated/`, `app/elohim-app/src/app/generated/`, `app/elohim-library/projects/elohim-service/src/generated/`, plus a fifth: `app/elohim-library/projects/elohim-identity/src/generated/` — the new distribution dir per the session-bridge implementation plan §1.7).

### §4.3 — Relationship to Capability Profile spec §5.1 protocol-core Standings

The session-bridge implementation plan §6 Q15 names the parallel: these four lifecycle values become protocol-core Standings (HARD-enforced via the existing `<elohim-standing-refused>` slot pattern). A one-paragraph follow-up patch to `genesis/docs/superpowers/specs/2026-05-20-capability-profile-element-contract-design.md` §5.1 adds them to the protocol-core Standings enumeration. This spec doesn't author that patch; it ships the enum + the manifest substrate; the Capability Profile spec absorbs the Standings extension separately.

---

## §5 — Updates to `app-manifest.schema.json`

Two additive properties at the existing locations:

### §5.1 — Under `vocabulary` (sibling to `contentTypes`)

```jsonc
"vocabulary": {
  "type": "object",
  "properties": {
    "contentTypes": { ... existing ... },
    "contentFormats": { ... existing ... },
    "relationships": { ... existing ... },
    "signals": { ... existing ... },
    "observations": { ... existing ... },
    "stagedIntents": {
      "type": "object",
      "description": "Per-pillar staged-intent vocabulary. Each entry declares an intent shape the session-bridge holds pre-canonically and replays into a canonical entry at graduation. Optional — pillars without tentative-participation surfaces omit this section.",
      "additionalProperties": {
        "$ref": "#/$defs/StagedIntentDeclaration"
      }
    }
  }
}
```

### §5.2 — Top-level (sibling to `rendering`, `gates`, etc.)

```jsonc
"graduation": {
  "$ref": "#/$defs/GraduationPolicy",
  "description": "Per-pillar graduation policy. Names the ceremony registry IDs the bridge looks up at graduation time, the wisdom-invocation framing CID, the appraisal agent role hint, and the notarization mode. Required IF `vocabulary.stagedIntents` is non-empty."
}
```

### §5.3 — New `$defs` entries

`StagedIntentDeclaration` (per §2.2) and `GraduationPolicy` (per §3.2) are added under the existing `$defs` block at the bottom of `app-manifest.schema.json`. Both reuse the existing three-leg coupling sub-schema where applicable.

### §5.4 — Conditional validation

Add a JSON Schema `dependentRequired` rule: if `vocabulary.stagedIntents` is present AND non-empty, then top-level `graduation.deterministicCeremony` MUST be present. Pillars can declare staged intents without declaring graduation IF the section is empty (placeholder for future work).

### §5.5 — Backward compatibility

Existing manifests without these sections validate unchanged. The validation test `pnpm run schema:test` adds four assertions:
1. A manifest with `stagedIntents` validates clean.
2. A manifest missing `stagedIntents` validates clean.
3. A manifest with `stagedIntents` but missing top-level `graduation` fails with a clear error.
4. A `stagedIntents` entry missing `graduatesTo` fails with a clear error.

---

## §6 — Per-pillar codegen + §7 — Protocol-level codegen integration

> **Compacted-in-place 2026-06-02.** These two sections described now-LANDED codegen wiring; the truth lives on disk:
> per-pillar `staged-intents.ts` + `graduation-policy.ts` emitted from `elohim/sdk/domains/<pillar>/scripts/codegen.mjs`;
> the `SessionLifecycleState` enum + bridge wire types in the protocol enum/INTERFACE_FILES codegen path
> (`elohim/sdk/schemas/scripts/codegen-ts.mjs`), distributed to the standard four locations plus
> `@elohim/identity/generated/`; the schema-contract assertion in `elohim/elohim-storage/tests/schema_contract.rs`.
> Recover the original how-to from git history if a re-derivation is ever needed.

---


## §8 — Per-pillar manifest examples

### §8.1 — Lamad

```jsonc
{
  "id": "manifest-lamad",
  "name": "lamad",
  "version": "1.1.0",
  "vocabulary": {
    "contentTypes": { ... existing ... },
    "stagedIntents": {
      "staged-mastery-intent": {
        "description": "Tentative self-assessed mastery during sampling; graduates to a canonical ContentMastery entry.",
        "intentSchema": { "$ref": "./schemas/staged-mastery-intent.schema.json" },
        "graduatesTo": "ContentMastery",
        "actionableFrom": ["OauthIdentified", "PeerNativeSampling", "PeerNativeMember"],
        "resolutionMode": "deterministic",
        "coupling": { ... three-leg per §2.3 ... }
      },
      "staged-path-explored-intent": {
        "description": "Tentative path-exploration record during sampling; graduates to a HumanProgress entry update.",
        "intentSchema": { "$ref": "./schemas/staged-path-explored-intent.schema.json" },
        "graduatesTo": "HumanProgress",
        "actionableFrom": ["OauthIdentified", "PeerNativeSampling", "PeerNativeMember"],
        "resolutionMode": "deterministic",
        "coupling": { ... three-leg ... }
      }
    }
  },
  "graduation": {
    "deterministicCeremony": "lamad::DeterministicMasteryAndPathCeremony",
    "appraisalAgent": "home-elohim",
    "notarizeAppraisal": "on-request"
  }
}
```

### §8.2 — Imagodei

```jsonc
{
  "id": "manifest-imagodei",
  "vocabulary": {
    "stagedIntents": {
      "staged-consent-intent": {
        "description": "Tentative consent decision; anonymous consent is meaningless and is explicitly excluded from actionableFrom.",
        "intentSchema": { "$ref": "./schemas/staged-consent-intent.schema.json" },
        "graduatesTo": "Consent",
        "actionableFrom": ["OauthIdentified", "PeerNativeSampling", "PeerNativeMember"],
        "resolutionMode": "deterministic",
        "coupling": { ... }
      }
    }
  },
  "graduation": {
    "deterministicCeremony": "imagodei::DeterministicConsentCeremony"
  }
}
```

### §8.3 — Qahal

```jsonc
{
  "vocabulary": {
    "stagedIntents": {
      "staged-membership-application-intent": {
        "description": "Tentative qahal membership application; sponsor witnesses accrue during sampling; graduates to MembershipApplication entry.",
        "intentSchema": { "$ref": "./schemas/staged-membership-application-intent.schema.json" },
        "graduatesTo": "MembershipApplication",
        "actionableFrom": ["OauthIdentified", "PeerNativeSampling", "PeerNativeMember"],
        "resolutionMode": "either",
        "coupling": { ... }
      }
    }
  },
  "graduation": {
    "deterministicCeremony": "qahal::DeterministicMembershipApplicationCeremony",
    "negotiatedCeremony": "qahal::NegotiatedMembershipApplicationCeremony",
    "framingCid": "epr:qahal:graduation-framing-v1",
    "appraisalAgent": "commons-elohim",
    "notarizeAppraisal": "on-request"
  }
}
```

### §8.4 — Shefa

```jsonc
{
  "vocabulary": {
    "stagedIntents": {
      "staged-economic-event-intent": {
        "description": "Tentative REA EconomicEvent held pre-incarnation; intent shape is byte-identical to M-REA-1's LamadEventIntent. Graduation delegates to the existing POST /api/v1/lamad/events coordinator.",
        "intentSchema": { "$ref": "./schemas/staged-economic-event-intent.schema.json" },
        "graduatesTo": "EconomicEvent",
        "actionableFrom": ["OauthIdentified", "PeerNativeSampling", "PeerNativeMember"],
        "resolutionMode": "deterministic",
        "coupling": { ... }
      }
    }
  },
  "graduation": {
    "deterministicCeremony": "shefa::DelegateToMRea1Ceremony"
  }
}
```

### §8.5 — Mishpat (no staged intents)

```jsonc
{
  "id": "manifest-mishpat",
  "vocabulary": {
    "signalKinds": { ... }
    // No stagedIntents — mishpat governance moves require accrued standing per session-bridge spec §7
  }
  // No graduation section
}
```

### §8.6 — A hypothetical third-party pillar (tikvah)

```jsonc
{
  "id": "manifest-tikvah",
  "name": "tikvah",
  "version": "0.1.0",
  "vocabulary": {
    "stagedIntents": {
      "staged-pledge-intent": {
        "description": "Tentative future-state pledge expressed during sampling.",
        "intentSchema": { "$ref": "./schemas/staged-pledge-intent.schema.json" },
        "graduatesTo": "Pledge",
        "actionableFrom": ["OauthIdentified", "PeerNativeSampling", "PeerNativeMember"],
        "resolutionMode": "negotiated",
        "coupling": { ... }
      }
    }
  },
  "graduation": {
    "deterministicCeremony": "tikvah::DeterministicPledgeCeremony",
    "negotiatedCeremony": "tikvah::NegotiatedPledgeCeremony",
    "framingCid": "epr:tikvah:graduation-framing-v1",
    "appraisalAgent": "custom",
    "notarizeAppraisal": "always"
  }
}
```

The tikvah pillar lands its manifest + its Rust ceremony impls + a `register_tikvah_ceremonies` registry registration call. The bridge's runtime composition picks them up. No bridge-crate or doorway-routes modification needed.

---

## §9 — Relationship to runtime CeremonyRegistry

The manifest declares string IDs (`deterministicCeremony: "lamad::DeterministicMasteryAndPathCeremony"`). The runtime `CeremonyRegistry` (defined in the session-bridge crate per implementation plan §3 B-CRATE) maps these IDs to `Box<dyn GraduationCeremony>` impls.

The contract:
- **Manifest declares.** What ceremony name does this pillar use? What's the framing CID? What's the appraisal hint?
- **Pillar crate provides.** A `register_<pillar>_ceremonies(&mut registry)` function that registers each ceremony name → impl mapping.
- **Composition root composes.** Each runtime (doorway-service, elohim-storage, future third-party runtime) calls all pillar register functions at startup, passing them a shared `RuntimeCeremonyRegistry`.
- **Bridge looks up at graduation time.** Reads the manifest's ceremony ID, queries the registry, invokes the matching impl.

This spec covers ONLY the manifest declaration. The registry trait + composition pattern live in the bridge crate spec / implementation plan.

---

## §10 — Open design questions

These are NOT being resolved in this spec; flagging so the operator has the queue visible.

### §10.1 — Should `coupling` be required on staged intents?

The argument for required: substrate vocabulary always carries its relational signature; staged intents are no exception. The argument against: staged intents are pre-canonical operational vocabulary; their canonical entry (the `graduatesTo` target) already declares the relational signature in its `contentTypes` entry, so requiring it again on the staged-intent declaration is duplication.

**Spec's working default:** required (mirrors `contentTypes`). Operator can override.

### §10.2 — Ceremony ID namespacing convention

The spec proposes `<pillar>::<CeremonyName>` (e.g., `lamad::DeterministicMasteryAndPathCeremony`). Alternative: fully-qualified Rust paths (`lamad_pillar::ceremonies::DeterministicMasteryAndPathCeremony`). Alternative: bare names (`DeterministicMasteryAndPathCeremony`).

**Spec's working default:** `<pillar>::<CeremonyName>`. Operator can override.

### §10.3 — Per-staged-intent consent-required flag

Some staged intents may need explicit participant consent before staging (not just at graduation). Should the manifest declaration carry a `consentRequired: boolean` field? Or is that the bridge's runtime concern?

**Spec's working default:** out of scope for this manifest spec; bridge can layer consent gates on top per pillar. Operator can promote into the manifest if a use case surfaces.

### §10.4 — Manifest authorization for third-party pillars

The session-bridge implementation plan §6 Q14 raises this: what trust does the substrate require for a third-party pillar's manifest to be accepted? Manifest-signing? Stewardship attestation? Qahal-witness vouching?

**Spec's working default:** out of scope. The bridge's v1 default is compile-time + composition-root authorization (the operator deliberately adds the pillar crate to Cargo deps + invokes its register function). This spec's vocabulary is consumed by the bridge AFTER that authorization gate fires. Substrate-level authorization (e.g., a `Manifest{kind: "pillar-authorization"}` entry) is a separate spec.

### §10.5 — Cross-pillar staged intents

Some intents might touch multiple pillars at graduation (e.g., a `staged-stewardship-pledge-intent` that produces both a `Pledge` entry in tikvah AND a stewardship-allocation in shefa). The current schema assumes one pillar per intent. Should multi-pillar intents be supported?

**Spec's working default:** out of scope; multi-pillar graduation can be modeled as a compose of two single-pillar staged intents that the participant stages in sequence. If the substrate finds the workaround pattern is fragile, the schema extends.

### §10.6 — `resolutionMode: "either"` semantics

The spec defines `"either"` as "bridge tries deterministic first; falls back to negotiated." But the bridge needs to know HOW to decide. Possibilities: a predicate on the staged intent payload; a runtime hook the ceremony impl provides; a manifest-declared rule.

**Spec's working default:** the deterministic ceremony returns `GraduationFailure::RequiresNegotiation` to signal fallback to negotiated. Ceremony impl owns the decision. No manifest-level rule.

---

## §11 — Test surface + §12 — Implementation deltas

> **Compacted-in-place 2026-06-02.** These two sections enumerated the now-LANDED schema/codegen/test deltas; the
> truth lives on disk: the schema extension in `elohim/sdk/schemas/v1/manifest/app-manifest.schema.json`, the new
> `elohim/sdk/schemas/v1/enums/session-lifecycle-state.schema.json`, the seven validation assertions in
> `test-schema.mjs`, the per-pillar codegen extensions, and the backward-compat tests. **Caveat (HELD):** the
> per-pillar *manifests* do NOT yet declare any `stagedIntent` — the schema accepts them, but nothing exercises them
> end-to-end until the session-bridge consumer crate lands. Recover the original delta tables from git history if needed.

---

## §13 — Relationship to existing canon

| Canon | Relationship |
|---|---|
| `elohim/sdk/schemas/CLAUDE.md` (schema-before-code IoC) | Honored. The schema is the contract; Rust + TypeScript conform. |
| `elohim/sdk/domains/lamad/CLAUDE.md` (per-pillar codegen pattern) | Extended. New artifacts (`staged-intents.ts`, `graduation-policy.ts`) follow the existing distribution pattern. |
| `genesis/docs/architecture/elohim-sdk.md` §3.3 (`@elohim/storage-client` codegen) | Extended. New wire types + new distribution dir (`@elohim/identity/generated/`). |
| `genesis/docs/architecture/stewardship-over-sovereignty.md` §3 (substrate-as-steward) | Honored. Pillar vocabulary is pillar-stewarded; bridge composes against declarations, doesn't embed them. |
| `genesis/docs/architecture/cradle-to-grave-capability-gradient.md` (life-stage gradient) | Honored. The pre-stage gradient (anonymous → oauth-identified → peer-native-sampling → peer-native-member) is named in the substrate via `SessionLifecycleState` and resolves through the existing Capability Profile primitive. |
| `genesis/docs/superpowers/specs/2026-05-20-capability-profile-element-contract-design.md` §5.1 (protocol-core Standings) | Forward-references. The four lifecycle values land as protocol-core Standings via a one-paragraph patch (NOT authored here; authored alongside this spec's implementation). |
| Session-bridge spec (`2026-05-28-session-bridge-design.md`) | Companion. This spec covers the manifest substrate; that spec covers the bridge primitive. They compose. |
| Session-bridge implementation plan (`2026-05-28-session-bridge-implementation.md`) §1.7 | Direct implementation path. The plan's B-MANIFEST ticket lands this spec's schema changes. |

---

## §14 — What this spec is NOT

- Not the session-bridge primitive spec. That exists separately.
- Not the `CeremonyRegistry` trait spec. That lives in the bridge crate's implementation.
- Not the wisdom-invocation batch-extension spec. That's separate (deferred per the implementation plan §6 Q7 work).
- Not the Capability Profile `appStandings` schema (Sub-project #4 of the Capability Profile spec). Separately deferred there.
- Not a third-party pillar authorization spec. Q10.4 deferral.
- Not a runtime-execution spec. Manifest declares; runtime composes; this spec only covers the manifest contract.

---

## §15 — References

- Companion spec (the primitive this manifest substrate supports): `genesis/docs/superpowers/specs/2026-05-28-session-bridge-design.md`
- Implementation plan that lands this spec: `genesis/docs/superpowers/plans/2026-05-28-session-bridge-implementation.md` — see B-MANIFEST ticket + §1.7 + §1.8
- Canonical manifest example: `elohim/sdk/domains/lamad/manifest.json` (+ split files)
- App-manifest schema: `elohim/sdk/schemas/v1/manifest/app-manifest.schema.json`
- Schema-codegen canon: `elohim/sdk/schemas/CLAUDE.md`
- Per-pillar codegen canon: `elohim/sdk/domains/lamad/CLAUDE.md`
- SDK boundary canon: `genesis/docs/architecture/elohim-sdk.md`
- Stewardship canon: `genesis/docs/architecture/stewardship-over-sovereignty.md`
- Capability Profile spec (forward-reference for the lifecycle Standings landing): `genesis/docs/superpowers/specs/2026-05-20-capability-profile-element-contract-design.md`
- Schema-first IoC memory anchor: `project_schema_first_ioc`
- Manifest-as-policy-substrate memory anchor: `project_doorway_manifest_driven_routes`
- DNA-as-SDK-boundary memory anchor: `project_elohim_dna_as_sdk_boundary`
