---
title: "`.epr-meta` Policy Registry + Measure Tier — Define-Once-Bind-Many Rules (Mishpat::Precedent lineage)"
id: epr-meta-policy-registry-measure
tier: spec
status: Implemented
created: 2026-07-02
maintainers: Matthew Dowell + Claude Fable 5
class: process-meta
process_subdomain: doc-lifecycle
topic: [epr-meta, policy-registry, precedent, measure, loc-ceiling, architecture-review, flag-agent-canon-stasis, version-pin, eprfs]
context-tier: disclosed
steward: cartographer
graduation-trigger: superseded-by-brit-eprfs-precedent-substrate OR decompose-complete
refines:
  - genesis/docs/superpowers/specs/2026-06-25-epr-meta-compose-gate-design.md
cites:
  - epr-meta-compose-gate | the P1 mechanism this extends — cascade, class ladder, resolver, vocabulary | sha256:6052ce071bfec509 | path: genesis/docs/superpowers/specs/2026-06-25-epr-meta-compose-gate-design.md
  - lens-version-dag-epr-policy-dependency-design | source of the pin doctrine — policy versions are a DAG; which version applies is a declared dependency, never recency | sha256:62e0f37f8f57c0ed | path: genesis/docs/superpowers/specs/2026-06-27-lens-version-dag-epr-policy-dependency-design.md
  - .claude/epr-meta/policies.yaml
  - .claude/scripts/_lib/epr_meta.py
  - .claude/hooks/epr-meta-resolver.py
  - elohim/holochain/dna/mishpat/zomes/mishpat_integrity/src/lib.rs
---

# `.epr-meta` Policy Registry + Measure Tier

> **Scope.** Extends the compose-gate (P1) with two coupled capabilities, both shipped and
> test-covered on landing: (1) a **policy registry** so a rule is defined once and bound by many
> manifests, and (2) the **measure enforcement tier** wired for source-file LoC ceilings — the
> observation-class rail that turns god-file growth into a fingerprinted architecture finding.
> Operator insight that forced the design: "this kind of rule feels like a Mishpat policy that
> could be defined once and reused among epr-meta files."

## 1. The policy registry (define once, bind many)

`.claude/epr-meta/policies.yaml` holds Precedent-shaped policy objects — deliberately the
`mishpat_integrity::Precedent` field shape (binding ladder, scope, full-reasoning why, status +
supersession lineage, citation count), because that entry type is the graduated home: when
epr-meta lifts into the brit/eprfs projection substrate, registry rows become Mishpat `Precedent`
entries (CID = entry_hash) and manifest bindings become cites. No new DHT entry type is needed —
compose, don't fork. The Precedent `binding` ladder maps onto the enforcement-class ladder:
constitutional/binding-network ≈ `deny` · binding-local ≈ `ask` · persuasive ≈ `inject` ·
observation ≈ `measure`.

**Two planes, one policy — the policy is EPR-shaped in itself.** A policy's *content* (scope,
predicate, defaults, why) is an ordinary EPR: content-addressed identity, version lineage as a
DAG under a head, consumers declaring **pinned** dependencies on it — exactly the shape the
lens-version-DAG spec pins for EPR→policy dependency declaration. What Mishpat adds is not a
different shape but the **standing plane**: a Precedent is the notarized attestation *about* that
EPR — what binding force it has, in what scope, `established_by` whom, active/challenged/
superseded. The v1 registry row deliberately conflates the two planes in one YAML object (
`id@version` is the repo-local stand-in for CID-under-head); the lift splits them — policy
content → EPR atom, standing → Precedent citing the policy CID — so revoking or re-scoping a
policy's force never rewrites its content, and two communities can grant different standing to
the same content-identical policy.

A manifest **binds** a policy instead of redefining it:

```yaml
rules:
  - id: rs-loc-ceiling
    policy: source-file-loc-ceiling@1     # id @ version — the pin is REQUIRED
    params: { loc-hard: 9000 }            # optional local variance, merged over policy defaults
    when: { write: "*.rs" }               # optional scope override
```

Contract (enforced by `validate_meta` + `expand_policies` in `_lib/epr_meta.py`):

- **Policy owns semantics** (class, predicate, measure defaults, why); **binding owns placement**
  (`when:` override + `params:`). A binding redeclaring class/predicates is schema-invalid.
- **The version pin is a declared dependency, never recency** (the lens-version-DAG doctrine
  applied to governance): tightening a policy = a NEW version entry; existing bindings keep their
  semantics until each re-declares. `status: superseded` + `superseded_by` carry lineage; a
  version with live bindings is never deleted.
- **Unknown/unpinned refs fail LOUD** — the rule is dropped with an advisory. A deny that
  silently vanished would be silent-allow; the advisory is the tell.
- **Inline-shadow dedupe**: an inline rule whose id exists in the registry draws a bind-don't-
  redefine advisory.
- Expansion happens at resolve time (after `merge_rules`, before `evaluate`) so the evaluator
  stays pure; expansion is idempotent (`policy-ref` stamps an expanded rule).

## 2. The measure tier — LoC ceilings as the architecture-review trigger

`measure` is the observation class: severity 0, never blocks, feeds signal ledgers. Wired
predicate: `measure: { loc-soft, loc-hard }` against the post-edit content the resolver already
synthesizes.

- **Over `loc-soft`** → an `inject` nudge ("new logic likely belongs in a module"), debounced per
  (rule, path) through the shared epr-meta advice store — informs once per working session, never
  nags per keystroke.
- **At/over `loc-hard`** → a `measure` verdict the resolver files as a fingerprinted finding in
  `.claude/data/architecture-findings.jsonl` (fp = sha256(rule|relpath)[:12]), then emits the
  sentinel-style dispatch directive: launch the policy's `dispatch-agent` (rust-architect for
  `*.rs`) in background to **canonicalize a modularization plan into the timeline backlog — never
  refactor inline**. A fingerprint already present suppresses re-dispatch (debounced citation
  only); the entry is deleted when the file drops back under ceiling. This is the
  flag→agent→canon→stasis pattern, instantiated for architecture debt.
- **Batch dual**: `measure_census()` walks the same cascade semantics on disk (submodule `.git`
  roots terminate — vendored trees are outside the constitution), surfaced by
  `placement-audit.py --epr-meta` (⛔ hard / ⚠ soft table) and the SessionStart headline
  (`· ⚠ N file(s) ≥ LoC hard ceiling`). Edit-time gate and census can never disagree: same
  registry, same expansion, same ceilings.

Seed binding: the repo-root manifest binds `source-file-loc-ceiling@1` (soft 3000 / hard 7000)
repo-wide for `*.rs`. Landing census: 3 hard (elohim-storage `http.rs` 14000, `content_store`
coordinator zome 12537, `p2p/mod.rs` 7832) — the refactor-safety asymmetry (coordinator zomes
hot-swap; integrity zomes move the DNA hash) rides in the policy why so a dispatched reviewer
inherits it. Ceilings **ratchet down** as the tail drains, never up. Second seed policy:
`memory-frontmatter-at-birth@1` (extracted from the inline `.claude/memory/.epr-meta` rule —
the first define-once-bind-many conversion).

## 3. What stays reserved

Count-shaped measures (`measure: {count, emit}`, `max-files`) and host-side `dispatch` remain
inert; the hard-ceiling directive instructs the session agent to dispatch — the hook never spawns.

## 4. Verification

`epr_meta_policy_test.py` (registry load / expansion / pin doctrine / dedupe / measure verdicts /
census / resolver ledger + debounce, in a temp repo) alongside the existing compose-gate suites;
live-verified against the real tree on landing (memory deny through its binding; `http.rs` filed
as finding a711583b7334).
