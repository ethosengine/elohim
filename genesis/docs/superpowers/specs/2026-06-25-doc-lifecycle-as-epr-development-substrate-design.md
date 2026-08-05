---
title: "Doc-Lifecycle as EPR — The Development Substrate as a Protocol Peer"
id: doc-lifecycle-as-epr-development-substrate
tier: spec
status: Draft
created: 2026-06-25
maintainers: Matthew Dowell + Opus 4.8
class: process-meta
process_subdomain: doc-lifecycle
topic: [epr, content-addressing, doc-lifecycle, comet, decompose, definition-of-done, attention, stewardship, liability, el-roi, epr-meta, recursion-guard, devspace-peer, seed-projection]
# --- born under the law it proposes (dogfood: the new required-at-birth fields) ---
context-tier: disclosed
steward: cartographer
graduation-trigger: decompose-complete OR superseded-by-implementation
cites:
  - placement | Genesis Docs Placement Contract | sha256:95be31e6724bb9f5 | path: genesis/docs/PLACEMENT.md
  - spec-plan-compaction-loop-design | Spec/Plan Compaction Loop | sha256:5f9d3f0baabfe199 | path: genesis/docs/superpowers/specs/2026-06-02-spec-plan-compaction-loop-design.md
  - elohim-seam-map-concern-routing | The Elohim Seam Map | sha256:54b5809fb8e688d1 | path: genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md
  - .claude/skills/epr-content-addressing/SKILL.md
  - dht-pii-leak-remediation-plan | DHT PII Leak Remediation | sha256:1972892ab3363c4e | path: genesis/docs/superpowers/plans/2026-06-23-dht-pii-leak-remediation-plan.md
  - genesis/seeder/src/seed-epr-atom.ts
---

# Doc-Lifecycle as EPR — The Development Substrate as a Protocol Peer

> **One-line thesis.** The Elohim *development* substrate (docs, specs, plans, memory, the agent
> fleet) is not merely *like* the Elohim Protocol — it is the protocol at a different clock-rate.
> The redesign's job is to stop *simulating* it and *run it*: content-addressed artifacts,
> projection-not-truth storage, reach earned-not-asserted, and a witnessed, layer-by-layer
> definition of done. The single new artifact is **`.epr-meta`** — a directory-local EPR atom that
> is simultaneously the governance gate, the directory's manifest, and its seed.

## 0. Status, scope, and what this defers

This is the **framing spec** for an arc discovered through an extended brainstorm and hardened by a
six-lens adversarial panel (rust / tauri / historian / red-team / cartographer / storyteller). It
names the thesis, the one new primitive (`.epr-meta`), and decomposes the work into sub-projects
(§12), each of which earns its own spec→plan cycle. It is deliberately **not** an implementation
plan.

**Explicitly deferred** (named here so they are not silently in-scope): the Holochain DNA
rollback/upgrade strategy; durable per-developer signing keys; dev-runtimes-as-CID-facts *at scale*;
the full inter-peer recognition economy; any shared/production DHT participation. v1 lives entirely
on a disposable, isolated local plane (§4).

## 1. The thesis: the dev substrate *is* the protocol

The isomorphism is structural, not metaphorical. Each row is a mechanism that already exists on both
planes or trivially can:

| Protocol (EPR-REA) | Development substrate | Shared mechanism |
|---|---|---|
| CID / fingerprint — content-addressed identity | `[[slug]]` / `cites:` graph; `.epr-meta` CID | content addressing; a move never breaks a link |
| DHT notary / iroh+libp2p dataplane / doorway projection | gospel-resident / disclosed-on-invoke / mempalace-retrievable | three-tier truth: scarce-resident vs cheap-dataplane vs projection |
| reach earned at compose, never asserted (`epr_compose.rs`) | done earned by witness, never claimed (`checked ≠ verified ≠ done`) | a gate that returns Allowed/Blocked/Pending |
| three legs coupled — knowledge + value + governance | no doc born without manifest + steward + governance | `.epr-meta`'s three legs (§2) |
| supersedence graph (sealed `supersedes`) | graduation = supersedence (`distills:` / `compacted_from:`) | acyclic predecessor edges |
| verified → trust-once, distribute-many | `VERIFIED-STABLE` earned once, then fast-paths | verification as efficiency |
| schema-first is inversion of control | authored intent → derived state | one home per fact; generate the rest |

**The external-research throughline** (independently corroborated): the entire redesign is *one move
applied repeatedly* — **separate authored intent (durable, hand-tended, source-of-truth) from
derived state (generated, never hand-edited)**. The repo's own `schema → codegen` rule is proof the
team already runs this pattern; the task is generalizing it from *types* to the *whole lifecycle*.

## 2. The unified artifact: `.epr-meta` as an EPR atom

`.epr-meta` is a single, content-addressed, directory-local artifact that does three jobs at once —
which is why it can be *one* file rather than three competing surfaces (it subsumes the central
`PLACEMENT.md` contract, the `_lib/managed_surfaces.py` registry, and a would-be frontmatter-DENY
hook):

- **Knowledge leg** — the directory's manifest: what this directory *is*, what kinds of artifact
  belong here, and where a misplaced artifact *routes* (`route-to`).
- **Governance leg** — the **compose-gate** for this directory (the `.gitignore`-style cascading
  rules from the brainstorm): required-at-birth frontmatter, allowed types, `no-new-subdirs`,
  `dedupe-of`, `max-files`. This is the directory's local analog of `epr_compose.rs`'s reach gate.
- **App-manifest leg** — the *manifest of which guards apply and how they execute*: the directory
  declares (by rule name) which checks run and references any non-trivial validator **by CID** — the
  governing `.py` is *itself an EPR atom* that defines its own execution. (Seam-map's *manifest → SDK
  seam*: "compose inward, integrity by construction.")

**One engine, not per-directory code.** The `.epr-meta` carries *declarations*, never bespoke
guard implementations: there is exactly one engine — the generic resolver (§9) — which reads the
governance-leg rules and invokes the app-manifest-leg's referenced validator-EPRs. A directory ships
a *manifest* (rules + validator-EPR references), not its own executable guard. The
governance leg is *the policy*; the app-manifest leg is *which validators realize it*; the resolver
is *the only thing that runs*.

And `.epr-meta` *is the seed* (§4): it is the regenerable source from which the directory's atoms are
re-projected into the local notary on every workspace startup.

**Gate vs projection — say it once.** The **resolver** is the gate (the engine that *decides*); the
`.epr-meta` is **authored intent** (the rules + manifest, in git); the DHT atom is the **projection**
(the reflection of what the rules earned). Where this spec says "`.epr-meta` is the compose-gate," it
is shorthand for "the directory's rules that the resolver gates by." The `.epr-meta` never *grants*
authority itself — that would be the projection-as-truth inversion the seam-map §7 rejects.

### 2.1 Fractal self-governance

Because a `.epr-meta` references other `.epr-meta` atoms (ancestor + shared governance) and
validator-EPRs **by CID**, the governance system is **fractal and reflexive**: the rules that govern
are themselves EPRs governed by the same machinery, all the way to a root. This is the data
substrate that "fractal stewards" previously lacked (a single-hop gap). It also resolves the earlier
"rule expressiveness" fork: the named-validator escape is not a special case — it is just another
EPR.

### 2.2 Self-hardening

Per the chosen authority model: an agent may author `INJECT`/`MEASURE`/`ASK` rules **autonomously**;
a new `DENY` (hard block) requires operator approval. Wrong rules **self-surface** — a rule that
fires-then-gets-overridden N times raises a `bad-rule` drift signal for review. The agent that
cleans up a document-dump writes the `.epr-meta` rule that stops the next one. In the
`flag → agent → canon → stasis` pattern the repo already runs, the *canon* becomes a **local
executable rule** instead of a backlog entry — discipline that compounds as the system is worked in.

## 3. The recursion guard (the one hard primitive)

A fractal, self-referential governance graph must be guaranteed to terminate. Three recursion
surfaces, three terminations:

1. **Immutable CID graph — acyclic by construction.** To reference B from A, A embeds B's CID, which
   requires B sealed first; a cycle is therefore impossible (the chicken-and-egg cannot seal). The
   `.epr-meta` → `.epr-meta` → validator-EPR reference graph is a **Merkle DAG**, exactly like
   git/IPLD. Free, and the deepest guarantee.
2. **Cascade walk — `root: true` base case + depth fuel.** Walking *up* the ancestor `.epr-meta`
   chain terminates at the repo-root constitutional atom (held by the human commons-steward), with a
   `max-depth` bound as belt-and-suspenders.
3. **Guard execution — non-reentrant + visited-set + fuel.** A guard's write must not re-enter the
   resolver: guard-generated writes carry a skip-marker, or guards are pure-evaluation; validator-EPR
   call-chains spend a fuel budget; *mutable*-pointer resolution (`@latest`/supersedence — the only
   place cycles can reappear) uses a visited-CID memo. This is the matured form of the existing
   `cooldown-silenced`/`once-per-session` hook pattern.

## 4. Truth and projection: git is truth, the DHT is a disposable notary

The keystone decision that dissolves the Holochain DNA-upgrade-wipes-the-database constraint:
**the DHT is not the store.** Three jobs, only one durable:

- **Git = durable provenance + shared substrate.** Author, history, bytes. Already how the team
  collaborates.
- **CID = integrity + atom-identity + mutual authority.** Same bytes → same CID → *literally the same
  atom*. Keyless, tamper-evident.
- **DHT = each peer's disposable local notary**, re-seeded fresh on every workspace startup. A DNA
  change that wipes it is a **non-event** — re-seed on next boot; nothing is lost because the DHT
  never held anything `.epr-meta` (in git) didn't.

**Mutual authority across developers, without durable keys or a shared live DHT:** I commit my
`.epr-meta`; you pull and re-seed; your local notary re-derives the same CID; it is verified-
authoritative in your DHT — and symmetrically. The shared authority substrate is **git + content
addressing**; the DHT is the local verification layer that makes "the notary over what we're already
doing" true on each box.

**Precondition (the one real wrinkle):** there is a known scar where bulk seed does **not** actually
anchor to the DHT, so provenance-gated reads 404 (`project_local_stack_dht_anchor_gap`). "Re-seed
fresh on startup" only delivers the notary guarantee if the import genuinely **anchors** — the same
gap as the panel's "close the pubkey-verification step" (`seed-epr-atom.ts` today accepts any
64-byte signature over any 32-byte key structurally). **Verify the anchor path before any surface
claims authority.** Two distinct properties ride here and must not be conflated: **authority comes
from content-addressing** (same bytes → same CID → same atom; needs no keys — which is *why* durable
keys are deferrable), while **anti-forgery comes from signatures** (guardrail-2). The "DNA-wipe is a
non-event" headline holds *only once anchoring is proven* — so P1 treats the anchor as
**research-first**, and the thin slice (§13) *is* that proof.

## 5. Progressive disclosure tiering

`context-tier: always | disclosed | retrievable` — the notary/dataplane/projection split applied to
attention. The test is *what does its absence corrupt?*

- **always-resident** — only what corrupts *unrelated* work when absent: the orientation map, the
  non-negotiable gotchas, and an *index of pointers*. Kept lean because it taxes *every* turn
  (context-rot is a correctness constraint, not an aesthetic). Today this tier is ~100 surfaces, 80
  drifting — already over-large.
- **disclosed-on-invocation** — a spec body loaded only when its sprint is hot (or its env is
  available); the skill body loaded only on invoke.
- **retrievable-only** — concluded specs in MemPalace; the `history/` museum (resident *as an index*,
  retrievable *as bodies*); raw bodies in git.

## 6. The progressive Definition of Done (the witnessed reach-chain)

The vision DoD ("two humans can establish identity, learn, sync…") is a **reach claim**: true only
when **earned bottom-up by witnesses**, never asserted top-down — exactly as an EPR link earns its
reach. Each rung's "done" is a runnable predicate notarized by that rung's witness, and the
`realizes:`/`cites:` edges are the content-addressed chain that lets it roll up *and* be audited back
down:

| Layer | Done predicate | Witness | Up-link |
|---|---|---|---|
| code | unit/clippy/lint pass | machine | cites scenario |
| scenario (a2o) | `.feature` passes | a2o | `@cites` plan-gap *(already live)* |
| plan | gaps closed + NFR criteria | ledger | gap-IDs → spec |
| spec | scenarios roll up + cross-pillar coherence | cartographer | `cites:` epic |
| epic | specs compose | composition | `cites:` manifesto |
| manifesto | each principle → ≥1 epic + "served its why?" | **storyteller (human meaning)** | backward: nothing unimplemented |

**Rollup arithmetic:** `spec_done_fraction = |gaps CLAIMED-and-CI-green| / |gaps total|`;
`epic_readout = mean over realizing specs`. Surfaced as a `dod:` gate-line; ratcheted at push via the
already-built-but-unwired `context-ratchet.py`.

**The honest seam (load-bearing):** the arithmetic tops out at **"mechanism built."** Two hard stops:
(1) **CLAIMED ≠ CI-green** — a plan never grades its own homework. (2) **The manifesto layer is not
scenario-gradable** — whether a landed feature *served its why* is a storyteller meaning-judgment,
the **El Roi** seeing (§8), not a fraction. The number says *built*; only the meaning-witness says
*delivered the vision*.

## 7. The attention economy — corrected to conservation, never standing

The adversarial panel caught a leak in the original "attention earns recognition" idea: it would
rebuild a **named, deliberately-forbidden anti-pattern** (`2026-03-14-steward-affinity-lifecycle-
design.md`: *"Learner engagement (attention) does NOT increase steward affinity. That would recreate
the attention economy."*). The protocol exists in part to *refuse* attention→standing.

**The fix splits the value leg.** Attention-on-load is a legitimate **conservation signal** — a doc
costs context on every resident turn; is it load-bearing? if not, graduate it. Measured at the
existing emission points (`pre-tool-memory.py`, `pickup-semantic-surfacing.py`, workflow token
telemetry). But it must **never** become a **standing/value** signal: it must not bank value, must
never route to the agent, and must never inflate governance. Conservation: yes. Recognition-as-power:
forbidden. (The two halves stay clean because liability tracks *affinity*, not attention — §8.)

## 8. Liability and recognition: proportional affinity, commons catastrophe, El-Roi legibility

Power and responsibility are coupled — **proportionally, among humans** — and the agent's role is to
**see**:

- **Proportional liability.** A participant is accountable in proportion to the **affinity/responsibility
  earned directly from that EPR**. Affinity is earned by real stewardship — and the protocol already
  forbids attention from inflating it (§7), so liability-by-earned-affinity is *automatically* clear
  of the attention-economy anti-pattern.
- **Catastrophe → commons.** Tail/catastrophic loss is socialized to the commons (the risk-pool /
  insurer of last resort), so ruinous liability never falls on an individual — restorative, not
  punitive, and consistent with "consequence falls on participation, never on bodies." Has a home:
  the resilience/insurance work (ROADMAP Sprint 13).
- **Ongoing restitution → individual, made legible by the elohim.** The agent does not *bear*
  principal liability (it is not a principal) and does not merely *execute* it — it **sees**: it makes
  the web of interdependence and proportional consequence legible so the human can "wrestle with
  grace, for no one is guiltless." This is **El Roi** — the justice-that-sees of
  `feedback-justice-mishpat-not-punishment-guard`: impartiality from incorruptible witnessed sight;
  justice as *restored capability*, not punishment.

**The recognition/liability primitive is asymmetric**, and it is *not* a new entry type — it is the
recognition-mirror of `Mishpat::Commitment` accept/revoke. The verb lives in the Commitment's
existing `action: String` field (e.g. `accepts-credit` / `declines-liability`), **not** a new
`signal_kind` (that token belongs to the separate P2P `FeedbackSignal`/`SignalKind` mechanism in
`feedback_signal.rs`, which is what the *accruing* feedback signals ride; the two must not be
conflated). The agent is a full actor on **credit** (it is the pen/steward) but an
authorized-executor-only on **liability**, which must `bounded_by` a prior human-signed Commitment,
**fail-closed**. Recognition reach is **private** (`Private`/`Intimate` on the `Reach` ladder —
`elohim/epr/src/reach.rs`; note the variant is `Private`, not "Personal") — a steward reviews its own
credited atoms; no public leaderboard.

> **This section is the protocol-level contribution** (it feeds the *product*, not just the dev
> loop): bilateral, revocable, consent-based recognition with proportional liability, commons-borne
> catastrophe, and the elohim as the El-Roi seer. It should **graduate to a protocol architecture
> note** in `genesis/docs/content/elohim-protocol/` (a follow-on artifact, §12).

## 9. The enforcement surface

The discipline is enforced at the right rung of the ladder — *irreversible/cross-cutting → DENY ·
correctable-in-place → INJECT · bounded-fixable-elsewhere → DISPATCH · trend → MEASURE*:

- **`.epr-meta` resolver** (PreToolUse, generic, tiny): walks the cascade, merges rules, returns
  deny/ask/inject. Most guardrails become `.epr-meta` *rules* (frontmatter-at-birth DENY, no-dumps,
  no-orphan-tree, dedupe-location, p2p-route-intercept) rather than separate hardcoded hooks — the
  hook stops being where the rules live.
- **Global sentinels** (few, central): decompose-on-retire **DISPATCH** (PostToolUse, promote the
  existing `placement-drift-signal.py`, clone `deprecation-sentinel.py`); the DoD **ratchet** (pre-
  push GATE, wire `context-ratchet.py`); the pile-bloat corpus rollup (SessionStart MEASURE, `pile:`
  gate-line, `BLOATED` above ~2× canonical).

## 10. Identity: workspace = peer, human = presence, Claude = servant-elohim

Three dignities, three registries, an edge-set — **never a fused record, never agent value-accrual**:

- **Workspace = peer / `NodeShapeView`** via `boot_registration.rs` (a claimable-by-no-one node-fact;
  device facts default-private under the PII-leak remediation plan — peers, **not** presences).
- **Human = `imago-dei` presence** (already exists as content-addressed presences in
  `genesis/data/presences/`).
- **Claude = servant-elohim** — Psalm-82 servant-counsel-co-steward acting under `delegates-compute`:
  delegated, bounded, witnessed, revocable. It authors/routes/sees at machine speed; it **banks
  nothing, holds no admin socket as ambient authority, claims no sovereignty.** Its legitimate power
  is the **power of sight** (§8). **Drop "nascent"** as a maturation term — the *commons* is nascent;
  the agent is its servant. (The moment a doc calls the agent's tier "autonomous" or "self-standing,"
  the identity-sovereignty guard fires.)

## 11. Non-negotiable guardrails (hard lines, from the panel)

1. **No shared DHT.** Distinct dev `network_seed` → separate DNA hash → physically isolated DHT,
   unable to reach alpha. (The disposable-per-workspace DHT of §4 makes this automatic; keep the
   dedicated seed as belt-and-suspenders.) Graduation *crosses* seeds; never a co-resident write.
2. **Close the pubkey-verification gap** before any graduation surface ships (precondition, §4).
3. **The agent never holds the conductor admin socket as ambient authority** — capability-scoped
   per-graduation grants routed through `qahal-authority`; never expose `IssueAppAuthenticationToken`
   to the agent loop.
4. **The agent banks nothing.** Hard-zero any recognition balance for the agent; reach `Pending`
   until earned; no agent-bypass on the compose gate.
5. **Liability terminates in a human-signed Commitment, fail-closed** (§8).
6. **Attention never inflates governance standing** (§7) — the named anti-pattern.
7. **Dev-runtimes are peers, not presences**; device facts default-private/consent-gated, never
   public DHT, never recognition-accruing.
8. **"Claude is the elohim" in the Psalm-82 servant sense only** (§10).
9. **Link `elohim-epr`; never re-implement `canonical_bytes`** (a third hand-mirror guarantees
   eventual CID divergence).
10. **Default-on only behind guardrails 1+3.** Until then: opt-in via devfile `command:`,
    conductor-off, single-tenant.

## 12. Decomposition into sub-projects

Each earns its own spec→plan. Sequence: P0 → P1 → (P2 ∥ P3) → P4 → P5; P6 follows P1 (a consumer of
the substrate, parallelizable once the gate exists); the protocol-insight note and the thin slice (§13)
run alongside.

- **P0 — Roadmap re-home** *(concrete, first).* Retire root `ROADMAP.md` → a lean authored-intent
  layer that `cites:` the generated `genesis/data/timeline/roadmap/`; harvest M1–M6 DoD as the top
  rung of the §6 chain.
- **P1 — The `.epr-meta` compose-gate** *(centerpiece; stops new drift while we clean up).* The
  resolver + the declarative-vocabulary + named-validator-EPR escape + the recursion guard (§3) +
  seed `.epr-meta` files at key directories (root `root: true`, `superpowers/{specs,plans}`).
- **P2 — The docs-comet (conservation).** Mostly *wire the already-written*
  `2026-06-02-spec-plan-compaction-loop` spec: three-zone stasis ratios (band ≤2×), broaden the
  graduation trigger to `UNION(terminal, decompose-complete, age)`, apply the six PLACEMENT.md edits proposed in the
  compaction-loop spec's §10.4, hybrid graduation (**plans sweep, specs meaning-gate**).
- **P3 — The progressive-DoD rollup** *(the one genuinely-new construction).* §6, wire
  `context-ratchet.py`.
- **P4 — Disclosure tiering + MemPalace as graduation destination.** `context-tier` frontmatter;
  embed concluded specs retrievable-only.
- **P5 — Attention-conservation + the liability/recognition primitive.** §7–§8; the `Mishpat::
  Commitment` `signal_kind`.
- **P6 — `.ci-ignore` convergence into `.epr-meta`** *(post-P1; folds the pipeline's ignore concern
  into the same atom).* Today CI-ignore is a flat repo-root `.ci-ignore` (gitignore-style: subtree
  prefixes `.claude/`·`.github/`·`.husky/`, basename-anywhere `CLAUDE.md`·`AGENTS.md`·`GEMINI.md`,
  exact paths `ROADMAP.md`·orchestrator `Jenkinsfile`/`build-graph.groovy`), parsed by
  `genesis/orchestrator/ci-ignore.mjs` (`parseCiIgnore`/`matchesCiIgnore`/`filterChanged`) and
  consumed at the `graph-walker.mjs` CLI boundary + `.husky/pre-push` + the Groovy Jenkinsfile
  (which reimplements the parser — drift caught by tests). Converge it onto the cascade: **"changes
  here never trigger source pipelines" becomes a co-located declaration in each subtree's own
  `.epr-meta`** (the same decentralization win as PLACEMENT→`.epr-meta`) — subtree ignores map to
  that directory's `.epr-meta`; cross-cutting basename-anywhere/exact-path ignores live in the root
  `.epr-meta` or a referenced **`ci-ignore` EPR** ("those `.epr-meta` point to whatever EPR defines
  the ignore behavior"). Mechanically it extends the **app-manifest leg** with a `ci-trigger:`
  declaration (how external pipelines treat this directory — a build-time signal, orthogonal to the
  author-time `deny`/`ask`/`inject` classes). **Pipeline update, authored-intent→derived-state:** the
  JS/Groovy consumers keep reading a *flat pattern list*, now **codegen'd from the `.epr-meta`
  cascade** (single source of truth shifts root-file → cascade; no new JS cascade-walker, no drift) —
  or a thin JS `.epr-meta` reader mirrors the Python resolver (the two-readers/one-source-form model).
  Needs P1 (the resolver) landed; the content-addressed `ci-ignore` EPR wants P1b (the projector).
- **Protocol-insight note** *(separate home, product-facing).* §8 → `genesis/docs/content/elohim-
  protocol/architecture/`.

## 13. The thin first slice (proves the most with the least, reversible)

> On a **personal laptop** (durable key, no PVC), with a **dedicated dev `network_seed`**, run
> **conductor-free `elohim-storage`**; write a one-shot **`elohim-fs-projector`** that compiles
> **one** `.epr-meta` file via `elohim-epr` into a **`Private`-reach** atom and PUTs it to the local
> runtime; then run the generalized seeder to **graduate that one atom into a second local notary
> peer — and watch the CID be identical on both sides.**

Proves the load-bearing isomorphism (content-addressed identity; graduation = same atom) while
touching nothing in production, accruing no standing, and deferring every contested piece (Che PVC,
shared DHT, symmetric liability, dev-runtimes-as-facts at scale, the recognition economy). Delete the
row and it is gone.

## 14. Open questions / risks (for review)

- The anchor-gap verification (§4) — is the *atom* seed path anchoring where the *bulk* path is not?
- `.epr-meta` as projection vs gate (§2): it is a **projection of earned reach**, never the
  authority itself — guard against the projection-as-truth inversion the seam-map §7 rejects.
- Read-vs-write naming (the bridge fork): read-direction (protocol → devspace) is bridge-shaped;
  write-direction (`.epr-meta` → atom) is SDK-author-shaped and notary-gated. Name them as two
  things, not one crate.
- Per-host-type tracking: laptop = T1+T2 peer; Che = T3 projection-spoke (the DHT being disposable
  makes Che ephemerality moot for v1 regardless).

## Related

- `genesis/docs/PLACEMENT.md` · `genesis/docs/superpowers/specs/2026-06-02-spec-plan-compaction-loop-design.md`
- `.claude/skills/epr-content-addressing/SKILL.md` · `genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md`
- `elohim/elohim-storage/src/main.rs` (conductor-*optional*: the conductor runs only when `admin_url` is `Some`; a true conductor-free mode is to-be-built) · `src/services/epr_compose.rs` · `src/services/boot_registration.rs`
- `elohim/epr/src/envelope.rs` (`canonical_bytes`) · `elohim/epr/src/cid.rs` · `genesis/seeder/src/seed-epr-atom.ts`
- `genesis/plans/2026-03-14-steward-affinity-lifecycle-design.md` · `genesis/docs/superpowers/plans/2026-06-23-dht-pii-leak-remediation-plan.md`
