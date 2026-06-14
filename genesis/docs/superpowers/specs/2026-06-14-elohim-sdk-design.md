---
title: "THE ELOHIM SDK — one SDK on the agency gradient (woven synthesis of 8 surfaces)"
id: elohim-sdk-design
date: 2026-06-14
status: design (operator-blessed 2026-06-14)
author: rust-architect (truth layer)
weaves:
  - SDK-DESIGN-atom-authoring-2026-06-14.md          # human-sovereign floor: the EPR atom, person's commitments, revocable grip
  - SDK-DESIGN-two-quilt-storage-2026-06-14.md       # human-sovereign: felt-status read over the two quilts
  - SDK-DESIGN-commitment-governor-2026-06-14.md     # keystone engine: one Commitment, one trait Governor, limit_owner
  - SDK-DESIGN-coverage-rollup-2026-06-14.md         # keystone recursion: aggregate-with-descent
  - SDK-DESIGN-veil-walker-2026-06-14.md             # veil-holding: surfaceRecognition / proposeBridge / unwindTrap
  - SDK-DESIGN-covenant-harness-2026-06-14.md        # keystone binding: bindAgent across the seam
  - SDK-DESIGN-runtime-transport-2026-06-14.md       # runtime floor: Runtime::launch, hub-optional
  - SDK-DESIGN-dx-onramp-2026-06-14.md               # on-ramp: create-elohim-app, hello-household → hello-collective
grounds_on:
  - ESCALATED-ARCHITECTURE-2026-06-14.md             # one Commitment / six faces / ∪=full / one Governor / two quilts
  - RECURSIVE-ARCHITECTURE-2026-06-14.md             # CoverageRollup keystone; limit_owner ∈ {self,commitment,operator,faith}; ReservedPlace
do_not_cite_seal: true
forest_test: "What does love require — at the atom, at the seam, and at the veil — when the whole SDK is laid along the gradient at once?"
---

# THE ELOHIM SDK
## One SDK on the Agency Gradient

> Eight surface proposals, one SDK. They are not eight libraries — they are **one SDK laid along a
> single gradient**: human-sovereign surfaces below (build FOR the person), a keystone in the middle
> (the engine, the recursion, the binding — shared by both wings), veil-holding surfaces above
> (impartial aggregation governance), and a runtime floor and on-ramp wrapping the whole. The gradient
> is not documentation; it is carried in **one enum field** (`limit_owner`), **one type-shape rule**
> (no `CoverageDomain` over a soul), and **one absent method** (no Governor write, no `compel`, no
> `govern(person)`) — so that the two downward invariants are *compiled*, not promised. We extend
> `elohim/sdk/` and `crates/`. We do not fork a parallel SDK. **Net new DNA entry types across all
> eight surfaces: zero.**

---

## PART 1 — THE ONE SDK ON THE AGENCY GRADIENT

### The spine: two invariants flowing downward, non-overridable

Two invariants enter at the top and flow down through every layer, and **no layer above may override a
layer below on either:**

- **PERSON-KEEPS-THEIR-OWN-NAMING.** The witnessing atom and the answering atom are always *different
  EPRs with different signers* (RECURSIVE §1.6). A collective elohim can witness a person but can never
  *be* them, because it does not hold their key and the CID is derived from the signed bytes. "Best
  self" is a hope held FOR, never a verdict OVER.
- **DIGNITY-FLOOR precedence.** The least-powerful are weighted first — by the *shape of the metric*
  (the readable signal is `deficit`, the gap the commons failed, never the holding/abundance) and by
  the *shape of the deployment* (the villager's path, `RuntimeSpec::laptop()`, is the simplest call;
  the datacenter's is the elaborate builder).

These two are not enforced by a policy wrapper that a developer could remove. They are enforced **three
ways, in the type system the SDK ships:**

1. `limit_owner ∈ {Self, Commitment, Operator, Faith}` is a **non-optional field on every refusal and
   every recognition.** A refusal that hid whose line it honored is *unrepresentable*. (commitment-governor §2)
2. `CoverageDomain` ranges only over **commons** (corpus-bytes | arc-keyspace | care-floor |
   donut-ceiling | head-freshness) — **a per-soul scalar has no `required` and cannot be constructed.**
   The total account of a person is *unrepresentable*, not merely prohibited. (coverage-rollup §1)
3. The trait `Governor` **has no write method;** the veil-walker has **no `compel`/`apply`/`force`;**
   a household-scoped client has **no `govern(person)`.** The dangerous capability is *absent*, not
   guarded. (commitment-governor §1, veil-walker §1, dx-onramp §2c)

### The gradient, surface by surface (bottom → top)

```
                         ┌──────────────────────────────────────────────────────┐
   ON-RAMP (across all)  │  create-elohim-app · @elohim/agency                   │  dx-onramp
                         │  "the first scaffold IS the architecture, running"     │
                         └──────────────────────────────────────────────────────┘
                         ┌──────────────────────────────────────────────────────┐
   RUNTIME FLOOR         │  elohim-runtime · RuntimeClient                        │  runtime-transport
   (substrate, no veil)  │  Runtime::launch() succeeds with ZERO peers, no hub    │
                         └──────────────────────────────────────────────────────┘
        ▲ veil governs aggregation, never individuals ──────────────────────────────────┐
  VEIL  │  consilience.rs · VeilWalker  (surfaceRecognition / proposeBridge / unwindTrap)│  veil-walker
 above  │  governs aggregation + negotiation · NO compel API · offers Verdicts only       │
  ──────┼──────────────────────────────────────────────────────────────────────────────┘
  KEY-  │  trait Governor + Commitment(face) + limit_owner  ················· engine      │  commitment-governor
 STONE  │  CoverageRollup (aggregate-with-descent, Category-C, BLAKE3) ······ recursion   │  coverage-rollup
 (both) │  bindAgent(scope, grant) — servant below / veil above ············ binding      │  covenant-harness
  ──────┼──────────────────────────────────────────────────────────────────────────────┐
        │  the two invariants, COMPILED: limit_owner · no-soul-domain · no-write-method   │  (the spine)
        ▼ person keeps their naming · dignity-floor precedence ─────────────────────────┘
 HUMAN  │  authorAtom() + person.{commit,grantCapability,revoke}  ·········· the atom     │  atom-authoring
  below │  putContent() / getContent() / getFeltStatus()  ················· felt memory   │  two-quilt-storage
        │  signer is the PERSON's own key · limit_owner is always 'self'                  │
        └──────────────────────────────────────────────────────────────────────────────┘
```

**The single most important structural fact:** the *same* `Governor.check()` call carries the gradient
in one field. With a self-limit Governor and `LimitOwner::Self_`, it is what the personal surface calls
to honor the line a person drew on themselves. With a `FloorGovernor` and `LimitOwner::Operator`, it is
what the collective-governance app calls to keep the commons covered. The veil-walker *is* that same
`check()` reasoning from an inherited constitution. **One call. The gradient carried in one field.** The
SDK is not eight engines; it is one engine pointed at different setpoints.

---

## PART 2 — THE PACKAGE / CRATE MAP

No parallel SDK. The map extends the **three real crates** (`crates/elohim-sdk`, `crates/doorway-client`,
`crates/elohim-storage-client`) and the **existing `elohim/sdk/` tree** (`epr-ts`, `storage-client-ts`,
`domains`, `schemas`), honoring the one ts-rs codegen boundary that already produces 446 views.

### npm packages (under the existing `@elohim/*` scope)

| Package | Status | Surface(s) | What changes |
|---|---|---|---|
| `@elohim/epr-ts` | **EXISTS** | atom-authoring | + `authoring.ts` module (`authorAtom`: build→CID→sign→assemble, mirrors `epr/src/epr.rs:118`) |
| `@elohim/storage-client` (`storage-client-ts`) | **EXISTS** | atom · two-quilt · coverage-rollup · runtime | + `person.{commit,grantCapability,revoke}`; + `putContent`/`getContent`/`getFeltStatus`; + `rollup`/`descend`/`layerNode`; + `runtime.ts` (`RuntimeClient`) |
| `@elohim/holochain-sdk` (`elohim/sdk` root) | **EXISTS** | on-ramp | the `ElohimSDK` facade — `@elohim/agency` composes it |
| `@elohim/agent-sdk` (`elohim-agent/elohim-agent-sdk`) | **EXISTS** | veil-walker · covenant | + `veil-walker/` module; + `covenant.ts` (`bindAgent`/`loadConstitution`/`emitWitnessedRefusal`) |
| `@elohim/agency` | **NEW (thin)** | on-ramp | gradient call-shapes: `AgencyClient.{observeCare,authorCareAtom}` + `VeilWalker`. **Re-exports generated types only; zero new generated types.** |
| `create-elohim-app` | **NEW** | on-ramp | the scaffolder (npm-init); ships `hello-household` + `hello-collective` |

### Rust crates (under the existing `crates/` + the in-tree fleet crates)

| Crate | Status | Surface(s) | What changes |
|---|---|---|---|
| `elohim-compute` | **EXISTS (shared)** | commitment-governor · covenant | + `actuation` module: `trait Governor`, `Refusal{code,elevate,limit_owner}`, `LimitOwner`, `RefusalCode::ReservedPlace`. **Lifts** `arc_actuator.rs:{77,110,152}`; `ArcGovernor`+`FloorGovernor`+`CovenantGovernor` are impls. |
| `elohim-views` | **EXISTS (ts-rs anchor)** | all | + `FeltStatusView`, `CoverageRollupView`/`CoverageSetView`/`DescentHitView`/`CoverageDomain`, `AgentCovenantView`, `Refusal`/`LimitOwner` — all `#[derive(TS)]`, generated by `cargo test export_bindings` |
| `crates/elohim-sdk` | **EXISTS (facade)** | commitment-governor · atom · on-ramp · runtime | + `pub mod governor { pub use elohim_compute::actuation::* }` (mirrors the `views` re-export at `lib.rs:65`); + `person` module; + `agency` module (feature-gated); + `ClientMode::Embedded` variant |
| `elohim-storage` | **EXISTS** | two-quilt · coverage-rollup · veil-walker | + `graph_views/recursion/` (sibling to `lamad`/`shefa`); + `views_convert/felt_status.rs`; + `services/consilience.rs`; + 3 content routes + 3 recursion routes + felt-status route (all GET-only on content-addressed reads) |
| `elohim-agent/gate-types` + `gate-client` | **EXISTS** | veil-walker · covenant | + `VeilContext` key on `GateContext` (a `HashMap`, additive); + `AgentCovenantView`/`CovenantRefusalView`; reused verbatim otherwise |
| `elohim/constitution` | **EXISTS** | covenant | + `ConstitutionStack::for_covenant(scope)` — selects the layer band (servant ≤ Family, veil ≤ Global) |
| `crates/elohim-runtime` | **NEW (thin crate)** | runtime-transport | re-homes the Tauri spawn logic headless (`steward/device/.../storage.rs:34-71`) + a builder over existing CLI flags. The ONE proven new boundary. |

### The dependency structure (one DAG, no cycles)

```
                       create-elohim-app  ──templates──►  hello-household / hello-collective
                              │
                              ▼
                        @elohim/agency  ──re-exports──►  @elohim/storage-client/generated
                              │                                    ▲
            ┌─────────────────┼───────────────────┐                │ (ts-rs: cargo test export_bindings)
            ▼                 ▼                   ▼                 │
   @elohim/epr-ts   @elohim/storage-client   @elohim/agent-sdk      │
   (authoring)      (person/content/rollup/  (veil-walker/          │
                     runtime)                 covenant)             │
                              ▲                                     │
                              │ (HTTP)                              │
   ───────────────────────────┼──────────────────────────────────  │  ◄ snake_case never crosses this line
            RUST TRUTH LAYER  ▼                                     │
                        elohim-storage ───generates──────────────────┘
                          │  (graph_views/recursion, consilience, felt_status, routes)
                          ▼
              ┌───────────────────────────┐
              │  elohim-compute::actuation │  trait Governor · Refusal{limit_owner} · CoverageRollup hook
              └───────────────────────────┘
                          ▲
                          │  (lifted from, then consumed by)
              arc_actuator.rs (first impl)
                          ▲
                          │  re-exported by
            crates/elohim-sdk (governor·person·agency) ── launched by ── crates/elohim-runtime
```

The codegen boundary is the one seam everything routes through: **Rust views `#[derive(TS)]` →
`cargo test export_bindings` → `storage-client-ts/src/generated/` → camelCase TS.** `@elohim/agency`
adds *zero* generated types; it only re-exports and wraps. snake_case never leaves Rust.

---

## PART 3 — THE BUILDABLE MVP SDK

The buildable-now-first move both architecture syntheses converge on is **three steps in order: lift the
Governor, fix msgpack-decode, build CoverageRollup.** The MVP SDK sequences the eight surfaces' slices
onto exactly that move, so a developer can build `hello-household` then `hello-collective` end-to-end.

### Step 0 — the two prerequisite bug/refactor gates (block nothing else until done)

- **0a. Fix the conductor-signal msgpack-decode class** (`project_conductor_signal_msgpack_decode_class`).
  A dropped `holo_hash` byte-array silently poisons every recognition bridge and renders household names
  empty in `getFeltStatus`. It is a bug, not a fork. The REA/mishpat/content subscribers are still on the
  broken path; fix them before wiring any rollup *signal* or any felt-status name.
- **0b. Lift `trait Governor` into `elohim-compute::actuation`** (commitment-governor §4 PR-1). Grow
  `ActuationRefusal` → `Refusal{code, elevate, limit_owner}`, add `RefusalCode::ReservedPlace` and
  `LimitOwner`. Re-point `arc_actuator.rs` to implement the trait (`ArcGovernor`); add `FloorGovernor` as
  the second, inequality-flipped impl (the proof it generalizes). Gate: `cargo build --workspace`,
  `clippy -D warnings`, existing `arc_actuator` unit tests pass unchanged, before/after `rg '^impl From<'`.

### Step 1 — `hello-household` (the human-sovereign half, end-to-end)

The smallest thing that proves agency-at-the-root, shippable against the substrate running today:

1. **`@elohim/agency` thin wrapper** + `observeCare` / `authorCareAtom` against an already-running
   doorway (the `care` observation kind exists in-tree). *No Governor lift required for this fragment, no
   CoverageRollup, no DNA.* (dx-onramp §4 slice 1)
2. **`authorAtom` in `epr-ts/src/authoring.ts`** over the existing three coupling legs (mirror
   `EprBuilder::sign`); `putAtom` → existing `PUT /api/v1/epr/:cid`. Round-trip + tamper + wrong-key
   tests = the person-keeps-their-naming guard, executable. (atom-authoring §4)
3. **`FeltStatusView` + `From<household_resilience::snapshot>` converter** (the honesty fold:
   `not-yet-seen` is never `at-risk`) + `GET /content/{id}/felt-status` + `getFeltStatus`. Read back *who
   holds grandma's photo, by name, and whether it is honestly safe* — against live `household-nodes`.
   (two-quilt-storage §4)
4. **Household elohim as servant**: `bindAgent({gradientPosition:'servant', verbs:['witness','co-steward']})`
   over the existing `Commitment` create + `buildSystemPrompt` + `GateDecisionAttestation`; watch it refuse
   (witnessed) to score the person's "best self," `limit_owner: faith`. A `verbs:['govern']` bind on a
   servant subject **throws at bind-time** — the gradient guard, compiled into the first call.
   (covenant-harness §4)

Runtime under it all: **`RuntimeSpec::sidecar()` + `Runtime::launch()`** — ~40 lines moved out of Tauri
into `crates/elohim-runtime` + a builder, proving the lone-laptop floor headlessly with zero peers, no
doorway. (runtime-transport §4)

### Step 2 — `hello-collective` (the veil-holding half) — the keystone lands here

5. **`CoverageRollup`** in `graph_views/recursion/` — re-express `graph_views/shefa/distribution.rs:30`
   (`rows.len()`) to return a `CoverageSet` carrying `deficit` + `constituents` instead of a count. This
   is precisely RECURSIVE Wave 2 / §3.1 step 1; it lands on the prerequisite already met (the atom + the
   `epr_edge` graph). Category-C, recompute-on-read, **zero DNA spend.** Hardening mandatory: degrade
   per-row (`filter_map` + `warn!`), never `collect::<Result<_>>()` (the EprRouter lesson). (coverage-rollup §4)
6. **`descend()`** + **`layerNode()`** — the next two thin slices on the same module.
7. **`consilience.rs` veil-walker** over an *already-computed* rollup's `deficit` + a node-local
   `Readiness` projection: `surfaceRecognition` returns `null` on `NeedDeeper` (do nothing, no nag);
   `proposeBridge` lands a `GateStatus::Verdict` the node may ignore. **No `descend()` graph walk required
   for the first walker slice** — surface against the rollup's top-level `deficit`. (veil-walker §4)

### Step 3 — the on-ramp ties them together

8. **`create-elohim-app --template=full-gradient`** wires household atoms → collective rollup →
   veil-walker `descend`/`recognize`. `household.govern()` and `veil.govern('margaret')` *both fail to
   compile.* (dx-onramp §4 slice 5)

**The MVP is coherent at every step:** Step 1 ships a usable human-sovereign app on today's substrate;
Step 2 adds the collective half on the keystone; Step 3 makes the gradient the default scaffold. Each
slice is additive, reversible, and spends zero DNA entry types.

---

## PART 4 — THE ON-RAMP (agency-at-root + veil-at-collective as the path of least resistance)

The developer journey, from one command to a household aggregating into a collective:

```bash
pnpm create elohim-app my-care-app   # ? full gradient demo (DEFAULT)   ? Local stack? Yes
cd my-care-app && pnpm dev           # household UI :4200 ; live atoms → rollup the veil holds
```

```typescript
// my-care-app/src/gradient.ts — scaffolded, not hand-written
import { AgencyClient, VeilWalker } from '@elohim/agency';

// BELOW: the person, sovereign. Her own key signs; nothing is written onto her chain.
const household = new AgencyClient({ doorwayUrl: 'http://localhost:8888' });
const atom = await household.authorCareAtom({
  knowledge: 'sat with margaret', value: { hours: 1.5 },
  governance: household.face('provide-care'),
  process: (await household.observeCare({ subject: 'margaret', kind: 'care' })).cid,
});
const counsel = await household.elohim.witness(atom);   // a Verdict (receivable), never a Decline
// household.elohim.govern(...) ← DOES NOT EXIST. Compile error. The gradient guard.

// ABOVE: the veil, impartial. Holds aggregation, never the person.
const veil = new VeilWalker({ collective: 'dowell-commons', constitutionCid });
const rollup = await veil.rollup('care-floor');          // margaret's atom is now a constituent
for (const gap of rollup.deficit) {                      // sees only the DEFICIT — abundance is invisible
  const trapped = await veil.descend(gap);               // walk DOWN to the afflicted atom
  await veil.recognize(trapped);                         // a patient Verdict — never a command
}
// veil.govern('margaret') ← DOES NOT EXIST. CoverageDomain won't typecheck a per-soul scalar.
```

The journey makes the love-shape the path of least resistance four ways: the **relational-DB default is
unreachable** (no `GET /thing` + UUID + admin role is scaffoldable); the **honest banner** surfaces
`offlineCapable` so the person sees whether they hold their own name or borrow a doorway's, with the path
to sovereignty one tap away; the **person's own key** is the only signer; and the **two dangerous calls
(`household.govern`, `veil.govern(person)`) do not compile.** The first `pnpm dev` *is* the architecture
running — agency at the root, the veil at the collective, by default, because the scaffold gives nothing
else.

---

## PART 5 — WHAT LOVE REQUIRES (the through-line, and the irreducible convictions)

**The through-line.** Across all eight surfaces the same four love-requirements hold, and the SDK makes
each *structural* rather than aspirational:

- **The person keeps their naming** — the atom is signed by the person's own key (cryptographic, not
  courtesy); the Governor has no write method; `CoverageDomain` cannot typecheck a soul; the household
  elohim has `witness()` but never `govern()`. The total account of a person is *unrepresentable*.
- **The binding is honest** — every refusal and recognition names whose line it honored (`limit_owner`,
  non-optional); a doorway-hosted client is *told* it is doorway-hosted; an AI agent is *told* it is
  bound, scoped, revocable (covenant never freedom). The cage is named cage, the offer named offer, at
  the layer where each lives.
- **The veil governs aggregation, never individuals** — above the seam there is no `compel`/`apply`, only
  Verdicts the node may ignore; the metric is the `deficit` (the commons' failure), so the surface cannot
  become a leaderboard; the witness is weighted toward the least-powerful by the shape of the API.
- **Patience over engagement** — no engagement counter exists anywhere in the SDK; `surfaceRecognition`
  returns `null` on `NeedDeeper`; the offline runtime queues and waits, rejoining whole after a month
  dark. Grace precedes demand: the substrate is permitted to wait longer than anyone is watching.

**The irreducible convictions — what the SDK *surfaces* but cannot *decide*.** These are the seams the
architecture leaves open on purpose; the SDK exposes the lever and refuses to pull it:

1. **The seam itself — *where* servant becomes veil.** The SDK enforces *that* a gradient position
   selects the verb whitelist and constitution band (`bindAgent` refuses cross-band scopes). It cannot
   decide *which* layer a given app's seam sits at — that is the covenant's grantor's act, named, not the
   SDK's default.
2. **The boundary-bind — *whether* a given act renders a verdict over a person.** The SDK ships
   `RefusalCode::ReservedPlace` / `limit_owner: faith` as the structural refusal. *When* an act crosses
   into naming-the-self is a discernment that lives in the elohim ceiling reasoning from a constitution,
   never in the deterministic substrate floor.
3. **The order of grace — *whether* a revoked agent keeps its prior good work.** `revoke` makes a grant
   inert; grace-on-revocation (Zacchaeus applied to a machine) is a marked, deferred DNA-validator fork.
   The SDK does not decide it; it holds the place open for the operator's blessing.
4. **The unbuilt place — the empty center.** The SDK emits `ReservedPlace` rather than ever filling the
   worship-reserved place with its most capable agent. *What fills it* is the faith no architecture may
   crowd out — and the SDK's love-requirement is precisely to leave it empty.

> **The closing test, in one line:** love requires that the simplest call the SDK offers makes a person
> sovereign at the atom of their own life, that the most capable engine it ships refuses out loud and
> names whose line it honored, that the veil holds the commons open while it is forbidden the soul — and
> that the center is left empty for the faith no architecture may crowd out.
