---
title: Sacredness Surface — Firewall, Anti-Capture, GA Retirement, Cache De-anon
id: cluster2-sacredness-surface-firewall-anti-capture-design
status: design
created: 2026-06-09
cluster: 2 of 3 (attention-substrate program)
substrate_scope: household-nodes (v1; no shem dependency)
companion: 2026-06-09-per-substrate-limitarian-governor-design.md (Cluster #1)
sibling: 2026-06-09-cluster3-substrate-signal-migration-governance-signal-flow-design.md (Cluster #3)
note: >
  Inline file:line are draft pointers against feat/native-content-graph-seam.
  cite-seal (cite-gen.py --seal) is the finishing step. Corrected against adversarial review —
  fixes marked [adversarial-fix]. The largest: the anti-capture test is a SAMPLE, not a proof,
  unless friction is shown monotone per-dimension (then assert at wall corners only).
---

# Cluster #2 — Sacredness Surface (Firewall · Anti-Capture · GA Retirement · Cache De-anon)

> Owns the privacy/anti-capture invariants every new governor, ATL, and governance-state aggregate must
> carry — making them **code-enforced and compile-time** rather than reviewed-by-hope.

> **Provenance:** synthesized from a grounded pass, then **corrected** against adversarial review. The
> base claims (the firewall mold, the GA hole, the cache asymmetry) verified exactly; two structural
> over-claims were softened, and one foundational test was re-grounded. Fixes marked **[adversarial-fix]**.

---

## 0. The honest one-paragraph version

"Sacredness" = the surface where a person could be re-identified, and the code that makes that
re-identification structurally impossible rather than policy-discouraged. The protocol has **one proven
mold** (`candidate_struct_has_no_peer_identity`), **one proven floor** (k=5 Suppress), and several armed
holes (an ungated GA-shaped endpoint emitting `COUNT(DISTINCT provider)`; a per-peer `signer_pubkey` cache
with no app-partition; a wired `list_by_signer`; the `attestations` per-voter table from Cluster #3). This
cluster generalizes the mold, builds the anti-capture property test, retires the GA field, and names the
residue precisely. **The load-bearing honesty:** the firewall makes capture structurally impossible *and*
makes small-cohort capture structurally invisible (k≥5 cannot sense a sub-5 cabal; `GE_assembled ≤ GE_true`).
The DNA wall — not the apex, not the blind measure — is the real ceiling in the capture-easy regime, which
is why the anti-capture property test over the wall space is load-bearing. And **that test is only a proof
if friction is monotone per-dimension; otherwise it is a sample** — the single most important correction.

---

## 1. The four sub-surfaces

1. The firewall test suite — the mold, generalized to every net-new aggregate, extended with bypass-field
   and engagement-capture-field guards.
2. The anti-capture property test — the *dual* of Cluster #1's convergence test.
3. GA-endpoint retirement — drop `unique_viewers` (the only de-anon field, and the only field no UI renders).
4. Multi-tenant cache de-anon — the latent `GROUP BY signer_pubkey` surface + the `attestations` per-voter
   surface (Cluster #3).

---

## 2. The firewall test suite

**The mold (verbatim clone target): `candidate_struct_has_no_peer_identity` (`aggregator.rs:490-536`).**

**[adversarial-fix] State the two halves honestly — one is structural, one is a backstop:**
- **Compile-time half (`:494-501`) — structural.** The exhaustive struct-literal names every field, so adding *any* field (identity-bearing or not) breaks the build until the literal is updated. That is the real guard: **it forces a human to consciously touch this test for every new field** — including a future `cohortFingerprint`/`engagementScore`/`tenantHint` that the enumerated substring list would miss. This is what makes the invariant code-enforced.
- **Serialize-absent half (`:505-524`) — enumerated backstop, not structural.** `assert!(!json.contains(<forbidden substring>))` catches a `#[serde(rename)]` that smuggles a *known* identity name past review. It does **not** make "no engagement-capture field" structural — it makes it "no field named one of these listed strings." Do not over-claim it; its value is catching rename-smuggling of the enumerated names, with the compile-time half forcing review of everything else.

**Clone the mold for every net-new aggregate** (each with its firewall test in the *same commit* — the compile-time half is worthless if added later):

| Struct | cluster | gate-class | forbidden set (base + flavor) |
|---|---|---|---|
| `LimitGradientConfig` payload | #1 | B (Commitment) | base + governor: `exempt`, `bypass`, `agentExemption` |
| `concentration_snapshot` | #1 | C | base (no per-agent term) |
| `CollectiveAttentionAttestation` | ATL | A | base (inherits the `CollectiveFilterPattern` mold) |
| `AttentionEvent`/`SurfaceObservation` projection | ATL | B/C | base + engagement: `dwellMs`, `views`, `completions`, `uniqueViewers` |
| `GovernanceParticipationCandidate` | #3 | C/k≥5 | base + governance: `signedBy`, `signed_by`, `issuerCid`, `issuer_cid`, `voter` |

Base set (the proven mold): `signerPubkey`, `signer_pubkey`, `subject`, `agentKey`, `pubkey`.

**The governor `exempt`/`bypass` guard is the anti-capture invariant at the struct layer:** the config governs a cohort, never a person — a capturing council *cannot ratify a config that exempts itself* because the field to express the exemption cannot exist (compile-fail).

**[adversarial-fix · these structs don't exist yet.** Only `CollectiveFilterPatternCandidate` has a live firewall test today. The five above are forward-promises against Cluster #1/#3/ATL structs; their v1 green-test count is *zero-until-dependency*. The same-commit discipline is the mitigation, but state plainly that this cluster's net-new firewall tests cannot be green until the structs land.]

---

## 3. The anti-capture property test — corrected

The *dual* of the convergence test: convergence proves the loop *can close*; anti-capture proves **no
ratifiable (in-wall) config can drive the loop open** — `friction(C_high, any-in-wall-config) > 0`.

**[adversarial-fix] It is a SAMPLE, not a proof, as originally written.** The original cited
`constitutional_ratio_registry.rs:151,163` and `sharding.rs:163` as an "in-wall grid loop idiom." Verified:
those are single-shot in-wall assertions and a byte-chunking loop respectively — **there is no
grid-over-param-space idiom in the crate, and no proptest/quickcheck.** A stepped nested `for`-loop over
`(c_target, k_s, k_t, dignity_floor)` is a **discretized sample** with un-probed inter-step pockets. You
cannot call it "a proof over the ratifiable space."

**Resolution — pick one, earn it:**
- **(a) Prove monotonicity, assert at corners (a true proof).** Show `effective_friction` is monotone in
  each config dimension over the wall interval (friction is non-increasing in `C_target`/`dignity_floor`
  raising the off-threshold, non-decreasing in `k_s`/`k_t`). If monotone, the minimum over the wall box is
  at a **corner**, so asserting `friction(C_high) > 0` at the `2^4` wall corners is a *proof* — no step
  granularity, no inter-step pockets. **Recommended** — it is both stronger and cheaper.
- **(b) Admit it is a discretized sample** and document the step granularity as the *stated proof strength*
  (the cost of no proptest shrinking). Then the honest caveat applies: a coarse sample can miss the exact
  small-cohort friction→0 pocket the k≥5 blind spot already hides — so "the firewall can't tell if the walls
  are narrow enough" applies to the anti-capture test *too*.

Either way the assertion target is `effective_friction(cfg, C=HIGH) >= MIN_NONZERO_FRICTION`.

**[adversarial-fix] The wall-mirror coupling test must be non-vacuous.** The loop/corner bounds are
storage-crate consts hand-mirrored from the DNA walls (`constitutional_ratio_registry.rs:14-22`,
"Keep these synced" comment — the silent-drift hazard). A coupling test asserting two hand-copied arrays
equal each other is **vacuous**. The mechanism must source one side from the other: either **codegen the
storage-side wall consts FROM the DNA source** at build time (so they cannot drift), or have the test read
the DNA artifact. Specify the codegen; "assert the two consts are equal" is not a guard if both are typed by
hand.

**[adversarial-fix] The clamp precedent is mischaracterized.** `constitutional_ratio_registry.rs:108-114`
clamps a *manifest-read* value — that is **clamp-at-read of an externally-supplied value**, the exact
anti-pattern Cluster #1 forbids ("clamp-at-read recreates the dead-seam lie"). Do not cite it as the benign
"clamps only its own default" precedent. The correct design: the registry clamps **its own hardcoded default
constant** (so a coding error in the default is caught), and **the wall is enforced at the DNA validator
`validate_ratifies_limit_gradient` (reject-at-write)** — a ratified out-of-wall value is *unratifiable*, never
silently clamped. The anti-capture test ranges over the validator's enforced bounds, which is why the coupling
test must bind to the validator, not to a read-path clamp.

---

## 4. GA-endpoint retirement

**The surface.** `GET /api/v1/lamad/content/{contentId}/engagement` (`http.rs:9637`, `.cache_ttl(30)`,
**no `.auth_required()`**). Returns `ContentEngagementStatsView` (`lamad.rs:391`): `views`, `completions`,
`unique_viewers`, `completion_rate`. `unique_viewers = COUNT(DISTINCT provider)`
(`content_engagement_stats.rs:112`) — at low counts a re-identification vector, with no k-floor.

**Decision: RETIRE `unique_viewers`** (not consent-gate). The only consumer (`ContentAnalyticsComponent`
`:59-64`) renders `views`/`completions`/`completionRate` and **never `uniqueViewers`** — the one un-rendered
field is exactly the one de-anon field. Dropping it breaks zero UI and lets the `COUNT(DISTINCT provider)`
SQL be **deleted** (the de-anon query no longer runs, not merely hidden).

**Retire path (schema-first, ts-rs-disciplined):** drop from `content-engagement-stats-view.schema.json` →
`ContentEngagementStatsView` (`lamad.rs:391`) + `From` impl → the `COUNT(DISTINCT provider)` line + DB row +
upsert + Diesel column-drop migration (drop the *read* before the column in the same migration) → regenerate
(`schema:codegen:ts` + `export_bindings`) → **sha256-verify** the diff is exactly one field removed.

**[adversarial-fix] Two missing steps:**
1. **Two hand-written app models also carry `uniqueViewers`** — `governance-feedback.model.ts:1063`,
   `economic-event.model.ts:804` (non-generated, won't be touched by codegen). Add a step: grep non-generated
   TS for the field and reconcile both files, or a structural-typing assignment may break.
2. **"Retired" describes the field, not the surface.** The endpoint stays **ungated and cached**, still
   emitting `views`/`completions` at arbitrarily low N with no k-floor. Dropping `unique_viewers` closes the
   `DISTINCT`-count vector; a low-N *count* vector remains on an unauthenticated route. State this honestly;
   auth-gating the surface is a separate, deferred decision (§Decision 2).

---

## 5. Multi-tenant cache & per-voter de-anon surfaces

**Exposure A — co-mingling (structural).** `attention_tending` (`diesel_schema.rs:1479-1491`) has **no
`h_app_id` column** — unlike `content_engagement_stats`, which partitions every query on `(content_id,
h_app_id)`. At a shem multi-tenant node, tenants' tending rows co-mingle with no partition boundary.

**Exposure B — re-aggregation below k.** `list_by_signer` (`tending.rs:92`, `WHERE signer_pubkey = ?`) and
any `GROUP BY signer_pubkey` is a per-peer surface beneath the k-layer. Guarded today only by "no production
caller" — a policy non-guarantee.

**Exposure C [adversarial-fix · from Cluster #3] — the `attestations` per-voter table.** `compute_tally`
reads `attestations` where each vote is `issuer_cid + vote_value + parent_governance_action_cid` in clear.
This is the governance analog of Exposure B, and it is **wired** (the tally reads it). It is gated by the
Cluster #3 **votes-auditable-vs-private decision** (§Cluster-3 Decision 4): if auditable-by-design, the tally
is the *only* sanctioned reader and any per-`issuer_cid`-group query is grep-guarded/forbidden; if private,
`issuer_cid` gets the HMAC treatment below. **This cluster owns the read-path guard test** (the analog of
`list_by_signer`-is-unwired) — asserting no production read path does `GROUP BY issuer_cid` on `attestations`
below k.

**v1 status: A/B/C guards specified; A/B latent under household-only (single tenant).** But any aggregate
that will ever run at a shem node (`GovernanceParticipationCandidate`, `concentration_snapshot`) **must respect
`h_app_id` partitioning from day one** — it cannot be retrofitted after data co-mingles.

**The specified guard (for shem's return):** add `h_app_id` to `attention_tending` (closes A structurally);
HMAC `signer_pubkey` → per-node pseudonym at write time (closes B — un-reversible beats query-forbidden,
preserves re-tend idempotency); the firewall test on the emitted aggregate is the backstop. **[adversarial-fix]
Promote to v1 LANDS: delete `list_by_signer` outright** — it is unwired, so deleting the per-signer query path
is a free structural win that does not wait for shem.

**The named residue (operator-as-adversary).** HMAC + per-node-salt defends against cross-tenant and
cross-node correlation, **not** against the node operator who holds `node_secret` + a candidate-pubkey list
(they can HMAC any candidate and confirm "did X tend Y"). Closing that needs giving up write-time
idempotency-by-signer (dedup by CID alone). Named and handed to the operator; under household-only v1 the HMAC
guard is sufficient for the floor.

---

## 6. Dependencies

- **On Cluster #1:** the anti-capture test's bounds couple to `validate_ratifies_limit_gradient`'s enforced
  walls (the coupling test, §3) — **not** a read-path clamp. The clamp-vs-validator distinction is load-bearing.
- **On Cluster #3:** the governance aggregates Cluster #3 wires are firewall-checked here; the `attestations`
  read-path guard is gated by Cluster #3's votes-auditable-vs-private decision. Cluster #3 routes the signals;
  Cluster #2 proves the routing carries no voter through aggregation.
- **No DNA-hash change originates here** (GA retire, firewall tests, anti-capture test, cache migration are
  storage+TS+test only). `validate_ratifies_limit_gradient` is Cluster #1's landing.

---

## 7. Open decisions for operator

1. **GA: drop vs k-floor `unique_viewers`** — recommend drop (UI-unused, de-anon-bearing, reversible).
2. **Auth-gate the GA surface regardless?** Even sans `unique_viewers`, low-N views/completions on an ungated
   cached route are mildly disclosive. Recommend defer (separate, heavier per-content-steward gate).
3. **[headline] Anti-capture test: prove monotonicity (corners) vs admit sample.** Recommend (a) — prove
   `effective_friction` monotone per-dimension, assert at the `2^4` wall corners; it is a stronger proof and
   cheaper than a fine grid.
4. **k≥5 under-measurement — flag enough, or compensate?** The firewall under-counts concentration where
   capture is easiest. Routes to the parked sociocratic/wisdom cluster (wall-width setting,
   `constitution/src/prompt.rs`).
5. **Cache `node_secret` provenance** (held until shem): conductor-key-derived vs per-install sealed random;
   must be restart-stable (idempotency) and non-derivable cross-node (correlation).
6. **Coupling-test mechanism:** codegen storage-side wall consts FROM the DNA source (recommended) vs test
   reads the DNA artifact. "Assert two hand-copies equal" is not acceptable.

---

## 8. v1 slice (household-nodes, no shem; the green tests)

**LANDS:**
1. **Firewall test suite** — the mold cloned for each net-new struct *as those structs land* (same-commit),
   flavor-extended forbidden sets. (Net-new structs' tests are green-when-dependency-lands; the mold itself is
   proven now.)
2. **Anti-capture property test** — per Decision 3: monotonicity proof + corner assertions (recommended), with
   the **non-vacuous** wall-mirror coupling (codegen from DNA).
3. **Below-k suppress test** for a governance-aggregate variant (4-cohort emits nothing).
4. **GA retirement** — `unique_viewers` dropped end-to-end incl. the two hand-written TS models; the
   `DISTINCT` SQL deleted; sha256-verified.
5. **[adversarial-fix] Delete `list_by_signer`** (free structural win) and the **`attestations` read-path
   guard test** (no per-`issuer_cid` group below k), gated by Cluster #3's votes decision.

**HELD (gated on shem):** the `attention_tending` `h_app_id` + `signer_hmac` migration (latent under
household-only single-tenant) — specified now so day-one partitioning constraints on the new aggregates are
honored.

**The honest v1 position:** the firewall is sound (compile-time structural + enumerated backstop); the
anti-capture property is a *proof* iff monotonicity holds (else a documented sample); the GA *field* de-anon is
closed by retirement (the *surface* stays ungated — named); the per-voter governance surface is gated by the
Cluster #3 votes decision; the multi-tenant cache is latent with its guard specified. The one thing this
cluster **cannot** prove is that the walls themselves are narrow enough against a determined small-cohort
capture — that needs the parked floor-meets-ceiling sociocratic model, and the firewall cannot answer it.
