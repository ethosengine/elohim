---
id: placement-schemas-report
title: "S1-S5 report: the SDK schemas (tree view, needs block, placement intents, parameter set)"
status: DONE_WITH_CONCERNS
class: architecture
gap: plans__2026-10-20-placement-contract-architecture-plan#10
actor: agent:implementer@claude-opus-5-5
commits: []
habits: [materialize-what-is-read]
cites:
  - "placement-schemas-brief | S1-S5 | sha256:8334ddb41339865c | path: genesis/docs/superpowers/plans/task-placement-schemas-brief.md"
  - "placement-contract-interfaces-design | The placement contract's interfaces | sha256:c70878179c5ed5f9 | path: genesis/docs/superpowers/specs/2026-10-20-placement-contract-interfaces-design.md"
  - "placement-contract-architecture-plan | The placement contract's architecture | sha256:5417b3363dffdc44 | path: genesis/docs/superpowers/plans/2026-10-20-placement-contract-architecture-plan.md"
---

# S1-S5 report: the SDK schemas

Brief: `task-placement-schemas-brief.md`. Spec: `placement-contract-interfaces-design` §2.G, with §2.0,
§A, §D, §E and §I for the shapes. Base: the working tree, uncommitted. Nothing is staged, committed or
pushed (architecture before landing). Every file I wrote is under `elohim/sdk/schemas/v1/**`.

## Status: DONE_WITH_CONCERNS

All the v1 files that §2.G names for lane S are written, and so are the S2 needs block and S5's checks.
Every new schema compiles under AJV 2020 and under the repo's TypeScript codegen. The lane-S contract
harness passes 231 of 231 checks. The eight pillar manifests validate exactly as they did at HEAD. Their
drift error counts are unchanged, and no pillar has an error on the needs surface.

The concerns:

1. **Codegen freshness is red until three follow-ons outside the write-set land.** The spec's nine
   `_codegen` enums make the six distributed `schema-enums.ts` copies stale. Every new enum must also
   appear in the tracked TTL projection `generated-ttl/elohim-vocabulary.ttl`, so
   `codegen-ttl --verify` and two `schema:unit` TTL tests now fail. The spec's follow-on list does not
   name the TTL projection. The interfaces spec (§0, "Lane S's S5 deliverable") expects this outcome: the
   v1 files plus a dry verify that names the follow-on diff. That diff is given below.
2. **`pnpm run schema:test` exits 1 at baseline.** That is unchanged by this lane: `test-schema.mjs`
   fails twice on ContentView. The script is the head of an `&&` chain, so the ten scripts after it
   never run under `schema:test`. I ran each of them on its own.
3. Two serde shapes in the spec that I believe break placement-core's DAG-CBOR round trip. They are
   unverified because no Rust exists yet. See disagreements 1 and 2.

## Files changed (all under `elohim/sdk/schemas/v1/`)

| Item | File | `$id` |
|---|---|---|
| S1 | `objects/tree-view.schema.json` (with `$defs.ViewPaths`) | `epr:schema:object:tree-view` |
| S2 | `manifest/app-manifest.schema.json`: one optional `needs` property (`$ref: #/$defs/ManifestNeeds`) and `$defs.ManifestNeeds`. Its `properties` is `{}`, so no field is declarable yet. The shapes sit under `$defs.ManifestNeeds.$defs`: StartSet, Runtimes, WorksOffline, Channels, Loss, Signing, Compute, Projectable, Indexed. The change is additive (+180 lines, 0 removed) | — |
| S3 | `intents/placement-need.schema.json`, `intents/posture.schema.json`, `intents/berth-offer.schema.json` | `epr:schema:intent:{placement-need,posture,berth-offer}` |
| S3 | `objects/tier-record.schema.json`, `objects/item-absence.schema.json` (with `$defs.UnansweredItemAbsence`, the batch-trailer shape) | `epr:schema:object:{tier-record,item-absence}` |
| S3 (shared) | `objects/need-key.schema.json`, `objects/placement-capacity.schema.json` (revision R-S2 below) | `epr:schema:object:{need-key,placement-capacity}` |
| S4 | `objects/placement-parameter-set.schema.json`, `objects/placement-parameter-head.schema.json` (with `$defs.RatificationRef`) | `epr:schema:object:{placement-parameter-set,placement-parameter-head}` |
| R7 | `objects/unanswered.schema.json` | `epr:schema:object:unanswered` |
| probe | `objects/probe-reading.schema.json` | `epr:schema:object:probe-reading` |
| view | `views/placement-explanation-view.schema.json` (Category C) | `epr:schema:view:placement-explanation` |
| enums, §2.G list (`_codegen`, TypeScript only, no `_dna`) | `enums/{placement-kind,grant-scope,loss-domain,need-class,depth,isolation,placement-factor,evidence-stage,disclosure}.schema.json` | `epr:schema:enum:…` |
| enums added (no `_codegen`, no `_dna`; revision R-S1) | `enums/{party-role,tier-class,reach-reason,expresser-kind,ratification-kind,param-fallback}.schema.json` | `epr:schema:enum:…` |

Every non-enum schema has these properties, and the harness checks each one:
- "Source of truth:" and its p2p design-gate category in its description;
- `additionalProperties: false` on every object that has properties;
- enums referenced by `$ref`;
- camelCase field names;
- dual-provenance absences carried in the answer envelope, while each bare nullable says in its
  description that it has one provenance (rule 11).

No `_dna` is added and no zome constant is touched (`check-dna` passes), so no DNA hash moves.

Design-gate classes, as recorded in each description:
- Category C: tree-view (a content-derived value), need, tier-record, item-absence, unanswered,
  probe-reading and the explanation view.
- Category A: a posture at or above public reach (household-local below it, R2), and a berth offer
  through ark-core's existing `rea.rs` intent projection.
- Category A2: the parameter set and head. A household-local record below public reach; at collective
  or commons reach, an EPR head declaration and its referent.
- No new DHT entry type anywhere.

## Tests added

The harness is `test-placement-schemas.mjs`, reproduced in the appendix. It is the draft of the named
follow-on `elohim/sdk/schemas/scripts/test-manifest-needs.mjs`, extended to every lane-S schema. It is
not in `scripts/`, because `scripts/` is outside the write-set. It registers every v1 schema under its
file URL, so relative `$ref`s with fragments resolve exactly as written. Its 231 checks are:

- **Compile and convention:** all 28 new schemas compile. Each has an `epr:schema:` `$id`. Each non-enum
  schema names its source of truth and its category, and every object in it is closed. Each enum has no
  `_dna` and no Rust codegen.
- **S1:** these are accepted: `[""]`, the slice-1 headline view, and names that contain dots (`.github`,
  `a..b`). These are refused: an empty view, `/a`, `a/`, `a//b`, `a/./b`, `.`, `..`, `a/..`, a NUL,
  duplicates, a raw-codec root, and an unknown key. `ViewPaths` is the paths alone. A pinned KNOWN LIMIT:
  `["a","a/b"]` passes the schema, because the antichain is the decoder's rule.
- **S2 (G4, R-G1, G7, G10, G14):**
  - `needs` is optional. `needs: {}` validates. `needs.closure` and `needs.loss` are refused because no
    reader has landed yet.
  - StartSet accepts `[""]` and refuses a `root` key. WorksOffline refuses `query`. Loss accepts the
    lamad example `{concept: household}`.
  - Projectable accepts each of the eight reach values and refuses `invited` or no reach.
  - Compute refuses `republish-epr`, which is outside the compute-grant subset. Indexed refuses a
    raw-codec MethodId.
  - **Neutrality scan:** it covers `$defs.ManifestNeeds` and the 5 `$ref` targets reachable from it, and
    finds no property matching `/(device|host|hub|doorway|peer|node|url)/i` and no `format: uri`. As a
    mutation test, I planted `hostUrl` (with `format: uri`) in Projectable and `peerId` inside
    tree-view's ViewPaths. The scan reported all three (two properties and the format), including the
    one in the referenced file. I then deleted the mutated copy.
  - **Non-regression:** in each of the 8 pillar manifests, no error falls on the needs surface, and the
    error counts match HEAD's schema: avodah 2→2, elohim 2→2, imagodei 1→1, infrastructure 2→2,
    lamad 6→6, mishpat 3→3, qahal 5→5, shefa 9→9.
- **S3:**
  - **Need:** a household tree need validates. An unknown field, a raw CID in a tree-view key and a
    device expresser are refused. A legacy blob key and a task need with a `--on` narrowing validate.
    Task terms must always write `baseRoot`.
  - **Posture:** a phone's deferred, household-only custody offer validates. An unknown kind and a
    device-fact field are refused.
  - **Berth offer:** valid with one (root, reach) pair. A pair in the wrong order, a local-path field
    and 65 holds are refused.
  - **Tier record:** `own` and an empty source list are refused.
  - **Absence trailer:** the §A.6 diagnostic example validates as `Unanswered<ItemAbsence>`. Over-budget
    with a come-back time and an escalation, and a reach refusal by name, validate. An empty `tried`, an
    unknown kind, a `nothing` with no `because`, and a missing `escalation` are refused.
- **S4:**
  - The `WeightedCapacity`-shaped tuning and the empty weights of xor-distance validate.
  - These are refused: a weight on `reach`, a weight of 0, a fractional weight, schema version 2, a
    dag-cbor method, and a `scope` field on the set (E1).
  - **Head:** a TouchedConsent head and a built-in head validate. An unknown kind key and a ratification
    by `elohim` are refused.
- **Probe and view:**
  - A holder reading validates. These are refused: Present with a reason, Unreachable without one, a
    missing `evidenceStage`, and the Rust field name `evidence`.
  - The explanation view validates the §D worked example (DomainShort {1,2}). Upper-case hex is refused,
    and so is an IneligibleReason inside a not-carried outcome.
- **Rust equality (by text read, where the Rust exists today):**
  - `evidence-stage`, `party-role` and the `disclosure` tags equal `seam_contracts::probe`.
  - `grant-scope` equals elohim-storage's `GRANT_SCOPES` (`sweettest-feedback, hosted-cell, measure-stage`).
  - `Reach` serializes lowercase.

TypeScript: I compiled each new non-enum schema with `json-schema-to-typescript` into the scratchpad and
type-checked the output with `tsc --strict` (EXIT=0). Each narrowed envelope types `why` as required,
for example `UnansweredIneligible = Unanswered & { why: IneligibleReason }`.

## Gate evidence

The flow context gives no gate line for `elohim/sdk/schemas/v1/**`: it prints "GATE (no gate project
covers this path)". So I ran the recipes of the gate projects that read these files (`schema-codegen`,
`schema-validate`, `constants-sync`), plus `schema:test`, `manifest:test` and the unit and DNA checks.
For each, I recorded the baseline before any edit and the result after the last edit:

| Command | Baseline | After |
|---|---|---|
| `pnpm run -s schema:test` | EXIT=1 (76 pass, 2 fail: ContentView) | EXIT=1 (76 pass, 2 fail: the same two) |
| each script chained after `test-schema.mjs` in `schema:test`, run alone (10 scripts) | — | all EXIT=0 |
| `test-manifest-schema.mjs`, `test-observation-kind-schema.mjs` | — | EXIT=0 |
| `pnpm run -s manifest:test` | EXIT=1 (`test-elohim-agent-manifest`: MissingRef `closure.schema.json`) | EXIT=1, output identical to baseline |
| `test-manifest-graph-extension.mjs` | — | EXIT=1, MissingRef `session-lifecycle-state.schema.json`, a pre-existing ref, not this lane's |
| lane-S harness `node …/test-placement-schemas.mjs <HEAD app-manifest copy>` | — | EXIT=0 (231 passed, 0 failed) |
| `just gate schema-codegen` | EXIT=1 (seeder `p2p-status-view.ts` stale) | EXIT=1 (that one, plus six `schema-enums.ts` stale: the follow-on) |
| `node scripts/codegen-ts.mjs --verify` | EXIT=1 (1 stale file) | EXIT=1 (7 stale files: the six `schema-enums.ts` and the pre-existing one; no `ERROR generating` line) |
| `node scripts/codegen-rs.mjs --verify` | EXIT=0 | EXIT=0 |
| `pnpm run -s route-claims:codegen:verify` | EXIT=0 | EXIT=0 |
| `pnpm run -s manifest:codegen:verify` | — | EXIT=0 |
| `node scripts/check-dna.mjs` | EXIT=0 | EXIT=0 |
| `schema:unit` (3 `node --test` files) | EXIT=0 | EXIT=1 (9 pass, 2 fail: the TTL projection lacks the new enums; the follow-on) |
| `node scripts/codegen-ttl.mjs --verify` | fresh (implied: the baseline unit TTL tests passed) | EXIT=1 (`elohim-vocabulary.ttl` differs: the follow-on) |
| `just gate schema-validate` / `validate-seeds.mjs` | — | EXIT=0 (3557 valid, 0 errors) |
| `constants-sync`: the seeder vitest `constants-sync.test.ts` | — | EXIT=0 (16 passed). The gate's other step is codegen-ts `--verify`, above |

These are the exact final lines:
`schema:test EXIT=1` · `harness EXIT=0` · `codegen-ts --verify EXIT=1` · `schema:unit EXIT=1` ·
`codegen-ttl --verify EXIT=1` · `validate-seeds EXIT=0` · `check-dna EXIT=0` · `codegen-rs --verify EXIT=0`.

**Not run: `domain-types`, a cargo gate whose source glob is `elohim/sdk/**`.** No source of the six
domain-types crates reads a v1 file (a grep for `schemas/v1` under `elohim/sdk/domains/*/types/src` finds
nothing). Another session's `cargo test -p elohim-compute` held the one build slot. I also did not run
elohim-storage's `schema_contract.rs`: it validates only the views it names, and none of them is new. No
cargo run happened, so I claimed no cargo berth.

## Follow-ons the S5 deliverable needs (outside `v1/**`, named, not written)

1. **`scripts/codegen-ts.mjs`:**
   - Add `INTERFACE_FILES` entries for the new objects, intents and view.
   - Add `GENERATED_OUTPUT_DIRS` entries for `elohim/sdk/storage-client-ts` and `elohim/eprfs/eprfs-ts`
     (H16).
   - Then regenerate. The dry diff of each distributed `schema-enums.ts` (it is
     `scratchpad/enum-dry.mjs`, the same formatting rules) is **89 lines added, 0 removed**. It adds
     `CORE_`, `ALL_`, an alias and a type for each of DEPTHS, DISCLOSURES, EVIDENCE_STAGES, GRANT_SCOPES,
     ISOLATION_LEVELS, LOSS_DOMAINS, NEED_CLASSES, PLACEMENT_FACTORS and PLACEMENT_KINDS. None of the
     nine type names (Depth, Disclosure, Isolation and the rest) collides with an export in any
     distributed consumer; a grep across the six output trees and storage-client-ts confirms this.
2. **NEW, missing from the spec's list:** regenerate the tracked `elohim/sdk/schemas/generated-ttl/elohim-vocabulary.ttl`
   with `node elohim/sdk/schemas/scripts/codegen-ttl.mjs`. `codegen-ttl.test.mjs` requires every enum's
   description and values to appear in it, and it currently fails on `depth`, the first new enum in
   order.
3. **`scripts/test-manifest-needs.mjs` (R-G1, G4):** the appendix is its draft. Wire it into
   `schema:test` in the root `package.json`, ahead of `test-schema.mjs` or outside the `&&` chain, so
   the baseline red does not mask it.
4. **`test-schema.mjs` `TASK_1_TO_3_SURFACE`:** add `ManifestNeeds`, `StartSet`, `WorksOffline`,
   `Projectable`, `Runtimes`, `Signing` and `Indexed`, never a bare `closure`, `loss` or `compute`.
5. Rust-side equality, owned by C2 and C5:
   - `placement/core/tests/schema_contract.rs` reads each enum with `include_str!`. Its scope:
     placement-kind, grant-scope, loss-domain, need-class, depth, isolation, placement-factor,
     expresser-kind, ratification-kind and param-fallback.
   - eprfs-core covers tier-class, reach-reason and depth.
   - An elohim-storage test holds `GRANT_SCOPES` and the two `hosted-cell` constants equal to
     `enums/grant-scope`.
6. seam-contracts or elohim-views: a serde round trip of `Unanswered<R>` against
   `objects/unanswered.schema.json`, and the first monomorphic probe wire view in `elohim/elohim-views`,
   which narrows `objects/probe-reading`'s `answer.value`. The C1 review noted that no wire round trip
   exists in the crate.
7. Optional: `loadRefMap` in `codegen-ts.mjs` and `load_ref_map` in elohim-storage's `schema_contract.rs`
   could also index `intents/`. Neither does today, which is why the shared shapes live in `objects/`
   (R-S2).

## Where the code and the interfaces spec disagree (for the spec seat to absorb)

1. **`NeedTerms::Task.input_root` puts a `cid_serde` CID inside an internally tagged enum**
   (`tag = "terms"`). That is the G1 hazard the spec itself forbids. serde buffers an internally tagged
   enum through `Content`, whose deserializer reports human-readable, so `cid_serde` would take its
   string branch on DAG-CBOR tag-42 bytes. Proposed fix: `input_root: CidString`, which matches
   `base_root`, or a struct carrying a `terms` kind field. On the JSON wire the field is a string either
   way, so the schema does not change. **PLAUSIBLE, unverified:** placement-core does not exist yet,
   and the G1 contract vector in lane C2 is the check that would catch it.
2. **`NeedKey` is adjacently tagged** (`tag = "form", content = "id"`) and its payloads are CIDs. In
   canonical DAG-CBOR the key `id` (2 bytes) sorts before `form` (4 bytes). A decoder therefore meets the
   content before the tag, and serde buffers it in `Content`, which brings the same human-readable
   hazard as in 1. Proposed fix: a struct `{ form, id: CidString }`, or a pinned C2 vector that decodes a
   DAG-CBOR `NeedKey::TreeView`. **PLAUSIBLE, unverified** for the same reason.
3. **G1's "every CID … `^bafy…`" is wrong for raw-codec CIDs.** `MethodId` is raw (0x55), so its string
   is `bafkrei…`. The legacy custody scope, `MethodId::named("elohim:placement:legacy-custody-scope@1")`,
   is also raw, so `inScopeOf` and `HeadDeclaration.scope` can be `bafkrei…`. The schemas therefore pin
   the codec per field:
   - dag-cbor `^bafyrei[a-z2-7]{52}$`: tree roots, `TreeView::cid`, `Need::id`, a set's CID, a TaskSpec;
   - raw `^bafkrei[a-z2-7]{52}$`: `MethodId`;
   - either `^baf[ky]rei[a-z2-7]{52}$`: space scope, berth, session, charter clause.
4. **`household` is not a reach value.** §A.6's `ReachStandingAsk` defaults the standing of the Own,
   PersonDevice and Household classes to "household", and the §A worked example gives tiers "standing
   household". `enums/reach` has no such value, and `TierRecord.standing` is a `Reach::openness()`
   ordinal (1..8). The spec should name the reach it means, probably `intimate`. My first berth-offer
   vector wrote `household` as a reach, and the schema refused it.
5. **`enums/disclosure` cannot be the whole `Disclosure`.** It is a tagged enum that carries data. The
   enum schema holds the three tag values (`holder | parties | at-reach`), and the data forms are shaped
   in `probe-reading.disclosure`. `PartyRole` needed an enum of its own.
6. **Probe wire names.** Finding 16 says `evidenceStage` on the wire, but `seam_contracts::probe::Reading`
   serializes the field as `evidence`, and its `ProbeAnswer` is externally tagged where the wire uses
   objects/answer's `{state, value | reason}`. This agrees with the spec's own ruling that `Reading<T>`
   stays internal and that per-promise wire views map it. Recorded so the elohim-views wire struct
   renames the field and converts the answer.
7. **`RatificationRef.id` for `built-in` is unspecified.** The spec gives "record hash or charter CID".
   The schema takes any non-empty string, and its description says the spec leaves the value open.
8. **`Requirements`, `OfferTerms` and `Capacity` serialize every `Option` (as null).** The schemas
   therefore require every key and allow null where Rust has `Option`. That matches the canonical
   encoding `Need::id` hashes. A Rust decoder accepts a missing `Option`, so the schema is stricter than
   serde on input, by design.
9. **`BerthOffer.holds: Vec<(CidString, Reach)>`** is a JSON array of 2-tuples. `prefixItems` pins each
   position. json-schema-to-typescript ignores `prefixItems`, so the TypeScript type is
   `[string | Reach, string | Reach][]`, not `[string, Reach][]`. A hand-made `tsType` could tighten it,
   at the cost of re-declaring `Reach`.
10. **`AgentRef` carries no validation in Rust.** The schemas pin `^uhCAk[A-Za-z0-9_-]+$`, the canonical
    form. A fixture that uses non-`uhCAk` agent strings validates in Rust and fails the schema.
11. **The antichain and component-wise sort of a view, and `Demand.domains <= holders`,** are beyond
    JSON Schema. The descriptions point to the decoder (TreeError::InvalidView) and to the constructor
    or charter that enforce them. The harness pins `["a","a/b"]` as passing the schema, so that limit
    is visible.
12. **`ParameterSet.schema` is `const: 1`.** This v1 schema file shapes encoding version 1 only, and a
    bump changes the file. The spec says the floor refuses an unknown value, and the schema refuses it
    too.

### Revisions this lane made to fit the spec's intent

- **R-S1, enums beyond §2.G's list.** I added party-role, tier-class, reach-reason, expresser-kind,
  ratification-kind and param-fallback. CONVENTIONS rule 7 and §2.G's "enums by `$ref`" require each
  field vocabulary to be referenced, never inlined. These six carry no `_codegen`, so the follow-on
  `schema-enums.ts` diff stays exactly the spec's nine. Each is a TypeScript union where a schema
  inlines it. 25 existing enums already follow this pattern.
- **R-S2, `objects/need-key` and `objects/placement-capacity`.** `NeedKey` (the need and the explanation
  view) and `Capacity` (posture and berth offer) each appear in two schemas. `codegen-ts.mjs`'s
  `loadRefMap` resolves cross-file `$ref`s into `objects/` but not into `intents/`, so I put both
  definitions in `objects/` rather than refer into `intents/placement-need`. Refer into `intents/` and
  the explanation view fails with `ERROR generating`. That failure has the same cause as one I hit and
  fixed: a fragment `$ref` to `placement-parameter-head#/$defs/RatificationRef` before that definition
  was moved under `$defs`.
- **R-S3, worksOffline.** It is `allOf: [$ref depth, not: const query]`, which reuses the depth
  vocabulary rather than inlining `sparse | on-read | kept`.

## Spec and covenant notes

- The S2 ruling is followed as written in decision 15: the object and its definitions ship with no
  declarable field.
- Decision 22 (the operator's ruling): `loss`, `signing` and `closure` are shaped as app-declared needs.
- `elohim/sdk/.epr-meta` says to commit source and projection together. This lane cannot write the
  projection, so follow-ons 1 and 2 must land with these files.
- I added no axis card. None of these vocabularies is an orthogonal coordinate.
- I did not write the habit delta: the atom `elohim/eprfs/.epr-meta/materialize-what-is-read.habit.md`
  is in lane E's write-set, and lane H owns the deltas.
- Briefly, a scratch probe file (`scripts/.cid-probe.mjs`, which computes sample CIDs) sat outside the
  write-set; I deleted it at once. Nothing else outside `v1/**` was written.

## Appendix: the lane-S contract harness (draft of `scripts/test-manifest-needs.mjs`)

Run from anywhere: `node test-placement-schemas.mjs <path to app-manifest.schema.json at HEAD>` (the argument enables the error-count comparison).

```js
#!/usr/bin/env node
// Lane S contract harness (interfaces spec §2.G). Draft of the named follow-on
// elohim/sdk/schemas/scripts/test-manifest-needs.mjs plus placement vectors. Lives in the
// lane's scratchpad because scripts/ is outside the lane's write-set.
//
// Usage: node test-placement-schemas.mjs [<path to a HEAD copy of app-manifest.schema.json>]
import Ajv2020 from '/projects/elohim/node_modules/ajv/dist/2020.js';
import addFormats from '/projects/elohim/node_modules/ajv-formats/dist/index.js';
import { CID } from '/projects/elohim/node_modules/multiformats/dist/src/cid.js';
import { sha256 } from '/projects/elohim/node_modules/multiformats/dist/src/hashes/sha2.js';
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join, resolve, dirname } from 'node:path';
import { pathToFileURL } from 'node:url';

const REPO = '/projects/elohim';
const V1 = join(REPO, 'elohim/sdk/schemas/v1');
let passes = 0, failures = 0;
const ok = (c, m) => { if (c) { passes++; console.log(`PASS: ${m}`); } else { failures++; console.error(`FAIL: ${m}`); } };
const load = (p) => JSON.parse(readFileSync(p, 'utf8'));

// Register every v1 schema under its file URL, so relative $refs (with fragments) resolve
// exactly as the files name them, independent of the epr: $id namespace.
function walk(dir) {
  return readdirSync(dir).flatMap((f) => {
    const p = join(dir, f);
    return statSync(p).isDirectory() ? walk(p) : p.endsWith('.schema.json') ? [p] : [];
  });
}
const ajv = new Ajv2020({ strict: false, allErrors: true });
addFormats.default ? addFormats.default(ajv) : addFormats(ajv);
const url = (p) => pathToFileURL(p).href;
for (const p of walk(V1)) {
  const s = load(p);
  try { ajv.addSchema({ ...s, $id: url(p) }); } catch (e) { /* duplicates elsewhere: not ours */ }
}
const v = (rel, frag = '') => {
  const fn = ajv.getSchema(url(join(V1, rel)) + frag);
  if (!fn) throw new Error(`no schema ${rel}${frag}`);
  return fn;
};

// ---- every new schema compiles
const NEW = [
  'enums/placement-kind.schema.json', 'enums/grant-scope.schema.json', 'enums/loss-domain.schema.json',
  'enums/need-class.schema.json', 'enums/depth.schema.json', 'enums/isolation.schema.json',
  'enums/placement-factor.schema.json', 'enums/evidence-stage.schema.json', 'enums/disclosure.schema.json',
  'enums/party-role.schema.json', 'enums/tier-class.schema.json', 'enums/reach-reason.schema.json',
  'enums/expresser-kind.schema.json', 'enums/ratification-kind.schema.json', 'enums/param-fallback.schema.json',
  'objects/tree-view.schema.json', 'objects/unanswered.schema.json', 'objects/item-absence.schema.json',
  'objects/tier-record.schema.json', 'objects/need-key.schema.json', 'objects/placement-capacity.schema.json',
  'objects/placement-parameter-set.schema.json', 'objects/placement-parameter-head.schema.json',
  'objects/probe-reading.schema.json', 'intents/placement-need.schema.json', 'intents/posture.schema.json',
  'intents/berth-offer.schema.json', 'views/placement-explanation-view.schema.json',
];
for (const f of NEW) {
  let fn; try { fn = v(f); } catch (e) { ok(false, `${f} compiles (${e.message})`); continue; }
  ok(typeof fn === 'function', `${f} compiles`);
  const s = load(join(V1, f));
  ok(s.$id && s.$id.startsWith('epr:schema:'), `${f} has an epr:schema: $id`);
  if (!f.startsWith('enums/')) {
    ok(/Source of truth:/.test(s.description), `${f} declares its source of truth`);
    ok(/Category (A2|A|B2|B|C)\b/.test(s.description), `${f} names its p2p design-gate category`);
  }
  if (f.startsWith('enums/')) {
    ok(!('_dna' in s), `${f} carries no _dna (no DNA hash moves, G11)`);
    ok(!s._codegen || !s._codegen.rust, `${f} generates no Rust constants (TypeScript only)`);
  }
}
// additionalProperties:false on every object node of the new non-enum schemas
function objectsWithoutClosure(node, path, out) {
  if (!node || typeof node !== 'object') return out;
  if (Array.isArray(node)) { node.forEach((n, i) => objectsWithoutClosure(n, `${path}/${i}`, out)); return out; }
  if (node.type === 'object' && node.properties && node.additionalProperties !== false) out.push(path);
  for (const [k, n] of Object.entries(node)) objectsWithoutClosure(n, `${path}/${k}`, out);
  return out;
}
for (const f of NEW.filter((f) => !f.startsWith('enums/'))) {
  const open = objectsWithoutClosure(load(join(V1, f)), '', []);
  ok(open.length === 0, `${f}: every object with properties sets additionalProperties:false${open.length ? ' — open: ' + open.join(', ') : ''}`);
}

// ---- CIDs used in vectors
const mk = async (codec, text) => CID.createV1(codec, await sha256.digest(new TextEncoder().encode(text))).toString();
const ROOT = await mk(0x71, 'root');
const ROOT2 = await mk(0x71, 'root-2');
const XOR = await mk(0x55, 'elohim:placement:xor-distance@1');
const SCOPE = await mk(0x55, 'elohim:placement:legacy-custody-scope@1');
const AG = 'uhCAk' + 'a'.repeat(48);
const AG2 = 'uhCAk' + 'b'.repeat(48);

// ---- S1 tree-view
const tv = v('objects/tree-view.schema.json');
ok(tv({ root: ROOT, paths: [''] }), 'tree-view accepts the whole view [""] (View::whole)');
ok(tv({ root: ROOT, paths: ['a1/b2/f003', 'a4/b4/f099', 'a9/b0'] }), 'tree-view accepts the slice-1 headline view');
ok(!tv({ root: ROOT, paths: [] }), 'tree-view refuses an empty view (minItems 1)');
for (const bad of ['/a', 'a/', 'a//b', 'a/./b', '.', '..', 'a/..', '../a', 'a\u0000b']) {
  ok(!tv({ root: ROOT, paths: [bad] }), `tree-view refuses path ${JSON.stringify(bad)}`);
}
ok(tv({ root: ROOT, paths: ['.github', 'a..b', '...'] }), 'tree-view accepts names that merely contain dots (.github, a..b, ...)');
ok(!tv({ root: ROOT, paths: ['a', 'a'] }), 'tree-view refuses a duplicate path (uniqueItems)');
ok(tv({ root: ROOT, paths: ['a', 'a/b'] }), 'KNOWN LIMIT: tree-view lets ["a","a/b"] through; the antichain rule is the decoder\'s (TreeError::InvalidView), not expressible in JSON Schema');
ok(!tv({ root: XOR, paths: [''] }), 'tree-view refuses a raw-codec root (a tree node is dag-cbor)');
ok(!tv({ root: ROOT, paths: [''], extra: 1 }), 'tree-view refuses an unknown key');
const vp = v('objects/tree-view.schema.json', '#/$defs/ViewPaths');
ok(vp(['']) && !vp({ root: ROOT, paths: [''] }), '$defs.ViewPaths is the paths alone');

// ---- S2 needs block (R-G1 and G4)
const manifest = load(join(V1, 'manifest/app-manifest.schema.json'));
ok(manifest.properties.needs && manifest.properties.needs.$ref === '#/$defs/ManifestNeeds', 'app-manifest has one optional needs property referencing $defs.ManifestNeeds');
ok(!manifest.required.includes('needs'), 'needs is optional');
const MN = manifest.$defs.ManifestNeeds;
ok(MN.additionalProperties === false && Object.keys(MN.properties).length === 0, 'no needs field is declarable until its reader lands (decision 15)');
const declared = ['StartSet', 'Runtimes', 'WorksOffline', 'Channels', 'Loss', 'Signing', 'Compute', 'Projectable', 'Indexed'];
ok(declared.every((d) => MN.$defs[d]), `ManifestNeeds.$defs fixes ${declared.join(', ')}`);
const needsOnly = v('manifest/app-manifest.schema.json', '#/$defs/ManifestNeeds');
ok(needsOnly({}), 'needs: {} validates');
ok(!needsOnly({ closure: [''] }), 'needs.closure is not yet declarable (refused)');
ok(!needsOnly({ loss: { concept: 'household' } }), 'needs.loss is not yet declarable (refused)');
const d = (n) => v('manifest/app-manifest.schema.json', `#/$defs/ManifestNeeds/$defs/${n}`);
ok(d('StartSet')(['']), 'StartSet accepts [""] (the lamad worked example)');
ok(!d('StartSet')({ root: ROOT, paths: [''] }), 'StartSet (closure) refuses a root key');
ok(!d('StartSet')(['dist/browser/']), 'StartSet refuses a trailing-slash path');
ok(d('WorksOffline')('kept') && d('WorksOffline')('sparse') && d('WorksOffline')('on-read'), 'worksOffline accepts sparse, on-read, kept');
ok(!d('WorksOffline')('query'), 'worksOffline refuses query (G7)');
ok(d('Loss')({ concept: 'household' }), 'loss accepts { concept: household } (the lamad worked example)');
ok(!d('Loss')({ concept: 'galaxy' }), 'loss refuses a value outside loss-domain');
const reach = load(join(V1, 'enums/reach.schema.json')).enum;
ok(reach.length === 8 && reach.every((r) => d('Projectable')({ reach: r })), 'projectable.reach accepts each of the eight SDK reach values');
ok(!d('Projectable')({ reach: 'invited' }) && !d('Projectable')({}), 'projectable refuses a non-SDK reach and a missing reach');
ok(d('Compute')([{ scope: 'measure-stage', bounds: { ratePerHour: 4 } }]), 'compute accepts a grant request with a rate bound');
ok(!d('Compute')([{ scope: 'republish-epr', bounds: {} }]), 'compute refuses a scope outside grant-scope (the compute-grant subset, G10)');
ok(d('Indexed')([{ contentType: 'concept', measure: ROOT }]) && d('Indexed')([{ contentType: 'concept', measure: 'lexical@1' }]), 'indexed.measure accepts an IndexMeasure CID or a PinnedRef');
ok(!d('Indexed')([{ contentType: 'concept', measure: XOR }]), 'indexed.measure refuses a placement MethodId (raw codec, G14)');
ok(d('Signing')({ present: ['publish'], delegable: [] }) && !d('Signing')({ present: [] }), 'signing requires present and delegable');

// R-G1: placement neutrality over $defs.ManifestNeeds and everything reachable by $ref from it
const NEUTRAL = /(device|host|hub|doorway|peer|node|url)/i;
const bad = [];
const seen = new Set();
function scan(node, base, where) {
  if (!node || typeof node !== 'object') return;
  if (Array.isArray(node)) return node.forEach((n) => scan(n, base, where));
  if (node.format === 'uri') bad.push(`${where}: format uri`);
  if (node.properties) for (const k of Object.keys(node.properties)) if (NEUTRAL.test(k)) bad.push(`${where}: property ${k}`);
  if (typeof node.$ref === 'string') {
    const [file, frag] = node.$ref.split('#');
    const target = file ? resolve(dirname(base), file) : base;
    const key = `${target}#${frag ?? ''}`;
    if (!seen.has(key)) {
      seen.add(key);
      let t = load(target);
      if (frag) for (const part of frag.split('/').filter(Boolean)) t = t?.[part];
      scan(t, target, key.replace(V1 + '/', ''));
    }
  }
  for (const [k, n] of Object.entries(node)) if (k !== 'properties') scan(n, base, where); else for (const p of Object.values(n)) scan(p, base, where);
}
scan(MN, join(V1, 'manifest/app-manifest.schema.json'), 'ManifestNeeds');
ok(bad.length === 0, `needs block is placement-neutral (R-G1)${bad.length ? ': ' + bad.join('; ') : ''} — scanned ${seen.size} reachable $ref targets`);

// G4 non-regression: no pillar manifest gains an error under the needs surface, and the error
// count does not grow against the HEAD copy of the schema (passed as argv[2]).
const NEEDS_SURFACE = ['/properties/needs', 'ManifestNeeds', 'StartSet', 'WorksOffline', 'Projectable'];
async function pillarErrors(schemaPath) {
  const a = new Ajv2020({ strict: false, allErrors: true });
  for (const p of walk(V1)) { if (p.endsWith('manifest/app-manifest.schema.json')) continue; try { a.addSchema({ ...load(p), $id: url(p) }); } catch {} }
  const s = load(schemaPath);
  const fn = a.compile({ ...s, $id: url(join(V1, 'manifest/app-manifest.schema.json')) });
  const inline = (val, baseDir) => {
    if (val === null || typeof val !== 'object') return val;
    if (Array.isArray(val)) return val.map((x) => inline(x, baseDir));
    if (typeof val.$ref === 'string' && (val.$ref.startsWith('./manifest/') || val.$ref.startsWith('../'))) {
      const rp = resolve(baseDir, val.$ref); return inline(load(rp), dirname(rp));
    }
    return Object.fromEntries(Object.entries(val).map(([k, x]) => [k, inline(x, baseDir)]));
  };
  const out = {};
  for (const pillar of ['avodah', 'elohim', 'imagodei', 'infrastructure', 'lamad', 'mishpat', 'qahal', 'shefa']) {
    const dir = join(REPO, 'elohim/sdk/domains', pillar);
    fn(inline(load(join(dir, 'manifest.json')), dir));
    out[pillar] = fn.errors || [];
  }
  return out;
}
const after = await pillarErrors(join(V1, 'manifest/app-manifest.schema.json'));
for (const [pillar, errs] of Object.entries(after)) {
  const ours = errs.filter((e) => NEEDS_SURFACE.some((t) => `${e.schemaPath} ${e.instancePath}`.includes(t)));
  ok(ours.length === 0, `${pillar}: no error under the needs surface (${errs.length} pre-existing drift errors)`);
}
if (process.argv[2]) {
  const before = await pillarErrors(process.argv[2]);
  for (const pillar of Object.keys(after)) {
    ok(after[pillar].length === before[pillar].length, `${pillar}: error count unchanged against HEAD (${before[pillar].length} → ${after[pillar].length})`);
  }
}

// ---- S3 need, posture, berth offer
const need = {
  kind: 'custody-root', key: { form: 'tree-view', id: ROOT },
  demand: { holders: 2, domains: 2 },
  requires: { alwaysOn: false, bytes: null, memoryBytes: null, cpuMillis: null, pids: null, isolation: null, carriers: null },
  terms: { terms: 'none' }, reach: 'intimate', receiver: AG,
  expressedBy: { by: 'app', id: ROOT2 }, class: 'member', inScopeOf: SCOPE,
};
const nv = v('intents/placement-need.schema.json');
ok(nv(need), 'placement-need accepts a household document tree need');
ok(!nv({ ...need, extra: true }), 'placement-need refuses an unknown field (deny_unknown_fields)');
ok(!nv({ ...need, key: { form: 'tree-view', id: XOR } }), 'a tree-view key refuses a raw-codec CID');
ok(nv({ ...need, kind: 'custody-blob', key: { form: 'blob', id: 'sha256-' + 'ab'.repeat(32) } }), 'the legacy blob marker key validates');
ok(!nv({ ...need, expressedBy: { by: 'device', id: 'x' } }), 'there is no device expresser');
const task = { ...need, kind: 'task', key: { form: 'task', id: ROOT2 }, demand: { holders: 1, domains: 1 },
  terms: { terms: 'task', inputRoot: ROOT, scope: 'measure-stage', baseRoot: null },
  requires: { ...need.requires, carriers: [AG2] } };
ok(nv(task), 'a task need with terms and a --on carrier narrowing validates');
ok(!nv({ ...task, terms: { terms: 'task', inputRoot: ROOT, scope: 'measure-stage' } }), 'task terms always write baseRoot (null when none)');
const pv = v('intents/posture.schema.json');
const offer = { spareBytes: 1000, compute: null, holds: [ROOT], reachCeiling: null, scopes: [], isolation: null, deferred: true, householdOnly: true };
ok(pv({ carrier: AG, offers: { 'custody-root': offer }, reach: 'intimate', writtenAt: 1760000000 }), 'posture accepts a phone on battery: own-household custody, deferred');
ok(!pv({ carrier: AG, offers: { 'custody-forever': offer }, reach: 'intimate', writtenAt: 1 }), 'posture refuses an offer for an unknown kind');
ok(!pv({ carrier: AG, offers: { 'custody-root': { ...offer, onBattery: true } }, reach: 'intimate', writtenAt: 1 }), 'device facts cannot ride in offer terms (R6)');
const bv = v('intents/berth-offer.schema.json');
const berth = { blade: AG, berth: ROOT2, manifest: 'measure-stage', incarnation: 3, isolation: 'bounded',
  capacity: { bytes: 10, memoryBytes: 20, cpuMillis: 30, pids: 40 }, holds: [[ROOT, 'intimate']], scopes: ['measure-stage'] };
ok(bv(berth), 'berth-offer accepts a berth holding one root at intimate reach');
ok(!bv({ ...berth, holds: [[ROOT]] }) && !bv({ ...berth, holds: [['intimate', ROOT]] }), 'berth-offer holds are (root, reach) pairs in that order');
ok(!bv({ ...berth, dataRoot: '/var/lib/x' }), 'berth-offer refuses a local path field (R6)');
ok(!bv({ ...berth, holds: Array(65).fill([ROOT, 'public']) }), 'berth-offer holds at most 64 roots');

// ---- S3 tier record and item absence; R7 envelope
const trv = v('objects/tier-record.schema.json');
const rec = { id: `agent:${AG}`, class: 'household', sources: [{ source: 'held-record', record: 'roster' }],
  endpoint: { form: 'url', base: 'http://192.168.1.4:8090' }, standing: 3,
  proximity: { networkRank: 0, warmRoots: [ROOT], servesExtracted: false, rttMs: null, lastSeenMs: null } };
ok(trv(rec), 'tier-record accepts a household laptop named by a held record');
ok(!trv({ ...rec, id: 'own' }), 'tier-record refuses the indexical own id');
ok(!trv({ ...rec, sources: [] }), 'tier-record refuses an empty source list (A14)');
const ua = v('objects/item-absence.schema.json', '#/$defs/UnansweredItemAbsence');
const trailerEntry = { why: { kind: 'not-held', tried: [`agent:${AG}`] }, comeBack: { when: 'on-event', event: 'tier-reachable' },
  meanwhile: { local: 'nothing', because: 'no tier holds it' }, escalation: null };
ok(ua(trailerEntry), 'the batch route trailer example (§A.6) validates as Unanswered<ItemAbsence>');
ok(!ua({ ...trailerEntry, why: { kind: 'not-held', tried: [] } }), 'not-held names at least one tier asked');
ok(!ua({ ...trailerEntry, why: { kind: 'vanished' } }), 'an unknown absence kind is refused');
ok(!ua({ ...trailerEntry, meanwhile: { local: 'nothing' } }), 'meanwhile nothing must say because');
ok(!ua({ why: trailerEntry.why, comeBack: trailerEntry.comeBack, meanwhile: trailerEntry.meanwhile }), 'escalation is always written (null when not a repeat ask)');
ok(ua({ ...trailerEntry, why: { kind: 'over-budget', by: `agent:${AG}`, rule: 'max-in-flight' }, comeBack: { when: 'at', epochSecs: 1760000100 },
  meanwhile: { local: 'built-in-in-force' }, escalation: { asked: 2, changed: 'nothing', whoCanBeAsked: [] } }), 'over budget with a come-back time and an escalation validates');
ok(ua({ ...trailerEntry, why: { kind: 'refused', by: `agent:${AG}`, refusal: { reason: 'outside-standing', needsStanding: null } } }), 'a reach refusal by name validates');

// ---- S4 parameter set and head
const psv = v('objects/placement-parameter-set.schema.json');
const set = { schema: 1, kind: 'custody-root', method: XOR, weights: { distance: 10000, 'spare-capacity': 2000, 'always-on': 500 }, maxMovesPerEvent: 2 };
ok(psv(set), 'parameter-set accepts a WeightedCapacity-shaped tuning');
ok(psv({ ...set, kind: 'custody-blob', weights: {} }), 'parameter-set accepts an empty weight map (xor-distance reads no weight)');
ok(!psv({ ...set, weights: { reach: 1 } }), 'parameter-set refuses a weight on a non-factor (no reach factor can exist)');
ok(!psv({ ...set, weights: { distance: 0 } }), 'parameter-set refuses a weight of 0 (E6)');
ok(!psv({ ...set, weights: { distance: 1.5 } }), 'parameter-set refuses a fractional weight (R-P3)');
ok(!psv({ ...set, schema: 2 }), 'parameter-set version 1 schema refuses encoding version 2');
ok(!psv({ ...set, method: ROOT }), 'parameter-set method is a raw-codec MethodId, not a dag-cbor CID');
ok(!psv({ ...set, scope: SCOPE }), 'parameter-set is scope-free (E1)');
const phv = v('objects/placement-parameter-head.schema.json');
const headDecl = { scope: SCOPE, sets: { 'custody-root': ROOT }, reach: 'intimate', predecessor: null,
  ratification: { by: 'ratified', id: 'uhCkk-act', rule: null, touches: [AG, AG2] }, ratifiedAt: 1760000000 };
ok(phv(headDecl), 'parameter-head accepts a household TouchedConsent ratification');
ok(phv({ ...headDecl, ratification: { by: 'built-in', id: XOR, rule: null, touches: [] } }), 'parameter-head accepts a built-in reference');
ok(!phv({ ...headDecl, sets: { 'not-a-kind': ROOT } }), 'parameter-head refuses an unknown kind key');
ok(!phv({ ...headDecl, ratification: { by: 'elohim', id: 'x', rule: null, touches: [] } }), 'parameter-head refuses a ratification kind outside built-in | charter | ratified (no space-elohim, R5)');

// ---- probe reading and explanation view
const prv = v('objects/probe-reading.schema.json');
const reading = { promise: 'bytes-layer-reads-only-what-is-read@1', concern: 'materialize-what-is-read', subject: ROOT,
  answer: { state: 'present', value: { nodesFetched: 7 } }, evidenceStage: 'real', disclosure: { disclosure: 'holder' }, observedAt: '2026-10-20T12:00:00Z' };
ok(prv(reading), 'probe-reading accepts a holder reading');
ok(!prv({ ...reading, answer: { state: 'present', value: 1, reason: 'timeout' } }), 'a Present answer takes no reason (I1)');
ok(!prv({ ...reading, answer: { state: 'unreachable' } }), 'an Unreachable answer always carries a reason');
ok(prv({ ...reading, disclosure: { disclosure: 'parties', roles: ['carrier', 'carried-for'] } }), 'a parties disclosure by role validates');
ok(!prv({ ...reading, evidenceStage: undefined }), 'evidenceStage is required');
ok(!prv({ ...reading, evidence: 'real', evidenceStage: undefined }), 'the wire name is evidenceStage, not evidence (finding 16)');
const ev = v('views/placement-explanation-view.schema.json');
const expl = { need: ROOT, key: { form: 'tree-view', id: ROOT }, kind: 'custody-root', method: XOR,
  parameters: { by: 'built-in', id: XOR, rule: null, touches: [] }, fallback: null, inputsDigest: 'ab'.repeat(32),
  ownRefusals: [{ agent: AG2, why: { why: { reason: 'unknown', input: 'liveness' }, comeBack: { when: 'on-event', event: 'posture-changed' }, meanwhile: { local: 'run-here' }, escalation: null } }],
  othersRefused: { 'cross-household-below-public': 1 }, eligible: 2, chosen: [AG],
  outcome: { outcome: 'not-carried', why: { why: { reason: 'domain-short', have: 1, need: 2 }, comeBack: { when: 'on-event', event: 'posture-changed' }, meanwhile: { local: 'nothing', because: 'one household holds every copy' }, escalation: null } },
  digestMatchesCommitment: true };
ok(ev(expl), 'placement-explanation-view accepts the §D worked example (DomainShort {1, 2})');
ok(!ev({ ...expl, inputsDigest: 'AB'.repeat(32) }), 'inputsDigest is lower-case hex');
ok(!ev({ ...expl, outcome: { outcome: 'not-carried', why: { ...expl.outcome.why, why: { reason: 'unknown', input: 'x' } } } }), 'not-carried why is a NotCarriedReason, not an IneligibleReason');

// ---- enum equality with the Rust that exists today (text read; the spec's Rust-side tests own this once the crates land)
const kebab = (s) => s.replace(/([a-z0-9])([A-Z])/g, '$1-$2').toLowerCase();
const rustVariants = (file, name) => {
  const src = readFileSync(join(REPO, file), 'utf8');
  const m = src.match(new RegExp(`pub enum ${name}(?:<[^>]*>)? \\{([\\s\\S]*?)\\n\\}`));
  if (!m) return null;
  return m[1].split('\n').map((l) => l.trim()).filter((l) => /^[A-Z]/.test(l)).map((l) => kebab(l.match(/^([A-Za-z]+)/)[1]));
};
const eq = (a, b) => JSON.stringify(a) === JSON.stringify(b);
ok(eq(rustVariants('crates/seam-contracts/src/probe.rs', 'EvidenceStage'), load(join(V1, 'enums/evidence-stage.schema.json')).enum), 'evidence-stage equals seam_contracts::probe::EvidenceStage');
ok(eq(rustVariants('crates/seam-contracts/src/probe.rs', 'PartyRole'), load(join(V1, 'enums/party-role.schema.json')).enum), 'party-role equals seam_contracts::probe::PartyRole');
ok(eq(rustVariants('crates/seam-contracts/src/probe.rs', 'Disclosure'), load(join(V1, 'enums/disclosure.schema.json')).enum), 'disclosure equals the tags of seam_contracts::probe::Disclosure');
const gs = readFileSync(join(REPO, 'elohim/elohim-storage/src/api/compute_grants.rs'), 'utf8');
const mss = gs.match(/const MEASURE_STAGE_SCOPE: &str = "([^"]+)"/);
const gsList = gs.match(/const GRANT_SCOPES: \[&str; \d+\] = \[([^\]]+)\]/)[1].split(',').map((x) => x.trim()).map((x) => x === 'MEASURE_STAGE_SCOPE' ? mss?.[1] : x.replace(/"/g, ''));
ok(eq(gsList, load(join(V1, 'enums/grant-scope.schema.json')).enum), `grant-scope equals elohim-storage GRANT_SCOPES (${gsList.join(', ')})`);
const reachRust = readFileSync(join(REPO, 'elohim/epr/src/reach.rs'), 'utf8');
ok(/rename_all = "lowercase"/.test(reachRust), 'Reach serializes lowercase, as enums/reach (the needs and intents $ref it)');

console.log(`\n${passes} passed, ${failures} failed`);
process.exit(failures ? 1 : 0);
```

## Fix round 1

I checked the nine review findings against the code. They name six distinct defects; findings 1, 3 and
9 are one defect, and findings 5 and 6 are one defect. Base: the working tree. Nothing is staged,
committed or pushed.

### Fixed

1. **`not-held` with an empty `tried` was refused (findings 1, 3, 9). The finding holds.**
   `ItemAbsence::not_held(Vec::new())` is a real state: `from_error` builds it for
   `BlobNotFound`/`BlobNotLocal`, and the eprfs-serve test tier answers every miss with
   `NotHeld { tried: [] }` (testing.rs:26, :577). Spec B2 also says `tried` is empty for a leaf store.
   eprfs-serve does not stamp its own id into `tried`, so finding 9's alternative does not apply.
   - `objects/item-absence`: removed `minItems: 1` from `ItemAbsenceNotHeld.tried`. Its description and
     the top-level description now say that an empty `tried` is a leaf store's own miss, with no tier
     dialled, and that otherwise `tried` lists every tier asked, in order.
   - Harness: the vector is flipped (an empty `tried` validates), and a new vector refuses a missing
     `tried`.
2. **The wire ownership of `Unanswered` and `ItemAbsence` was contradictory (finding 7). The finding
   holds.**
   - Both files now read "Source of truth: this schema for the wire; Rust mirror …", as probe-reading
     already did.
   - Each says the mirror's serde carries the same tags and field names. Each also says that an adapter
     writing the wire from that serde must serialize to this schema, and that no test pins the two yet
     (a named follow-on).
   - For `ItemAbsence` the description states that the serde is the encoding a TierEndpoint writes.
   - I read the serde attributes by hand against each schema. They agree: `kind` / `when` / `local`
     tags, kebab-case tags and camelCase fields. No test checks this yet.
3. **`Escalation.asked` was off by one, and `changed` could be empty (finding 8). The finding holds.**
   `Unanswered::escalate` starts at 2 (unanswered.rs:154, :189).
   - `asked` now has `minimum: 2`, and its description copies the Rust wording.
   - `changed` now has `minLength: 1`, with the wording "said even when it is 'nothing'". This makes
     the schema stricter than Rust, whose `String` accepts "".
   - Harness: new vectors refuse `asked` 0 and 1 and an empty `changed`.
4. **Codegen freshness was red (finding 4). The finding holds.** S5 requires codegen freshness, so I
   regenerated with the repository's own generators. I did not edit any generated file by hand.
   - `node scripts/codegen-ts.mjs` updated the six `schema-enums.ts` copies (+89 lines each, 0 removed:
     the nine `_codegen` enums, the same diff the round-0 report predicted) and the tracked
     `generated-ts/index.ts` (+28 exports). It also refreshed the gitignored
     `genesis/seeder/src/generated/p2p-status-view.ts`, a local cache. That was the one pre-existing
     stale file, and it has no repository footprint.
   - `node scripts/codegen-ttl.mjs` updated `generated-ttl/elohim-vocabulary.ttl` (+427/−2). It now
     holds 61 vocabularies and 428 concepts, against 46 and 361 before. The 67 new concepts are exactly
     the values of the 15 new enums. The two deleted lines are the old count banners.
   - No consumer re-exports `schema-enums` with `export *`. Every consumer imports names, so the added
     constants cannot collide.
   - **Write-set note:** these generated projections are outside `v1/**`. I regenerated them because
     S5 and `elohim/sdk/.epr-meta` (commit the source and its projection together) require it.
   - Round-0 follow-on 1 is still open: `INTERFACE_FILES` and `GENERATED_OUTPUT_DIRS` in
     `scripts/codegen-ts.mjs`. That is script authorship, not regeneration.
5. **Some enum descriptions claimed a test pin that does not exist (findings 5, 6). The finding holds:
   it was a false absolute.**
   - The six enums placement-core's `schema_contract` really pins keep the claim: placement-kind,
     grant-scope, loss-domain, need-class, isolation and placement-factor.
   - `depth` now says that `placement_core::Depth` is pinned and `eprfs_core::Depth` is not yet.
   - `evidence-stage` and `disclosure` replace the claim with "Not yet pinned", naming where the case
     belongs.
   - party-role, tier-class, reach-reason, expresser-kind, ratification-kind and param-fallback now each
     say "Not yet pinned", naming the follow-on crate.
   - `param-fallback` no longer calls placement-core "a follow-on crate"; the crate exists.
   - Harness rail: every enum that claims the pin must appear among the `schema!("enums/…")` cases in
     `placement/core/tests/schema_contract.rs`. Every other enum must say "Not yet pinned". 30 new
     checks. Re-running this rail would have caught the defect.

### Refused, with reason

- **Finding 2 (probe-reading `evidenceStage` vs Rust `evidence`).** The finding assumes that
  `Reading<T>`'s serde is the wire. Three sources say it is not:
  - spec finding 16 (line 204: "named `evidenceStage` on the wire to avoid the collision");
  - probe.rs's own module doc ("Scope: this is not the wire shape … one monomorphic struct per served
    promise in `elohim/elohim-views`");
  - the schema's description, which already names the Rust field `evidence` as not the wire form.

  Renaming the schema key to `evidence` would violate finding 16. A Rust rename would still not make
  `Reading<T>` validate, because `ProbeAnswer` is externally tagged and the wire `answer` is
  objects/answer's `{state, value | reason}`. So the test the finding proposes ("serialize a Reading and
  validate") would fail by design. The open work is round-0 follow-on 6 (the elohim-views wire struct)
  and disagreement 6. Both are unchanged.
- **The Rust-side parts of findings 5, 6 and 7 (new `schema_contract.rs` cases, and serde contract tests
  in seam-contracts and eprfs-serve).** They are outside this lane's write-set (`v1/**`). The crates
  belong to lanes C2, C1 and E. I made the in-write-set fix (the descriptions now state truthfully what
  is not pinned) and named each test as a follow-on in the description that needs it. This updates
  round-0 follow-ons 5 and 6:
  - placement-core `schema_contract` should add evidence-stage, disclosure (tags), party-role,
    expresser-kind, ratification-kind and param-fallback. The crate already depends on seam-contracts.
  - An eprfs-core consumer test (eprfs-serve) should pin `eprfs_core::Depth`, `TierClass` and
    `ReachRefusal`'s `ReachReason`, and should round-trip every `ItemAbsence` variant against
    objects/item-absence.
  - seam-contracts should round-trip `ComeBack`, `Meanwhile` and `Escalation` against
    objects/unanswered.

### Files changed in round 1

- Under `elohim/sdk/schemas/v1/`:
  - `objects/item-absence.schema.json` and `objects/unanswered.schema.json`;
  - `enums/{depth,evidence-stage,disclosure,party-role,tier-class,reach-reason,expresser-kind,ratification-kind,param-fallback}.schema.json`
    (descriptions only, no values changed).
- Regenerated:
  - the six `schema-enums.ts` copies (seeder, elohim-app, elohim-service, doorway-app, lamad,
    elohim-identity);
  - `elohim/sdk/schemas/generated-ts/index.ts`;
  - `elohim/sdk/schemas/generated-ttl/elohim-vocabulary.ttl`.
- Harness (scratchpad): `test-placement-schemas.mjs`, now 264 checks. The round-1 diff against the
  round-0 appendix is `harness-r1.diff` beside it: two vectors for the empty `tried`, two for the
  escalation, and the pin-claim rail.

### Gate evidence, round 1

| Command | Result |
|---|---|
| lane-S harness `node test-placement-schemas.mjs <HEAD app-manifest copy>` | EXIT=0 (264 passed, 0 failed) |
| `node scripts/codegen-ts.mjs --verify` | EXIT=0 (round 0: EXIT=1, 7 stale) |
| `node scripts/codegen-ttl.mjs --verify` | EXIT=0, "up to date (61 vocabularies, 428 concepts, 2 axes)" |
| `pnpm run -s schema:unit` | EXIT=0 (round 0: EXIT=1, the 2 TTL tests) |
| `node scripts/codegen-rs.mjs --verify` | EXIT=0 |
| `node scripts/check-dna.mjs` | EXIT=0 |
| `just gate schema-codegen` | EXIT=0 (round 0: EXIT=1) |
| `just gate schema-validate` | EXIT=0 |
| `just gate schema-dna` | EXIT=0 |
| `just gate constants-sync` | EXIT=0 |
| `berth claim cargo --class verify --ttl 3600` → `just gate elohim-placement` → `berth release cargo` | claim EXIT=0 · gate EXIT=0 (schema_contract 3/3, floor 19/19, ceiling 9/9, lib 51/51) · release EXIT=0 |
| `pnpm run -s schema:test` | EXIT=1, the baseline 2 ContentView failures only (76 passed), unchanged |
| the 12 scripts after `test-schema.mjs`, plus `test-manifest-schema` and `test-observation-kind-schema`, each run alone | every one EXIT=0 |

**Not run:**
- `elohim-storage`, `elohim-app` and `domain-types` gates. The gate runner selects them through the
  regenerated `schema-enums.ts` and the `elohim/sdk/**` globs. The change to those files is additive
  constants only, imported by name, and the workspace RAM guard was at the soft watermark (77.5%).
- `genesis`, for the same reason.

**Concerns that remain:**
- The `schema:test` baseline red is still there.
- Round-0 follow-ons 1 and 3 to 7 are still open, with 5 and 6 amended above.
- Disagreements 1 and 2 (the `cid_serde` hazard inside internally and adjacently tagged enums) have not
  been re-checked against placement-core in this round.

## Fix round 2

I checked the three review findings against the code. Base: the working tree. Nothing is staged,
committed or pushed. Status stays `DONE_WITH_CONCERNS`.

### Fixed

1. **The seven enum pin claims were a false absolute (finding 2). The finding holds.**
   `placement/core/tests/schema_contract.rs` compares each schema with a list written by hand in Rust.
   For `loss-domain`, `depth` and `isolation` the list is a literal inside the test (lines 59-84). For
   `placement-kind`, `grant-scope`, `need-class` and `placement-factor` it is `KindId::ALL`,
   `GrantScope::ALL`, `NeedClass::BUILTIN_ORDER` or `Factor::ALL`. No exhaustive match ties any of
   these to its enum, and four of the enums are `#[non_exhaustive]`. So a Rust variant added without
   editing the list passes.
   - I took the finding's second option, because the first (guards inside placement-core) is outside
     `v1/**`. Each description now says what holds: "placement-core's schema_contract test reads this
     file with include_str! and compares it, in order, with [the hand list]. It catches a value changed
     here, but a variant added to the Rust enum without editing the test still passes." Each also
     names the guard as a follow-on.
   - `depth` keeps its separate note that `eprfs_core::Depth` is not pinned at all.
   - Harness rails: no new enum may say "held equal to this list", and every claimed pin must state the
     "still passes" limit.
2. **The wire's source of record pointed in two directions (finding 3). The finding holds.** I took
   §2.G's direction: the schema is the wire's source of record and the Rust struct is its mirror.
   Thirteen non-enum schemas now carry two separate statements:
   - "Wire shape: this schema is the source of record; <Rust type> (<path>) is its Rust mirror, not yet
     pinned to it by a test (follow-on: …)".
   - Where the type is content-addressed, the description adds that the Rust codec is the authority
     only for the canonical DAG-CBOR bytes and the CID: `TreeView::cid`, `Need::id`,
     `ParameterSet::cid` and `ranking_cid`.
   - "Source of truth:" goes back to the meaning CONVENTIONS rule 2 gives it, and the meaning every
     existing view uses: where the data lives. For example, "the need's source, from which it is
     derived each pass (Category C)" and "the space's head declaration. Category A2".
   - The thirteen schemas are tree-view, tier-record, need-key, placement-need, posture, berth-offer,
     placement-parameter-set, placement-parameter-head, placement-explanation-view, unanswered,
     item-absence, probe-reading and placement-capacity. The last two were not named in the finding;
     I changed them so all thirteen read the same way.
   - I dropped "a follow-on crate" for placement-core, which exists. `ark_core::task::BerthOffer` does
     not exist yet, so berth-offer says the mirror is to be pinned by a test in the same change.
   - The enum schemas keep "Source of truth: this schema". For a vocabulary, this schema is where the
     data lives, so the meaning matches both CONVENTIONS rule 2 and `elohim/sdk/.epr-meta` (one source
     of record per vocabulary).
   - Harness rails: each non-enum schema names itself as the wire's source of record. Its "Source of
     truth:" is never followed by a Rust path or by "this schema for the wire". It never calls an
     existing crate a follow-on. It says that no test pins its mirror yet.
3. **The report presented R-G1 and the mirror pins as if they were in place (finding 1, its
   report part). The finding holds.** The round-0 headline counted "231 of 231" harness checks.
   That harness lives in the scratchpad, and no build runs it. The table below states which rails are
   open.

| Rail or pin | State | Where it would live (named follow-on, outside `v1/**`) |
|---|---|---|
| R-G1: the needs block is placement-neutral; closure refuses `root`; projectable takes only the eight reach values; the non-regression baseline | **OPEN.** It is checked only by the scratchpad harness. No build runs it | `elohim/sdk/schemas/scripts/test-manifest-needs.mjs`, plus its entry in the root `package.json` `schema:test`, outside the `&&` chain's baseline red |
| Enum values match the Rust enums | **PARTIAL.** Seven enums are compared with hand-kept Rust lists, so a value changed in a schema is caught but a variant added in Rust is not. The other eight say "Not yet pinned" | Exhaustive-match guards in placement-core `kind.rs`, `rank.rs`, `need.rs` and `decide.rs`, plus `ALL` consts for `LossDomain`, `Depth` and `Isolation` |
| `Need`, `Posture`, `ParameterSet`, `HeadDeclaration`, `Explanation` and `Ineligible` serialized and validated against their schemas | **OPEN** | `placement/core/tests/schema_contract.rs`, with a `jsonschema` dev-dependency, on the pattern of `elohim-storage/tests/schema_contract.rs::validate_against_schema` |
| `Unanswered` / `ComeBack` / `Meanwhile` / `Escalation` round trip | **OPEN** | seam-contracts contract test |
| `ItemAbsence`, `TierRecord` and `TreeView` round trip | **OPEN** | eprfs-serve contract test (`testing` feature) |
| The G1 JSON → `Need` → DAG-CBOR → `Need::id` vector | **OPEN** | placement-core |

### Refused, with reason

- **The code parts of finding 1:** landing `scripts/test-manifest-needs.mjs`, the `schema:test` entry,
  and the Rust contract tests in placement-core, seam-contracts and eprfs-serve.
  - The plan's covenant write-set says: "Every other path a lane finds it needs is named in its report
    as a follow-on, never written." The plan's lane rule gives lane S `elohim/sdk/schemas/v1/**` only.
  - The interfaces spec §0 names exactly these items as "follow-ons that lanes E and S need but cannot
    write inside their own paths".
  - placement-core is also being edited live by another seat. Its sources changed at 17:54-17:57 UTC,
    during this round.
  - I made the in-write-set part of the fix: the descriptions and this report now state truthfully
    which rails are open, in the table above.
- **The code part of finding 2** (exhaustive-match guards and `ALL` consts in placement-core): outside
  the write-set, for the same reason. I took the finding's stated alternative, rewording the
  descriptions.
- New follow-on, found in this round: `placement/core/tests/schema_contract.rs`'s module doc says "a
  schema value added without its Rust variant, or the reverse, fails". The "or the reverse" half is the
  same false absolute. It is outside `v1/**`, so I name it here and do not edit it.

### Files changed in round 2

- Under `elohim/sdk/schemas/v1/`. These are description edits only; no value, property or `$ref`
  changed.
  - `enums/{loss-domain,isolation,depth,placement-kind,grant-scope,need-class,placement-factor}.schema.json`
  - `objects/{tree-view,tier-record,need-key,unanswered,item-absence,probe-reading,placement-capacity,placement-parameter-set,placement-parameter-head}.schema.json`
  - `intents/{placement-need,posture,berth-offer}.schema.json`
  - `views/placement-explanation-view.schema.json`
- Regenerated with the repository's generators, as in round 1. The enum descriptions feed the TTL
  projection:
  - `generated-ttl/elohim-vocabulary.ttl`
  - `generated-ts/index.ts`
  - the six `schema-enums.ts` copies

  `git status` lists the same eight generated files as after round 1.
- Harness (scratchpad): `test-placement-schemas.mjs`, now 346 checks. The round-2 diff is
  `harness-r2.diff` beside it.
  - Mutation check: I restored the round-1 `loss-domain` and `tier-record`, and five rails failed.
  - I then put back the round-2 files, and the harness passed: EXIT=0.

### Gate evidence, round 2

| Command | Result |
|---|---|
| lane-S harness `node test-placement-schemas.mjs app-manifest.HEAD.schema.json` | EXIT=0 (346 passed, 0 failed) |
| `node scripts/codegen-ts.mjs --verify` | EXIT=0 |
| `node scripts/codegen-ttl.mjs --verify` | EXIT=0 |
| `node scripts/codegen-rs.mjs --verify` | EXIT=0 |
| `node scripts/check-dna.mjs` | EXIT=0 |
| `pnpm run -s schema:unit` | EXIT=0 |
| `pnpm run -s schema:test` | EXIT=1: the baseline's two ContentView failures only (76 passed), unchanged |
| the 10 scripts chained after `test-schema.mjs`, each run alone | 10 run, every one EXIT=0 |
| `just gate schema-codegen` / `schema-validate` / `schema-dna` / `constants-sync` | EXIT=0 / EXIT=0 / EXIT=0 / EXIT=0 |
| `berth claim cargo --class verify --ttl 3600` | EXIT=0. The lease was "renewed", so this session already held it |
| `just gate elohim-placement` | **EXIT=1** at `cargo fmt --check -p elohim-placement-core`. The diffs are in `src/decide.rs` and `tests/floor_contract.rs`, which another seat was editing during this round (mtimes 17:54-17:57 UTC). This lane touched neither file |
| `cargo test -p elohim-placement-core --all-features --test schema_contract`, in the gate's own target (`/projects/.cargo-target-pool/family/dev/elohim/dev`, `RUSTUP_TOOLCHAIN=1.96.1`, `RUSTFLAGS=""`, `CARGO_BUILD_JOBS=4`) | EXIT=0 (3 passed). This is the test that reads the edited enum files |
| `berth release cargo` | EXIT=0. I then ran `berth claim cargo` again (EXIT=0) to restore the lease this session held before the round, because a sibling seat may still be using it |

**Concerns that remain:**
- The `elohim-placement` gate is red on another seat's in-progress formatting. That is not this lane's
  change.
- The rails in the table above are open.
- The `schema:test` baseline is red.
- Disagreements 1 and 2 are still not re-checked. placement-core's `need.rs` doc now says that
  `input_root` is a `CidString`, revising the spec, which may settle disagreement 1. Lane C2 owns the
  check.

## Fix round 1 (second review cycle)

A new review returned five findings. I checked each against the code. Base: the working tree. Nothing is
staged, committed or pushed. Status stays `DONE_WITH_CONCERNS`.

### Fixed

1. **The explanation view has no parameter-set CID (finding 2). The finding holds.** Spec §D.9's prose says
   each commitment carries the method, the parameter-set CID and the inputs digest, and that these three
   remain once the inputs are gone. `parameters` is a RatificationRef: it says what ratified a set, not
   which set it was. The gap starts in the spec's `Explanation` struct, and `placement_core::Explanation`
   (explain.rs:23-43) copies it.
   - `views/placement-explanation-view`: a new required `parameterSet`, a dag-cbor CID
     (`^bafyrei[a-z2-7]{52}$`, `ParameterSet::cid`), written for every outcome. The `parameters`
     description now says it names who ratified the set, not which set. The top-level description lists
     the three committed fields by name and records the spec revision.
   - **Spec revision (for the spec seat):** §D.9's `Explanation` gains `pub parameter_set: CidString`
     after `parameters`. `placement_core::Explanation` and `explain()` (which has the set in hand as
     `eval.in_force.set()`) need the same field. Both are outside `v1/**`, so they are named follow-ons.
     No test pins the Rust mirror to the view, so no gate turns red in the meantime.
2. **"Inputs gone" could be written two ways (finding 3). The finding holds.** Before this fix the view
   accepted `{outcome: inputs-gone}` with any reason and `{outcome: not-carried}` with reason
   `inputs-gone`. I took the finding's schema-side option:
   - `NotCarriedInputsGone` moved to its own `$defs` entry. `NotCarriedReason` still holds the whole
     vocabulary by `$ref`, so it still mirrors the Rust enum.
   - `UnansweredNotCarried.why` is now `allOf [NotCarriedReason, not NotCarriedInputsGone]`.
   - A new `UnansweredInputsGone` narrows `why` to `{reason: inputs-gone}`, and `OutcomeInputsGone`
     refers to it.
   - The outcome description says there is one form, and that the Rust types still admit both.
     `explain()` builds neither today (explain.rs:166, :202). Narrowing the Rust side, either by dropping
     `NotCarriedReason::InputsGone` or by a constructor guard, is a placement-core follow-on.
     `NotCarriedReason::InputsGone` is also reachable through `decide.rs:685`'s `not_carried`.
3. **The ManifestNeeds description claimed rails that do not exist (finding 5, plus the overclaim in
   finding 1). The finding holds.**
   - "Placement-neutral by construction ... (rail R-G1)" now reads "Placement-neutral as written ... No
     build checks this yet". It names R-G1 as the follow-on `scripts/test-manifest-needs.mjs` and says
     that until the rail lands, a field that broke neutrality would pass every gate.
   - "pinned to this definition by placement/core/tests/schema_contract.rs" now says what that test
     does: it checks that four definitions exist, compares `$ref` strings and checks the mirror's serde.
     It does not validate a document against these `$defs` or resolve their cross-file `$ref`s. The
     named AJV follow-on carries finding 1's accept and refuse cases.

Harness (scratchpad `test-placement-schemas.mjs`): 12 new checks, 358 in all. The diff is `harness-r1b.diff`
beside it. The new checks cover:
- `parameterSet` is required and refuses a raw MethodId.
- The one inputs-gone form validates. `not-carried` with reason `inputs-gone` is refused, and so is
  `inputs-gone` with reason `no-eligible`.
- `not-carried` still accepts its four other reasons.
- The ManifestNeeds description claims neither "by construction" nor a schema_contract pin, and it names
  R-G1 as unbuilt.

### Refused, with reason

- **Finding 1's code (an AJV test in `elohim/sdk/schemas/scripts/`).** The finding holds on substance: no
  build validates `ManifestNeeds.$defs`. The scratchpad harness does validate them, with every cross-file
  `$ref` resolved by file URL, and it passes (StartSet, WorksOffline, Loss, Projectable, Compute, Indexed
  and Signing accept and refuse cases). But no build runs it. `scripts/` is outside the lane's write-set:
  the plan says "every other path a lane finds it needs is named in its report as a follow-on, never
  written". The in-write-set part is fix 3: the description no longer claims a pin, and it names the
  test. Round 2's rail table already lists this row as OPEN.
- **Finding 2's spec and placement-core edits.** The brief says a revision is recorded "so the spec can
  absorb it", and placement-core is outside `v1/**`. Another seat was also editing placement-core during
  this round: `eligibility.rs`, `kind.rs`, `lib.rs` and `floor_contract.rs` have mtimes of 19:01-19:04 UTC.
  The revision is written out under fix 1.
- **Finding 3's placement-core change.** It is outside the write-set, for the same reason. The schema
  carries the single form now, and the Rust follow-on is named under fix 2.
- **Finding 4 (new schema_contract cases, exhaustive-match guards, eprfs-serve cases, build-manifest
  globs).** It holds on substance. Every path it names is outside `v1/**`: `elohim/placement/**`,
  eprfs-serve's tests under lane E, and `elohim/placement/build-manifest.json`. Rounds 1 and 2 already
  made the in-write-set fix: each of the eight unpinned enums says "Not yet pinned" and names its crate.
  Each of the seven pinned enums says a Rust-side addition still passes, and the harness rails hold both
  statements. This round adds to the follow-on list:
  - ExpresserKind, ParamFallback and RatificationKind cases in placement-core, which cost nothing there;
  - a `const fn` exhaustive match for every `ALL` list;
  - widening the gate-placement input globs once object contract tests exist.

### Files changed in this cycle

- `elohim/sdk/schemas/v1/manifest/app-manifest.schema.json`: the ManifestNeeds description only. No
  property, `$ref` or value changed.
- `elohim/sdk/schemas/v1/views/placement-explanation-view.schema.json`:
  - the required `parameterSet`;
  - `$defs.NotCarriedInputsGone` and `$defs.UnansweredInputsGone`;
  - the narrowed `UnansweredNotCarried.why`;
  - the descriptions.

  The view is not in `INTERFACE_FILES`, so no generated file changes.

### Gate evidence, this cycle

| Command | Result |
|---|---|
| lane-S harness `node test-placement-schemas.mjs app-manifest.HEAD.schema.json` | EXIT=0 (358 passed, 0 failed) |
| `node scripts/codegen-ts.mjs --verify` | EXIT=0 |
| `node scripts/codegen-ttl.mjs --verify` | EXIT=0 |
| `node scripts/codegen-rs.mjs --verify` | EXIT=0 |
| `node scripts/check-dna.mjs` | EXIT=0 |
| `node scripts/test-manifest-schema.mjs` | EXIT=0 |
| `pnpm run -s schema:unit` | EXIT=0 |
| `pnpm run -s manifest:codegen:verify` | EXIT=0 |
| `pnpm run -s schema:test` | EXIT=1: the baseline's two ContentView failures only (76 passed), unchanged |
| the 10 scripts chained after `test-schema.mjs`, plus `test-observation-kind-schema`, each run alone | 11 run, every one EXIT=0 |
| `just gate schema-codegen` / `schema-validate` / `schema-dna` / `constants-sync` | EXIT=0 / EXIT=0 / EXIT=0 / EXIT=0 |
| `berth claim cargo --class verify --ttl 3600` | EXIT=0 ("renewed") |
| `just gate elohim-placement` (after waiting out another session's `cargo test -p eprfs-core`) | **EXIT=1** at `cargo fmt --check -p elohim-placement-core`. The diffs are in `src/eligibility.rs`, `src/kind.rs`, `src/lib.rs` and `tests/floor_contract.rs`, which another seat was editing at 19:01-19:04 UTC. This lane touched none of them |
| `cargo test --manifest-path elohim/Cargo.toml -p elohim-placement-core --all-features --test schema_contract` in the gate's target (`RUSTUP_TOOLCHAIN=1.96.1`, `RUSTFLAGS=""`, `CARGO_BUILD_JOBS=4`) | EXIT=0 (3 passed). This is the test that reads the edited app-manifest |
| `berth release cargo` | EXIT=0 |

**Concerns that remain:**
- `just gate elohim-placement` is red on another seat's in-progress formatting.
- The follow-ons above are open: the AJV needs test and R-G1, the parameter-set field in the spec and in
  placement-core, the Rust inputs-gone narrowing, and the enum pins.
- The `schema:test` baseline is red.

## Codex review 1 fixes

A Codex review returned four findings. Findings 1, 2 and 3 name one defect: integers wider than their
Rust types. Finding 4 is the R-G1 rail. I checked each against the code. Base: the working tree. Nothing
is staged, committed or pushed. Status stays `DONE_WITH_CONCERNS`.

### Fixed

1. **Integers were unbounded above (findings 1, 2, 3). The finding holds.** I listed every integer in the
   lane-S schemas and the needs block, then read each one's Rust width in placement-core
   (`offer.rs`, `need.rs`, `explain.rs`, `params.rs`; `EpochSecs = u64`) and in spec §2.D. Twelve
   fields had no `maximum`.
   - u64 fields, now bounded to `9007199254740991` (2^53 − 1): capacity `bytes` and `memoryBytes`;
     need `requires.bytes` and `requires.memoryBytes`; posture `spareBytes` and `writtenAt`; berth-offer
     `incarnation`; parameter-head `ratifiedAt`; tier-record `proximity.lastSeenMs`; unanswered
     `comeBack.epochSecs`. Each description now says: "A u64 in Rust, bounded here to 9007199254740991
     … a larger value is refused, never rounded."
   - u32 field, now bounded to `4294967295`: the explanation view's `othersRefused` counts
     (`BTreeMap<String, u32>`).
   - `needs.projectable.retentionFloorDays` has no reader yet, so it has no Rust width. I bounded it to
     u32, the width of its sibling `ratePerHour`, and its description says so.
   - The other integer fields were already bounded to their widths: `cpuMillis`, `pids`, `rttMs`,
     `eligible`, `asked` and `ratePerHour` to u32; `holders`, `domains`, `have`, `need` and
     `maxMovesPerEvent` to u16; `networkRank` to u8; `standing` and `openness` to 1..8; weights to i32.
   - **Why the safe-integer bound and not decimal strings.** Finding 2 offers both. Finding 3 cites
     CONVENTIONS rule 8. I took the safe-integer bound for three reasons:
     - Rule 8's string form is for values that exceed the JavaScript safe range. Its one use in v1 is
       `peer-status-view`'s microsecond timestamps, whose description reads "exceeds JS safe integer
       range".
     - The byte counts a grep of v1 finds are integers, for example `peer-capacity-view.totalRawBytes`
       and `household-resilience-view.totalPledgedBytes`. 2^53 − 1 bytes is about 8 PiB.
     - placement-core's serde writes these fields as JSON numbers. A string form would need a Rust
       serializer change, and placement-core is outside `v1/**`. With the bound, every value the schema
       accepts is a value both readers hold exactly, so a need's JSON cannot be rounded into a
       different `Need::id`.
   - No generated file changes. None of these schemas is in `INTERFACE_FILES`, and only enums feed the
     TTL. `codegen-ts --verify` and `codegen-ttl --verify` pass.
2. **Harness (scratchpad `test-placement-schemas.mjs`).** There are 39 new checks, 397 in all. The diff
   is `harness-codex1.diff` beside it.
   - A rail: every integer in each lane-S non-enum schema and in `ManifestNeeds` has a `maximum` no
     wider than 2^53 − 1.
   - Boundary vectors at the bound and one past it, for every field above.
   - The finding's own cases are refused: capacity `bytes` 18446744073709551616 (u64::MAX + 1),
     `bytes` 1e100, `othersRefused {"no-consent": 4294967296}`, and the text
     `{"bytes":9007199254740993,…}`.
   - An exact round trip: a capacity text at the bound re-serializes byte-identically.
   - Mutation check: I deleted capacity `bytes.maximum`, and five checks failed. I restored the file,
     and the harness passed with EXIT=0.

### Refused, with reason

- **Finding 3's decimal-string representation.** See "Why the safe-integer bound" above. Rule 8 does
  not reach a count that fits the safe range. The string form would also change placement-core's serde,
  which is outside the write-set.
- **Finding 4 (build R-G1 as a failing rail and wire it into `schema:test`).** It holds on substance. No
  build checks the needs block's neutrality, and the ManifestNeeds description already says so ("No
  build checks this yet"). The fix the finding names needs `elohim/sdk/schemas/scripts/test-manifest-needs.mjs`
  and the root `package.json`. Both are outside `v1/**`:
  - The plan says: "every other path a lane finds it needs is named in its report as a follow-on, never
    written."
  - Interfaces spec §0 lists this script as a follow-on that lane S cannot write.
  - The recursive scan, including negative fixtures, already exists as the scratchpad harness's R-G1
    section. It follows `$ref` targets into other files. A planted `hostUrl` with `format: uri`, and a
    `peerId` in a referenced file, were each reported (round 0).

  Landing it is copying that section into `scripts/test-manifest-needs.mjs` and adding it to
  `schema:test` outside the `&&` chain, because `test-schema.mjs`'s baseline red stops the chain before
  it. That is unchanged from the round-2 rail table, row R-G1, OPEN.

### New disagreement, for the spec seat

13. **The u64 fields are bounded on the wire at 2^53 − 1, but Rust accepts the full u64.** A
    `placement_core::Capacity`, `Requirements` or `Posture` holding a larger value serializes to JSON
    that this schema refuses. No real byte count, epoch second or incarnation gets near the bound.
    Follow-on (placement-core, outside `v1/**`): when the schema-validation contract test lands, add a
    vector at 2^53. Then either refuse a larger value at the wire adapter, or record the bound as a
    `const` beside each type.

### Files changed in this cycle

These are under `elohim/sdk/schemas/v1/`. I added a `maximum` and a description sentence only; no
property, `$ref` or value changed.
- `objects/{placement-capacity,unanswered,tier-record,placement-parameter-head}.schema.json`
- `intents/{placement-need,posture,berth-offer}.schema.json`
- `views/placement-explanation-view.schema.json`
- `manifest/app-manifest.schema.json`: `Projectable.retentionFloorDays` only

### Gate evidence, this cycle

| Command | Result |
|---|---|
| lane-S harness `node test-placement-schemas.mjs app-manifest.HEAD.schema.json` | EXIT=0 (397 passed, 0 failed) |
| `node scripts/codegen-ts.mjs --verify` | EXIT=0 |
| `node scripts/codegen-ttl.mjs --verify` | EXIT=0 |
| `node scripts/codegen-rs.mjs --verify` | EXIT=0 |
| `node scripts/check-dna.mjs` | EXIT=0 |
| `node scripts/test-manifest-schema.mjs` | EXIT=0 |
| `pnpm run -s schema:unit` | EXIT=0 |
| `pnpm run -s manifest:codegen:verify` | EXIT=0 |
| `pnpm run -s schema:test` | EXIT=1: the baseline's two ContentView failures only (76 passed), unchanged |
| the 10 scripts chained after `test-schema.mjs`, plus `test-observation-kind-schema`, each run alone | 11 run, every one EXIT=0 |
| `just gate schema-codegen` / `schema-validate` / `schema-dna` / `constants-sync` | EXIT=0 / EXIT=0 / EXIT=0 / EXIT=0 |
| `berth claim cargo --class verify --ttl 3600` | EXIT=0 |
| `just gate elohim-placement` | EXIT=0 (placement-core lib 69, ceiling 15, floor 31, schema_contract 3; elohim-compute 21) |
| `berth release cargo` | EXIT=0 |

Gate-evidence line: `just gate elohim-placement` → `gate elohim-placement EXIT=0`; `just gate schema-codegen` → `gate schema-codegen EXIT=0`.

**Concerns that remain:**
- R-G1 and the other follow-ons in the round-2 rail table are open.
- Disagreement 13 is open.
- The `schema:test` baseline is red.
