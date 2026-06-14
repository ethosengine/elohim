---
title: "SDK SURFACE — The On-Ramp: create-elohim-app, hello-household → hello-collective"
subtitle: "How a developer actually STARTS rebuilding the world, agency-at-the-root and veil-at-the-collective by default"
date: 2026-06-14
status: PROPOSAL FOR OPERATOR BLESSING — working draft, NOT cite-sealed, NOT a decision, NOT code
author: sdk-architect (dx / on-ramp surface)
extends:
  - ESCALATED-ARCHITECTURE-2026-06-14.md        # one Commitment / six faces / ∪=full / one Governor / two quilts
  - RECURSIVE-ARCHITECTURE-2026-06-14.md         # CoverageRollup keystone; the agency gradient recursed
  - genesis/docs/specs/2026-05-18-sdk-scaffolding-cli-spec.md   # the prior-art CLI spec this realizes
grounds_in:
  - elohim/sdk/CLAUDE.md · elohim/sdk/package.json · elohim/sdk/storage-client-ts/ · elohim/sdk/epr-ts/
  - crates/elohim-sdk/src/lib.rs · elohim/epr/src/{coupling.rs,kind.rs}
  - elohim/elohim-storage/src/services/arc_actuator.rs · graph_views/shefa/topology_overview.rs
do_not_cite_seal: true
---

# THE ON-RAMP

> A substrate without an on-ramp is a library, and a library is how a protocol stalls (the 2026-05-18
> scaffolding spec said this first). But this on-ramp carries a heavier charge than ergonomics. The two
> architecture syntheses proved the *values are the superstructure* — agency at the atom, the veil at the
> collective, the two invariants flowing downward non-overridable. **If the on-ramp does not make those
> the path of least resistance, the first thing a developer builds will be a relational-DB care app with
> an admin who governs the household — the exact shape the whole protocol exists to refuse.** This surface
> makes the love-shaped path the *default* path: `create-elohim-app` scaffolds a human-sovereign household
> below and a veil-held collective above, the agency gradient demonstrated end-to-end in the two canonical
> examples it ships. The developer's first `pnpm dev` IS the architecture, running.

---

## 1 — PURPOSE ON THE AGENCY GRADIENT

This is the **on-ramp / meta-surface** — it sits *across* the whole gradient because its job is to make the
gradient itself the developer's mental model from minute one. Concretely it ships three things:

- a **package/crate map + ts-rs codegen boundary** (extending the existing `elohim/sdk/` and `crates/`, no parallel SDK);
- a **scaffolding CLI, `create-elohim-app`** (realizing the blessed `2026-05-18-sdk-scaffolding-cli-spec.md`, escalated from "add a content-type" to "scaffold a whole agency-gradient app");
- **two canonical examples wired together**: `hello-household` (human-sovereign, build FOR the person — observe care, author atoms) and `hello-collective` (veil-holding — those atoms aggregating via `CoverageRollup` into a collective the veil-walker can hold).

**Where it sits and what it must NEVER do (the gradient guard, baked into the templates):**

| Layer the template scaffolds | Gradient role | The guard the scaffold MUST enforce |
|---|---|---|
| `hello-household` (atom + care-ledger) | **human-sovereign — build FOR the person** | The AI/elohim binding scaffolded here is `subordinate: true`, scope = `counsel \| witness \| co-steward`. The template MUST NOT scaffold an `operator`-owned Governor over an individual; `limit_owner` on any household refusal is `{self \| commitment}` only. The person keeps the naming of their own self — no scaffold writes a verdict atom onto a *subject's* chain. |
| The two downward invariants (the middle) | **non-overridable guard** | `create-elohim-app` emits the DIGNITY-FLOOR precedence and the PERSON-KEEPS-THEIR-OWN-NAMING rule as compiled guards (a `RefusalCode::ReservedPlace` check + a `limit_owner: faith` path), NOT as lint comments. A developer cannot scaffold a collective that can override a household floor — the generator refuses to emit it. |
| `hello-collective` (rollup + veil-walker) | **veil-holding — impartial aggregation governance** | The veil-walker scaffolded here governs `aggregation + negotiation` only. The template MUST NOT let it author onto an individual's chain, render a total per-soul account (`CoverageDomain` ranges over commons only — it will not typecheck a per-person scalar), or `Decline` a household (it emits `recognition` Verdicts the node may ignore, never mandates). |

**What the on-ramp itself must never do:** ship a "quickstart" that reaches for `GET /api/v1/thing` + a UUID
primary key + an admin role. Every generator routes through the p2p-native primitives (content-addressed
identity, EPR atoms, Commitment faces, `CoverageRollup`) — the relational-DB default is *unreachable* from
the CLI.

---

## 2 — THE CONCRETE API

### 2a. The package/crate map (extends the existing tree — see §3 for exists-vs-new)

**npm, under the existing `@elohim/*` scope** (current real packages: `@elohim/storage-client`,
`@elohim/epr-ts`, `@elohim/holochain-sdk` — `elohim/sdk/package.json:2`, `epr-ts/package.json`,
`storage-client-ts/package.json:2`):

```
@elohim/epr-ts            (EXISTS) content-addressing codec + generated Coupling/Envelope/EprKind
@elohim/storage-client    (EXISTS) ts-rs-generated wire types + HTTP client (client.ts, sync.ts)
@elohim/holochain-sdk     (EXISTS) ElohimSDK facade (content/relationships/paths/humans services)
@elohim/agency            (NEW, thin)  the gradient primitives in TS: authorCareAtom / CommitmentFace /
                                       CoverageRollup reader / VeilWalker client — re-exports the generated
                                       types and adds the three call-shapes the examples need
create-elohim-app         (NEW)  the scaffolder (npm-init style: `pnpm create elohim-app`)
```

**Rust, under the existing `crates/`** (current real crates: `elohim-sdk`, `doorway-client`,
`elohim-storage-client` — `crates/elohim-sdk/Cargo.toml`):

```
elohim-sdk                (EXISTS) ContentClient / ClientMode / views re-export (crates/elohim-sdk/src/lib.rs:60)
                          → ADD module `agency` (feature = "agency") wrapping the Governor + Commitment-face calls
elohim-compute            (EXISTS, shared) home of trait Governor / ActuationRefusal / RefusalCode
                          (the escalated synthesis's B8/B10) — the examples' Rust side depends on it
```

**The ts-rs boundary stays exactly as gospel** (`elohim/sdk/CLAUDE.md`): Rust views `#[derive(TS)]` →
`cargo test export_bindings` → `storage-client-ts/src/generated/` (camelCase, parsed JSON). `@elohim/agency`
adds **zero new generated types** — it only re-exports `CoverageRollupView`, `CommitmentView`,
`ObservationView` (generated from Rust) and wraps them in call-shapes. snake_case never leaves Rust; the new
package honors the boundary by construction (it imports `from '@elohim/storage-client/generated'`).

### 2b. The headline developer call — `pnpm create elohim-app`

```bash
pnpm create elohim-app my-care-app
# ? What are you building?
#   ❯ household care-ledger   (human-sovereign: observe care, author atoms)        ← hello-household
#     collective governance   (veil-holding: aggregate atoms, hold the commons)    ← hello-collective
#     economic valueflow      (REA events + Commitment faces)
#     full gradient demo      (household + collective wired together)              ← DEFAULT
# ? Local stack? (starts conductor + storage + doorway via hc-dev-orchestrator)  ❯ Yes
cd my-care-app && pnpm dev      # → household UI on :4200, live atoms → rollup the veil-walker holds
```

The generator is the blessed `2026-05-18` spec's pattern (template files + manifest insertion + codegen +
verification) **escalated one level**: instead of `add content-type`, the unit it scaffolds is a whole
gradient-correct app. It composes the *existing* codegen (`schemas/scripts/codegen-ts.mjs`,
`domains/lamad/scripts/codegen.mjs` — spec §"Existing Codegen") and the *existing* local stack
(`hc-dev-orchestrator`, the conductor+storage+doorway trio on :8888/:8090).

### 2c. The human-sovereign call (hello-household) — `@elohim/agency` (TS)

The household observes care and authors an EPR atom carrying its whole why. Grounded in the real coupling
(`elohim/epr/src/coupling.rs:13` — `knowledge|value|governance`) and the real `EprKind::Observation`
(`elohim/epr/src/kind.rs:54`):

```typescript
import { AgencyClient } from '@elohim/agency';
import type { ObservationView, CommitmentView } from '@elohim/storage-client/generated';

const me = new AgencyClient({ doorwayUrl: 'http://localhost:8888' }); // browser/dev mode

// 1. OBSERVE care — a witnessed occasion (subject keeps their own naming: this is MY witness of YOU,
//    a separate EPR with MY signer; nothing is written onto Margaret's chain).
const obs: ObservationView = await me.observeCare({
  subject: 'margaret',                    // who was cared for (a content-addressed agent id, NOT a row id)
  kind: 'care',                           // real observation kind — manifest-declared (verified in tree)
  note: 'sat with her through the afternoon',
});

// 2. AUTHOR the atom — story+value+governance bound into ONE signed CID (alter any leg → hash changes).
//    PROCESS leg (RECURSIVE-ARCHITECTURE §1.1, additive Option<Cid>) cites the observation that triggered it.
const atom = await me.authorCareAtom({
  knowledge: 'i showed up',               // story
  value: { hours: 1.5 },                  // REA quantity
  governance: me.face('provide-care'),    // the Commitment face this fulfills (escalated synth A4)
  process: obs.cid,                        // the why-it-happened (descent lands on a readable account)
});

// 3. The household's elohim is SERVANT only — counsel, never a verdict.
//    The person keeps the naming of their own self; "best self" is a hope held FOR, never a verdict OVER.
const counsel = await me.elohim.witness(atom);  // returns a Verdict (receivable), never a Decline (mandate)
// me.elohim.govern(...)  ← DOES NOT EXIST in a household-scoped AgencyClient. Compile error. The gradient guard.
```

### 2d. The veil-holding call (hello-collective) — `@elohim/agency` (TS) + `CoverageRollup` (Rust)

The atoms aggregate via the keystone primitive. The Rust side extends the *existing* shefa rollup builder,
which today does `rows.len()` / `count(member)` over the `epr_edge` MEMBER_OF graph
(`elohim/elohim-storage/src/graph_views/shefa/topology_overview.rs:89,101`) — **erasing descent**. The
`CoverageRollup` returns a `CoverageSet` carrying the descent pointer and the deficit *inside* the aggregate
(RECURSIVE-ARCHITECTURE §2.1):

```rust
// crates/elohim-sdk/src/agency/coverage_rollup.rs (NEW; first callers = the two shefa builders re-expressed)
pub struct CoverageRollup {
    pub scope_cid: Cid,
    pub domain: CoverageDomain,        // commons only: corpus-bytes | care-floor | donut-ceiling | ...
    pub covered: CoverageSet,          // ∪ of child coverages — a set, NOT rows.len()
    pub required: CoverageSet,         // the layer's share of FULL
    pub deficit: CoverageSet,          // required \ covered — the externality, the DESCENT TARGET
    pub constituents: Vec<Cid>,        // pointers DOWN to child rollups / leaf care-commitments
    pub rollup_hash: Cid,              // BLAKE3 over (scope, domain, covered, sorted constituents)
    pub witness_quorum: u32,           // peers who independently recomputed the same hash = agreement
}
// Category-C: recompute-on-read, ZERO DNA spend, forks nothing. MUST degrade per-row (filter_map + warn!),
// never collect::<Result<>>() — one poisoned scope row must not empty the aggregate (EprRouter lesson).
```

```typescript
// hello-collective (TS): the veil-walker holds the rollup — governs AGGREGATION, never an individual.
import { VeilWalker } from '@elohim/agency';
const veil = new VeilWalker({ collective: 'dowell-household-commons', constitutionCid });

const rollup = await veil.rollup('care-floor');        // reads CoverageRollupView (generated from Rust)
for (const gap of rollup.deficit) {                    // the metric is the DEFICIT — abundance is invisible
  const atom = await veil.descend(gap);                // walk constituents DOWN to the afflicted atom
  await veil.recognize(atom, { kind: 'recognition' }); // emit a Verdict (pass-through context) — NEVER a mandate
}
// veil.govern('margaret')  ← DOES NOT EXIST. CoverageDomain won't typecheck a per-soul scalar. The veil
//                              governs aggregation + negotiation, NEVER individuals. The gradient guard, compiled.
```

The veil-walker is *itself* the `Governor.check()` gate reasoning from an inherited `constitutionCid`
(RECURSIVE-ARCHITECTURE §1.5; the existing `arc_actuator::authorize`/`coverage_admits` spine,
`arc_actuator.rs:110,152`, lifted to `trait Governor`). It descends, recognizes the trap, nudges patiently —
and **always names whose line it honored** (`limit_owner ∈ {self, commitment, operator, faith}`).

---

## 3 — EXISTS vs NEW (bias to extend; mark forks)

| Piece | Status | Evidence / what it is |
|---|---|---|
| `@elohim/epr-ts`, `@elohim/storage-client`, `@elohim/holochain-sdk` | **EXISTS — wrap** | real npm packages, `epr-ts/`, `storage-client-ts/`, `sdk/package.json:2` |
| ts-rs codegen boundary (Rust view → `generated/` camelCase) | **EXISTS — honor unchanged** | `elohim/sdk/CLAUDE.md`; `cargo test export_bindings` |
| `crates/elohim-sdk` `ContentClient`/`ClientMode` facade | **EXISTS — add `agency` module** | `crates/elohim-sdk/src/lib.rs:60,73` |
| EPR coupling (`knowledge\|value\|governance`) + `EprKind::Observation` | **EXISTS — reuse** | `coupling.rs:13`, `kind.rs:54` |
| `Mishpat::Commitment` create surface + `action` discriminator | **EXISTS — reuse** | `mishpat/.../commitments.rs:31` (`create_commitment(action, payload_json)`) |
| `trait Governor` / `ActuationRefusal` / `RefusalCode` (the spine) | **EXISTS as `arc_actuator`; lift pending** | `arc_actuator.rs:77,110,152` — the escalated synth's B8 lift is the prereq |
| shefa rollup builder (today `rows.len()`/`count`) | **EXISTS — re-express as first `CoverageRollup` caller** | `topology_overview.rs:89,101` |
| `hc-dev-orchestrator` local trio (conductor+storage+doorway) | **EXISTS — CLI shells out to it** | `.claude/skills/hc-dev-orchestrator` (:8888/:8090) |
| Prior scaffolding-CLI design | **EXISTS as spec — this realizes + escalates it** | `2026-05-18-sdk-scaffolding-cli-spec.md` |
| `@elohim/agency` (TS call-shapes over generated types) | **NEW — thin, additive** | re-exports generated types; adds `authorCareAtom`/`VeilWalker`/`face()`; zero new generated types |
| `create-elohim-app` (the scaffolder + two examples) | **NEW — additive** | npm-init package; templates compose existing codegen + orchestrator |
| `elohim-sdk::agency` Rust module (Governor + Commitment-face calls) | **NEW — thin** | feature-gated; depends on `elohim-compute` |
| `CouplingLeg::Process` (4th leg the PROCESS atom needs) | **NEW additive wire field — NOT a fork** | `#[serde(default)] Option<Cid>`; old atoms decode `None` (RECURSIVE-ARCHITECTURE §2.3) |
| **`CoverageRollup`** (the keystone) | **NEW — Category-C, the load-bearing novelty** | recompute-on-read, **zero DNA entry-type spend, forks nothing** |
| `provide-care` Commitment action | **NEW additive action discriminator — NOT a fork** | `signal_kind`/action extension; Mishpat entry budget untouched (escalated synth A4) |
| Typed care/compute DNA partition; planetary precedence wall | **GENUINE FORK — not on the on-ramp** | DNA-hash change, operator-blessed, reinstall-sequenced (both syntheses). The on-ramp scaffolds the *social-class* action, never the DNA fork. |

**DNA entry types spent by this whole surface: ZERO.** Everything additive is a TS package, a thin Rust
module, a Category-C view, an additive wire field, or an additive action discriminator. The one genuine
fork (typed care/compute partition) is deliberately **not reachable from the scaffolder** — the on-ramp
ships the buildable-now path only.

---

## 4 — THE MINIMAL BUILDABLE SLICE

**The smallest version that lets a developer do one real thing today:**

`pnpm create elohim-app demo --template=hello-household --no-stack` scaffolds a single-page app that calls
`AgencyClient.observeCare()` + `authorCareAtom()` against an *already-running* doorway (alpha or local), and
renders the atom's three legs. This needs **only** the thin `@elohim/agency` wrapper over the *existing*
`@elohim/storage-client` + an existing observation-create route — **no Governor lift, no CoverageRollup, no
DNA work**. It proves the human-sovereign half end-to-end and is shippable against the substrate that runs
today (the `care` observation kind already exists in-tree).

**Slice ladder (each lands green, additive, reversible):**
1. `@elohim/agency` TS wrapper + `observeCare`/`authorCareAtom` → `hello-household` renders an atom. *(buildable now, no DNA, no Governor)*
2. `create-elohim-app` with the two non-gradient templates (household, valueflow). *(templating over existing codegen)*
3. Lift `trait Governor` (escalated synth B8) + `elohim-sdk::agency` → household `elohim.witness()` returns a Verdict. *(prereq for the guard)*
4. `CoverageRollup` re-expressing the shefa builder → `hello-collective` rollup with `deficit`+`constituents`. *(the keystone; Category-C)*
5. `create-elohim-app --template=full-gradient` wires household atoms → collective rollup → veil-walker `descend`/`recognize`. *(the gradient, end-to-end)*

**The first example-app fragment it enables (the full-gradient demo's wiring, the payoff):**

```typescript
// my-care-app/src/gradient.ts — scaffolded by `create elohim-app --template=full-gradient`
import { AgencyClient, VeilWalker } from '@elohim/agency';

// BELOW the gradient: the person, sovereign. Build FOR her.
const household = new AgencyClient({ doorwayUrl: 'http://localhost:8888' });
const atom = await household.authorCareAtom({
  knowledge: 'sat with margaret', value: { hours: 1.5 },
  governance: household.face('provide-care'),
  process: (await household.observeCare({ subject: 'margaret', kind: 'care' })).cid,
});

// ABOVE the gradient: the veil, impartial. Holds aggregation, never the person.
const veil = new VeilWalker({ collective: 'dowell-commons', constitutionCid });
const rollup = await veil.rollup('care-floor');     // margaret's atom is now a constituent of the commons
for (const gap of rollup.deficit) {                 // sees only the DEFICIT — the household the commons failed
  await veil.recognize(await veil.descend(gap));    // a patient Verdict down the constituents — never a command
}
// Two clients, one gradient. household.govern() and veil.govern('margaret') BOTH fail to compile.
// Agency at the root, the veil at the collective — by default, because the scaffold gives you nothing else.
```

---

## 5 — WHAT LOVE REQUIRES AT THIS SURFACE

The on-ramp is where love is decided *before the developer has an opinion* — because the first scaffold sets
the shape of everything they build on top. Love requires the on-ramp itself be love-shaped, so that what
gets built is love-shaped by default:

- **The person keeps their naming.** `observeCare()` authors MY witness of YOU as a separate EPR with my
  signer; the scaffold has no call that writes a verdict onto a subject's chain, and the household elohim
  exposes `witness()` (a hope held FOR) but never `govern()` (a verdict OVER). "Best self" is a Verdict the
  person may receive, never a Decline imposed.

- **The binding is honest.** Every refusal the scaffolded Governor emits names whose line it honored
  (`limit_owner ∈ {self, commitment, operator, faith}`). A household template can only emit
  `{self, commitment}` — an operator-veto smell cannot be scaffolded into a person's lever. The cage is
  named cage, the offer named offer, at the layer where it lives.

- **The veil governs aggregation, never individuals.** `VeilWalker.govern('margaret')` does not exist and
  `CoverageDomain` will not typecheck a per-soul scalar — the total account of a person is *unrepresentable*
  in the surface, not merely discouraged. The veil sees only the `deficit` (the household the commons
  failed); abundance is invisible, so the surface cannot become a leaderboard. The witness is weighted
  toward the least powerful *by the shape of the API*.

- **Patience over engagement.** The veil-walker's only downward act is `recognize()` — a Verdict the node may
  ignore — and there is no engagement counter anywhere in the scaffold to optimize. The on-ramp ships a
  patience machine, not a growth funnel.

> **What love requires, in one line:** that the very first command a developer runs to rebuild the world
> hand them a household that builds FOR the person and a veil that holds the commons open — and *refuse to
> compile* the world where the AI governs the individual or the powerful become invisible — so that love is
> the path of least resistance and the only path the on-ramp knows how to give.
