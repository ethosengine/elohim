---
name: elohim-epr-metafile
description: Authoring guide for `.epr-meta` — the directory-local compose-gate manifest read by the epr-meta-resolver PreToolUse hook. Use when establishing or tightening governance for a directory — doc-shaped or code (the latter governing the over-engineering/YAGNI drift) — or (the main loop) when closing a drift you just cleaned by writing the co-located rule that prevents the recurrence. Teaches the closed signal vocabulary, the enforcement-class ladder, the cascade, and — the heart of it — how to choose the lightest signal that drives a directory toward stasis instead of nagging. Companion to the spec (mechanism) and to p2p-design-gate / epr-content-addressing.
metadata:
  runtime: codex
  sourceRuntime: claude
  master: package
  sourcePath: .epr-meta/elohim/packages/skills/elohim-epr-metafile.json
  packageKind: SkillPackage
governance: "epr:elohim-agent/skills/elohim-epr-metafile"
---

# Authoring `.epr-meta` — Directory-Local Governance That Converges

An `.epr-meta` is a directory's own manifest, written in the same frontmatter-YAML + markdown
the directory's docs use. It says three things: **what lives here** (the prose body + `purpose:`),
**what belongs** (the `rules:` — the compose-gate signals), and **which EPRs realize the harder
rules** (`validators:`). The `epr-meta-resolver` PreToolUse hook reads it on every `Write`/`Edit`
under it and returns a verdict.

It is the **canon step of `flag → agent → canon → stasis`, made co-located and executable.** When
you clean up a drift — a dump in the wrong directory, an orphan subtree, a frontmatter-less doc — the
old reflex is to file a backlog entry. The `.epr-meta` reflex is to write the *rule* that prevents the
recurrence, right where it recurs. A good manifest, applied over rounds, holds its directory at low
drift. A bad one nags forever. **This skill is about telling them apart.**

> **You are authoring judgment, not mechanism.** The spec
> (`genesis/docs/superpowers/specs/2026-06-25-epr-meta-compose-gate-design.md`) is the mechanism —
> cascade, recursion guard, schema. This skill is *which signal, at which class, with what `why:`* —
> the part that decides whether the gate converges or just annoys.
>
> **And it is honest about what v1 actually enforces.** Several vocabulary keys are *declared but not
> yet wired* (called out inline below). The gate is young; the spec describes its full intent, the code
> is partway there. Authoring against an unwired key produces silent inertia — the single worst outcome
> for a governance tool — so the enforced-vs-reserved line is load-bearing. Trust the **Enforced in v1**
> column; treat everything marked **reserved** as documentation of intent.

## When to reach for this skill

- **Establishing governance** for a directory that has none (a new authored surface that will accrue
  files — specs, plans, research, a content tree).
- **Tightening a child** — a subdirectory needs a stricter rule than its parent (nearest-wins override).
- **Closing a drift** (the main loop) — you just cleaned something; write the rule so it can't recur.
- **Onboarding a new subtree** — a new directory under a governed root needs its own `.epr-meta`
  (the no-orphan-tree rule).

If you are about to add a *backlog entry* describing a directory-hygiene rule you wish were enforced,
stop — that rule wants to be an `.epr-meta` rule, co-located with the drift, not a note someone reads later.

## The mental model (carry these four into every rule you write)

1. **Three legs, one file.** Knowledge (`purpose:` + body — what this directory *is*) · Governance
   (`rules:` — the signals) · App-manifest (`validators:` — which EPRs realize the rules, CID-pinned,
   fuel-bounded). A rule with no knowledge leg behind it (no clear "what belongs here") is a rule you
   can't justify.
2. **Authored source ↔ canonical envelope.** What you hand-write and commit is the *source*. Its
   durable identity is the DAG-CBOR → CID *envelope* computed over the canonical fields — so reformatting
   the body never changes the atom, and the *same* canonical rule is authoritative to another machine.
   You author source; the projector mints the envelope. (See `epr-content-addressing`.)
3. **Reach is earned at compose, never asserted.** A rule's authority is something it *earns* (by being
   a valid, attested, ultimately notarized governance atom), not something it declares. Author boldly,
   but know what your manifest has earned: a brand-new local `.epr-meta` is valid governance the moment
   it parses, but it is *Private-reach* — local intent that hasn't earned cross-machine notarization, so
   another peer is not bound by it until the substrate agrees. (Don't overload this: the malformed →
   `ask` softening below is about **validity**, not reach-tier — see §strict-but-recoverable.)
4. **The manifest reads like the docs it governs.** The body is real prose a human reads to learn
   what the directory is for. Don't write a config file with a comment; write a manifest with rules in it.

## The one principle: signals that converge (the stasis test)

A rule earns its place only if, applied over rounds, it holds the directory **at** low drift. But
"converge" means different things for two kinds of rule — don't apply the same test to both:

- **Teaching rules** decay as authors learn the convention (`require-frontmatter` against repeat
  authors, `route-to` against humans who relocate). For these, *firing less over time is the success
  signal* — if it'd fire just as often six months on, it's tolling, not teaching; re-shape it.
- **Structural guardrails / invariants** fire at the rate of *naive input* and should keep firing
  (`no-new-subdirs`, `require-sibling`/no-orphan-tree, and `dedupe-of` on an accumulating directory).
  A steady fire rate here is the floor doing its job — every naive attempt caught — **not** a failure.
  Don't "re-shape it for firing less"; its convergence is *coverage*, not decay.

Before you commit a rule, run it through five questions. If it fails any, it's noise.

1. **Does it name a *real, recurring* drift you have actually seen here?** Not a hypothetical. The rule
   is the canon you write *after* cleaning the drift the first (or third) time. No lived drift → no rule.
2. **Is it the *lightest intervention* that addresses the drift?** The ladder is `inject` (advise) →
   `dispatch` (continue + named background reviewer) → `ask` (stop for the pilot) → `deny` (block).
   Most directory hygiene wants `dispatch`: placement, duplication, generated-file discipline,
   architecture coaching, and ontology review are advisor work. Reserve `ask` for consequential
   authority, confidentiality, resource, irreversibility, or genuine unresolved vision decisions.
3. **Does its `why:` teach the fix?** The author hits the `why:` *as an instruction* — it must name the
   destination, the missing field, the canonical path. "Plans live in `plans/`, not `specs/`" converges;
   "invalid location" nags.
4. **Is it overridable?** You don't have to be right. A wrong `ask` costs one keystroke to override; a
   wrong `deny` silently blocks a teammate — that asymmetry is *why* the class ladder matters. (Note:
   the spec's self-hardening — counting overrides to raise a `bad-rule` signal — is **not wired in v1**,
   so there is no automatic "wrong rules surface" net yet. Author conservatively and lean on review.)
5. **Does it match at the right *moment*?** Birth-time concerns (placement, frontmatter, orphan-tree)
   gate at creation only — `when: { new: true }` — so editing a file to *fix* it is never trapped. A
   teaching rule that fires on every edit is a rule that fights maintenance.

## The signal vocabulary — enforced in v1 vs. reserved

The vocabulary is a **closed set** — declarative rules, never inline code (inline logic would recreate
the bloated surface `.epr-meta` exists to prevent). Anything needing real logic delegates to a
**validator-EPR by name** (the escape hatch). Each rule is a mapping: an `id`, an enforcement `class`,
one predicate, a `when:` matcher, and a `why:`.

**Enforced in v1** — these produce real verdicts today (`_eval_rule` in `.claude/scripts/_lib/epr_meta.py`):

| Predicate | Fires when… | Verdict carries | Typical class |
|---|---|---|---|
| `require-frontmatter: [fields]` | a matched write's frontmatter is missing any listed field | the missing fields + `why:` | `deny` (at birth) |
| `route-to: { dest: path }` | `when:` matches (the `when:` glob *is* the misplacement test) | "`<file>` routes to `<dest>`" + `why:` | `ask` |
| `no-new-subdirs: true` | the write creates a new child directory | "new subdirectories are not allowed here" + `why:` | `deny` |
| `require-sibling: ".epr-meta"` | a new subtree is created without its own `.epr-meta` | "a new subtree must carry its own `.epr-meta`" + `why:` | `deny` |
| `dedupe-of: path` | `when:` matches (anti-duplicate; the glob is the trigger) | "this concern already lives at `<path>`" + `why:` | `ask`/`deny` |
| `validator: epr:<name>` | the named validator-EPR returns true for the write | "validator `<name>` flagged this" + `why:` | any¹ |
| `measure: { loc-soft, loc-hard }` | post-edit line count at/over a ceiling | over `loc-hard` → a `measure` verdict (fingerprinted architecture finding in `.claude/data/architecture-findings.jsonl` + review-dispatch directive); over `loc-soft` only → an `inject` nudge, debounced per path | `measure` (never blocks) |
| `policy: <id>@<version>` (binding, not a predicate) | always — expands at resolve time into the registry policy's concrete rule | the policy's class/predicate/why; the binding adds only `params:` / `when:` local variance | whatever the policy declares |

¹ **Only `epr:validator-p2p-design-gate` is registered in v1.** A `validator:` ref *not* in the registry
(`REFERENCE_VALIDATORS`) silently degrades to an `inject` advisory **regardless of the rule's declared
class** — so `class: deny` + an unregistered ref fires only an advisory, not a block. `validate_meta`
won't warn. Until a new validator-EPR is wired into the registry, treat its rule as advisory-only.

> `route-to`'s `type:` subkey (seen in older examples) is **documentation-only** — the evaluator reads
> only `dest`; `when:` does all the matching. Don't rely on `type:` to narrow which matched files route.

**Reserved (declared, schema-valid, NOT yet wired in v1)** — author these only with eyes open; **they
currently fire on nothing.** Treat them as documentation of intent, not enforcement:

| Predicate / key / norm | State in v1 | What to do instead, today |
|---|---|---|
| `allowed-types: [globs]` | **Phantom** — no `_eval_rule` branch; validates clean, counts as "actionable", produces no verdict | express the intent with `route-to` (misplaced type → dest) or `require-frontmatter`; do **not** rely on `allowed-types` to block anything |
| `measure: { count, emit }` / `max-files: {…}` | **Inert** — only the LoC-ceiling keys (`loc-soft`/`loc-hard`) are wired; count-shaped measures and `max-files` produce no verdict | if you need a count signal now, emit it from a deterministic script into `.claude/data/*.jsonl` |
| `max-cascade-depth: N` (top-level) | **Inert** — `collect_cascade` always bounds on the hardcoded `MAX_CASCADE_DEPTH = 32`; the per-manifest key is parsed and discarded (there is no "default 8") | rely on `root: true` to terminate the cascade; the depth cap is a fixed 32 you cannot tune per-directory |
| `extends: <path>` (top-level) | **Inert** — the cascade parent is pure filesystem ancestry; `extends:` is never read (a non-ancestor pointer is a silent no-op) | place a directory under the governed tree to inherit; shared/non-ancestor governance is a deferred slice |
| self-hardening `bad-rule` override counter | **Not wired** — overrides aren't counted | rely on review |
| **operator-approval for a new `deny`** | **Convention only — NOT gate-enforced** (see below) | author the `deny`, then *surface it* to the operator yourself; the gate will not stop you |

**Enforcement classes (the ladder, per rule):**

- `deny` — block the write.
- `ask` — stop for the pilot. Consequential decisions only.
- `inject` — advise and proceed (adds context, never blocks). Free to author.
- `measure` — the observation tier: never blocks, feeds signal ledgers. **Live for LoC ceilings**
  (soft → debounced nudge; hard → fingerprinted architecture finding + dispatch directive, the
  flag→agent→canon→stasis shape). Count-shaped measures remain reserved.
- `dispatch` — permit plus a mandatory named background-review directive. The current session agent
  launches it through the Agent tool; the hook itself cannot spawn an agent process.

> **Authority is gate-enforced.** Agents may author `inject`/`measure`/`dispatch` rules autonomously.
> Introducing `ask` or `deny` requires a version-pinned registry policy with deliberation provenance;
> the repo-root `governance-escalation-ladder` routes an unbacked introduction to pilot review. This is
> also the interruption budget: routine judgment belongs in dispatch, and a new pilot stop must name
> the consequential authority it protects.

## The policy registry — define once, bind many

A rule wanted in more than one directory (or one that is really a repo-wide norm) is a **policy**,
not an inline rule. Policies live in the registry `.claude/epr-meta/policies.yaml` as
Precedent-shaped objects (the Mishpat::Precedent lineage: binding ladder, scope, why,
supersession) — the graduated home when epr-meta lifts into brit/eprfs is a Mishpat `Precedent`
entry (CID = entry_hash), with manifest bindings becoming cites. A manifest binds one with:

```yaml
rules:
  - id: rs-loc-ceiling                      # the binding's local id
    policy: source-file-loc-ceiling@1       # registry id @ version — the pin is REQUIRED
    params: { loc-hard: 9000 }              # optional local variance, merged over the policy's measure
    when: { write: "*.rs" }                 # optional scope override
```

Contract, enforced by `validate_meta` + `expand_policies`:

- **The policy owns semantics** (class, predicate, measure defaults, `why:`); **the binding owns
  placement** (`when:` override, `params:`). A binding that redeclares class/predicates is
  schema-invalid.
- **The version pin is a declared dependency, never recency.** Tightening a policy = adding a new
  version entry; every binding keeps its old semantics until it re-declares. `status: superseded`
  + `superseded_by` record lineage; never delete a version that still has bindings.
- **Unknown/unpinned refs fail LOUD**: the rule is dropped with an advisory (never silently kept,
  never silently enforced-as-something-else). A silently vanished `deny` would be silent-allow —
  the advisory is the tell.
- **Inlining a rule whose id exists in the registry** draws a dedupe advisory: bind, don't redefine.

## Choosing a signal: the decision procedure

For the directory in front of you:

1. **Name what it is** (the knowledge leg). One sentence of `purpose:` + a short body: what artifact
   types belong here, what this directory is *for*. If you can't, you're not ready to govern it.
2. **Name the drift you keep cleaning here.** Be concrete — "loose `*-plan.md` files land in `specs/`",
   "new docs show up without `cites:`", "someone starts a sibling tree with no manifest".
3. **Map drift → predicate** (use only the *enforced* column):

   | The drift | The signal |
   |---|---|
   | docs born without required metadata | `require-frontmatter: [...]` at `class: deny`² , `when: { new: true }` |
   | right artifact, wrong directory | `route-to: { dest }` at `class: ask` |
   | this concern already lives elsewhere | `dedupe-of: <path>` at `class: ask` |
   | subtree sprawl / flat-dir invariant | `no-new-subdirs: true` at `class: deny` |
   | new subtrees with no governance of their own | `require-sibling: ".epr-meta"` at `class: deny` |
   | a content-shaped concern needing real logic (e.g. a P2P-design check) | `validator: epr:<name>`, class to taste |
   | god-file growth (a source file outgrowing review/refactor scale) | bind `policy: source-file-loc-ceiling@1` (or re-bind with `params:` for a region's legitimate variance) |
   | the same rule wanted in a second directory | STOP inlining — lift the definition into the policy registry and bind `policy: <id>@<version>` in both places |

   ² `deny` is justified here only because a doc with no `cites:`/`id` genuinely *can't* be linked into
   the substrate — proceeding is broken, not merely untidy (see step 4 / §Authority). Otherwise, step 4.

   **If no row above fits the drift, STOP — do not author a rule.** This binds `inject` too: an advisory
   is still a rule and still needs one of the *enforced* predicates. A bare `when:`+`why:` with no
   predicate is the `fires on nothing` footgun — and a predicate-less rule is a **malformed** manifest,
   so it doesn't merely no-op: strict-but-recoverable downgrades *every* write in the subtree to `ask`
   until you fix it (a toothless `inject` is **worse** than no rule — it tolls the whole directory). Do
   what the reserved-key and filename-pattern cases do: document the convention in the body as
   "authoritative, not gate-enforced in v1" and author only the predicates that *do* fit (e.g. the real
   `no-new-subdirs` invariant the directory still exposes). Don't reach for `require-sibling` to rescue a
   companion-file concern ("every `X.rs` needs a test"): it is **narrow** — it fires only when a *new
   subtree* is created and checks that the first file written is the named sibling (its sole real use is
   "a new dir must be born with its own `.epr-meta`"). It cannot pair `X.ts` with `X.spec.ts` in an
   existing directory, and can't see an inline `#[cfg(test)]` test. When no predicate fits the actual
   drift, decline and document.

4. **Pick the lightest class that addresses it.** Default routine judgment to `dispatch` with an
   existing packaged agent and a bounded, report-only prompt. Use `ask` only when the editing agent and
   reviewer cannot safely decide for the pilot; `ask` and `deny` require a ratified policy binding.
5. **Write the `why:` as the fix instruction** — name the dest/field/path the author needs.
6. **Scope the `when:`.** `write: "<glob>"` matches the **filename only** (basename, case-insensitive).
   Add `new: true` for birth-time concerns. Add `contains-any: [...]` for content-triggered rules
   (the p2p-design-gate pattern). An absent `when:` matches every write — usually too broad, and (for an
   accumulating directory) the difference between a guardrail that *catches duplicates* and one that
   *tolls every legitimate addition*.

## Governing a code directory — minimalism is the drift

The predicates above are doc-shaped — frontmatter, placement, dedupe. A *code* directory's most
common drift is a different animal: **over-engineering** — an agent installing a framework for a date
picker, hand-rolling what stdlib already does, building a component where a native element exists.
That's content-shaped, not placement-shaped, so it takes the `validator: epr:<name>` escape hatch (the
p2p-design-gate shape), never a placement predicate.

The check worth encoding is a **YAGNI decision-ladder**, run *before* the code is written (the idea is
borrowed from ponytail's agent-minimalism framework — "the best code is the code you never wrote"):

1. Does it need to exist at all? — skip if no
2. Already in the codebase? — reuse
3. Stdlib or a native platform feature covers it? — use it
4. An installed dependency covers it? — use it
5. A one-liner? — one line
6. Only then write the minimal code — with the safety floor intact (validation, error handling,
   security, accessibility are never the thing you skip)

**Be honest about v1.** No YAGNI validator-EPR is registered (only `epr:validator-p2p-design-gate` is
wired — see the reserved table), so a `validator: epr:yagni` rule degrades to an `inject` advisory
today. That's still a useful born-in-the-directory nudge — but document the ladder in the `.epr-meta`
body as *authoritative, not yet gate-enforced*, and lean on review until the validator-EPR is wired,
exactly as the reserved keys instruct.

The deeper rule is self-referential, and it's the whole reason this lands here: **the rule that asks
for minimal code must itself be minimal.** `inject`/`ask`, never a reflexive `deny`; the lightest
signal that converges (§the stasis test) governing the lightest code that works. A heavy gate demanding
lean code is its own anti-pattern.

## Habits — the other atom in the package (2026-08-27)

An `.epr-meta` directory holds more than `manifest.md`. A **habit** — what this system reliably
does, bound to a runnable check — is declared as its own atom beside it:

    <dir>/.epr-meta/<id>.habit.md      frontmatter = declaration, body = evidence ledger

Placement is the whole point: the habit lives in the governance package of the directory whose
**behaviour** it describes, so its scope is the scope you already resolve. It is NOT a second
scope authority mirroring `seam-registry.yaml` — that would be two hand-written homes for one
truth, a failure mode this repo has paid for twice (`cluster-state.yaml` vs
`ELOHIM_REMOTE_COMPUTE_STATUS`; the `deployments.json` `suspended` flags that drifted until they
were made derived). `genesis/manifests/habits.yaml` is the **generated projection** of the walk.

Required frontmatter (`deny` at birth, via the `habit-declaration-at-birth@1` policy bound at the
repo root): `epr-habit-version: 1` · `id` (MUST equal the filename stem — the filename is the
address the cascade resolves) · `invariant` · `status: green | red | unwired` · `retire-when:`
(an exit CONDITION, never a date; `never: <why this is a floor>` is legitimate and the point is
that it becomes countable). `checks:` is required unless `status: unwired`, and refused when it
is — that conditional, plus the roll-ups no single manifest can make (one habit per id, one habit
per `@concern:`, `max 2 active`), is enforced by the census in
`.claude/scripts/_lib/epr_habits.py`, not by the write-gate.

| You want to… | Do this |
|---|---|
| declare a habit for a lane | write `<dir>/.epr-meta/<id>.habit.md`, then `.claude/scripts/habits-project.py` |
| see the register | `.claude/scripts/habits-status.py [--full]`, or `habits-project.py --census` |
| know which habits govern a file | `epr_habits.in_scope(path, root)` — the upward walk, nearest first |
| rank a new habit | `.epr-meta/habits-covenant.md` `order:` — declaration is local, PRIORITY is not |

Two things carry the same discipline as `retire-when:` on a rule, for the same reason. `unwired`
is a habit we have committed to with **no way to observe whether we keep it** — declared and
counted on purpose, exactly as a reasoned `never:` is. And a habit's `checks:` string carries the
`@concern:` tag that is the `check_id` in a sprint report: composing declaration must never
fragment that namespace, so concern ids stay globally unique even though declaration is local.

## The cascade (governance composes up the tree)

Resolution walks **up** from the target — `.gitignore`/`.editorconfig`-style — collecting `.epr-meta`
files until it hits a `root: true` base, bounded by a hardcoded depth cap of 32 (the per-manifest
`max-cascade-depth` key is reserved — not read in v1; see above).

- **Nearest-ancestor wins on a rule `id`.** A child **overrides or relaxes** a parent rule by
  re-declaring the same `id` (see `specs/.epr-meta`'s `doc-frontmatter-at-birth`, which tightens the
  base rule from `[id, status, cites]` to the full 7-field lifecycle set).
- **Rules accumulate** across the chain unless an `id` collides — a child *adds* to the parent's set.
- **Tighten downward, don't bloat upward.** A concern specific to one subtree belongs in *that
  subtree's* `.epr-meta`, not as a special-case in the parent. When a parent rule grows conditionals,
  that's the signal to push it into a child.
- **`root: true` only at the constitutional base.** Exactly one ancestor in any cascade should carry
  it. A cascade that reaches the repo/depth bound with no root **advises** (v1, strict-but-recoverable)
  — "this subtree's governance has no anchor"; add a `root: true` parent.
- **`covers: subtree` claims downward responsibility (the coverage signal).** The dual of `root` (which
  terminates the cascade *up*): a manifest with `covers: subtree` is *fully responsible* for everything
  beneath it, so the **coverage walk** (`placement-audit.py --epr-meta`) terminates there and that region
  counts as OWNED — integrity by construction, never re-audited (seam-map §3.7: the core never re-checks an
  app-manifest's vocabulary). It feeds the `epr_meta_coverage` stasis dimension + the `epr-meta:` headline
  token. Opt-in: an incidental manifest (a `ci-trigger` config) without it never trivially "covers" the
  tree. **Ownership ≠ enforcement:** a `covers` claim with *no* rules is the considered-coverage outcome
  ("this region is owned, no edit-time gate warranted yet"); finer rule-bearing manifests still cascade
  inside it. Resolve a `--epr-meta` gap with ONE claim at the altitude you're willing to vouch for — not a
  gate in every leaf.
- **No orphan trees.** A new subtree under a governed root should carry its own `.epr-meta`
  (`require-sibling`), so governance is never silently dropped by descending one directory.

## Authoring mechanics + the footgun guards

Reserved top-level keys: `epr-meta-version` (must be `1`), `id` (slug → CID), `root: true` (cascade
base), `covers: subtree|dir-only` (coverage-walk responsibility — validated; `subtree` consumed by the
coverage signal), `purpose:`, `rules:`, `validators:`, `cites:`. (`extends:` and `max-cascade-depth:` are
schema-valid but **inert in v1** — see the reserved table.) Schema:
`elohim/sdk/schemas/v1/objects/epr-meta.schema.json` (`additionalProperties: false`). Every rule needs
`id` + `class`. Note: the *runtime* gate does not run JSON-schema validation — it uses the hand-rolled
`validate_meta`/`check_meta` in the engine — so the guards below are what actually catch your mistakes:

- **`fires on nothing`** — an enforcing rule (`deny`/`ask`/`inject`) with no actionable predicate is
  flagged. (Caveat: `allowed-types` *dodges* this warning because the validator counts it as actionable,
  yet the evaluator ignores it — the phantom above. Don't be fooled by a clean validate.)
- **`unknown key(s)`** — a typo'd predicate (`reqire-frontmatter`) is flagged by `validate_meta`'s
  known-key check, so it isn't silently ignored at runtime.
- **Size + depth caps** — a manifest over 64KB, or with flow-nesting past depth 64, is refused
  *pre-parse* (parse-bomb guard). Keep manifests small; they're meant to be read.

## Self-test before you trust a manifest

Never assume a new manifest behaves — **prove it.** Two cheap proofs:

```bash
# 1) The vocabulary/eval + resolver harnesses still pass (no pytest in this repo — plain scripts):
python3 .claude/scripts/_lib/__tests__/epr_meta_eval_test.py
python3 .claude/scripts/_lib/__tests__/epr_meta_resolver_test.py

# 2) Health-check YOUR manifest directly (schema + caps + footguns; [] means healthy):
python3 -c "import sys; sys.path.insert(0,'.claude/scripts'); from _lib import epr_meta; \
from pathlib import Path; print(epr_meta.check_meta(Path('genesis/docs/superpowers/specs/.epr-meta')))"
```

To prove a *verdict*, drive the resolver hook with a synthesized write (the resolver test shows the
exact payload shape) — and use a **non-existent target filename** so nothing is written to the repo
(the hook only reads `file_path`+`content`; it never creates the file): a compliant new file → silent
allow (empty stdout); a non-compliant new file → `deny`/`ask` JSON; an **edit of an existing file →
allow** (birth-time rules carry `new: true`). If a rule you expect to fire stays silent, check three
things in order: did `when:` match the *basename*? is the predicate in the *enforced* column? is the
rule's `id` shadowed by a nearer ancestor?

## Strict-but-recoverable (why a typo never bricks the tree)

A malformed `.epr-meta` does **not** hard-`deny` its subtree. While a manifest in the cascade is
malformed, the resolver short-circuits: editing the manifest itself stays advisory (the fix is always
reachable), and **every other write in the subtree is downgraded to `ask`** — a blanket fail-safe
applied *before* rule evaluation, so even a write that matches no rule (and would otherwise silently
allow) gets a one-confirmation prompt until the manifest is fixed. The discriminator here is
**validity, not reach**: a malformed manifest isn't yet a valid governance atom, so it can *propose*
(`ask`) but can't *bind* (`deny`). (A *well-formed* local manifest is equally un-notarized, yet it
binds — which is why the softening is about parse-validity, not reach-tier; don't conflate the two.)
You can author a rule, watch it misfire, and fix it in place — the gate never locks you out of its own
correction. Author with that safety in mind: boldness is cheap here.

## Worked examples

### A — the constitutional base (`genesis/docs/superpowers/.epr-meta`)

```yaml
---
epr-meta-version: 1
id: superpowers-docs-governance
root: true                                   # the cascade stops here
purpose: >
  Specs and plans — the authored design surface. Born-linked, decomposes to gaps,
  graduates to history; never parked.
rules:
  - id: doc-frontmatter-at-birth
    class: deny
    when: { write: "*.md", new: true }       # birth-time only — edits to fix are never trapped
    require-frontmatter: [id, status, cites]  # the minimum any doc must be born with
    why: "No doc born without id + status + cites (the .epr-meta law)."
cites:
  - 2026-06-25-doc-lifecycle-as-epr-development-substrate-design
---
# superpowers/ — authored design surface
The base governance for specs and plans. Child `.epr-meta` files tighten this per-directory.
```

Why it converges: `deny` is justified (a doc with no `cites:` *can't* be linked into the substrate —
proceeding-anyway is genuinely broken), it's birth-time, and the `why:` names the three fields. It's a
*teaching* rule against repeat authors — after a round or two of learning the shape, it fires rarely.
(The live file also carries `max-cascade-depth: 8`; that key is inert in v1 — omitted here so you don't
copy a no-op.)

### B — a child that tightens + routes + delegates (`specs/.epr-meta`)

```yaml
rules:
  - id: doc-frontmatter-at-birth              # SAME id as the base → overrides it (nearest-wins)
    class: deny
    when: { write: "*.md", new: true }
    require-frontmatter: [id, status, class, context-tier, steward, graduation-trigger, cites]
    why: "Specs need the full lifecycle frontmatter (overrides the base rule, nearest-wins)."
  - id: route-plans-out
    class: ask                                # a misplacement is a prompt, not a block
    when: { write: "*-plan.md", new: true }   # the when: glob does the matching
    route-to: { dest: genesis/docs/superpowers/plans/ }
    why: "Plans live in plans/, not specs/ — routed at creation, not on every edit."
  - id: p2p-design-gate
    class: ask
    when: { write: "*.md", contains-any: ["GET /api/v1", "PRIMARY KEY", "uuid"], new: true }
    validator: epr:validator-p2p-design-gate  # the one registered validator-EPR in v1
    why: "New data-entity designs pass the p2p-design-gate (creation-time, not on every edit)."
validators:
  - ref: epr:validator-p2p-design-gate
    fuel: 200
```

Three signals, three classes, each matched to its drift: a hard `deny` where born-incomplete is truly
broken; an `ask` to *route* a misplaced plan (with the destination in the message); an `ask` that
delegates a content-shaped check to the *registered* validator-EPR. Note the `new: true` on all three —
none of them fight you while you edit.

### C — closing a drift with a canon rule (the main loop)

You keep finding loose cross-pollination surveys dumped into `genesis/research/` that re-survey a topic
the **Research Index** (`README.md`) already catalogues — the README even carries `hypha-dao ≠
hyphacoop ≠ hypha-network` collision guards, scar tissue from exactly this. You clean the dup. Now write
the canon, co-located:

```yaml
  - id: register-survey-before-adding
    class: ask
    when: { write: "*cross-pollination*.md", new: true }   # the survey-naming convention, not every *.md
    dedupe-of: genesis/research/README.md                  # the survey INDEX (not research-manifest.json, which lists clone-repos)
    why: "Before adding a cross-pollination survey, check the Research Index (README.md) — this topic may already be surveyed (e.g. the name-collision-guarded hypha* surveys). Extend the existing entry and link it from the index, rather than starting a parallel note."
```

`ask`, not `deny` (a genuinely new survey should proceed with one confirmation), birth-time, and the
`why:` names the index to check *and* the action to take. Two things make it converge rather than toll:
the `when:` is scoped to the *survey* naming convention (not every `*.md`, which would prompt on every
legitimate file in an accumulating directory), and `dedupe-of` is a **structural guardrail** — it fires
on each new *survey* by design, catching duplicates at the rate they're attempted. That steady catch
rate *is* its convergence (coverage), not a teaching-rule failure.

## Key files

| File | Purpose |
|------|---------|
| `.claude/scripts/_lib/epr_meta.py` | The engine: cascade · merge · `validate_meta`/`check_meta` · pure `evaluate`/`combine`. The **source of truth for what's enforced** — read `_eval_rule` to see which predicates produce verdicts. |
| `.claude/hooks/epr-meta-resolver.py` | The thin PreToolUse hook: stdin → cascade → verdict JSON. Shows the `write` payload shape and the strict-but-recoverable + missing-root advisories. |
| `elohim/sdk/schemas/v1/objects/epr-meta.schema.json` | The frontmatter schema (`additionalProperties: false`). Note: enforced at *codegen/contract* time, not at hook time — the runtime gate uses the engine's hand-rolled validator. |
| `.claude/scripts/_lib/__tests__/epr_meta_{eval,resolver,cascade,schema,examples}_test.py` | Runnable proofs (no pytest). Copy their payload shapes to self-test your own manifest. |
| `genesis/docs/superpowers/.epr-meta` · `genesis/docs/superpowers/specs/.epr-meta` | The live exemplars (Examples A/B). |
| `.claude/memory/.epr-meta` · `.claude/hooks/.epr-meta` | Live exemplars of two more shapes: a rules-bearing manifest paired with a deterministic projector + PostToolUse budget signal (memory discipline — `deny` at birth, `ask` on the generated index), and a rules-free considered-coverage claim over a code tree whose real gates are structural (settings.json registration, `_lib` tests). |
| `genesis/docs/superpowers/specs/2026-06-25-epr-meta-compose-gate-design.md` | The spec — full mechanism (cascade, recursion guard, schema-first, the §8 worked manifest). Note: the spec describes the *intended* gate; this skill's reserved table marks what v1 has actually wired. |

## Related skills

- **`p2p-design-gate`** — the validator-EPR the `specs/.epr-meta` delegates to; the canonical example of
  "a content-shaped check too rich for a declarative predicate."
- **`epr-content-addressing`** — the envelope / DAG-CBOR / `canonical_bytes` / reach-at-compose machinery
  the source↔canonical split rests on.
- **`memory-ceremony`** and **`memory-kit`** (the `/hygiene-sweep` cadence lives in `memory-kit`) — the
  broader stasis cadence an `.epr-meta` rule participates in: the rule is the *local, executable* canon;
  the ceremonies are the *global* sweep.
