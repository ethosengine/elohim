# Bootstrap Steward — Authority Frame

**Date:** 2026-04-21
**Status:** Design memo (brainstorm substitute — async orchestration context)
**Source question:** Wave 1 execution plan §1.2 Q1
**Triggering commits:** `462609c8` (bootstrap-steward ports), `d6c1cac4` (imagodei reference)

---

## Context

The wave-1 execution plan asked for `superpowers:brainstorming` on a genuine
philosophical question, not a naming nit:

> In a protocol with no sovereigns, who holds bootstrap authority? Possibilities:
> (a) the bootstrap steward is a temporary role that dissolves once a minimum
>     quorum is reached,
> (b) it remains but with explicit accountability to later-joining stewards,
> (c) it's a rotating position.

The reshape sprint was spawned autonomously (orchestration session holds the vision
but is not in the brainstorm seat), so this memo substitutes for the live
collaborative dialogue the skill expects. It applies the same discipline —
2-3 approaches, trade-offs, recommendation — and is surfaced for user review
before any code change lands on the recommendation.

## Current state in code

`bootstrap_steward.rs` (imagodei reference + mishpat / node-registry / lamad
ports) provides:

- `bootstrap_steward()` → `AgentPubKey` (who is the bootstrap steward for this DNA)
- `maybe_bootstrap_steward()` → `Option<AgentPubKey>` (same, but `None` if not configured)
- `am_i_bootstrap_steward()` → `bool`
- `get_bootstrap_steward` / `is_bootstrap_steward` extern entry points

The module deliberately stops at identity: it does **not** add integrity-zome
validators that reject actions unless taken by the bootstrap steward. That
restraint was the pragmatic resolution this memo now validates or revises.

## Constraints the frame must honor

1. **Stewardship philosophy** (`project_stewardship_philosophy.md`). Authority
   is graduated; capability is earned through demonstrated responsibility,
   not assigned by role. Accountability is relational.
2. **No sovereignty** (`project_no_sovereignty_stewardship_over_ownership.md`).
   No actor owns data, authority, or identity. Reject the vocabulary.
3. **Graduated recovery authority** (`project_graduated_recovery_authority.md`).
   Authority graduates from intimate circle through qahal governance to global
   elohim witness. Absolute lockout is a design failure.
4. **Holochain primitive constraints.** `progenitor_pubkey` lives in DNA
   modifier properties — set at install time, immutable without a new DNA
   hash. A true key rotation would require a new DNA network.
5. **Existing stewardship infrastructure.** imagodei's integrity zome already
   defines `STEWARD_CAPABILITY_TIERS` ("self", "guide", "guardian",
   "coordinator", "constitutional") and `StewardshipGrant` /
   `StewardshipAppeal` entry types with a graduated capability + appeal
   mechanism. Any bootstrap-steward frame must compose with this.

## Three approaches

### (a) Dissolution — bootstrap steward role ends at quorum

The bootstrap steward holds privileged install-time authority until some
quorum of other stewards is reached. At that point the role formally
dissolves: the bootstrap pubkey stays on the chain as historical lineage
but carries no runtime capability.

- **Pros.** Makes the "fade" explicit and visible; matches the intuition
  that bootstrap should be transitional; straightforward to explain.
- **Cons.** Creates a bimodal system (pre-quorum / post-quorum) that
  every validator must be aware of. Quorum number is arbitrary — who
  sets it, and via which DNA? If the network never reaches quorum, the
  bootstrap steward is stuck as de facto sovereign indefinitely. The
  Holochain modifier is immutable, so "dissolution" has to be coordinator-
  layer state, not a modifier change — which means it is not actually
  atomic across the network.

### (b) Persistent identity, graduated authority — the current code

The bootstrap steward's pubkey is a permanent historical fact (first
signer of the genesis). It carries **no exclusive** capability at any
point. Authority is always attested — anyone holding a stewardship grant
at the appropriate tier may take the action. The bootstrap steward is
"agent zero" in the stewardship attestation chain, by construction: they
are the first agent who can issue grants, because they are the only
agent present at install.

- **Pros.** Clean model — no state transitions, no bimodal validators.
  Composes directly with the existing `StewardshipGrant` graduated-
  capability framework: bootstrap steward is simply the initial holder
  of the `constitutional` tier by virtue of install-time priority.
  Accountability is built in: later stewards can observe, challenge
  (`StewardshipAppeal`), and override via mishpat. Survives all the
  "what if quorum never happens" edge cases because there is no quorum
  threshold.
- **Cons.** Requires the stewardship-attestation mechanism to exist
  before graduated authority becomes real. Until another steward is
  attested into the `constitutional` tier, practical authority
  effectively defaults to "whoever holds the bootstrap pubkey" — not
  by design, but by lack of alternatives. This is a scaffolding gap,
  not a frame gap.

### (c) Rotating bootstrap — designation moves on a cadence

Some time- or attestation-count-based trigger rotates the "bootstrap
steward" designation to an elected successor.

- **Pros.** Prevents permanent entrenchment of the founding party.
- **Cons.** The Holochain primitive `progenitor_pubkey` is immutable.
  True rotation requires either a new DNA (unfeasible — defeats lineage
  continuity) or becomes a coordinator-layer re-designation that is
  indistinguishable from (b)'s attestation chain. If it's the latter,
  (c) collapses into (b) with extra ceremony. Also introduces a
  governance problem (who picks the successor?) that mishpat is not yet
  equipped to resolve.

## Recommendation

**Validate (b). Make it explicit.**

(a) requires a quorum mechanism that we do not have and should not invent
for the sole purpose of managing a bootstrap role. (c) collapses into (b)
given Holochain's constraints. (b) is the only frame that composes with
the existing stewardship infrastructure and honors the constraints above.

### What validating (b) requires codifying

1. **Name the bootstrap steward's tier.** The bootstrap steward is the
   initial `constitutional`-tier steward at DNA install time. Document
   this in `bootstrap_steward.rs` module docs so the frame is discoverable.

2. **Keep the current code as-is.** Do not add integrity-zome validators
   that gate actions on `is_bootstrap_steward`. Gating specifically on
   the bootstrap pubkey would calcify exclusive authority — the opposite
   of graduated authority. If validators need to gate, they gate on
   "holds a `StewardshipGrant` at tier X with matching scope" — a check
   that the bootstrap steward happens to pass trivially at install time,
   and that later stewards pass once attested.

3. **Document the scaffolding gap openly.** Until `StewardshipGrant`-
   based validation is wired in the four DNAs that adopt the bootstrap-
   steward pattern (mishpat, node-registry, lamad; imagodei is the
   reference), practical authority defaults to the bootstrap pubkey by
   lack-of-alternative. This is temporary scaffolding state, not the
   intended end state. Wave 2+ must close this gap before the first
   non-alpha network publishes.

4. **Bootstrap-steward externs return identity only.** Nothing about the
   return contract should imply "capability holder." Callers looking for
   capability checks must use the (future) steward-grant resolution
   layer, not `is_bootstrap_steward`.

### What the current code already gets right

- `bootstrap_steward()` is named as identity, not capability.
- `is_bootstrap_steward()` returns a bool identity check, not an
  authorization.
- No integrity-zome gates on the bootstrap pubkey (absence is deliberate).
- Error messages describe configuration state, not access-denied
  semantics.

### What to change

One doc-only change: extend the `bootstrap_steward.rs` reference module's
docstring to say explicitly:

> The bootstrap steward is the initial `constitutional`-tier steward at
> DNA install time. Authority is not exclusive to this pubkey at any
> point; this module exposes only **identity**. Authority checks must go
> through the stewardship-grant resolution layer — which trivially
> accepts the bootstrap steward at install time and accepts any later
> agent who holds a matching `StewardshipGrant`.

Propagate that docstring to the three ported modules so the frame is
consistent.

### What to defer explicitly (out of Wave 1 §7 scope)

- Implementation of `StewardshipGrant`-based validators in mishpat,
  node-registry, and lamad. Each DNA defines which actions need
  stewardship gating; that's a per-DNA design exercise that belongs
  with the DNA's feature sprints, not with the bootstrap-steward
  refactor.
- Decisions about `constitutional` tier bootstrapping across DNAs
  (does an imagodei constitutional steward automatically hold mishpat
  constitutional tier? probably not — each DNA is sovereign in its own
  stewardship chain, so bootstrap per DNA). This is a cross-DNA
  stewardship question that wants its own memo.

## Sweettest implication

The real sweettest scenario this session's spike exercises is exactly the
(b) frame: bootstrap-steward is identifiable, a second agent is not the
bootstrap steward, coordinator `get_bootstrap_steward` returns the
install-time pubkey. Those assertions test the identity contract, not a
capability claim — consistent with the recommendation.

## Memory updates needed

None new. The recommendation consolidates existing memory:
- `project_stewardship_philosophy.md` (graduated capability)
- `project_no_sovereignty_stewardship_over_ownership.md` (no exclusive ownership)
- `project_graduated_recovery_authority.md` (authority graduates)

If this memo lands as an authoritative frame, consider a one-line note
referencing it in `project_stewardship_philosophy.md` under "applied
examples" — but only if the memo is durably adopted, not if it is
superseded later.

## Verdict

**Validate the current pragmatic resolution.** The code is right; the frame
should be named and documented. Make the one docstring change across the
four bootstrap-steward modules, surface this memo for orchestration review,
and proceed.

If orchestration disagrees with (b), the revision path is:
- Towards (a): add a quorum entry type to imagodei integrity, a transition
  action in coordinator, and a bimodal validator. Non-trivial; reshape Wave 1.
- Towards (c): almost certainly collapses to (b); treat as a naming
  preference only.
