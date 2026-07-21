---
title: "Sealed Contract Edges — Conformance Seals on the Flow Graph, Governor Delegation, and the File-Leave Frontier"
id: sealed-contract-edges-governor-frontier
tier: spec
status: Draft
created: 2026-07-21
maintainers: Matthew Dowell + Claude Fable 5
class: process-meta
process_subdomain: doc-lifecycle
topic: [sealed-edges, contract-edges, governor, enforcement-class, frontier, dirty-set, file-leave, cite-seal, cidv1, eprfs, epr-meta, flow-walk, reach, attestation, precedent, cascade-consistency, triage, stasis]
context-tier: disclosed
steward: cartographer
graduation-trigger: decompose-complete OR edge-seal-tooling-landed-with-genesis-stage-edges-drained
refines:
  - genesis/docs/superpowers/specs/2026-07-18-epr-rea-valueflow-fabric-design.md
  - genesis/docs/superpowers/specs/2026-06-02-semantic-computable-links-design.md
cites:
  - epr-rea-valueflow-fabric | the spec this REFINES — supplies EdgeSpec/walk/Frontier, edges-in-hashed-bytes, the three-plane floor/ceiling table and fractal scope this promotes to sealed contract edges | sha256:1cec32527dbff6d7 | path: genesis/docs/superpowers/specs/2026-07-18-epr-rea-valueflow-fabric-design.md
  - semantic-computable-links-design | the spec this REFINES — the cite envelope, seal/propagate/refresh tooling, tool-managed status:, and FRONT-soft/BACK-hard enforcement generalized here beyond the doc graph | sha256:1460bc102580ab0d | path: genesis/docs/superpowers/specs/2026-06-02-semantic-computable-links-design.md
  - cite-fingerprint-cid-convergence | one digest two renderings — licenses machine-facing sidecar edges to seal full CIDv1 (bafkrei) while doc envelopes stay short-form; eprfs cid stays the single encoder | sha256:0a657c9c1b0c43e7 | path: genesis/docs/superpowers/specs/2026-07-12-cite-fingerprint-cid-convergence-design.md
  - epr-meta-policy-registry-measure | define-once-bind-many + the content/standing two-plane split that governor policies reuse verbatim and that translates byte-level force to social standing | sha256:474eee1686e3123b | path: genesis/docs/superpowers/specs/2026-07-02-epr-meta-policy-registry-measure-design.md
  - epr-meta-compose-gate | the P1 cascade + lightest-signal enforcement-class ladder the governor ontology and the seal-only-the-governance-gap rule descend from | sha256:6052ce071bfec509 | path: genesis/docs/superpowers/specs/2026-06-25-epr-meta-compose-gate-design.md
  - findings-sentinel-pattern-design | the flag→agent→canon→stasis shape the dirty-set ledger (.claude/data/edge-findings.jsonl) and its escalation dispatch instantiate | sha256:c284074fe38e2450 | path: genesis/docs/superpowers/specs/2026-06-06-findings-sentinel-pattern-design.md
  - elohim/lvi/docs/specs/2026-07-20-elohim-native-devspace-design.md
  - elohim/epr-rea/src/walk.rs
  - .claude/epr-meta/recipes.yaml
derived_from:
  - genesis/docs/content/elohim-protocol/architecture/2026-05-24-records-lifecycle-design.md
---

# Sealed Contract Edges

> **One-line:** promote the flow graph's edges from *reference* to *sealed contract edge* — every
> dependency an artifact takes carries either a **conformance seal** (upstream CIDv1 at the moment
> the downstream last conformed) or a **governor citation** (the stronger mechanism — compiler,
> codegen, schema contract, test — that already enforces it); upstream drift makes sealed edges
> stale *by construction*, the forward walk turns the stale set into a deterministic work-list, and
> a file-leave trigger surfaces the frontier to the agent **in flight** — reseal, declare deviation,
> or escalate. Query + obligation = the Spring-style cascade, without a totalizing compiler.

## 1. The gap this closes (and what it composes from)

The walk exists (`epr-rea` `FlowWalk::walk_forward` → `Frontier{dependents, outputs, unfulfilled}`;
`epr-cli flow`; the dev-pipeline recipe in `.claude/epr-meta/recipes.yaml`). It answers *"what
depends on this?"* on demand. Nothing converts a change into an **obligation** on that dependency
set — the three properties a compiler's cascade has and the walk lacks:

1. **Change-coupled checking** — conformance is re-checked *because* the upstream changed,
   in the same gate, not when someone thinks to run the walk.
2. **Enumerable breakage** — "dependent" ≠ "now-broken"; there is no per-edge notion of *stale*.
3. **Zero-residue enforcement** — a stale dependent today just stays unstable forever without
   being red (the perpetually-unstable genesis stages are exactly this class).

The missing primitive already exists, applied only to docs: the **cite-seal** (`cites:` envelope —
slug + desc + fingerprint + tool-managed status/path). This spec generalizes it to *every*
dependency edge on the flow graph, adds the enforcement-class ontology that keeps it from
duplicating stronger systems, and wires the emission-time trigger. It composes (never forks) from:

- **epr-rea-valueflow-fabric** (REFINED): `EdgeSpec{validators, meaningful}`, the walk, the
  three-plane floor/ceiling table, edges-in-hashed-bytes (§2.3), fractal scope (§2.5).
- **semantic-computable-links** (REFINED): the envelope, cite-gen/propagate/refresh, the
  tool-managed `status:` field, HELD ≠ DEAD, FRONT-soft/BACK-hard enforcement.
- **cite-fingerprint-cid-convergence**: one sha2-256 digest, two renderings — human envelopes stay
  `sha256:hex16`; machine-facing surfaces seal **full CIDv1 raw (`bafkrei…`)**. Sidecar edge
  records are machine-facing: they seal full CIDs (the operator's "cidv1 citations on that
  dependency"). Encoder single-sourced in `eprfs cid`; Python decodes, never encodes.
- **epr-meta-policy-registry**: define-once-bind-many; the two-plane split (policy content = EPR;
  force = standing) that makes byte-level governance translate upward (§6).
- **findings-sentinel**: flag → agent → canon → stasis for the dirty-set ledger.

## 2. The edge model — seal OR governor, never both, never neither

Every declared dependency edge `downstream → upstream` carries exactly one conformance mechanism:

```yaml
# doc artifact (in-body cites: envelope — EXISTING form, unchanged)
- ref: humans-agent-cid-contract
  desc: the join-key namespace contract this seeder conforms to
  fingerprint: sha256:ab12…            # short-form rendering of the body CID
  # status:/path: tool-managed as today

# code artifact (sidecar edge record — .eprfs/ dag-cbor line, machine-facing)
{ from: "elohim/elohim-storage/src/views.rs",
  to:   "elohim/sdk/schemas/v1/views/resilience.schema.json",
  sealed_cid: "bafkrei…",              # upstream body CID at conformance time (raw 0x55)
  governor: "cite-seal",               # this edge's enforcement class (§3)
  sealed_by: "<agent_cid>", sealed_at: …,
  status: null }                       # tool-managed: stale | held(reason, valid_from, superseded_by)
```

**Placement rule (why two homes, one vocabulary):** docs carry edges inside their hashed bytes
(fabric §2.3 — tamper-evident by construction, already live as `cites:`). Code files cannot carry
frontmatter and must never carry governance metadata the compiler sees, so their edges live in the
`.eprfs/` sidecar as CID'd records. Both project into **one edge index** the walk consumes; the
envelope vocabulary (`ref/desc/fingerprint/status/path` ↔ `to/desc/sealed_cid/status`) is the same
schema in two renderings — never a third system.

**Staleness is derived, never stored as truth:**
`stale(e) ⇔ e.governor = cite-seal ∧ body_cid(e.to) ≠ e.sealed_cid`. Going green per edge =
**conform-and-reseal** (`epr flow seal`), or an explicit **declared deviation** (`status: held` with
reason + `valid_from`/`superseded_by` — transition semantics, so backward-compat edges like
read-legacy-write-canonical are *representable*, not "fixed away"). A bespoke handroll either
reseals against the contract or is visibly held — never silently coexisting.

## 3. Governor delegation — the enforcement-class ontology

The operator's constraint that makes this honest: *if an external mechanism is binding for a file,
cite the mechanism; don't cite-seal an edge that is self-governing by a more efficient and
necessarily robust system.* Every edge declares its **governor** — who enforces conformance:

| governor | Edge class | Conformance evidence | Seal? |
|---|---|---|---|
| `compiler` | same compilation unit (cargo workspace, tsc project) | the build itself — breakage surfaces at compile | NO — cite the unit (`cargo:elohim-storage`) |
| `codegen` | generated artifact ↔ generator source (ts-rs, schema:codegen) | regeneration byte-identity (pre-push freshness check) | NO — cite the pipeline |
| `schema-contract` | wire struct ↔ view schema | the named contract test (`schema_contract.rs`) | NO — cite the test |
| `test` | any edge a named test pins | that test green | NO — cite the test id |
| `cite-seal` | **residual**: cross-boundary semantic contracts no stronger mechanism sees (prose→code, seeder→renderer, seed-data→manifest, stage→stage in genesis) | fingerprint conformance (§2) | **YES — only this class seals and enters the dirty-set** |

Rule: **seal only the governance gap** — the compose-gate's lightest-signal principle generalized.
A governed edge is still an *edge* (the walk traverses it; impact summaries include it); it just
delegates enforcement and never goes stale in the ledger — its red arrives from the stronger
system, faster.

**Governor assignment is policy, and mostly derived.** `.claude/epr-meta/governors.yaml` (sibling
registry, same Precedent conventions + `id@version` pins; separate file because policies.yaml's
fail-loud validation rightly rejects non-enforcement classes) carries the versioned detector list,
consumed by `epr flow seal` rather than the resolver hook.
The resolver auto-derives defaults mechanically — both endpoints in one cargo workspace →
`compiler`; `.rs` ↔ `generated/*.ts` → `codegen`; struct named in a view schema → `schema-contract`;
doc ↔ doc → `cite-seal` — so agents hand-author only the residual cross-boundary edges. A wrong
default is overridden at the binding, never by editing the policy (variance via `params:`).

## 4. In-flight: the file-leave trigger

A PostToolUse (Edit/Write) hook — debounced per (file, session) through the existing epr-meta
advice store, **inject/measure class, never deny** — fires when an agent leaves a file:

1. Recompute the file's body CID; look up its forward edges in the edge index (`walk_forward`).
2. **Partition by governor.** Self-governing edges → one summary line: *"N dependents governed by
   compiler/codegen/schema-contract — the gate you must pass will enumerate them"* (the chain that
   "fires and hits validations all the way down" is the existing gate stack; we cite it, we don't
   re-run it per keystroke). Sealed edges whose upstream is now this changed file → stamped stale
   in the ledger + surfaced inline: *"3 sealed dependents now stale: […] — reseal each
   (`epr flow seal`), declare deviation, or escalate."*
3. **New-dependency seam visibility:** if the session's read-set (files Read while editing this
   one) contains cite-seal-class artifacts with no declared edge, the hook offers the one-liner
   that seals it now (`epr flow seal <file> --on <upstream>`) — born-governed at emission, the
   same FRONT discipline that made new cites born-linked.
4. **Escalation:** when the stale frontier exceeds the policy's bound, or crosses authority the
   agent doesn't hold (another pillar's surface, a held doc), the hook files the fingerprint and
   emits the sentinel dispatch directive — background triage agent canonicalizes into the timeline
   backlog. Deterministic ledger flag → agent → canon → no re-fire on blocked → stasis sweep.

The dirty-set ledger is `.claude/data/edge-findings.jsonl` (operational; fp =
`sha256(from|to)[:12]`; reconstructable by recomputing seals vs tree). The gate reads it: **the
push gate is red while cite-seal-class stale edges touching the pushed paths are neither resealed
nor held** — BACK-hard, exactly where the semantic-links spec put the dissolution gate. Agents of
any vendor converge on the same cascade because the gate, not agent judgment, defines done.

**Cold-start pickup is by-construction, not by-reading:** the process teaches itself at the
moment of relevance — the file-leave hook hands an agent the frontier *and the exact verb*
in-context; the SessionStart headline carries the `edges:` gauge beside every other pressure;
`--json` on all three verbs makes the frontier machine-consumable. An agent that has read
nothing still meets the governance in flight, the same way the cite-seal and compose-gate hooks
already govern this repo's own authoring.

## 5. The triage — everything already written is unsealed edges

The existing corpus is the backlog: contracts living in CLAUDE.md prose, the
perpetually-UNSTABLE genesis stages (contracts checked at seed/deploy that were never declared at
authoring), and the recipe's `validators: []` placeholders. Drained as a measured budget, not a
big-bang:

- **First tranche (highest signal): the unstable genesis stages.** For each stage that "always
  resolves unstable," author its input edges — what it consumes, from where, under which governor.
  A stage red then names *which edge* broke instead of re-diagnosing from scratch; a stage that
  can't go green gets a declared deviation with a reason, visibly held instead of ambiently amber.
- **Second: prose contracts in gospel surfaces** (the join-key class — every cross-layer invariant
  currently enforced only by CLAUDE.md text gets an edge whose `desc` is the announcement and
  whose governor names the tripwire/test that enforces it, or `cite-seal` if none exists yet —
  which is itself the signal to promote it).
- **Third: recipe edges** — `EdgeSpec.validators` entries become governor citations.

**The drain is memory governance, not mechanical cleanup (decided 2026-07-21).** A stale edge
is the re-verify queue; the two verbs are two different kinds of act and route differently:
*reseal* is a re-verification judgment ("the downstream claim still holds against the moved-on
upstream") — librarian/ceremony work, delegable, drained on the memory-stasis-loop cadence;
*hold* is a **governance decision** — a declared deviation with reason + validity window, policy
not hygiene — surfaced by the ceremony as a menu with the **operator confirming holds**. Nothing
auto-blesses drift by construction (reseal is stale-gated and explicit). Contested or
precedent-heavy edges get the four-lens read (historian: has this drift shape appeared before;
storyteller: did the lesson already graduate).

**Legacy-corpus doctrine:** never big-bang. Governor auto-derivation (#4) first — a pre-#4
`0 governed` count is an artifact, not reality; compiler/codegen absorb the bulk mechanically —
then author/seal only the residual cross-boundary edges tranche-wise, holding (with reasons)
what genuinely deviates. Stasis = 0 stale · unsealed only shrinking · every hold reasoned and
time-windowed with a re-check sweep. New debt stays zero because edges are born-governed at
emission (§4) while the stock drains as a measured budget — self-cleaning going forward, clear
and concise: FRONT-soft at authoring, BACK-hard at the push gate (#7).

Scoreboard: `placement-audit.py` headline gains `edges: N unsealed · M stale · K held`;
`memory-stasis-loop` gains the matching discipline (stasis = 0 stale, unsealed draining), and
the memory-ceremony's librarian phase owns the judgment tier of the drain. New edges are
born-governed by §4, so the backlog only shrinks — development *keeps* stasis instead of
re-earning it.

## 6. Above the filesystem — how policy aggregates, translates, adapts upward

File-level governance is the floor, not the system. When epr/epr-meta cross the network boundary,
the same seal shape rides the fabric's three planes — nothing is re-invented per layer:

| Depth | The seal is… | The mechanism |
|---|---|---|
| Byte floor (offline) | a `FlowEvent` (verify-action) in `.eprfs/status/`, full-CID, local | deterministic; no network needed |
| Network boundary (push) | a **reach-earned Attestation** — the push already attests CID-vs-source at the deterministic floor; the edge-set becomes the attested content (existing Attestation entry type; B2: granular seals stay sidecar, the crystallized proof notarizes) | fabric's observation→crystallization gradient |
| Social substrate | **standing about the policy**, not new state: governor assignments and seal policies are EPR content whose *force* is a `Mishpat::Precedent` grant; deviations are `Mishpat::Commitment`s (cid = entry_hash) with validity windows | policy-registry two-plane lift |

- **Aggregate by fold:** a container's coherence standing = fulfillment ratio over its edges'
  seal states (the fabric's existing pure fold, `in_scope_of` the container EPR) — a crate, a
  pillar, a household node each reads as one gauge, computed not stored.
- **Translate by the two-plane split:** the *content* of a rule never changes crossing layers;
  what changes is the *standing* a scope grants it (deny here, measure there, persuasive
  elsewhere) — so byte-level rules become social-substrate policy without rewriting.
- **Adapt by scope binding:** the `.epr-meta` cascade is the fractal container-appropriateness
  judgment (fabric §2.5) — each container decides, one level up, which edges are meaningful joints
  and how much force their seals carry. Reach mechanisms consume the same gauges: earned reach at
  the deterministic floor graduates to community-witnessed standing, same anchor, deeper
  validation.

### 6.1 The workspace slice — brit · rakia · lvi

This is deliberately an **early cross-cutting slice through brit, rakia, and lvi**, enabled in
our own workspace first. The `.claude/` hook + jsonl-ledger floor is scaffolding with a named
successor at every layer: **brit** supplies the CID engine and the graph the edge index rides
(`BritCid`, `GraphConnections`, dev-pipeline attestation nodes); **rakia** is where governed
edges meet the build substrate — a `compiler`/`codegen` governor citation is, in rakia terms, a
build-closure dependency whose conformance evidence is an output CID, and "reach IS the
build-output graduation" means a sealed, attested edge-set is exactly what earns an artifact's
reach; **lvi** is where the file-leave trigger stops being a Claude-harness hook and becomes the
devspace peer-runtime's own edit-time surface — the workspace watching its own seams, for any
agent or human working inside it. Designing the seal as sidecar FlowEvents + policy-registry
rows (not hook-private state) is what keeps that migration a re-homing, not a rewrite.

## 7. Non-goals

- No new DHT entry types, no DNA action mints (Attestation / Precedent / Commitment suffice).
- No per-keystroke validation chains — self-governing edges delegate to the gates that already
  run; the hook informs and stamps, `inject`/`measure` class only.
- No retroactive big-bang sealing — the triage drains as a budget; §4 keeps new debt at zero.
- Automatic edge *inference* from build graphs (mining Cargo/tsc dependency graphs into declared
  edges wholesale) — a later accelerator for governor auto-derivation, not v1.

## 8. Decomposition (gap-items)

- [x] Sidecar edge-record schema (dag-cbor: from/to/sealed_cid/governor/sealed_by/sealed_at/status) + the one-vocabulary mapping to the cite envelope; full-CID seal via `eprfs cid` (Python never encodes) (§2).
- [x] Edge index: project doc `cites:` + sidecar records into one graph the `epr-rea` walk consumes (`FlowStore`/brit-graph adapter); staleness derivation `body_cid(to) ≠ sealed_cid` (§2).
- [x] `epr flow seal <file> --on <upstream>` + `epr flow reseal` + `epr flow hold --reason --valid-from` in epr-cli (mirror cite-gen `--seal`/`--refresh` UX) (§2).
- [ ] Governor vocabulary + the `edge-governor-defaults@1` registry (`.claude/epr-meta/governors.yaml`, Precedent-shaped, `id@version` pins) + mechanical auto-derivation at seal time (workspace/codegen/view-schema/doc-doc detection; test:<id> always explicit) (§3).
- [ ] File-leave hook: PostToolUse resolver leg — recompute CID, partition forward edges by governor, stamp stale, surface frontier summary, offer seal one-liner for undeclared read-set deps; debounced via epr-meta advice store (§4).
- [ ] Dirty-set ledger `.claude/data/edge-findings.jsonl` (fp = sha256(from|to)[:12]) + sentinel dispatch directive on bound-exceeded/authority-crossed; suppression on already-filed (§4).
- [ ] Push-gate leg: red while cite-seal-class stale edges touching pushed paths are neither resealed nor held (BACK-hard) (§4).
- [ ] Triage tranche 1 — author + govern the input edges of every perpetually-UNSTABLE genesis stage; each stage red names its edge or is visibly held (§5).
- [ ] Triage tranche 2/3 — prose-contract edges from gospel surfaces; recipe `validators:` → governor citations (§5).
- [ ] Scoreboard + governance wiring: `placement-audit.py` `edges:` headline + `memory-stasis-loop` edges discipline (stasis = 0 stale) + memory-ceremony connection — librarian drives mechanical re-verify, ceremony menus contested edges, operator confirms holds (§5).
- [ ] Graduation leg: push crystallizes the sealed edge-set into an Attestation (B2; granular seals stay sidecar); deviation → `Mishpat::Commitment` (cid = entry_hash); policy standing → `Mishpat::Precedent` — existing types only, verify headroom before ANY mint (§6).

## 9. Open questions

- **Read-set capture fidelity** — the session read-set is a heuristic for "new dependency taken";
  good enough to *offer* a seal, never to auto-seal. Is transcript-derived read-set available to
  the hook cheaply, or does v1 restrict to explicitly-declared deps?
- **Edge identity under file renames** — `from`/`to` are paths at the floor; docs have slugs. Do
  code artifacts get eprfs-package identities before or after v1 (path-only floor + propagate-on-
  move, like the cite `path:` cache, is the v1 lean)?
- **Held-edge review cadence** — deviations with `valid_from` windows need a re-check sweep;
  fold into the stasis loop's existing blocked-item re-check or a dedicated pass?
