---
title: "`.epr-meta` — The Directory-Local Compose-Gate (P1)"
id: epr-meta-compose-gate
tier: spec
status: Draft
created: 2026-06-25
maintainers: Matthew Dowell + Opus 4.8
class: process-meta
process_subdomain: doc-lifecycle
topic: [epr-meta, compose-gate, content-addressing, cascade, recursion-guard, validator-epr, schema-first, governance, self-hardening, dag-cbor, canonical-bytes]
context-tier: disclosed
steward: cartographer
graduation-trigger: decompose-complete OR superseded-by-implementation
refines:
  - genesis/docs/superpowers/specs/2026-06-25-doc-lifecycle-as-epr-development-substrate-design.md
cites:
  - doc-lifecycle-as-epr-development-substrate | Doc-Lifecycle as EPR | sha256:4b87bca1eb683441 | path: genesis/docs/superpowers/specs/2026-06-25-doc-lifecycle-as-epr-development-substrate-design.md
  - .claude/skills/epr-content-addressing/SKILL.md
  - elohim-seam-map-concern-routing | The Elohim Seam Map | sha256:54b5809fb8e688d1 | path: genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md
  - placement | Genesis Docs Placement Contract | sha256:95be31e6724bb9f5 | path: genesis/docs/PLACEMENT.md
---

# `.epr-meta` — The Directory-Local Compose-Gate (P1)

> **Scope.** This is **P1** of the framing spec
> (`2026-06-25-doc-lifecycle-as-epr-development-substrate-design`). It pins the one new primitive: the
> `.epr-meta` file format, its schema, the rule vocabulary, the cascade, the single generic resolver,
> and the recursion guard (including the §3.3 skip-marker mechanism the framing spec left open). It is
> the foundation every other slice builds on, so it ships first (after P0).

## 1. Two forms: authored source ↔ canonical envelope

`.epr-meta` exists in two representations — the same authored-intent / derived-state split the whole
arc rests on:

- **Authored source — frontmatter-YAML + markdown body.** What a developer (or the elohim) hand-writes
  and commits to git. The **frontmatter** carries the governance + app-manifest legs (structured,
  schema-validatable YAML); the **markdown body** carries the knowledge leg (the directory's
  human-readable manifest — so an `.epr-meta` reads like the docs it governs). This is the form the
  cascade walks, `git diff` shows, and the cite tooling seals.
- **Canonical envelope — DAG-CBOR → CID.** The durable, language-agnostic, content-addressed form,
  produced by `elohim-epr`'s `canonical_bytes` (`elohim/epr/src/envelope.rs`) → `compute_cid`
  (`elohim/epr/src/cid.rs`). This is the bootstrapping/import/interop artifact: what the seeder
  imports, what peers exchange, what the DHT notarizes. **No new format is invented** — it is the EPR
  wire format the protocol already ships (DAG-CBOR with a plain-text fallback; the ~500-byte envelope).

**The CID is computed over the canonical bytes, not the source file** — so reformatting whitespace or
editing the prose body never changes the atom's identity. Same canonical fields → same CID → *the same
atom*, which is exactly what makes one developer's `.epr-meta` authoritative to another (§4 of the
framing spec). The projector (`elohim-fs-projector`, §13 thin slice) compiles source → canonical
envelope; it never hand-mirrors `canonical_bytes` (framing guardrail 9).

## 2. The schema (schema-first, three legs)

The `.epr-meta` frontmatter is governed by a JSON schema in `elohim/sdk/schemas/v1/` (alongside the
view schemas), validated the same way the wire shapes are: **author the schema first → Rust/TS comply
via codegen → a contract test catches drift.** The resolver validates a parsed `.epr-meta`'s
frontmatter against this schema before applying any rule. A malformed `.epr-meta` is
**strict-but-recoverable** (§5), never an unrecoverable hard `deny`.

The three legs, as frontmatter keys:

| Leg | Frontmatter | What it holds |
|---|---|---|
| **Knowledge** | `purpose:` + the markdown body | what this directory *is*; what belongs; prose manifest |
| **Governance** | `rules:` | the compose-gate rules (the closed vocabulary, §3); each rule carries its own enforcement `class` |
| **App-manifest** | `validators:` + `extends:` + guard config | *which* validator-EPRs realize the rules and *how* they execute (CID-pinned, fuel-bounded); the cascade pointer |

Reserved top-level keys: `epr-meta-version` (integer, for migration), `id` (slug → CID), `extends`
(cascade parent path, or `root: true`), `covers` (`subtree`|`dir-only` — coverage-walk responsibility,
§4b), `max-cascade-depth`, `cites:`.

## 3. The rule vocabulary (closed set + named-validator escape)

Rules are **declarative**, drawn from a small **closed vocabulary** — never inline code (that would
recreate the bloated surface `.epr-meta` exists to prevent). Anything needing real logic references a
**validator-EPR by CID** (the escape hatch). Each rule is a mapping with a `when:` matcher, a
predicate, an enforcement `class`, and a `why:`.

Closed vocabulary (v1):

| Rule key | Meaning | Typical class |
|---|---|---|
| `require-frontmatter: [fields]` | new file must carry these frontmatter fields | `deny` |
| `allowed-types: [globs]` | only these artifact types may be authored here | `deny`/`ask` |
| `route-to: { type: glob, dest: path }` | a misplaced type routes elsewhere (with the message) | `ask` |
| `no-new-subdirs: true` | forbid new child directories | `deny` |
| `require-sibling: ".epr-meta"` | a new subtree must carry its own `.epr-meta` (no-orphan-tree) | `deny` |
| `dedupe-of: path` | this concern already lives at `path` (anti-duplicate) | `ask`/`deny` |
| `max-files: { glob, n }` | local bloat ceiling → emit a count signal | `measure` |
| `measure: { count: glob, emit: name }` | emit a deterministic counter (no block) | `measure` |
| `validator: epr:<name>` | delegate to a validator-EPR (the escape) | any |

**Enforcement classes** (the ladder, per rule): `deny` (block) · `ask` (prompt) · `inject` (advise,
proceed) · `measure` (counter only) · `dispatch` (fire background work). Authority (§7): an agent may
author `inject`/`measure`/`ask`/`dispatch` rules autonomously; a new `deny` requires operator approval.

## 4. The cascade

Resolution walks **up** the directory tree from the target path, collecting `.epr-meta` files —
`.gitignore`/`.editorconfig`-style — until it hits a `root: true` base case (the repo-root
constitutional `.epr-meta`, held by the human commons-steward). Merge semantics:

- **Nearest-ancestor wins on conflict** for a given rule `id`; a child may **override or relax** a
  parent rule by re-declaring the same `id` (with the change logged).
- Rules **accumulate** across the chain (a child adds to, doesn't replace, the parent's rule set)
  unless a rule `id` collides.
- `extends:` may name a non-ancestor `.epr-meta` (shared governance) **by CID** — these are merged
  too, subject to the recursion guard (§5).

## 4b. Coverage — `covers: subtree` and the directory-governance signal

The cascade (§4) answers "what governs *this* write?" — a single edited path, resolved upward. The
**coverage** question is the dual: *is every governable region of the codebase owned by some
self-responsible manifest?* — the directory-level analog of how a CLAUDE.md owns progressive init
context. It needs one self-contained declaration and one deterministic walk.

- **`covers: subtree`** (top-level, opt-in) — declares the manifest **fully responsible for everything
  beneath it**. It is the *downward* dual of `root: true` (which terminates the cascade *upward*):
  `root` says "inherit nothing from above me"; `covers: subtree` says "own everything below me." Opt-in
  is load-bearing — an incidental manifest (a `ci-trigger` config, one local rule) must NOT trivially
  claim the whole repo, so absence (or `dir-only`) means "does not claim the subtree."
- **The walk** (`subtree_coverage` in `_lib/epr_meta.py`, surfaced by `placement-audit.py --epr-meta`)
  descends the file graph; on a `covers: subtree` manifest it marks the subtree OWNED and **terminates**
  — *integrity by construction*, the claimed subtree's internals are never re-audited, exactly as the
  core never re-validates an app-manifest's vocabulary (seam-map §3.7, the core/app-manifests split). A
  structurally-substantial directory reached with no covering ancestor is an unclaimed **GAP**.
- **Ownership ≠ enforcement.** A `covers: subtree` claim with *no* rules is the *considered-coverage*
  outcome — "this region is owned; no edit-time gate is warranted yet" — which is why coverage can reach
  1.0 without spraying gates that would fire on nothing (the M2 footgun §3 guards). Finer rule-bearing
  manifests still cascade *inside* a claimed subtree; they enforce, they don't re-claim.
- **The signal.** Coverage = owned regions / (owned + gap) regions; it rides the `epr_meta_coverage`
  stasis dimension and the `epr-meta:` SessionStart headline token, with substantiality thresholds +
  exclusions tuned in `context-coverage.yaml` (`epr_meta_governance`). This is *prevention as a measured
  surface* — the complement to the stray-doc census (remediation): the gate makes new docs born-governed;
  the coverage signal makes the *standing* governance debt visible and drives it toward stasis, region by
  region, each resolved by ONE claim at the altitude an author is willing to vouch for.

## 5. The resolver — one generic engine

A single, small, generic engine (PreToolUse on `Write`/`Edit`, and on directory creation). It does not
contain any directory's rules — it *interprets* them. Algorithm:

1. From the target path, walk the cascade (§4), collecting `.epr-meta` source files.
2. Health-check each against the schema (§2). A malformed `.epr-meta` is **strict-but-recoverable**
   (operator decision, 2026-06-25): it does NOT hard-`deny` the subtree — editing the `.epr-meta`
   itself is never blocked (the typo is always fixable), and other writes downgrade `deny → ask`
   (overridable) until it is fixed. An unvalidated manifest is Private-reach authored intent that has
   not earned cross-machine DHT notarization, so its governance is *proposed* (`ask`), not *binding*
   (`deny`). A 64KB size cap + a flow-depth guard refuse a parse-bomb before PyYAML (no RecursionError).
3. Merge the rule sets (§4); resolve any `validator:` references to validator-EPRs by CID.
4. Evaluate each rule against the proposed write. **Guards are pure** (§6): input = (proposed write,
   merged rules); output = a verdict (`deny`/`ask`/`inject`/`measure`/`dispatch`) + payload. The guard
   itself performs **no writes**.
5. Combine verdicts (most-severe-wins: `deny` > `ask` > `inject`; `measure`/`dispatch` are
   side-channels). Return to the harness.
6. **The host (resolver), not the guard, performs side-effects**: increment a `measure` counter, fire a
   `dispatch`, log an override. These writes go to non-governed paths (e.g. `.claude/data/*.jsonl`) or
   carry an origin marker (§6.3) so they never re-enter the resolver.

The resolver reuses the repo's existing frontmatter parser and the cite/managed-surface plumbing; it is
the matured, generalized form of the central `managed_surfaces.py` + the would-be frontmatter-DENY hook.

## 6. The recursion guard (three surfaces, three terminations)

A fractal, self-referential governance graph must provably terminate.

### 6.1 Immutable CID graph — acyclic by construction
A `.epr-meta` (or validator-EPR) references another **by CID**; to embed B's CID, B must already be
sealed. A cycle A→B→A cannot seal (chicken-and-egg). The reference graph is therefore a **Merkle DAG**,
exactly like git/IPLD. Free, and the deepest guarantee. (Applies to `extends:` and `validator:`
references in their *canonical* form; see 6.3 for mutable source-path references.)

### 6.2 Cascade walk — `root: true` base case + depth fuel
The upward cascade (§4) terminates at the `root: true` constitutional `.epr-meta`. A `max-cascade-depth`
(default 8) bounds it as belt-and-suspenders. A cascade that exceeds depth without reaching a root is a
misconfiguration; v1 **advises** (the same strict-but-recoverable posture as a malformed manifest, §5),
with a hard `deny` deferred until the constitutional root is repo-wide.

### 6.3 Guard execution — pure guards + non-reentrant side-effects + fuel + visited-set
The framing spec left the skip-marker mechanism open. **Pinned here:**

- **Guards are pure functions** (§5.4): they read and return a verdict; they never write. This alone
  eliminates the dominant recursion risk — a guard cannot trigger itself because it produces no write.
- **Host side-effects are non-reentrant.** The resolver's own writes (counters, dispatch ledgers,
  override logs) target **non-governed paths** (outside any `.epr-meta`-governed tree, e.g.
  `.claude/data/`) **or** carry an `origin: epr-meta-resolver` marker the resolver skips on re-entry.
  This is the matured `cooldown-silenced`/`once-per-session` pattern the guardrail audit found in
  `p2p-plan-audit`.
- **Validator-EPR chains spend fuel.** A validator that delegates to another (by CID) decrements a
  `fuel` budget (declared per-validator in the app-manifest leg, default 200); fuel exhaustion → fail
  closed. Combined with 6.1's acyclicity this is doubly-bounded.
- **Mutable-pointer resolution uses a visited-CID memo.** Source-path `extends:` (pre-CID) and any
  `@latest`/supersedence resolution — the *only* places a cycle can reappear before sealing — track a
  visited-CID set; a re-encounter stops the walk.

## 7. Self-hardening & authority

- **Autonomous soft, operator-gated hard.** An agent may author `inject`/`measure`/`ask`/`dispatch`
  rules autonomously; a new `deny` rule requires operator approval (a hard block is a governance act).
- **Wrong rules self-surface.** Every fired rule that is then **overridden** is logged (host
  side-effect, §6.3). A rule whose override-count crosses a threshold raises a `bad-rule` drift signal
  (a `measure` on the resolver itself) for review — so a wrong rule surfaces instead of silently
  blocking work.
- **The canon becomes a local executable rule.** When an agent cleans up a drift (a dump, an orphan
  tree), it writes the `.epr-meta` rule that prevents the recurrence — the `flag → agent → canon →
  stasis` loop with the *canon* as a co-located rule, not a backlog entry.

## 8. A concrete `.epr-meta` (poke holes in this)

`genesis/docs/superpowers/specs/.epr-meta`:

```yaml
---
epr-meta-version: 1
id: specs-dir-governance
extends: ../.epr-meta                 # cascade up; repo-root .epr-meta carries `root: true`
purpose: >
  Design specs — the 'what/why', born from /brainstorm. Authored intent that
  decomposes to gaps (decompose.py) and graduates to history; never parked.
rules:
  - id: spec-frontmatter-at-birth
    when: { write: "*.md", new: true }
    require-frontmatter: [id, status, class, context-tier, steward, graduation-trigger, cites]
    class: deny
    why: "No spec born without its three legs + lifecycle fields (framing §2)."
  - id: route-plans-out
    when: { write: "*-plan.md" }
    route-to: { type: "*-plan.md", dest: genesis/docs/superpowers/plans/ }
    class: ask
    why: "Plans live in plans/, not specs/."
  - id: pile-bloat
    measure: { count: "*.md", emit: specpile-specs-count }
    class: measure
    why: "Feed the BLOATED gate-line (framing §12 P2); ratio judged centrally."
  - id: p2p-design-gate
    when: { write: "*.md", contains-any: ["GET /api/v1", "PRIMARY KEY", "uuid"] }
    validator: epr:validator-p2p-design-gate
    class: ask
    why: "Data-entity designs pass the p2p-design-gate before REST-shaping."
validators:
  - ref: epr:validator-p2p-design-gate
    cid: bafy...                       # CID-pinned; the .py is itself an EPR atom
    fuel: 200
max-cascade-depth: 8
cites:
  - 2026-06-25-doc-lifecycle-as-epr-development-substrate-design
---

# specs/ — what lives here

The 'what/why' of a change, born from `/brainstorm`. Each spec decomposes to gap-items and either
graduates to `history/` or is superseded — it is never parked in the live tree. Live status for any
spec lives in its gap-items + the ledger, not in prose here.
```

## 9. Acceptance criteria (the DoD this spec must earn)

1. The `.epr-meta` JSON schema exists in `elohim/sdk/schemas/v1/` with a contract test.
2. The projector compiles a source `.epr-meta` → canonical envelope via `elohim-epr` (no hand-mirrored
   `canonical_bytes`); reformatting the source body leaves the CID unchanged.
3. The resolver: walks the cascade, validates against schema, merges rules, evaluates pure guards,
   returns combined verdict; wired PreToolUse on `Write`/`Edit` + dir-create.
4. Recursion guard (v1 half): the cascade is depth-bounded (`MAX_CASCADE_DEPTH`) and stops at
   `root: true`; a missing-root cascade **advises** (strict-but-recoverable, §6.2); a parse-bomb
   manifest is refused pre-parse (64KB size + flow-depth caps, no RecursionError). The `extends:`-by-CID
   visited-set and validator-chain fuel ship with the CID-pinned validator slice (deferred).
5. A seeded repo-root `root: true` `.epr-meta` + a `specs/.epr-meta` (the §8 example) demonstrably
   `deny` a frontmatter-less new spec and `ask`-route a `*-plan.md` written into `specs/`.
6. Self-hardening: an overridden rule increments the `bad-rule` counter; a new `deny` rule authored by
   an agent surfaces for operator approval rather than taking effect silently.

## 10. Defers / what the plan must verify against code

- **Verify against code in the plan** (not assumed here): the exact `elohim-epr` envelope/`canonical_bytes`
  API and whether a non-content `.epr-meta` atom-kind needs an `EprKind` variant; the frontmatter parser
  the resolver reuses; the PreToolUse hook-registration shape in `settings.json`; whether the validator-EPR
  execution sandbox already exists or is new.
- **Defers to other slices:** the attention-conservation `measure` semantics (P5); the DoD `dod:`
  gate-line (P3); the full recognition/liability primitive (P5). P1 only needs the rule *vocabulary* to
  *name* a `measure`/`dispatch`, not the downstream economy.
- **Defers:** validator-EPR *authoring* ergonomics (how the elohim writes a new validator-EPR) — v1
  ships the resolver + the closed vocabulary + one reference validator-EPR (the p2p-design-gate, cloned
  from the existing `p2p-plan-audit` detector).

## Related

- Framing spec: `genesis/docs/superpowers/specs/2026-06-25-doc-lifecycle-as-epr-development-substrate-design.md`
- `.claude/skills/epr-content-addressing/SKILL.md` (envelope, DAG-CBOR, `canonical_bytes`, `compute_cid`)
- `genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md` (manifest→SDK seam)
- `genesis/docs/PLACEMENT.md` (the contract `.epr-meta` decentralizes into executable rules)
- `elohim/epr/src/envelope.rs` · `elohim/epr/src/cid.rs` · `elohim/sdk/schemas/v1/views/CONVENTIONS.md`
