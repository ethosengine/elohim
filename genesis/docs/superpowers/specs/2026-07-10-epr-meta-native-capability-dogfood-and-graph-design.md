---
title: "Dogfood .epr-meta Native Governance + Certify Claude→Elohim-Native Translation (+ eprfs Package Graph)"
id: epr-meta-native-capability-dogfood-and-graph
tier: spec
status: Implemented
created: 2026-07-10
maintainers: Matthew Dowell + Claude Opus 4.8
class: process-meta
process_subdomain: doc-lifecycle
topic: [epr-meta, eprfs, elohim-agent, capability-packages, projection-manifest, claude-translation, dogfood, graph]
context-tier: disclosed
steward: cartographer
graduation-trigger: decompose-complete OR superseded-by-eprfs-package-graph-productization
refines:
  - genesis/docs/superpowers/specs/2026-06-25-epr-meta-compose-gate-design.md
cites:
  - epr-meta-eprfs-elohim-native-sotu-2026-07-09 | the SOTU this sprint acts on — readiness split + closeout list | path: genesis/docs/analysis/2026-07-09-epr-meta-eprfs-elohim-native-sotu.md
  - epr-meta-compose-gate | the live compose-gate mechanism this dogfoods — cascade, class ladder, resolver | path: genesis/docs/superpowers/specs/2026-06-25-epr-meta-compose-gate-design.md
  - elohim/eprfs/eprfs-core/src/projection.rs
  - elohim/sdk/domains/elohim-agent/scripts/package-projections.mjs
---

# Sprint: Dogfood `.epr-meta` Native Governance + Certify Claude→Elohim-Native Translation

## Goal (operator statement)

> Start natively dogfooding the `.epr-meta`/eprfs native governance system in *this*
> Eclipse-Che repository, AND have high confidence that we're ready to start translating
> Claude skills and subagents into elohim-native capabilities — usable like a graph in any
> submodule repository or filesystem.

## Grounded starting state (evidence, 2026-07-10)

The previous slice (SOTU `epr-meta-eprfs-elohim-native-sotu-2026-07-09`) already built the
machinery and left it green-but-uncommitted. Verified afresh this session:

- **Governance is already live here.** `.claude/hooks/epr-meta-resolver.py` is a registered
  `PreToolUse` hook (`.claude/settings.json:138`) that reads the `.epr-meta` cascade. The repo
  root is now the directory form `.epr-meta/manifest.md` (constitutional base: the `ci-trigger:`
  ignore set + one observation-tier `rs-loc-ceiling` rule). This very spec's write was DENIED by
  the specs/ `.epr-meta` rule until it carried full lifecycle frontmatter — the gate works.
- **The full Claude surface is already translated.** 36 skills (`.claude/skills/*/SKILL.md`) +
  23 agents (`.claude/agents/*.md`) → all packaged under `.epr-meta/elohim/packages/`, **0
  outstanding**. `node …/package-projections.mjs verify` is green at **672 checks** (the one
  `orchestrate` drift was closed during grounding; process exits 1 on failure — the gate is real).
- **Import is byte-lossless.** Probe: `orchestrate` (a never-before-imported human skill) →
  `import` → `project` reproduced its `SKILL.md` **byte-identical**. `project(import(source)) ===
  source` on a real case.
- **`.claude` is still authored source.** The `p2p-design-gate/SKILL.md` diff is a fresh human
  content edit to the tracked source. So packages are today a **certified-lossless mirror**, not a
  clobbering master. The flip to package-master is a later, explicit decision — NOT this sprint.
- **eprfs is resolver-only, not the live gate.** `eprfs-meta` (Rust) resolves the directory-form
  root but the Python hook is what enforces. `eprfs-core` defines the domain-neutral
  `ProjectionManifest`; `eprfs-local` materializes one into a filesystem tree. No package→manifest
  adapter exists yet.

Conclusion: the translation *data* is done. This sprint hardens it into a **committed, governed,
drift-gated, fidelity-certified** system, and adds the **eprfs package graph adapter** so the
package layout is a portable content-addressed graph, not just a convention.

## Framing: deterministic floor, elohim ceiling — a capability's accountability to an EPR

The governance this sprint builds is the **deterministic floor**: it holds with zero ubiquitous
compute, offline, when deep peer validation is unavailable or unneeded. Every skill/subagent
artifact is **accountable to an EPR** — an accountability anchor representable in the runtime that
mints the projection. The projected backref is a **relationship snapshot**: *what is minimally
expected of this capability at the moment of generation.* It respects that the model has real
agency (and can, and will, fail to read its instructions) by making the relationship **explicit and
accountable** rather than assuming compliance.

The **elohim ceiling** — a behavioral "challenge-this-subagent" judge that decides, from what an
agent actually did, whether it honored the gates in good faith, its trust *earned through the p2p
trust substrate* — is **assumed-possible by the protocol but not affordable on every meaningful
invocation today.** So we build the floor **shaped to accept the ceiling**: the accountability
anchor is an `EprRef` (the same anchor `eprfs-core::ProjectionEntry` already carries), so when the
substrate is reachable the anchor resolves into deep-validated, earned trust, and when a peer is
offline the backref snapshot *is* the trust. One accountability object, two validation depths.

This is where **Pillar A meets Pillar C**: the package graph (C) supplies the content-addressed EPR
anchors that the governance snapshots (A) reference; floor-compliance is checkable offline against
that graph; the ceiling is deep trust over the same anchor.

**Verification-as-attestation (named seam): a gate's result is a mintable EPR attestation.** The B2
parity fixture is a *claim* — "the Python and Rust `.epr-meta` interpreters resolve this manifest to
the *same* ordered rules" — backed by *dual-witness evidence* (both suites pass). That is the shape
of an EPR **attestation**, one content-address + one conductor signature from mintable: content-
address the fixture-plus-expected-result to a `BlobCid` (exactly what the Pillar C adapter computes),
sign it, record the pass/fail as the observation/feedback leg. So the parity fixture is a
proto-attestation wearing a CI costume — and it is *feedback-leg evidence a governance capability
earns reach with*: "my meaning is provably interpreter-invariant" is a gradeable claim, as is "my
translation is provably lossless" (the B1 fidelity gate). It is two-sided attestation applied to
*implementations* — two independent interpreters co-witness the meaning rather than one asserting it.
Nothing is minted this sprint (the fixtures stay tests), but this is why the pillars converge: **the
eprfs adapter (C) is the content-addressing step that turns a verification artifact (B) into an EPR
atom.** See
[[project_earned_reach_governance_pr_ceremony_vision]] and
[[project_rea_compute_commitment_primitive]] for the ceiling's substrate lineage.

**Substrate seam (named, not built this sprint): the `EprRef` is also the value-flow anchor.**
The same EPR a governance backref points at carries the capability's REA/ValueFlows — its
feedback/externality legs (value + governance + feedback = the reach-earning machinery; see
`elohim/sdk/CLAUDE.md`). So the floor/ceiling shape repeats on the *value* plane: the floor is
**eprfs offline usage collection** — tokens spent against a capability, view counts, pages read,
minutes listened — accumulated locally against the `EprRef`; the ceiling is **honest value-flow /
REA reconciliation on reconnect** to the peer network. eprfs's `.eprfs/status/` awareness-sidecar
pattern is the natural home for that offline meter. This sprint neither builds nor blocks it — it
keeps the `EprRef` seam clean so the value plane can attach later. For agentic capabilities the
usage signal is invocation cost (tokens-against-capability); for a video it is watch-minutes — same
substrate, different content.

The pre/post-hook EPR triggers are, generalized, the **value-native analog of a tag-manager /
campaign-instrumentation layer** — except the events they emit are not analytics for a platform to
harvest, they are **REA economic events flowing to the EPR itself.** The canonical shape is the
espresso: *which beans, from which farm* (resource + provenance) · *who made it* (provider agent) ·
*what we charged* (value/measure) · *when* (event time) · *who gets credit* (attribution agent) ·
*who consumed it* (consumer agent) — Resource·Event·Agent, generalizing to a skill invocation, a
video view, a page read, an espresso: same shape, different content. **The design discipline it
demands is choosing the *right level of complication per context* — instrument the economically
meaningful joints, not every water molecule.** Over-instrument and signal drowns in cost/noise;
under-instrument and value cannot flow honestly. That "what matters here" judgment is itself a
governable design choice — closing back to the reflexive gates: *what an EPR reports is as governable
as how it is gated.* The floor this sprint builds neither picks nor forecloses that granularity; it
keeps the `EprRef` the clean join where the REA report attaches.

**Integrity of the offline meter (named, not built): the conductor is the local trust root.** An
offline usage stream must not be forgeable ("I watched 1,000 hrs") when it reconnects. The on-device
Holochain conductor holds the agent's signing key, so the meter records **signed, source-chain-ordered
events against the `EprRef`, never bare counters** — hash-chained + append-only makes the stream
tamper-evident and non-reorderable; DHT anchoring on reconnect witnesses "existed before time T";
physical-rate validation rules (Mishpat) reject the impossible. That is the honest reach of
cryptography here: **authenticity, integrity, ordering, and time-bounding**.

Crucially, the signing key is **grounded in the social substrate, not a lone personal keystore** —
partly for this very reason. Identity is **community-backstopped**, never self-sovereign-apex (see
[[feedback_identity_sovereignty_ontology_guard]]): the commons backstops the individual, so the
signing identity is itself socially attested rather than cheaply minted. This is why the naive
"trusted-client" limit does not simply apply — **social validation of the substrate data is the
*primary* ground-truth mechanism, not a bolt-on ceiling**: a fabricating identity can be socially
challenged and revoked, and Sybil farms are resisted because identities are socially grounded. Social
recovery and fraud-resistance fall out of the same design. The honest residual: a *colluding social
cluster* could still inflate stats among themselves — social grounding raises the bar enormously but
does not make collusion impossible — so it is bounded by **earned reach graded across the wider
commons** + governance (Mishpat), with optional hardware attestation as a further ceiling. Crypto
(conductor + source-chain + anchor) supplies integrity/ordering/time; the **social substrate**
supplies identity-groundedness and data validation.

And the ceiling is **active, not passive**: fully-distributed intelligence enables **peer audit of
what a machine is actually doing** — if an elohim agent detects an algorithm skimming value or
padding stats, the commons imposes correction *from above* (the top-down side of the reflexive
gates, now as enforcement). Along the reach epic's **compute-curve-of-trust**, discipline is
**graduated and restorative** (Mishpat, not punishment): a **standing hit**, **raised compute cost
during a disciplinary period**, up to indefinite *only* for the rare certifiable-bad-faith actor.
This is what actually answers the colluding-cluster residual — not that collusion is impossible, but
that the commons can *see it and price it*. The felt experience is the design: because a user knows
bad-faith rules/signals are auditable, correctable, and costly to standing, **good faith is the
equilibrium, not an assumption.** Floor/ceiling on the integrity axis; the `EprRef` +
socially-grounded conductor signature is the seam it all attaches to, and the commons audit is the
enforcement it invites.

**The gates are reflexive — rules-as-capabilities.** The deterministic pre/post-hook tooling is
*self-authorable by the agent, for itself*: an actor writes the co-located `.epr-meta` gate it needs
because it does not trust itself on some set of files (the envelope system). Gates are equally
**authorable top-down by an elohim affirmation** — *"we notice you keep making this mistake; try
doing this"* — a formative act of care, not punishment (Mishpat again), and rules authored either
way can always be revaluated later. Bottom-up self-discipline and top-down affirmation converge on
the same governable object. But gates are
**instrumental, not intrinsic** — an envelope system only earns its keep when it is *needed* to
reach the deeper value; with enough discipline the lighter instrument is strictly better. So a
self-authored gate is itself an **EPR-anchored, governable capability** in the same graph as the
skills and agents it governs: the `capability-governance` policy is reflexive (it governs the gates
too), and governance flows **both directions** — a judgment can *relax or retire* an
over-constraining gate, not only add one ("you're rich enough now; let it go"). That is Mishpat, not
punishment — restored capability and negotiated boundaries
([[feedback_justice_mishpat_not_punishment_guard]]); limitarianism, governance, and judgment are one
principle: bounded authority that is itself accountable. The floor this sprint builds treats
gate-definitions as first-class graph objects and lets the lodging ledger carry *"relax this gate"*
findings; the both-directions *judgment* is the ceiling's remit.

**Capstone seam — the reach shape of a value-generation rule.** A rule for how a resource generates
REA signals is not universal; it has a **reach gradient = its scope of affirmation.** At one extreme
a *self-declared* rule ("10 jumping jacks every time I do Y") needs no one's affirmation — reach =
self. At the other a *sovereign* rule ("mint a $1") is wholly owned by an apex authority — reach =
central, not yours to write. The entire governed **middle** — family-credit, community-credit,
municipal-credit — is where a value-rule is *earned and affirmed at a layer*: household → community →
municipality → … . Signals **translate and re-negotiate across those layers** (subsidiarity — a
household's rule composes upward into the community's without being flattened). And for a genuinely
**new resource** the network has never seen (the unfamiliar machine), many people run slightly
different value-rules; **elohim sees the distribution of those experiments** and affirmations propose
rule-upgrades that fit the aggregate together *while respecting each component* — human + elohim
**co-authoring new EPRs from the accumulated wealth of REA experience across every prior resource**,
which crystallize (as systems informed by mutual flourishing crystallize) into affirmed, governed
**reach shapes**. This sprint builds the *atom* of exactly that: the `capability-governance` rule is
a low-reach (repo-local, near-self-declared) value-rule; the `EprRef` backref is its content-
addressed identity; the fidelity/parity gates are how a rule *earns trust*; reach-weighting +
peer-audit is how its affirmation is *graded*. The reach epic is the machinery by which such
rule-atoms climb self → household → community → municipality, translating as they go — the same
compute-curve-of-trust that governs content, now governing the rules that generate value itself.

## Definition of Done

### Pillar A — Dogfood governance live: capabilities accountable to an EPR (deterministic floor)
- **A1 (done in grounding).** Full surface packaged; `verify` green at 672 checks.
- **A2a — Govern the capabilities themselves.** An `.epr-meta` bound to
  `.epr-meta/elohim/packages/` binds a `capability-governance` policy to the skills/agents at their
  *source* (govern the skill, not merely nudge the projection). Authoring-time class per the policy
  registry; `.claude` hand-edits remain a nudge (still authored source this sprint), but the
  package is where governance *lives*.
- **A2b — Projections carry a governance backref (the relationship snapshot).** The projector
  injects a governance stanza into every `.claude`/`.codex` artifact anchored on an **`EprRef`**:
  *"this capability is accountable to `<epr-ref>` (governing package `<id>`, policy `<policy>`);
  honor gates `<…>`; non-compliance is lodged at `<ledger>`."* The projection self-describes its
  governance — the "hook back" — so an actor **is aware** of what is minimally expected. The
  `EprRef` degrades gracefully: offline it stands as the floor snapshot; with the substrate it
  resolves to deep-validated trust (the ceiling).
- **A2c — `verify` renders the red/green (artifact compliance).** Every projection must carry a
  valid, in-sync backref to a real governing package. Missing / stale / forged backref → **red**.
  This is the offline-checkable floor judgment on the artifact.
- **A2d — A lodging surface for non-compliance.** A `governance-findings.jsonl` ledger, reusing the
  deterministic *flag → agent → canonical backlog* pattern
  ([[feedback_deterministic_flag_agent_canon_stasis_pattern]]), where drift / missing-backref /
  gate-bypass becomes a lodged complaint. This is the "surface by which complaints can be lodged" —
  and the seam the behavioral ceiling judge will later write to.
- **A3.** `node …/package-projections.mjs verify` (now including A2c) wired into
  `.husky/pre-push.bash` so projection/compliance drift is caught like any other generated-artifact
  freshness gate. PVC-pressure-neutral (pure node, no cargo); skippable only via the existing
  `--no-verify` path (itself a lodgeable gate-bypass signal).

### Pillar B — High confidence to bulk-translate
- **B1.** A standing **round-trip fidelity gate**: for every `.claude/skills/*/SKILL.md` and
  `.claude/agents/*.md`, assert `project(import(source)) === source` byte-for-byte, computed in a
  temp workspace so the guarantee holds independent of workflow order (does not depend on runtime
  files having just been projected). Converts the one-off `orchestrate` probe into a repeatable,
  always-valid property. **This is the confidence mechanism.** Wired into the packages test.
- **B2.** Python↔Rust `.epr-meta` resolver **parity fixtures** (directory-form root + 2–3 cascade
  cases: root-first, nearest-wins, legacy-nested). One shared fixture set, asserted identically by
  a Python test and a Rust test, so the two governance interpreters cannot silently diverge (SOTU
  closeout #1; standing hazard `genesis/data/timeline/backlog/epr-meta-python-rust-parser-parity.md`).
- **B3.** The `elohim-package-authoring` skill states the **directionality contract** plainly:
  import (Claude→package) is certified lossless; `.claude`/`.codex` are authored source today;
  packages are a certified mirror; the package-master flip is deferred with its named prerequisite
  (B1 + A2 must be trusted first).

### Pillar C — eprfs package graph (the "graph in any filesystem")
- **C1.** A thin **Rust domain adapter** (new crate, home below) that:
  1. reads a package tree rooted at `.epr-meta/elohim/packages/` (skills + agents) plus its
     generated projections under `.epr-meta/elohim/projections/`, treating each file as
     **byte-bearing content**;
  2. computes each entry's `BlobCid` with **eprfs-core's own `BlobCid::compute`** (no re-implemented
     CID in JS — that would replay the Python/Rust parity hazard);
  3. builds a `ProjectionManifest` with `ProjectionSource { namespace: "elohim-agent", kind:
     Content|Container, id: "<Kind>:<name>" }` per entry, and passes `ProjectionManifest::validate()`;
  4. is **local-first**: a `LocalOnly` materialization backed by the tree's own bytes — no
     elohim-storage HTTP, no DHT.
- **C2.** A round-trip test: adapter builds the manifest from the real package tree → `eprfs-local`
  materializes it into a fresh temp directory → the materialized tree is **byte-identical** to the
  source tree. This is the honest proof that the package layout is a portable, content-addressed
  graph reusable at *any* filesystem root (the submodule/arbitrary-root claim, demonstrated).

### Cross-cutting
- Commit everything **path-scoped to the epr-meta concern** on the current shift branch
  (`feat/frontend-eyes-sprint`), never touching the ambient unrelated changes in this shared
  worktree. **Integrator pushes** — this sprint ends at committed-on-branch.

## Architecture

### Pillar C adapter — placement and shape

Per the eprfs boundary discipline (`elohim/eprfs/CLAUDE.md`: *"Do not embed git semantics in
`eprfs-core`… domain meaning belongs in a domain adapter"*), the package→manifest adapter is a
**domain adapter for the agent-capability-package domain**. It does NOT belong in `eprfs-core` and
NOT in the `eprfs/*` crates (those are the substrate; `brit` is the git-domain analog living
outside).

**Home:** a new crate `elohim/sdk/domains/elohim-agent/adapter/` (name: `elohim-agent-adapter`),
co-located with the JS authoring CLI it complements. The elohim-agent domain then owns both the
authoring surface (JS: `package-projections.mjs`) and the graph surface (Rust: the eprfs adapter).

**Dependency direction:** `elohim-agent-adapter → eprfs-core (+ eprfs-local for the round-trip
test) → (no storage)`. It depends only on `eprfs-core` for `ProjectionManifest`/`BlobCid` and, as a
dev-dependency, `eprfs-local` + a local test-double storage for the materialization test.

**What it does / does not do:**
- It **reads** existing package JSON and projection files as opaque bytes and maps them to manifest
  entries. It does **not** re-render projections (that stays in JS) and does **not** parse package
  semantics beyond the filename→id mapping.
- CID is the single source of truth: `eprfs-core::BlobCid::compute` (`CIDv1(dag-cbor, sha2-256)`,
  matching `elohim_epr::cid::compute_cid`). No second CID implementation is introduced.

**Native-build hygiene (this repo's rails):** the crate is added to the workspace members; native
builds set `CARGO_TARGET_DIR` per the cargo-target-pool. Because it is a pure-Rust, non-WASM crate
with no storage/DHT deps, it does not touch edge Dockerfiles or Nexus wiring in this sprint (it is
a local dev/test artifact, not a shipped runtime dependency). If a future sprint ships it inside a
runtime, the New-path-dep-needs-Dockerfile-COPY rule applies then.

### Pillar A governance — shape

Governance is bound to the **capability at its source** (A2a): an `.epr-meta` at
`.epr-meta/elohim/packages/` binds a `capability-governance` policy to the package tree. The
projection surfaces (`.claude/skills`, `.claude/agents`, `.codex`) do not need their own deny rule —
instead they carry a **backref** minted by the projector (A2b). Because the compose-gate keys off
`.epr-meta` files on the ancestor path of the *edited* file, a `.claude` hand-edit still trips a
nudge to re-import; but the authoritative governance object is the package's EPR anchor, not the
projection.

The **backref stanza** (A2b) is the relationship snapshot. Minimal floor shape: an `EprRef`
identifying the governing capability record, the governing package id + policy, the gates the
capability is expected to honor, and the lodging ledger path. It is emitted by the *same* projector
that renders the runtime surface, so it cannot be minted out-of-band; `verify` (A2c) recomputes the
expected backref from the package and asserts byte-equality with the on-disk stanza — a forged or
stale backref is a red. Exact policy binding chosen from the live registry
(`.claude/epr-meta/policies.yaml`) during implementation — reuse an existing observation policy if
one fits; only add a new policy if none does.

The **lodging ledger** (A2d) follows the existing findings-ledger contract
(`.claude/data/*-findings.jsonl` + cursor): a deterministic fingerprint per non-compliance, so the
same drift does not re-fire, and a canonical backlog entry is the terminal state. This is a
*scaffold* this sprint — the writer wired to `verify`-red — not a running behavioral judge.

### Pillar B1 fidelity gate — shape

A new check inside `package-projections.mjs` (or a sibling invoked by the packages test): for each
discovered Claude source, import it into an in-memory/temp package, project it back, and assert
byte-equality against the on-disk source — WITHOUT writing to the real tree. This isolates import
fidelity from the "runtime was just regenerated" confound. It runs under
`pnpm run elohim-agent:packages:test`.

## Test Plan

- `node elohim/sdk/domains/elohim-agent/scripts/package-projections.mjs verify` → green.
- `pnpm run elohim-agent:packages:test` (now includes the B1 fidelity gate) → green.
- `pnpm run elohim-agent:test` → green.
- `python3 .claude/scripts/_lib/__tests__/epr_meta_cascade_test.py` (+ the new B2 parity fixtures)
  → green.
- `python3 .claude/scripts/_lib/__tests__/ci_trigger_test.py` → green.
- New Rust parity test (B2) and adapter tests (C2), with `CARGO_TARGET_DIR` set per the pool and
  `RUSTFLAGS=""` (native, non-WASM):
  - `cargo test -p eprfs-meta` (parity), `cargo test -p elohim-agent-adapter` (graph round-trip),
    `cargo fmt --check` + `cargo clippy -- -D warnings` on the new crate.
- `.husky/pre-push.bash` dry-run confirms the A3 gate fires on projection drift and passes clean.
- `git diff --check`.

## Out of scope (explicitly deferred — the elohim ceiling, enabled but not built)

- **The behavioral "challenge-this-subagent" judge** — deciding from what an agent *did* whether it
  honored the gates in good faith, its trust earned through the p2p substrate. The floor built here
  (EprRef backref + lodging ledger) is its enabling seam: the actor is already aware (A2b) and the
  lodging surface already exists (A2d). The runtime judge itself is the named next layer.
- Resolving the backref's `EprRef` through the **p2p trust substrate** (DHT/notary/earned reach)
  for deep-validated trust. This sprint's `EprRef` stands as the offline floor snapshot only.
- Making eprfs the *live enforcement gate* (replacing the Python hook).
- EPR content-addressing / storage-backing of packages through `elohim-storage` (DHT, custody,
  replication). C is local-first only.
- CLI root-configurability (`--repo-root` etc.) for arbitrary external repos. The *adapter* proves
  arbitrary-root reuse; the *JS CLI* staying monorepo-shaped is acceptable this sprint.
- Flipping `.claude`/`.codex` to package-master (generated-and-clobbered). Deferred behind trusted
  B1 + A2.
- Bulk-translating *command*-form skills (`.claude/commands/**`) or single-file `.md` skills — the
  adapter/CLI target directory-form skills + agents only.

## Risks & mitigations

- **Python/Rust governance drift** → B2 shared parity fixtures make divergence a failing test.
- **Silent projection clobber of a human edit** → A2 is nudge-not-block; B1 proves round-trips are
  lossless so re-import never loses content.
- **New Rust crate inflating build/PVC pressure** → pure local crate, `CARGO_TARGET_DIR` pooled, no
  edge/Nexus wiring; test-only materialization.
- **Shared-worktree cross-commit** → commit path-scoped to the epr-meta concern only.

## Delivered (2026-07-10)

Implemented subagent-driven over 10 commits (`aefa1b2cf..8f322dfdf` on `feat/frontend-eyes-sprint`),
each per-task reviewed clean. Full-suite green: package `verify` 848, elohim-agent manifest 22,
Python `.epr-meta` cascade+parity 21, `ci_trigger` 12, cargo `eprfs-meta` + `elohim-agent-adapter`
all pass, `cargo fmt` + `git diff --check` clean.

- **A** — `capability-governance@1` (measure class) binds at `.epr-meta/elohim/packages/.epr-meta`;
  every package + Codex projection carries the `EprRef` backref; `verify` red/greens it and lodges to
  `.claude/data/governance-findings.jsonl` (dedup-fingerprinted); `.husky/pre-push.bash` gates drift.
  The root `.epr-meta` flat→directory migration (`manifest.md`) was also committed.
- **B** — `verifySourceFidelity` standing gate (`project(import(source))===source`, +58 assertions);
  shared Python↔Rust parity fixtures proving both interpreters resolve identical ordered rule-ids
  (SOTU closeout #1); directionality contract in the `elohim-package-authoring` skill + domain
  CLAUDE.md.
- **C** — `elohim-agent-adapter` crate (homed in the `elohim/eprfs` workspace) maps the package tree
  to a validated `eprfs-core::ProjectionManifest` and round-trips it through `eprfs-local`
  byte-identically (SOTU closeout #5) — the package layout is a proven portable content-addressed
  graph.

Deferred as designed (the elohim ceiling): the behavioral judge, p2p-substrate deep-trust
resolution of the `EprRef`, the value-plane REA usage meter, and cross-repo CLI root-configurability.
