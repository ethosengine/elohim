---
title: Per-Substrate Limitarian Governor — Design Specification
id: per-substrate-limitarian-governor-design
status: design
created: 2026-06-09
cluster: 1 of 3 (attention-substrate program)
substrate_scope: household-nodes (v1; no shem dependency)
siblings:
  - cluster-2: sacredness surface (firewall tests · GA-endpoint retirement · multi-tenant cache de-anon)
  - cluster-3: data-arch + substrate_signal migration
note: >
  Inline `file:line` references are draft pointers. cite-seal as a finishing step
  (python3 .claude/scripts/memory-kit/cite-gen.py --seal <this file>) before this
  doc is treated as managed-surface canonical.
---

# Per-Substrate Limitarian Governor — Design Specification

> **What this is:** a per-substrate, community-ratifiable *limitarian* friction governor — the
> mechanism by which the Elohim Protocol prices concentration of a substrate (attention first;
> reach, compute, storage, energy, time later) as smooth relational friction rather than an
> absolute per-agent wall. It is the economic spine the Attention-Tending Lens (the reflexive
> sensing layer) feeds, and it is `cluster #1` of the attention-substrate program.

> **Provenance:** synthesized from a grounded multi-agent design pass and then **corrected against
> three adversarial reviews** (governance-capture, math-rigor, coherence/value-propagation). The
> corrections are load-bearing and are marked **[adversarial-fix]** where they change the design.

---

## 0. The honest one-paragraph version (read this first)

The protocol already contains the machine and it is wired wrong in one place: a **dead ratification
seam**. There is a working governance pipeline (`propose → vote → tally`) and a working limit
consumer (the responsibility-demand decay curve), but nothing connects a *passed tally* to a
*ratified limit*, and the consumer is driven by the wrong statistic (`median_estimate`, which
self-extinguishes) through a *saturating step* (which runs away above 5%). This spec connects the
seam, replaces the driver with a **scale-invariant concentration measure**, makes the friction
**super-linear** so the loop actually closes, and carries the limit as a **governed EPR**
(a Mishpat `Commitment`) so it is witnessed, immutable, superseded-not-mutated, and renewable.

**The load-bearing honesty (from adversarial review):** the *only* non-capturable part of this whole
design is the **DNA clamp wall** that core sets once and no community can open. Everything above it —
the ratifying community, the overriding bioregion, the apex — is captured by exactly the gradient
this governor exists to correct, and capture gets *cheaper as you ascend* (more authority, smaller
electorate, same incentive). So the governed-EPR / subsidiarity / renewal apparatus is **tuning
inside a box core defines**. We keep the subsidiarity story **only** because of a genuine
incentive-divergence argument (§6.1) — and we are explicit that the wall, not the ceremony, is the
backstop.

---

## 1. Problem & framing

Some systems cannot set the limit they most need, because the local incentive gradient that *defines*
them is the thing that needs limiting. The attention economy is canonical: every participant benefits
locally from more concentration, so no participant — and **no community composed of them** — will
ratify the friction that would correct it. The limit must be supplied from a layer that internalizes
the externality the lower layer externalizes, and it must be **renewable** (a stale setpoint is worse
than none: it confers false control).

The two halves already exist, disconnected:

- **Governance decision pipeline (real, wired):** `propose_governance_action` → `vote_on_governance_action`
  (child attestations) → `ApprovalTally::tally` computing `quorum_met` + `recommendation="pass"`
  (`content_store/src/governance_action.rs:260/337/389`, `elohim-storage/src/tally/approval.rs:69`).
- **Limit consumer (real, but static & wrong-driver):** `evaluate_position` keys a 5-band curve on
  `balance / median_estimate` (`responsibility_demand_service.rs:139`), feeding `calculate_decay_rate`'s
  4-rung step `{Supported 0.0, Normal .001, Elevated .005, High .02, Extreme .05}`
  (`token_decay_service.rs:57-65`), clamped `.max(config.dignity_floor)` (`:164`).

**The dead seam:** `responsibility_demand_configs` carries `ratified_by`/`ratified_at`/`dht_anchor_hash`
columns (`migrations/.../up.sql:703`) that **nothing writes** — `handle_create_config` hardcodes them
`None` (`api/token.rs:302-304`); `threshold_reached` writeback exists only for recovery/revocation
(`signals.rs:1167,1306`).

Two consumer defects this spec fixes:

1. **`median_estimate` is the wrong driver** — write-once (default `1000.0`), no recomputing writer, and
   *non-scale-invariant*: median is both attractor and governed variable, so as friction compresses the
   distribution the bands collapse toward zero and the governor **extinguishes its own pressure while
   inequality persists** (DC-drift-to-zero).
2. **The step saturates at 5%** — above `Extreme`, friction `0.05·b` is *linear*; under rich-get-richer
   inflow `I=c·b` with `c>0.05`, `db/dt=(c−0.05)·b>0` diverges. The loop is open.

### The recursion this spec builds

```
externality-sensing    a layer that CAN'T self-correct surfaces a NEED (not a change):
 surfaces a need   →    measured concentration C(D) breaches a target, OR valid_until expires
      │
      ▼
governance ratifies →   a HIGHER layer proposes a LimitGradientConfig (a governed EPR);
 the limit              M-of-N tally passes; can_override supplies the limit the lower layer won't
      │
      ▼
enforcement        →    the ratified gradient drives continuous demurrage on the substrate's
      │                 distribution — relational friction over the commons, not a per-agent wall
      ▼
renewal / feedback →    valid_until creates a re-ratification obligation; an expired setpoint
                        falls back to the core value-laden default — never fails open
```

The operator's frame is the architecture: **the manifest reinterprets core per-community, but the limit
slice of the manifest is itself an EPR subject to governance.** Core carries value-laden defaults
(clamp walls); the manifest reinterprets *within* those walls; governance renews on a validity horizon.

---

## 2. The measure

The governor needs a number answering "how concentrated is this substrate's distribution?" satisfying
three criteria `median` fails:

1. **Scale-invariant** — `z(λ·D)=z(D)`; its zero is the equality manifold, not the origin of the balance
   axis. This is the direct fix for median's drift-to-zero.
2. **Tail-sensitive** — one mega-concentrator in a large population must move it (pure Gini does not).
3. **Decomposable across collectives** — assemblable from per-group summaries `(N_g, μ_g, measure_g)`
   *without transmitting per-agent values*, so it composes with the k≥5 anonymity firewall
   (`aggregator.rs:90`) and the `ConstitutionalLayer` subsidiarity lattice at the measure level.

Let `D={x₁…x_N}`, `x_i≥0`, `μ=(1/N)Σx_i`.

| term | scale-invariant | tail-sensitive (one giant, large N) | decomposable |
|---|---|---|---|
| **Gini** `G=ΣΣ|x_i−x_j|/(2N²μ)` | ✅ | ❌ weak `O(x_mega/(Nμ))` | ❌ residual interaction term |
| **Top-q share** `S_q=Σ_{top q}x_i / Σx_i` (q=0.01) | ✅ | ✅ strongest (it *is* the tail) | ⚠️ aggregable, discards the body |
| **Generalized Entropy** `GE(α)=1/(α(α−1))·(1/N)Σ[(x_i/μ)^α−1]` | ✅ | ✅ tunable (α↑ weights upper tail) | ✅ **exactly additive** |

### The chosen composite

```
C(D) = w_e · squash(GE(α)) + w_s · S_q          (default w_e=0.6, w_s=0.4)
```

- **GE = shape term** and the load-bearing reason the composite (not Gini) is in core: only GE composes
  with subsidiarity at the measure level (below).
- **S_q = explicit tail fix** (the operator's concern).
- **Gini ships as a human-readable diagnostic** (`gini(&[f64])`) and as the convergence-test target — it
  is **not** a friction driver.

**[adversarial-fix · math] `squash` must be specified and scale-invariant.** `GE(α)` is unbounded above,
so the normalizer carries the entire scale-invariance guarantee. **Use `squash(g)=g/(1+g)`** — a fixed
monotone map, homogeneous-degree-0-preserving (it is a function of the already-scale-invariant `GE`,
introducing **no `N`- or `μ`-dependence**). **Do NOT** use `GE/GE_max(N)`, which reintroduces an
`N`-dependence that drifts as the very compression dynamics the governor induces change the population.

### Decomposability — the federated-household property

For groups `g` with population share `f_g=N_g/N`, income share `s_g=(N_gμ_g)/(Nμ)` (Shorrocks):
```
GE(α) = Σ_g s_g^α f_g^{1−α} GE_g(α)  +  GE_between(α)
          └──── within (weighted) ───┘   └ inequality of group means ┘
```
A bioregion's GE assembles from its communities' `(N_g,μ_g,GE_g)` **without transmitting per-agent
values**. Gini cannot (needs the raw cross-tab → breaches k=5). **This is why GE, not Gini, is the shape
term: it mirrors `can_override` at the *measure* level.**

**[adversarial-fix · math] Two preconditions, stated honestly:**
- **Single-membership:** the decomposition is exact only if every agent belongs to exactly one collective
  *per layer* (non-overlapping, exhaustive partition). A P2P agent in two households at once introduces a
  residual. v1 assumes single-membership and validates it.
- **k-suppression makes it a lower bound, not an identity:** dropping sub-k groups from the between-term
  breaks exhaustiveness, so `GE_assembled ≤ GE_true` (signed: it *under*-counts concentration in exactly
  the small collectives where capture is easiest). This is not just a blind spot — it makes the additive
  identity false. v1 bends toward the firewall (suppress, accept the lower bound) and flags small-collective
  concentration for a later operator-gated audit (§Decision 4). Never de-anonymize to close it.

### Default α

α is a **governed, layer-defaulted parameter clamped to `[1,2]`**. Small-N (a household of 4) → `α=1`
(Theil-T; its variance term doesn't noise-amplify on tiny populations; still decomposes). Large-N
(bioregion, attention commons) → `α=2` (maximally tail-sensitive). **A community may reinterpret α within
`[1,2]` but cannot set `α=0`** (which would blind the measure to the tail) — the clamp encodes "we will
not let a community govern away tail-sensitivity." Core default `α=1`, layer-default registry raises to 2
at community/bioregion scope.

---

## 3. The friction function

Maps the *whole distribution* (plus the agent's relative position) to a demurrage multiplier. **Relational**:
an agent experiences friction because the *commons* is concentrated, not because an absolute balance
crossed a fixed wall.

```
floor_factor(b_i) = 0  if b_i < dignity_floor   (Supported — decay OFF; sufficientarian gate)
                  = 1  otherwise

shape_factor(D) = 1 + k_s · relu( squash(GE_α(D)) − C_target )
tail_factor(D)  = 1 + k_t · relu( S_q(D) − S_target )
rank_weight(i)  = (b_i / μ)^γ                                  # the agent's RELATIONAL position

multiplier_i = floor_factor(b_i) · shape_factor(D) · tail_factor(D) · rank_weight(i)
rate_i       = clamp( base_rate · multiplier_i , 0 , k_max )
new_b_i      = (b_i − b_i·rate_i).max(dignity_floor)           # dignity clamp REUSED verbatim (:164)
```

- **floor = level** (sufficientarian): decay off below `dignity_floor`; supplies the absolute level the
  relational measure lacks. Preserves `.max(config.dignity_floor)` verbatim.
- **measure = shape** (limitarian): rises with how unequal the substrate is.
- **top-share = tail**: rises with what the top fraction holds.
- **rank = relational position** `(b_i/μ)^γ`. `base_rate` default `0.001` — **the current `Normal` rung
  becomes the floor of a continuous curve, backward-compatible.**

### Why a limitarian gradient, not an absolute ceiling

| | absolute per-agent ceiling `C` | relational gradient (this design) |
|---|---|---|
| shape | step at `b_i=C` | smooth, monotone in concentration |
| gaming | **invites sybil-split** to stay under `C` | **self-defeating to game**: splitting lowers `b_i/μ` *and* GE *and* S_q — the thing we want |
| bunching | spike at `C−ε` | **no wall ⟹ no attractor to bunch at** |
| scale | absolute number → loose or self-extinguishing as the pool grows/shrinks | scale-invariant → presses iff concentrated |

**floor + shape are complementary** — the relational gradient alone would press an agent below subsistence
in a skewed distribution; sufficientarianism supplies the categorical "no one is pressed below enough."
Limitarian (spread) + sufficientarian (level): neither substitutes for the other.

### What reuses `calculate_decay_rate` vs what is new (honest costing)

- **VERBATIM reuse — `apply_decay` Steps 4-6** (`token_decay_service.rs:160-196`): `decay_amount=balance·rate`,
  the dignity clamp `.max(dignity_floor)` (`:164`), `dignity_floor_protected` (`:166`), the
  `token_decay_events` audit row (`:185-196`). The whole **downstream of the rate** is reused byte-for-byte.
- **`calculate_decay_rate` (`:57-65`) is REPLACED, not reused.** The 5-value `match` → one of 5 constants is a
  different signature/domain/range from `calculate_decay_rate_continuous(b_hat, C, &gradient) -> f32`. **Zero
  lines survive.** `ObligationLevel` may survive only as an audit *label* derived from `C`-bands.
- **`evaluate_position` (`:139`) is REPLACED**: median-relative bands → read `C` from a `concentration_snapshot`
  + compute `b_i/μ`.
- **Genuinely NEW:** (a) `elohim-core::measure` (greenfield — no Gini/GE/Theil/top-share/HHI exists in any
  `.rs`/`.ts`, grep-confirmed); (b) `concentration_snapshot` table + projection; (c) the aggregate tick;
  (d) the continuous rate function; (e) the ratify-writeback projector (the dead seam); (f) **the DNA
  wall validator** (§6.2).

The five `test_decay_rate_*` rung tests (`:217-265`) become **curve-sample tests** (`rate(C=0)≈base_rate`,
monotone increasing in `C`).

---

## 4. Stability — corrected against the math review

### 4.1 The existence/stability inequality (step case — correct as far as it goes)

Model one balance as a leaky integrator: `db/dt = I − f(b)`, `f(b)=r(b)·b`. A fixed point `b*` satisfies
`I=r(b*)·b*`. For the step (`r≤r_max=0.05`):
```
r_max · b* ≥ I        ⟺   max_friction × winner_balance ≥ max_inflow
```
Absorbable inflow at the top rung: `I_max = 0.05·b*` per period; for `I=c·b`, capacity is a **slope** —
`c<0.05` absorbable, `c≥0.05` divergent. Correct, but governs only the step.

### 4.2 Super-linearity closes the loop — **but only in the unsaturated region** [adversarial-fix · math]

Choose `γ>0` so the friction exponent exceeds 1: `f(b)=base_rate·μ·(b/μ)^{1+γ}·h(C)`, `h≥1`. Against linear
inflow `I=c·b`, a finite fixed point exists: `(b*/μ)=(c/(base_rate·h(C)))^{1/γ}`. **Two corrections the raw
synthesis overclaimed:**

1. **The shipped rate is clamped at `k_max`.** Above `b_sat = μ·(k_max/(base_rate·h(C_target)))^{1/γ}`, the
   rate saturates and `f(b)=k_max·b` is **linear again — the step pathology with `0.05` renamed `k_max`.**
   So the continuous-rate fixed point governs only `b<b_sat`. Above it, closure must come from the **cadence
   invariant**, not the rate.
2. **The real threat is super-linear inflow.** Rich-get-richer / preferential attachment gives
   `I=O(b^{1+ε})`. Loop-closure by the rate term holds **only for `ε < γ`.** This is a **modeling assumption
   the operator must defend** (or set `γ` adaptively above the measured inflow exponent). State it; do not
   silently assume linearity is the worst case.

### 4.3 The cadence invariant — corrected to compounding form [adversarial-fix · math]

Keep `k_max` (top-side dignity: never confiscate an unbounded fraction in one tick — itself a limit a higher
layer should set). Keep `γ` **modest** (`γ=1`, exponent 2 — super-linear enough to close, not so steep
`rank_weight` becomes a bunching wall). Move the *unbounded-in-concentration* restoring force out of the rate
curve and into the **redistribution cadence**. Loop-closure in the saturated regime then requires that the
per-horizon **compounded** confiscation dominate the per-horizon inflow:
```
1 − (1 − k_max)^{ticks_per_horizon}   ≥   per-horizon inflow fraction
```
**NOT** the linear `k_max·ticks ≥ c_max` the raw synthesis wrote (that overstates confiscation). And `c_max`
is **ill-defined as a scalar for preferential-attachment inflow** — so the invariant must be stated against an
explicitly bounded inflow class (`I=O(b^{1+δ}), δ<γ`), not a constant slope.

**The blind-spot fill (the operator's mechanism, made code-enforceable):** the `(k_max, cadence)` pair that
provably cannot close its own loop is **rejected by the DNA validator at ratification time**, and the bound it
is checked against is **supplied by the ratifying (higher) layer** via `can_override` precedence — the layer
that can see the externality the lower layer cannot. This is "help a system set a limit it can't set for
itself" as a validator rule, not a slogan.

### 4.4 Self-extinguishing — the honest claim [adversarial-fix · math]

The raw synthesis claimed "turns off only at equality (`C=0`)." **False as written**, because `shape_factor`
uses `relu(C−C_target)` with `C_target>0`: the governor extinguishes at a **governed nonzero target**, not at
zero. The *correct* claim, and it is still a real fix vs median:

> The governor extinguishes at a **fixed, scale-invariant target `C_target`** — not at median's **drifting,
> scale-variant attractor**. With median, friction relaxed as the distribution merely *shrank* (drift-to-zero
> while still unequal). With scale-invariant `C` and a fixed target, friction relaxes iff concentration is
> *actually at or below the chosen target*. The pathology is resolved; the extinction point is a deliberate
> governed setpoint, not zero.

---

## 5. The governed-EPR limit architecture

### 5.1 `LimitGradientConfig` as a governed EPR

Carried **two-homed**:

- **Notarization carrier = a Mishpat `Commitment`** with action `"ratifies-limit-gradient"`. Inherits
  immutability (`mishpat_integrity/lib.rs:500` — "Commitment entries are immutable; create a new Commitment to
  supersede"), per-action payload validation (`commitments.rs:185`), audit-trail, DHT-truth/SQL-cache
  projection — **for free. No new entry type** (Mishpat stays 11/~100). **CID = `entry_hash`** of the
  Commitment (per `project_mishpat_commitment_cid_is_entry_hash`: `cid=entry_hash`, never `action_hash` —
  returning `action_hash` silently breaks every bounds-gate yet passes per-task tests).
- **Vocabulary/manifest expression = a `limit-gradient` Manifest EPR** — adds `"limit-gradient"` to
  `MANIFEST_KINDS` (`manifest.rs:37`) + a branch to `manifest-epr.schema.json` ($ref a new
  `limit-gradient-floor.schema.json`; the `standing-policy`/`tending-policy` branches are the mold).

**[adversarial-fix · coherence] v1 ships the Commitment home ONLY.** The Manifest home — the
*manifest-reinterpretation* story the operator's frame describes — needs `create_manifest` authority gating
(Phase-3.5) and is **explicitly deferred out of v1.** Headline, not parenthetical: *v1 governs via the
Commitment carrier; the manifest-reinterpretation loop does not ship in v1.* (Note also: the **lamad**
app-vocabulary manifest `elohim/sdk/domains/lamad/manifest.json` is a static checked-in file and is NOT the
governed home — the content_store DHT Manifest is. Do not conflate them.)

Payload:
```json
{ "substrate_signal":"attention", "governance_layer":3,
  "measure":{"alpha":2,"q":0.01,"w_e":0.6,"w_s":0.4},
  "shape":{"C_target":0.15,"k_s":0.5,"S_target":0.20,"k_t":4.0,"gamma":1.0},
  "base_rate":0.001,"k_max":0.05,"cadence":{"kind":"wall-clock","period_secs":86400},
  "dignity_floor":50.0,"valid_from":"…","valid_until":"…",
  "loosening_acknowledged":false,"ratified_by_governance_action_cid":"…" }
```
**[adversarial-fix · coherence] Every numeric above is a DEFAULT, not a decided value.** The widths of the
DNA walls that bound them are unargued today (§Decision 2). Treat the schema as *shape decided, values TBD-operator.*

`responsibility_demand_configs` is **demoted from CRUD surface to read-projection** of this EPR; source-of-truth
comment flips to `-- Source of truth: DHT`; the dead `ratified_by`/`ratified_at`/`dht_anchor_hash` columns
finally get written.

### 5.2 Walls enforced at the DNA validator — NOT clamp-at-read [adversarial-fix · coherence — THE soundness fix]

The single most important correction. The raw synthesis put the value walls in a **storage-side registry clamp**
(clamp-at-read). That **recreates the exact dead-seam lie this spec exists to kill**: a community ratifies
`C_target=0.9`, the Commitment is immutable and notarized carrying `0.9`, and storage silently enforces a
clamped `0.3` — *the ratified truth diverges from the enforced truth.*

**Therefore the walls are an integrity-validation concern:** `validate_ratifies_limit_gradient` (the new
`commitments.rs` dispatch arm + its integrity-side defense, `lib.rs` pattern) **rejects at `create_commitment`**
any payload whose params fall outside the DNA walls. A config that exists is, by construction, in-wall. The
storage registry still *clamps its own default output* (that half is fine — `constitutional_ratio_registry.rs:108`
`.clamp(MIN,MAX)` is the mold), but it **never silently overrides a ratified value** — because an out-of-wall
value cannot be ratified in the first place. Walls are reject-at-write or they are nothing.

### 5.3 The renewal loop

`valid_from`/`valid_until` in the payload. Because `Commitment` is immutable, **renewal is supersession**
(a new Commitment referencing the prior CID). Past `valid_until`:
- the effective-gradient reader `query_effective_limit_gradient(s,ℓ)` (copy the recovery
  "most-recent-effective-wins" pattern) **falls back to the core value-laden default** — fail-safe-to-coherent,
  never fail-open;
- the sensing surface raises a **re-ratification obligation** via `FeedbackSignal` (kind exists, `kind.rs:25`),
  routed to the governing collective — the **first generic "ratified value → validity horizon → re-ratification"
  primitive for an economic setpoint** (today this loop exists only for *key* renewal in imagodei).

**[adversarial-fix · governance] Renewal defends against staleness, NOT capture.** A captured layer simply
re-ratifies its captured config each horizon. Do not sell renewal as anti-capture; it is anti-stale-false-control.

### 5.4 Subsidiarity — and why it is not just relocated capture [adversarial-fix · governance]

`can_override` is **pure precedence** (`constitution/src/types.rs:46`, verified: `Individual=1 … Community=3 …
Bioregional=6 … Global=7`; `self.precedence() > other.precedence()`). It has **no direction check** — so the
"tighten-only, loosening-witnessed" asymmetry is **new validator code we must write**, not a property of the
primitive. Specifically:
- A **directionality validator**: compare the proposed gradient's *effective friction at a reference
  distribution* against the inherited one; an override that *loosens* (raises `dignity_floor`→0, raises
  `C_target`, lowers `k_t`) requires `loosening_acknowledged=true` **AND** a real cost — a **super-quorum**
  (`passage_threshold` is configurable, `approval.rs:81`) and/or a higher-layer counter-signature and/or a
  mandatory deliberation window. A self-attested bool alone is decorative.

**Why subsidiarity is not merely capture-relocated (the missing argument the review demanded):** the higher
layer is justified **only** by a genuine **incentive divergence**, not by altitude. A single community's optimum
externalizes the cross-community cost of its own concentration (its members feel their local gain, not the
commons-wide harm). The layer that *contains* multiple communities **internalizes** that cross-community
externality — so its *self-interested* optimum is **less concentrated** (standard Pigouvian / fiscal-federalism
logic). Subsidiarity is sound **iff** each substrate's ratifying layer is the smallest layer that internalizes
that substrate's externality (§5.5). Where no layer internalizes it (a truly global commons), the apex is the
backstop — and the apex has **no structural non-accumulability in code today** (`Global=7` is just the top of an
ordinal). That is a genuine gap (§Decision 1), and until it is closed the **DNA wall, not the apex, is the real
ceiling.**

### 5.5 Subsidiarity table (which layer internalizes which externality)

| substrate | ratifying layer | rationale (internalization) |
|---|---|---|
| **attention** | Community (3) / Collective | the attention-economy externality is felt at community scale; a household can't fix the commons it sits in — the v1 proving ground |
| storage/bandwidth/compute | Household (2) → Community (3) | hub-optional: a laptop-alone household sets its own; shared infra one layer up |
| energy | Bioregional (6) | ecological limits are bioregional |
| time | Individual (1) → Household (2) | most-personal |
| recognition/reach | Community (3) → Global (7) | apex must hold a non-accumulating ceiling (**not yet structural — Decision 1**) |

### 5.6 p2p-gate entity table

| Name | class | identity | truth | new/changed | reuses |
|---|---|---|---|---|---|
| `LimitGradientConfig` (Commitment) | **A** | CID = `entry_hash` | DHT (Mishpat Commitment) | new payload action + DNA wall validator | Commitment immutability, action-dispatch, `reach_elevation_acknowledged` escalation mold |
| `ratify-limit-gradient` GovernanceAction | **A** | CID | DHT | new `governance_kind` | propose/vote/tally wholesale |
| `limit-gradient-approval` attestation | **A2** | agent-composite, link-derived | DHT | new kind→child map entry | `child_attestation_kind_for_governance_action`, closes-at validator |
| `concentration_snapshot` | **C** | slug `(signal,layer,computed_at)` | SQLite (rebuildable by event replay) | new operational table; **carries no per-agent identity** | `aggregator.rs` k=5 floor + `candidate_struct_has_no_peer_identity` firewall mold |
| `responsibility_demand_configs` row | **C** | logical key = Commitment CID | SQLite (DHT-derived projection) | demoted CRUD→projection; dead columns now written | table DDL `up.sql:703` |
| `LimitGradientConfig` Manifest home | A | CID (`compute_cid`) | DHT (content_store Manifest) | **deferred to Phase-3.5** | `manifestKind` whitelist, `validate_floor_sub_object` |
| `EconomicEvent.substrate_signal` | field on existing **A** | n/a | DHT-notarized field | **Cluster #3 prerequisite — does not exist today** | — |

**Anti-pattern checks (corrected):** relational-config-as-truth → demoted to projection; new entry type → reuses
Commitment+Manifest, zero new entry types; REST-first → coordinator designed before any HTTP route; three address
formats → one canonical (Commitment CID); granular data on DHT → `concentration_snapshot` is a k≥5 aggregate.

---

## 6. Core defaults & value propagation

`elohim-core` carries three things so coherence propagates even when no community has ratified — the operator's
"the values again become important":

1. **`elohim-core::measure` (greenfield)** — pure no-I/O: `ge_alpha`, `top_quantile_share`, `gini`, `ge_decompose`,
   `composite_concentration`. Property-tested: scale-invariance `z(λb)==z(b)`, equality-zero `z(equal)==0`,
   decomposability `GE_total==within+between` (under single-membership).
2. **`LimitGradientRegistry`** cloned from `constitutional_ratio_registry.rs` — `effective_gradient(substrate,layer)`
   returning the param vector, **clamped to DNA-mirrored walls** (the registry clamps its *own default output*; the
   DNA validator rejects *ratified* out-of-wall values — §5.2). **Fix the CID stub** at
   `constitutional_ratio_registry.rs:141` (`format!("manifest-fingerprint:{path}")` → `epr::cid::compute_cid`), or
   the "governed EPR" is notarized-in-shape but not content-addressed-in-fact.
3. **The DNA-wall mirror** (native+WASM constants, `constitutional_ratio_registry.rs:17-22` pattern).

**[adversarial-fix · coherence] The walls are the value statement, and their *width* is unargued today.** A
scale-invariant measure whose zero is the equality manifold genuinely encodes a value (not a magic number) — that
much is real. But `α∈[1,2]`, `C_target∈[0.1,0.3]`, etc. are **asserted, not derived.** Either derive each wall from
an explicit value premise, or ship only the **wall *shape*** in v1 (that a wall exists; that loosening is witnessed;
that α cannot blind the tail) and mark every numeric `TBD-operator`. There is **no value-neutral width** — picking
it is unavoidably political, done once, by whoever writes core (§Decision 2). This spec's stance: walls narrow
enough that an expired/misconfigured EPR degrades to *clamped-but-coherent* (never self-extinguishing zero, never
friction-abolished), wide enough that genuine per-community reinterpretation is real.

**The `x_i` adapter is the most value-laden choice and must be gated [adversarial-fix · coherence].** "What counts
as a unit of the substrate's distribution" (attention = engagement-seconds? inbound-links? unique-viewers?) is a
larger value choice than any wall, and it is **per-substrate code**, not a registry row. v1 must define the
attention adapter explicitly and put it under the same review as a wall — it is not adapter-author discretion.

Propagation chain:
```
core default gradient (registry, value-laden, in-wall)     ← layer 1 (WriteThroughState::from_registry)
  → community reinterpretation, REJECTED-AT-WRITE if out-of-wall (§5.2)   ← layer 2
    → community ratification (M-of-N governance-action)    ← layer 3, the witnessed flip
      → renewal (valid_until → re-ratify, else fall back)  ← the feedback horizon
```

---

## 7. Cross-substrate generalization

**Strictly uniform (one implementation, all substrates):** the friction-function shape, the GE+top-share measure,
the decomposition, the stability result, the scale-invariant-target result, the aggregate-tick clock, the EPR
carrier, the M-of-N gate, `can_override` subsidiarity, the k=5 firewall, the renewal horizon. **Adding a substrate
= a registry row + a default + an adapter + a ratifying-layer assignment, NOT new governor code.**

**Per-substrate:** the `x_i` adapter (§6, gated), the params, the ratifying layer, the plausible inflow exponent
`ε` (attention steep → tighter cadence/higher `k_max`; storage/compute capacity-bounded → gentle; energy hollow →
deferred).

**Reach is the one asymmetric substrate.** Reach is monotonic-from-standing and **never eroded** (`standing.rs:55`
`with_lift=max(score,lift)`; `reach_earning.rs:4` "never persists"). Limitarian counter-pressure on reach needs a
**new erosion path** — the skeleton does not bolt on. The v1-lock correctly kills "v2 bounded attention→standing
renewal behind the firewall" (a capped attention→reach edge is a *hole in* the firewall). If reach is ever governed,
friction acts on the *new reach-earning rate*, never on standing already held. **Reach is out of scope for v1.**

v1 ships **single-value `substrate_signal`** (defer multi-dim `place=energy+attention`) and proves the loop on
**attention**, keyed at Community — the richest externality, the clearest "can't self-set" story.

---

## 8. Implications for Cluster #2 (sacredness surface)

1. **Firewall test (a new test this spec REQUIRES).** The `LimitGradientConfig` payload struct AND
   `concentration_snapshot` each get the `candidate_struct_has_no_peer_identity` treatment (`aggregator.rs:491`):
   exhaustive-construction + serialize-absent, proving **no per-agent field** — the config governs a *cohort*, never
   a person, and **cannot carry a self-set-bypass field** ("exempt this agent"). **Plus the dual the review demanded:
   an anti-capture property test** — exhaustive over the in-wall param space, asserting *no ratifiable config can
   drive effective friction → 0 while concentration is high*. The convergence test proves the loop *can* close; this
   proves it *cannot be governed open*.
2. **GA-endpoint retirement (independent surface).** `get_content_engagement_stats` (`http.rs:9638`) is the
   still-armed GA surface and touches none of the governor wiring — Cluster #2 scopes it standalone.
3. **Multi-tenant cache de-anon (constrains the measure).** k=5 `Suppress`-below (`aggregator.rs:90`) is the
   anonymity floor `C(D)` must respect; GE-decomposability is what lets the higher layer aggregate without dropping
   below k. **Caveat (§2 lower-bound):** suppressing sub-k collectives makes `GE_assembled ≤ GE_true` — the governor
   under-measures exactly the small collectives where capture is easiest. v1 bends to the firewall and flags it.

## 9. Implications for Cluster #3 (data-arch + substrate_signal migration)

The governor **cannot compute until `EconomicEvent.substrate_signal` exists.** Today `SUBSTRATE_SIGNALS` is a *dead
const* (declared in `substrate-signal.schema.json` `_dna`, absent from the zome `lib.rs`; only
`CORE_/ALL_SUBSTRATE_SIGNALS` in `generated_enums.rs`; `EconomicEvent` has no field + no validator). **Hard ordering:**
1. Add `SUBSTRATE_SIGNALS` to the DNA zome `lib.rs` it claims (kills the dead const).
2. Add the `substrate_signal` column + validator to `EconomicEvent` via the manifest `columnMapping` pattern
   (`projector/mapping.rs:176`; `shefa_economic_event_column_mapping` is the template). **Single-value v1.**
   **Land field + validator together** — a field without its validator silently drops rows (the dangling-seam
   failure mode).
3. Backfill the projection.
4. **Only then** can the aggregate tick compute `D` per-substrate and write `concentration_snapshot`.

Homes: `manifests` table + `insert_manifest` upsert-by-CID (`db/manifests.rs:46`) for the deferred Manifest rows;
`responsibility_demand_configs` (`up.sql:703`) for the ratification projector; `concentration_snapshot` is the one
genuinely new operational table.

---

## 10. Open tradeoffs & decisions for operator

1. **Apex non-accumulability + `k_max` ratifiability.** `Global=7` has no structural non-accumulability in code; a
   captured higher layer is *more* leveraged (smaller electorate, same gradient). And if `k_max` is ratifiable, a
   captured council could set `k_max=1.0` and weaponize demurrage (confiscate everything in one tick). *Recommend:
   `k_max` ratifiable within a DNA wall `[k_max_min, k_max_ceiling]`, loosening witnessed (super-quorum); and treat
   "structural apex non-accumulability" as a named follow-on, because today the DNA wall — not the apex — is the
   real ceiling.* **This is a soundness question, not a preference.**
2. **DNA-wall width — the unavoidable politics.** No value-neutral width exists. Narrow imposes strong core values;
   wide makes them vacuous and re-opens the self-extinguishing hole. *Recommend: narrow enough that an
   expired/misconfigured EPR degrades to coherent-not-zero; ship the wall **shape** in v1 and mark numerics TBD.*
3. **The measure-computation oracle (the un-decentralizable heart).** `C(D)` needs a global view of the distribution
   no single hub-optional peer legitimately holds. Notarize the snapshot (bloats the DHT past ~3000 + de-anonymizes)
   **or** trust an aggregator (re-centralizes the rent surface) **or** treat `C(D)` as **advisory to human
   ratifiers**. *Recommend the third: `C(D)` is advisory input to the M-of-N proposal, never an automatic actuator —
   v1's lack of a clock makes this honest by construction. Caveat: advisory-to-the-capturable-layer means capturers
   can also choose to disbelieve the measure — the oracle and capture problems compound (§5.4).*
4. **Firewall vs limitarian goal under k-anonymity.** Dropping sub-k collectives under-measures concentration where
   capture is easiest. *Recommend: bend to the firewall (suppress, accept `GE_assembled ≤ GE_true`), flag
   small-collective concentration for a later operator-gated audit; never de-anonymize.*
5. **Default α and renewal cadence.** α layer-defaulted `[1,2]` (decided). Cadence: too short = governance fatigue,
   too long = stale. *Recommend attention ≈ quarterly, storage/time ≈ annual; operator picks per-substrate.*
6. **Failed re-ratification: fall back to core default, or freeze last ratified?** *Recommend fallback-to-core-default
   (fail-safe-to-coherent), because a stale setpoint conferring false control is the named failure this recursion
   exists to prevent. Operator may override per-substrate.*

---

## 11. v1 slice — the smallest honest governor

**Scope: household-nodes, NO shem, single-value `substrate_signal="attention"`, one governance layer,
reflexive-sensing (no clock), one green convergence test.** Proves deep on the stable floor (M/J/J multi-peer mesh,
`feedback_household_nodes_is_the_stable_floor`); cross-node discovery is the only thing needing `@requires:shem`, and
the governor does not.

**Lands:**
1. `elohim-core::measure` — `ge_alpha`, `top_quantile_share`, `gini`, `composite_concentration` (with
   `squash(g)=g/(1+g)`), property tests (scale-invariance, equality-zero). (`ge_decompose` greenfield-ready, not
   exercised in v1 — single layer.)
2. `calculate_decay_rate_continuous(b_hat, C, &gradient) -> f32` replacing `calculate_decay_rate`; `evaluate_position`
   reads `C` from a `concentration_snapshot`. **Reuses `apply_decay` Steps 4-6 verbatim.**
3. `concentration_snapshot` table + a Phase-A aggregator. *Note:* until the Cluster-#3 `substrate_signal` field lands,
   the only computable substrate is the existing per-agent token balance — so v1 computes `C` over **that**, and
   generalizes to true per-substrate once the field lands.
4. `LimitGradientRegistry` with in-wall value-laden defaults; **CID stub fixed** (`compute_cid` wired).
5. **The DNA wall validator** `validate_ratifies_limit_gradient` — reject-at-write for out-of-wall params (§5.2).
6. **The ratify-writeback projector** — consumes a passed `governance-action:ratify-limit-gradient` tally and writes
   `ratified_by`/`ratified_at`/`dht_anchor_hash` (the dead seam), copying `signals.rs:1167`. The Commitment action +
   governance-kind + child-attestation kind-map entry land with it.
7. **Firewall test** on the payload + `concentration_snapshot`, **plus the anti-capture property test** (§8.1).

**Explicitly deferred:** the aggregate-tick scheduler (v1 drives Phase A+B from the HTTP poke + test harness —
reflexive-sensing, no clock); the `limit-gradient` Manifest home (Phase-3.5, `create_manifest` authority gating); the
reach gradient (needs a new erosion path); multi-dim substrate; cross-collective GE aggregation; GA-endpoint
retirement (Cluster #2); structural apex non-accumulability (§Decision 1).

### The one green convergence test — corrected against the math review

```rust
#[test]
fn continuous_governor_restores_toward_target_under_rich_get_richer_inflow() {
    // Run the SHIPPED CLAMPED model (k_max set), not the idealized unclamped one.
    // base_rate=0.001 (the real default), gamma=1.0 (friction exponent 2), k_max=0.05.
    // Inflow LINEAR c=0.20 — 4x the k_max ceiling (regime where the STEP provably diverges).
    let g = GradientConfig { base_rate: 0.001, b_floor: 100.0, gamma: 1.0, k_max: 0.05,
                             c_target: 0.15, /* …in-wall defaults */ };
    let c = 0.20;
    let mut balances = vec![100_000.0_f32, 100.0, 100.0, 100.0, 100.0];
    let mut series = vec![];
    for _ in 0..2000 {
        let cc = composite_concentration(&balances, &g);
        series.push(cc);
        for b in balances.iter_mut() {
            let inflow = c * *b;                                   // rich-get-richer ∝ b
            let rate = calculate_decay_rate_continuous(*b / g.b_floor, cc, &g).min(g.k_max); // CLAMPED
            *b = (*b + inflow - rate * *b).max(g.b_floor);
        }
    }
    let top = balances.iter().cloned().fold(0.0, f32::max);
    // (a) BOUNDED where the step diverges  — proves closure beats the saturating step (via cadence, k_max clamped)
    assert!(top.is_finite() && top < 1.0e9, "runaway: {top}");
    // (b) MONOTONE DESCENT toward target — proves a RESTORING force, not mere settling [adversarial-fix]
    let tail = &series[series.len()/2..];
    assert!(tail.windows(2).all(|w| w[1] <= w[0] + 1e-4), "C not non-increasing in the tail");
    // (c) RESTORES TO THE TARGET, not just somewhere [adversarial-fix]
    assert!((series.last().unwrap() - g.c_target).abs() < 0.05, "settled away from C_target");
    // (d) SELF-EXTINGUISHING-WHEN-JUST: an equal-start run stays equal & friction -> base_rate
    //     (proves it turns off at the target, not while unequal — the median bug).
}
```
Three corrections vs the raw synthesis test: **run the clamped model** (k_max set), assert **monotone descent toward
`C_target`** (not just boundedness/settling), and assert **`C* ≈ C_target`** (restoration, not "settled somewhere").
A DB-backed follow-on folds the real `apply_decay` over a multi-agent fixture, asserting `token_decay_events` rows
grow while the layer's `concentration_snapshot.gini` decreases tick-over-tick toward the target.

**Done when:** that test is green on household-nodes, the DNA wall validator rejects an out-of-wall config, and the
dead ratification seam is written by a passed tally.

---

## 12. Key files

`mishpat/zomes/mishpat/src/commitments.rs:185,365` (action dispatch + escalation mold) · `mishpat_integrity/src/lib.rs:500`
(immutability) · `content_store/src/governance_action.rs:260,337,389` (propose/vote/kind-map) ·
`elohim-storage/src/tally/approval.rs:69,81` (tally + configurable threshold) · `signals.rs:1167,1306` (projector mold),
`:627` (CommitmentCommitted) · `db/responsibility_demand_configs.rs` + `api/token.rs:302` (dead seam) ·
`services/token_decay_service.rs:57,160-196` (replace rate / reuse downstream) · `services/responsibility_demand_service.rs:139`
(median curve to replace) · `services/constitutional_ratio_registry.rs:108,141` (clamp mold + CID stub) ·
`constitution/src/types.rs:22,41,46` (ConstitutionalLayer: Individual=1…Global=7, `precedence`, `can_override`) ·
`content_store_integrity/src/manifest.rs:37,155` (manifestKind whitelist + floor mold) · `manifest-epr.schema.json` ·
`epr/src/cid.rs:12` · `projector/mapping.rs:176` (substrate_signal column template) · `aggregator.rs:90,491`
(k=5 floor + firewall mold) · **greenfield:** `elohim-core::measure`.
