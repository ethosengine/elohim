---
name: project_inequality_curve_as_bounded_standard
title: Inequality curve as a bounded standard
description: Operator's origin idea (pre-AI, pre-Gesell) — a token that knows its place on the global inequality curve and applies friction to the gradient (dignity floor + limitarian cap); now held as a Mishpat policy over the REA record, door kept open for legacy currency — bites when designing economic bounds, currency posture, or the shefa/economy pillar
metadata:
  type: project
---

The operator's own evolution on inequality (shared 2026-09-05):

- **The standard humans already hold.** The 2012 "Wealth Inequality in America" video
  (Norton & Ariely survey, 5,000+ Americans): 92% across parties chose an *ideal* curve far more
  egalitarian than reality yet still meritocratic (top ~10-20x the poorest, poverty line off the
  chart, thick middle). Perception sits between ideal and reality; reality (top 1% ≈ 40% of wealth,
  bottom 80% ≈ 7%) is invisible to most. The ideal is the STANDARD; perception is the model;
  reality is what must be measured — the same triad as the habit register (declared status ·
  observed status · evidence, covenant rule 4).
- **Limitarianism (Ingrid Robeyns):** a political upper limit to wealth is desirable, and the
  limit is CONTEXTUAL to the floor a society has collectively agreed to bear — US survey
  participants perceive the limit ~10x higher than Dutch ones, plausibly because the US safety
  net is thin. So the cap is a function of the floor, never a universal number.
- **The origin idea (before AI, before Gesell/demurrage):** a token (think ETH-like) that could
  know where it sits on the global inequality curve and apply mathematical friction to the
  gradient — building the floor and capping the excess — as the candidate for addressing the
  generator function of inequality at its root, rather than redistributing after the fact.
- **Where it sits now:** an objective answer WITH context exists for what is healthy for
  humanity, and only a system like the Elohim Protocol can hold it uncorrupted — as a
  **Mishpat policy** (justice = restored capability, friction on a gradient, never punishment or
  confiscation). The fabric already carries the primitive: `Bound { limit, unit, threshold_pct,
  sense: Floor|Ceiling, source: Declared|rolled-up }` on a Commitment (elohim/epr-rea model.rs),
  with the algedonic channel firing at the band edge — Robeyns' cap is a Ceiling bound, dignity
  is a Floor bound, both DECLARED per scope (the contextual part), measured as position on the
  curve stock.
- **Door held open:** legacy currency (money) is not foreclosed — it can be admitted as one
  reading over the REA record if it can be held under such a bounded, stasis-aware policy.
  Currency is information ([[project_monetary_posture_currency_is_information]]); tokens are play
  over the REA floor ([[feedback_framing_guards]]). Don't close on it, don't mint it early
  (Stance I.4 preconditions).

**Why it matters:** this is the economic north star behind the shefa/economy pillar and the
Value Scanner's "Tokens Come Last" section; it explains why bounds have floor AND ceiling
sense, why limits are declared per scope, and why the protocol refuses to be a currency issuer
before the bounded-commitment plane can hold the curve. See
[[feedback_justice_mishpat_not_punishment_guard]], [[project_georgist_common_inheritance_framing]],
[[project_rea_compute_commitment_primitive]].

**The sensing arm (operator, 2026-09-05; corrected same day):** lamad and psephos are the
Elohim's sensors. **psephos = the psychometric assessments** (stated preferences, self-knowledge —
imagodei-surfaces-design.md:58 lists "psephos psychometric results" under declared preferences),
rendered by Sophia's discovery mode; it is NOT ballot hygiene. The Norton–Ariely survey is exactly
a psephos instrument: it measures the PERCEIVED curve and the IDEAL curve a person holds. lamad
(learning, mastery paths) measures capability and growth. The register/stocks measure REALITY.
VSM System 4 sensing feeds System 5 policy: the Mishpat plane declares the floor/ceiling bound per
scope from those readings, continually, so the whole helps us realize our ideals without any
individual's bias hijacking the whole — aggregation is witnessed, reach-gated and plural
(plural-mishpat-lenses plan), never one actor's number. Formal ballots are qahal's governance
MECHANISM (levels 3–7), a separate thing from sensing. Neither species is terminal; the method is
([[feedback_human_loop_not_terminal_authority]]).

**Naming drift to reconcile (noted 2026-09-05, not fixed):** `app/elohim-app/src/app/qahal/components/psephos-ballot-wrapper/`
and `app/lamad/src/generated/mechanism-selection.ts` (`renderTarget: 'angular' | 'psephos'` for
formal voting) use "psephos" for the ballot renderer, and the a2o surface census row for
collective-governance.feature says "ballot renders via Psephos". The operator's definition is
psychometrics; the ballot naming is drift (or an overloaded pebble metaphor) — raise before
touching either surface.
