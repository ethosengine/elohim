---
title: "Playnet / Free-Association — Cross-Pollination: the labour-time planner that independently reached for our substrate, and what we take from it"
status: Capture
date: 2026-08-05
sovereignty-frame: adversary
---

> **Frame note.** This survey quotes and argues against the crypto *self-sovereignty* ontology; it
> never adopts it. Elohim's apex identity tier is imago dei — an inviolable right **backstopped by
> community and institutional expression**, not a self-asserted cryptographic primitive. Where
> Playnet is measured against that standard (§5.3, §5.4) the finding is that their apex —
> citizenship, *"standing in the association itself"* — is **closer to ours than the crypto-SSI
> space**, and that their defect is the *missing floor beneath membership*, not the identity
> primitive. Canon: `genesis/docs/architecture/stewardship-over-sovereignty.md`; `values-forward.md`
> Stance II.4.

# Playnet / Free-Association Cross-Pollination — August 2026

**[Playnet](https://playnet.earth)** is a Berlin-centred volunteer collective (GitHub org
`interplaynetary`, fiscal host Open Collective Europe) that published *"Playnet — literally an
actual alternative to capitalism"* (Draft v0.5, 2026-06-29, 93 pp.): a complete, equation-level
specification of a **labour-time planning economy** for a federation of producers, ValueFlows-native,
with a working convex solver, a shipped browser client, and a legal appendix. Where
[Holepunch](epr:holepunch-p2p-dataplane-cross-pollination-2026-06-24) mirrors our transport and
[p2panda](epr:p2panda-cross-pollination-2026-08-04) mirrors our crate discipline, **Playnet mirrors
our economics** — it is the only project surveyed to date that has independently built toward the
same REA/ValueFlows substrate *and* answered the question our shefa pillar has not: what is the
unit, and how does the ledger close?

**This survey is self-sufficient by design.** A 6,740-line machine-readable translation of v0.5 was
produced as working scratch and is **discarded on close** — it is a derivative of a document that
states no licence, and it does not belong in the repo. Every equation, mechanism, and quantity we
need in order to re-implement or re-reason is reproduced below, so nothing of value leaves with it.

**Re-retrieving the source, if a later implementation needs the full document.** The v0.5 PDF is
served from inside the `playnet.earth` client (Typst 0.15.0-produced; no `.typ` source was public as
of survey date, though the live Radicle repo may now carry one). Two transcription traps, learned the
hard way and worth the minutes they save: **formulas must be read from rendered pages, never from
`pdftotext`** — text extraction tears under-brace annotations from their operands and reflows them
into unrelated lines, silently corrupting every display equation; and **the public Radicle seeds are
a stale, architecturally-superseded mirror** (§1). Two known internal inconsistencies in the source
itself: Figure 1 does not balance (fourteen cells sum to 10,200 h against a header reading ~10,000 h
— illustrative only; Figure 2 balances exactly and is safe as a fixture), and bytes-per-nonzero is
given as ≈88 in one figure and ~250 in the §7.13 world-scale extrapolation — **use ~250** for any
capacity estimate.

**The one-line verdict: Playnet is the most rigorous economic-planning artifact anyone in our orbit
has produced, and its highest-value export to us is not code but a *unit discipline* — the closure
property (`EQ-2.7`) that makes a surface a ledger rather than a chart, and the soft/hard tension
distinction that makes limits legible without making them negotiable. We take the discipline, take
the visual grammar, and refuse the monism: labour-hours as one currency among our six, never as the
numéraire. Their solve, their peer-attested capacity score, and their exit accounting are all
architecturally incompatible with law we have already ratified.**

**Method.** Sixteen agents across one 12-agent workflow and four targeted deep-dives: full-corpus
read of v0.5; headless render of `playnet.earth` (14 views, `pnpm look`); on-disk clone and read of
`free-association` (1,222 files) and `planner` (824 files); Radicle seed API; a `file:line` audit of
our own `epr-rea` / `bridges/valueflows` / `elohim-storage` / shefa surfaces; a red-team pass; and a
completeness critic that adjudicated contradictions and downgraded four overclaims (including one of
mine — see *Corrections*). **Verification key:** ✅ verified in source · ◐ single-source/plausible ·
⚠ web-only/unverified. Network facts are stamped **as of 2026-08-05** and carry re-run recipes.

---

## 1 · Source registry — read this first

Playnet's sources are unusually hard to find, and the reason is structural, not accidental: **their
most complete work sits behind their least discoverable front door, and no surviving surface links
to any other surviving surface.**

| Surface | URL | What lives there | Status (2026-08-05) | How you would ever find it |
|---|---|---|---|---|
| **The client + spec** | `playnet.earth` | Hand-rolled 144 KB single-page app, unminified ES modules, no bundler; MapLibre vendored locally so it "works over the onion / offline / Tor-only, no external CDN call". The v0.5 spec is served from inside it. | **LIVE** ✅ | Essentially only by being told. Links out to exactly three things: its own `/radicle`, a SimpleX invite, and the W3C SVG namespace. |
| **The real code** | Radicle `rad:z27dFkN28LQAsSsG3DVHcfBqYAArQ` | `module/playnet/planner.scm` — the convex QP, ~4,763 lines at live head ◐. Project "playnet", *"a self-evolving commons computer"*. | **LIVE**, low activity ✅ | Via `playnet.earth/radicle`. Their own seed is **Tor-only** (`.onion:8776`), `seedingPolicy: block`, and `/api/v1/repos` **404s** — the repo set is un-enumerable by design. |
| **GitHub org** | `github.com/interplaynetary` | 16 repos. Org display-name is "playnet"; slug is not. | **LIVE, push-dormant** since 2026-04-09 ✅ | Only via Open Collective, or by searching "free-association". |
| **The namesake trap** | `github.com/playnet` | Unrelated user, 3 forked devops repos. | — | **This is what every search for "playnet" hits.** Also unrelated: `playnet.xyz`, `play.net`, Playnet Inc. (WWII Online), `@playnetofficial`. |
| **Money + the link hub** | `opencollective.com/playnet` | *"Playnet experiments with games, organizations, and economies that enable societal flourishing."* Fiscal host: Open Collective Europe Foundation. | **LIVE**, last transaction 2026-08-01 ✅ | The **only** surface that ties `playnet.lol`, `github.com/interplaynetary`, and `lu.ma/playnet` together. |
| **Current public front** | `openassociation.org` | *"Free-Association \| Technology for Better Global Coordination."* React SPA (empty `<div id="root">` to crawlers). Calendly booking link; `info@` / `coalition@openassociation.org`. | **LIVE** ✅ — DNS CNAMEs to `interplaynetary.github.io` | Only from the `free-association` README. |
| **Events** | `lu.ma/playnet` | Playlabs (Berlin, Recife). Adds Instagram / YouTube / TikTok `@interplaynetary`, listed nowhere else. | **LIVE but shows zero events**; individual pages survive as direct links ✅ | From Open Collective. |
| **Docs** | `playnet.gitbook.io/docs` | Thin, undated. | **LIVE** ✅ | **Linked from nowhere we found.** A genuinely orphaned surface. |
| **Chat** | SimpleX group (invite on `playnet.earth`) | *"private, no accounts"* | Presence confirmed; liveness ⚠ | Only from `playnet.earth`. |
| **`playnet.lol`** | — | The former ludic front ("a distributed social game", Playlabs, Telegram). | **DEAD — NXDOMAIN** ✅ | Still listed as the website on GitHub, on Open Collective, and on every event page. |

**Re-run recipe (as of 2026-08-05):**
`curl -s https://playnet.earth/api/v1/node` (seed identity/state) ·
`curl -s https://playnet.earth/api/v1/stats` · `curl -s https://playnet.earth/scopes` (live scope
count) · `curl -s https://seed.radicle.garden/api/v1/repos/rad:z27dFkN28LQAsSsG3DVHcfBqYAArQ`
(**stale mirror — see the staleness trap below**).

### ⚠ The staleness trap — the single most important operational note

The **public Radicle seeds are ~9 weeks behind and architecturally superseded** ◐. At the garden-seed
head (`b69428ba`, 2026-06-02 ✅) the planner is a *relational / miniKanren* engine; at live head that
engine was **deleted and replaced by the convex QP**. An agent doing the obvious thing clones the
garden seed, reads the wrong architecture, and reports confidently wrong findings. **Every
Playnet-derived claim must carry a head sha and the source endpoint** (`playnet.earth/api/v1/...`,
not the garden seed).

### People and funding — handled deliberately lightly

The Open Collective page names ~10 admins/contributors and per-donor amounts, and the git history
carries ~7 further author names, **several of which are probably fixture personas** (one appears 48×
as a `Co-Authored-By` trailer alongside model names) ◐. A permanent research document should not
carry unverified names or private donation amounts, so this survey cites **totals and URLs only**.
The load-bearing, verifiable facts:

- **One person authored the overwhelming majority of every repo** ✅ (~838/1257 commits at the org
  level; 550/909 in `free-association`; 49/51 in `planner`). Radicle delegate `threshold: 1`.
- **Lifetime funding €13,945 received / €12,418 spent / €77.92 balance**, ~€15/month recurring, with
  **64.5% of all income from a single €9,000 contribution on 2025-07-09** ✅.
- **Lynn Foster — a co-author of the ValueFlows specification itself — has one commit to
  `interplaynetary/protocols`, titled `Update VF definitions` (2026-02-21)** ✅. This is the single
  strongest external signal in the org, and it is a *shared-lineage* fact for us: ValueFlows/hREA is
  already a named foundation in our own research index.
- Licences: `planner` and `free-association` **AGPL-3.0 + §7 additional terms** (user-facing
  attribution; protocol-fidelity notice); `playtime` **Apache-2.0**; `councils`/`dpi` MIT;
  **`protocols` — the repo holding the VF wire model — has no licence file at all** ✅. **The v0.5
  document itself states no licence** ✅.

---

## 2 · What Playnet actually is — three programmes, not one

The most common way to misread this project is to assume the spec and the GitHub org describe one
system. They do not. Development is spread across three largely disjoint programmes sharing a brand,
a namespace, and one author.

| | **A · The spec + its client** | **B · ValueFlows planning** | **C · Recognition allocation** |
|---|---|---|---|
| Home | `playnet.earth` + Radicle | `protocols` → `match` → `planner` | `free-association` ↔ `protocol-lib` ↔ `councils` ↔ `mesh` |
| Substance | 93 pp / 42 equations; hand-rolled SPA; `planner.scm` convex QP | 22 VF record types as AT Protocol lexicons; DDMRP planner; ~8,500-line Guile/Goblins port | Mutual-recognition + distributed IPF app, 909 commits |
| Unit | **labour-hours** | **SNE / SNLT** (effort-hours) | recognition shares (dimensionless) |
| ValueFlows? | Yes (declared) | **Yes, deeply** ✅ | **No — zero VF vocabulary** ✅ |
| Status | Live, unpopulated | Push-dormant Apr 2026 | Push-dormant Feb 2026 |

**Where the seams are weak** — this matters, because it is the ceiling made visible (§6). `planner`
touches `free-association` through exactly **three type-only Svelte imports** of
`@playnet/free-association/schemas`, **undeclared in `package.json` and absent from the lockfile** ✅;
they survive only because `import type` erases at build. None of the recognition mathematics crossed
over. Four identity substrates run in parallel across the org — `did:key`+VC+UCAN, GUN SEA
ECDSA-P256, EAS on-chain attestations, AT Protocol `did:plc` — plus object-capabilities in two
flavours ✅. That is not indecision; it is one person exploring five right answers with no crew to
converge them.

---

## 3 · The mechanisms worth remembering

Reproduced here because the working corpus is discarded. Equation ids are Playnet's own and are
stable across their document.

### 3.1 The objective

- **`EQ-1.1` Harmony** — `H = −(Σ tensions)`. The plan is the allocation that maximises harmony,
  i.e. total tension negated. Units: labour-hours.
- **Desire-tension** (`EQ-1.2`, **soft** — bounded, trades against other tensions):
  `D_d = w_d · g_d²`.
- **Gap** — `g_d = max(0, (q*_d − q_d)/q*_d)` ∈ [0,1]. **One-sided**: surplus reads as exactly zero.
- **Priority weight** — `w_d = β^ℓ_d`. ⚠ **The number to remember, corrected:**
  **β = 1000^(1/9) ≈ 2.1544**, so the *full nine-level span* is 1000×, and a level-9 vs level-6
  desire differs by **β³ = 10×** ✅ (verified verbatim at `harmony.scm:105,109` in the stale-head
  clone, independently of the live-head read). Our discarded corpus asserted "β ≈ 1000, so
  level-9/level-6 = 1000×" at two sites — **that gloss was wrong by 100× and is superseded by this
  paragraph.** It was also self-contradictory: with β=1000 the ratio would be 10⁹, not 10³.
- **Possibility-tension** (**hard** — diverges near its limit, out-pulls everything), via the
  interior-point log-barrier `B_t(u,c) = −t·log(c − u)` applied to skill limits `L_k ≤ A_k`,
  ecological limits `D_r ≤ C_r`, and each governance-set fragility profile.

**The soft/hard distinction is the single most transferable idea in the document** (§5.1).

### 3.2 The labour surface and its closure property

- `EQ-2.6` / `EQ-2.7` — the labour surface is an **area-preserving cell map**: each cell's area is a
  quantity of labour, and **the cells sum exactly to total social labour**. The document calls this
  *"the closure property that makes the surface a ledger rather than a chart."*
- `EQ-2.2` social free time `f`; `EQ-2.3` citizen satisfaction `σ_p` ∈ [0,1]; **`EQ-2.4` social
  desire satisfaction `Φ` = the *harmonic* mean of all `σ_p`** — deliberately worst-off-sensitive.
- **`EQ-3.5`** decomposes desire-tension on the daily turn into **two parts**: `ḡ_d` (day-mean gap =
  *scarcity*) and `Var_θ(g_d)` (variance of the gap around the clock = *cadence*). A good is scarce,
  or badly-timed, or both — and the model distinguishes them.
- `EQ-3.2` — the sourdough worked example: the document consistently teaches a formula by working
  one concrete case before generalising.

### 3.3 Distribution

- **`EQ-3.8` consumer voice** — three terms:
  `v_p^cons = l(1−w)·(L_p/L)  +  l·w·((1−a_p)/Σ(1−a_q))  +  f/N`
  i.e. **contribution + solidarity + a citizenship dividend**. As free time `f → 1` the citizenship
  term absorbs the whole thing. Children are citizens from birth.
- **`EQ-2.10` drawable credit** = `K · v_p^cons`, where `K` is a federation-wide pool sum.
- **`EQ-3.9` capacity `a_p`** — `a_p = Σ_{q≠p} L_q · x^cap_{q,p} / Σ_{q≠p} L_q`, glossed as capacity
  to labour *"attested by fellow workers … excluding self-attestation, weighted by producer voice."*
  **We refuse this** — see §5.3.
- **Producer voice** = `L_{p,s}/L_s` (share of scope labour). Governs recipe selection, plan
  acceptance, scope formation, and capacity-recognition.

### 3.4 Value rollup and skills

- **`SNLT_crystallized(r) = SNLT_direct(r) + Σ_x a_xr · SNLT_crystallized(x)`** — the recursive
  embodied-labour rollup. Their own appendix calls it *"the single most useful equation."* Their
  shipped code implements a **scalar** variant, `SNE` (Socially Necessary Effort), collapsing direct
  labour + embodied inputs + equipment depreciation (`duration/lifespan × SNE`) into one number, and
  excluding `cite` flows (knowledge/IP, amortised ≈ 0) ✅.
- **Skills are conformance, not certificates** ✅. A person *has* a skill when they hold a resource
  conforming to its specification. A spec carries a `satisfied-by` disjunction of skill bundles, each
  with a minimum practised-hours threshold — so *electrician + surgeon* conforms to *electrosurgery*
  without anyone holding "electrosurgeon". Effective skills are the **transitive closure**. There is
  **no certification flag**: whether a skill is learn-by-doing or must-be-trained-first is *emergent
  from the topology* of the substitution graph. Practised hours are summed from the same work log the
  credit ledger sums, so the two can never disagree.

### 3.5 §9 — the credential substrate (their answer to portable trust)

*"The observer is a fold over an immutable log of economic events"* — everything derived, never
stored as a mutable field, so nothing can drift and a voided event simply drops from every total.
Each event becomes a **signed, edge-chained credential** anchored to a self-certifying identifier:

| Credential | Issued when | Edges to | Carries |
|---|---|---|---|
| **Work-event** | work performed | commitment, intent (validation anchor), skill | the hours |
| **Skill** | first performance | its founding work event | *no hours* — a membership token, minted once per (person, skill) |
| **Intent / Commitment** | at planning | — | desired quantities |
| **Claim** | drawing credit for a good | *(none)* | labour-hours spent |

Balance = **gross** (work-event hours) − **claimed** (claim hours). Revocation needs no
reconciliation: drop a credential, every total recomputes. Because cryptographic chains prove what
*exists* but not that nothing is *missing*, a periodic **signed derived-state snapshot** enumerates
the expected set and edges to every credential it should contain — closing the completeness gap. The
in-memory credential store is interface-identical to the live witnessed one, so the same substrate
runs in a test and in the field.

**Two defects we found in it** (§7 — these are gifts to send them): the Claim credential has **no
outbound edges**, so the graph cannot answer *what a claim bought*; and §9's two-term local balance
contradicts `EQ-2.10`/`EQ-3.8`, where balance is a *share of a federation-wide fund* that rescales
when reserves are released — so their offline-verifiability claim does not hold in their own design.

### 3.6 Free-association (Programme C) — a different, cleaner core

- **`MR(a,b) = min(R(a→b), R(b→a))`** — mutual recognition as the elementwise minimum of two
  directions ✅. Recognition weights are self-declared, **sum to 1.0**, and are **non-transferable**,
  so inflating false recognition necessarily deflates true recognition and costs you real allocation.
  Symmetric by construction; purely local; needs no reputation oracle. Self-recognition is exempt from
  the `min`.
- Recognition is *derived*, not declared directly: a points-weighted **tree of goals** whose leaves
  credit contributors (and **anti-contributors**), normalised to the published weight vector.
- **Allocation is a distributed Iterative Proportional Fitting fixed point**: `A_pr = K_pr · x_p · y_r`,
  where the provider owns the row scaling `x_p` (under a fair-share cap) and the recipient owns the
  column scaling `y_r`. Seed kernel `K_pr = (p_prov + ε)^γ · (p_recip + ε)^(1−γ)` with **γ = 0.5**, i.e.
  the geometric mean of both sides' priorities. Two scalars per side reach the same fixed point a
  central solver would.
- **The ε "hidden demand" trick** ✅ — every compatible pair keeps a tiny nonzero edge so the network
  cannot deadlock, then filters at ε² *"because valid hidden-demand seeds are O(ε); if we filtered at
  ε we'd kill the hidden-demand connectivity."*
- **Two-tier allocation**: recipients with `MR > 0` normalise among themselves (tier 1); one-way
  recognition forms a separate "generous" tier 2.
- **Observer-Claim-Effect** (`protocol-lib/src/commons/docs/vc.md`) ✅:
  `Effect = Sign_Observer(Entity, Attribute, ΔValue, Timestamp)`;
  `State(Entity, Attribute)_t = Σ Effect_i`. *"No Central State, Only Effects… We do not trust the
  'Database'. We trust the Observer's Signature on the Effect."*
- **Publish/Derive with sovereign state**: *"There is no 'Global Database'. There are only Local
  Declarations and Network Shadows… The Golden Rule: never persist Derived data as the source of
  truth. Always re-derive it."* Topic namespacing is `{pubkey}/{path}` with the rule *only cache data
  where `topic_prefix == signing_pubkey`* — authenticity and spam-resistance from the namespace shape.
- **Interval Tree Clocks** (not vector clocks, not a CRDT library) for causality — dynamic membership
  without coordinator-assigned replica IDs.

### 3.7 Programme B mechanisms

- **"A scope IS a vat"** ✅ — an organisational unit (commune, ecological agent) is made to coincide
  exactly with a Goblins vat, so all intra-scope operations are **turn-atomic** (single turn =
  all-or-nothing) and free, and inter-scope communication is forced to be explicit. Persistence comes
  free from `make-persistence-env` + `spawn-persistent-vat`.
- `^observer` is *"architecturally a left fold"* over five pure phases: append → index → apply VF
  action effects → implied transfer → track fulfilment ✅.
- **DDMRP buffers as the planning primitive**: six buffer types (`ecological | strategic | reserve |
  social | consumption | metabolic`) × four zones, ordered by `compositeBufferPriority(tier, zone)` so
  *ecological-yellow outranks metabolic-red*. Ecological buffers carry a `tippingPoint` and emit a
  **ConservationSignal** rather than a replenishment signal ✅. Their framing:
  **"Buffer health is intergenerational justice made computable."**
- Their planner **explicitly repudiates the global-optimum framing**: *"We are not trying to find a
  globally optimal solution to need satisfaction… That problem is NP-hard, politically captured, and
  premised on the assumption that the current distribution of productive capacity is fixed."* ✅ —
  note this directly contradicts Programme A's convex solve. The two halves of the project disagree.
- **Metabolic coherence** `coherence[k] = internal_flow[k] / total_flow[k]` ∈ [0,1].

### 3.8 The visual grammar (the reason this survey started)

Rendered and inspected ✅. Nine "faces" over one dial: **time** (the labour-time turn), **map**,
**calendar**, **share** (needs/offers with inline declaration), **shop**, **matter**, **law**,
**fragility**, **governance**.

- **The dial** — area-true radius (area, not radius, encodes quantity), a "hand" you drag to scrub
  time, alt-click a band to re-parent it in the desire taxonomy. Pinch to change turn scale.
- **A single domain-keyed palette registry** — one hue per *domain concept*, so nourishment-coral is
  the same coral on the fragility wheel as on the plan. This is why twelve faces read as **one
  instrument** rather than twelve dashboards.
- **The shop prices goods in hours** — `bread 0.6 h / loaf`, `winter coat 9 h / item` — over a credit
  bar reading `33.5 h DRAWABLE CREDIT`, `12.5 h drawn · 46 h capacity`, and
  **`v_cons 6.2% · f 31% · worked 46 h`**. `EQ-3.8`'s three terms, live on screen.
- **"The metabolic rift"** — a Sankey of C/N/P/K through food → household → excreta → composting →
  soil → field → runoff, with per-substance closure chips (`N 49%`, `P 100%`).
- **The fragility wheel** — planetary boundaries as wedges; **scroll to warp the sector angles by
  "pull" `φ′ = w/s`**, double-click to return to equal sectors. Limits you can *feel* the weight of.
- **`law`** — a choropleth of legal burden per jurisdiction with three readings: *legal burden · who
  operates · dual power*.
- **What is never drawn:** nothing in the entire document plots a quantity **against time**. For a
  §5 titled *The living plan*, whose central claim is continuous metabolisation, the total absence of
  a trajectory or convergence plot is diagnostic. They visualise states, never paths.

---

## 4 · Grounded Elohim reality — what the verdicts adjudicate against

Audited 2026-08-05 from source. The load-bearing facts, because several verdicts below turn on them:

- **`elohim/epr-rea/src/fold.rs` (111 lines) opens with the same sentence as Playnet §9** — and has
  **no signature on any `FlowRecord`**, **no revocation or supersession filter**, **non-deterministic
  f64 accumulation** in both economic folds (the sort-by-CID determinism discipline exists 320 lines
  away in `epistemic.rs` and was never applied to them), and **zero callers outside its own tests** ✅.
  Against a live sidecar where all 305 events are counts of artifacts and all 4,132 commitment
  quantities are `null`.
- **`graduation/`** — 5 files / 318 LoC, `pub mod graduation` at `lib.rs:58`, **only caller is its own
  test** ✅. Its `total_quantity` sums `payload_json["bytes"]` — **hardcoded to a byte count**, so it
  cannot carry hours without modification.
- **We have no optimizer of any kind.** `FlowPlanningApiService.optimize()` is a fully-typed Angular
  client calling `POST /plans/{id}/optimize` against `api/flow_planning.rs`, which is **35 lines
  returning 501** ✅. We built the interface to a solver before the solver — and before the ledger the
  solver would read.
- **We do not model labour-time anywhere on the economic path.** `care` = 0 hits in `epr-rea` and
  `bridges/valueflows`; shefa declares **one** observation kind ✅.
- **The Value Scanner is narrative only** — `value_scanner` is first in `epicsWithZeroTests`; the
  shefa surface is one dashboard fed by zero-filled views with **10 of 21 routes as
  `ShefaPlaceholderComponent`** ✅.
- Our canon already declares **six currencies with decay rates** (`shefa.md:299-306`), Time among
  them — so a labour-time currency is *additive* to our design, not a replacement for it ✅.

**The honest statement: our economic substrate is a set of correct shapes that have never been run.**
Playnet's is a complete economy with no network (`/scopes` → `{"count":0}` ◐, as of 2026-08-05). Both
projects have built half of the same thing, and the halves are opposite.

---

## 5 · Verdicts

Per the research discipline: **adopt** means *re-implement it properly in our architecture*, never
vendor the code. We are not shipping their bytes, so licence compatibility is a minor question and
**credit is a major one** — every adopted idea below carries its origin, and should carry it in the
code too.

### 5.1 Adopt as standard

| # | Standard | Why it is the best answer we found | Where it lands |
|---|---|---|---|
| **A1** | **Soft vs hard tension as a first-class semantic primitive.** A *soft* constraint is bounded and trades against others; a *hard* one diverges near its limit and out-pulls everything. | This is the cleanest solution we have seen to a problem we actually have: making a limit legible *without* making it negotiable. It gives ecological and dignity floors a representation that is honest by construction — you cannot buy past a hard tension, and the UI can show *why* by the shape of the curve. | A graphos primitive (`elohim-tension-*`, Library A) taking `{kind: soft\|hard, value, limit, curve}`; and a semantic field on constraint-bearing views. |
| **A2** | **The closure property as a test-time invariant.** *A chart that fails its own conservation law is a bug.* Every surface whose cells claim to partition a total must sum to that total, verified in test. | `EQ-2.7` is the difference between a ledger and a decoration. We have shipped exactly this failure class before — our own `graphos_dead_binding_classes` record names "theming theater," and our shefa dashboard renders placeholder data through a real idiom. | An a2o scenario family + a shared assertion helper. Cheap, and it makes every future economic visual honest. |
| **A3** | **Domain-keyed palette registry** — one hue per *domain concept*, resolved from a single registry, never per-component. | It is the entire reason their twelve faces read as one instrument. It is also the direct countermeasure to the six dead-binding classes we have already catalogued (ghost names, inline hardcodes, `setProperty` clobber, inert kebab attrs). | graphos, ~1 day. **Highest value-per-hour item in the survey**, and it touches nothing economic or political. |
| **A4** | **Two-part tension: scarcity *and* cadence** (`EQ-3.5` — day-mean gap plus variance around the clock). | A shortfall and a badly-timed sufficiency are different problems with different remedies, and almost every dashboard conflates them. For a household care economy this is exactly the distinction between "we don't have enough" and "it never arrives when it's needed." | The Value Scanner's care-balance model; any lamad pacing view. |
| **A5** | **Observer-signature over database-trust** — `Effect = Sign_Observer(Entity, Attribute, Δ, t)`, `State = Σ Effects`. | We already believe this (P1, DHT-as-manifest / storage-as-projection). What Playnet adds is the *discipline*: the signature is on the Effect, and the Effect is the product. It is the missing half of our own `fold.rs`. | `epr-rea` — sign `FlowRecord`, add a revocation/supersession filter, apply the `epistemic.rs` sort-by-CID determinism to the quantity folds. |
| **A6** | **Skills as conformance with substitution closure, and no certification flag.** Competence is summed from the same work log the credit ledger sums; whether a skill is learn-by-doing or must-be-trained is emergent from the substitution graph's topology. | This is a genuinely elegant answer to credentialing that avoids a registry of authorities — and it is exactly the shape lamad's mastery model needs. The "hours derive from the same log as credit, so they can never disagree" invariant is the good part. | lamad mastery / `mastery-attestation-credential-epic`; p2p-design-gate first. |
| **A7** | **Area-preserving surface with a conservation test** — cells whose *areas* (not radii) encode quantity and sum to the declared total. | Survives the scale change to a household (see §5.5). Paired with A2 it is an honest care surface rather than a suggestive one. | Value Scanner care-balance; graphos Library A primitive. |

### 5.2 Study seriously (gated)

- **Labour-time as *one* currency among our six**, denominated in minutes, as a `shefa:care-*`
  observation-kind family — **with no closure claim and no matching gate** (see the erasure argument
  in §5.3). This is the achievable form of the thing that would unblock the Value Scanner, which today
  has *no unit and no mint*.
- **The `SNE`/`SNLT_crystallized` recursive rollup** — if we ever price composite goods. Gated on
  having a `Process`/`Recipe` writer at all, which we do not.
- **DDMRP buffers with ecological tipping-points emitting conservation rather than replenishment
  signals**, and the `compositeBufferPriority` ordering that lets ecological-yellow outrank
  metabolic-red. The best framing of ecological limits we have encountered.
- **`MR = min(A→B, B→A)` as *a* mutuality signal** — with a live caveat: elementwise-min structurally
  under-serves asymmetric care (a parent↔infant relationship has no reciprocal weight to floor
  against). Adopt as one signal, never as the allocation gate.
- **The ε "hidden demand" invariant** — never let a compatible edge go to zero, so the network cannot
  deadlock. Plausibly a general principle for our matching surfaces, not just a numerical trick.
- **"A scope IS a vat"** as a transaction-boundary discipline. We do not run Goblins, but the pattern
  — make the consistency boundary coincide exactly with the governance boundary — is architecture-level
  and portable.

### 5.3 Leave behind, and why

- **Labour-hours as *the* numéraire — refuse the monism.** Being fair to them first: Playnet does
  *not* claim labour-time is value (they quote Gotha directly — labour "does not appear here as the
  value of these products"), care work is a named cell class, care capacity is a named fragility
  profile, and the solidarity term pays *more* to those who can work less. That is a materially
  better position than the strawman. **But the matching gate is an erasure mechanism**: hours count
  only if the planner issued a work-intent, you matched it, and the plan was accepted. Care work is
  definitionally *unmatched, unplanned, self-initiated*. The mental load — noticing that an intent is
  *needed* — has no primitive in their ontology; intent-generation is filed under administration, the
  one category whose stated design goal is *"to minimise this category."* **Their own document
  concedes it**: persons are *"not yet first-class as reproductive labour. This is the design gap most
  urgent to close."* If we adopt hours, we must invert the gate — **witness-first, intent-optional** —
  which is what our observation layer already does structurally, and is the one place we are ahead.
- **`EQ-3.9` peer-attested capacity — blocking.** Your entitlement to a larger solidarity share is set
  by a labour-weighted vote of *other people* about how disabled you are; you are barred from the vote
  about yourself; and it feeds your food. No appeal, no floor. This collides head-on with ratified law:
  Constitution Article II defines DIGNITY as *"the inherent worth of each human that exists prior to
  and independent of their utility, productivity, or social standing"* — and `a_p` **is** an assigned
  social standing indexed to productivity. Our IDD principle is *supported decision-making, not
  substituted judgment*; excluding self-attestation is substituted judgment by construction. **Do not
  port `EQ-3.9`.**
- **The global solve.** Their own hardware bill reads: region 3M → 1 server; nation 70M → 25; globe →
  ~2,600. They frame this as cheap; inverted, **the world's economy is plannable by whoever holds
  2,600 machines**, and the architecture *prefers* centralisation wherever RAM allows (federation is
  forced only when no single machine can hold the problem). That is the exact inverse of our
  hub-optional floor, where a laptop is a full participant and hub-required features are a capture
  smell. A gap certificate proves the plan is optimal *given the model* — and they concede the solver
  *"consumes a model it cannot produce."* **Whoever assembles the model is the planner, regardless of
  who runs the arithmetic.**
- **Credits lapse on loss of citizenship** — with no procedure, appeal, ward status, or residual
  floor. This is the *mirror* failure of crypto self-sovereignty: crypto says no community can touch
  your identity; Playnet says your identity *is* your community standing and nothing else. Our guard
  rejects both — the community exists to **hold** the right, not to hold it hostage. Related: **exit
  is confiscatory by construction**, and there is no rule for what property leaves with a seceding
  scope. Every real federation (Mondragón, the kibbutzim, housing co-ops) has died on exactly this.
- **Human-terminal governance.** Every governance act is producer or consumer voice; ecological agents
  produce but do not vote; there is no non-human governing participant. By our own standing conviction
  that human-in-the-loop is not the terminal authority — *the method is* — that is a capture vector.
  Relatedly, their epistemic move is **certify** (a gap certificate proving optimality) where ours is
  **falsify**. Certification is not falsification. That is the deepest register difference in the survey.
- **§10 (property, law, borders) — never cite in a public artifact.** Much of it is mainstream
  reformism (waqf, ejido, community land trusts, the UK CIC asset lock, German *Verantwortungseigentum*,
  Whanganui personhood, the Wyoming DAO LLC) and its thesis — *"the crown is assembly, not invention"* —
  is legitimate. But the same indexed document specifies ownerless shells, M-of-N quorums so
  *"compelling one signer yields a key rather than a roster"*, rotating nominee fronts, and keeping
  *"the fiat membrane uncrossed."* We are a confessional project doing outreach to seminaries and
  civic-institutional partners; nobody has to read §10 to quote its table of contents.
- **Any code linkage.** AGPL-3.0 on a network-served component is a genuine copyleft event for
  `elohim-storage` and `doorway`, and **we have no root LICENSE to reason against** (only
  `sophia/LICENSE` exists) — that is a defect on our side, independent of Playnet, and worth its own
  backlog item. **Read their code; do not link it.** Ideas transfer freely; code transfer is a legal
  project.

### 5.4 What we already do (and should stop treating as a discovery)

- **Fold-over-immutable-log.** ⚠ **Downgraded from this survey's initial finding.** "State is a fold
  over an immutable event log" is a **commonplace** of event-sourcing / CQRS / Datomic — not a rare
  co-invention. Disjoint *economic* bibliographies do not establish independent derivation of a
  widely-published architectural pattern. The honest read is not "we're validated"; it is
  **"we have a beautiful uncalled function, and so — at ledger scale — do they."** The convergence
  worth naming is narrower and real: both projects independently chose **ValueFlows/REA as the
  economic grammar** and **content-addressed, agent-signed events as the substrate**.
- **ValueFlows/REA as our vocabulary** — and via Lynn Foster's commit, a literally shared lineage.
- **Agent-centric, offline-first, hub-optional participation** — where we are architecturally ahead.
- **Community-conferred standing as the apex identity tier** — worth saying plainly: measured against
  our own guard, **Playnet is closer to us than the entire crypto-SSI space**. Their AIDs are plumbing,
  not ideology, and citizenship — *"standing in the association itself"* — is the apex. The violation
  is the *missing floor beneath membership*, not the primitive.

### 5.5 The scale question, answered honestly

Playnet visualises a **federation planning production**; the Value Scanner visualises a **family
distributing care**. Per-device verdicts:

| Device | Survives to household scale? |
|---|---|
| Area-preserving surface | **Yes** — a care surface whose cells sum to the household's declared hours is honest and legible. |
| Soft/hard tension | **Yes**, and arguably better at small scale — a hard tension is exactly how you draw "this person is at their limit." |
| Scarcity vs cadence split | **Yes** — the most directly transferable of all. |
| Domain-keyed palette | **Yes** — scale-free. |
| Harmonic-mean satisfaction `Φ` | **Yes, and valuable** — worst-off-sensitivity is precisely right for a family. |
| `MR = min` | **Partially** — breaks on asymmetric care; needs a floor. |
| The convex solve | **No.** Meaningless below federation scale, and undesirable at any scale for us. |
| Producer voice `L_p,s/L_s` | **No.** A productivity franchise inside a household is the opposite of what the epic promises. |
| Metabolic-rift Sankey | **Surprisingly yes** — a household nutrient/waste loop is a real, teachable thing. |

⚠ A caveat inherited from the survey and marked as such: the claim that "17 of 31 figures are
federation-or-planetary only" is a classification presented as a count, and was single-sourced.

---

## 6 · The ceiling — shared condition, not their defect

The most valuable thing in this survey is not a mechanism. It is that **Playnet's public ledger dates
the week a peer collective hit the coordination ceiling**, and the ceiling is the field's, not theirs.

€13,945 lifetime is roughly seven weeks of one senior engineer's salary at a firm optimising checkout
funnels. Against that, this collective produced a 93-page specification with 42 equations, a complete
ValueFlows lexicon set on AT Protocol, a DDMRP planning engine, ~8,500 lines of Guile/Goblins, a
working distributed-IPF allocation app, an Interval-Tree-Clock causality layer, a W3C VC identity
stack, a hand-rolled offline-and-Tor-capable client with twelve coherent visual faces, and playlabs on
two continents. **The output-per-euro is extraordinary.** Reading that as underperformance inverts
the finding.

The asymmetry is structural, not incidental. The incumbent system pays €100k+ for people who build
more efficient rent-extraction, because that pays back *inside* existing network effects. An economic
alternative has no payback until it reaches scale, and cannot reach scale without capital that only
flows to things already inside the incumbent's logic. **A coordination alternative starves precisely
where it most needs network effects.**

What the evidence shows, read at the right scale:

- **The €9,000 bought a second developer for about three weeks.** In the single month it ran out, one
  developer's invoice was paid and another's was rejected. Spend then decayed €500 → €170 → €125 → €40,
  and stopped. A natural experiment showing the binding constraint is money-for-people, not will.
- **`free-association` carries 49 open issues — 26 tagged `help wanted` — and 10 open PRs, against a
  repo last committed 2026-02-05.** That is *demand they were starved of the capacity to absorb*.
  People showed up wanting to build this.
- **Four parallel identity substrates and four stacks.** One person exploring five right answers with
  no crew to converge them. We have the same question open and have narrowed it no further.
- **`playnet.lol` was allowed to lapse to NXDOMAIN** — still listed as the website on every remaining
  surface, silently breaking every inbound link.

**What they figured out about persisting anyway, which is transferable:** a fiscal host instead of
incorporation; **shrinking the unit of work** rather than stopping (€27.51 of workshop groceries,
€45.70 to print the spec as a brochure, €40 of playlab facilitation); and **continuing to promote
admins after the money ran out** — investing in membership when they could not invest in code.

And the line worth keeping: *this is a project whose thesis is that coordination should not require a
centre, and it has been unable to coordinate more than one committer at a time. Sovereignty was traded
against reach exactly as the design intends; the bill arrived as an un-mergeable PR queue.* **That is
our condition too, stated in someone else's ledger.**

---

## 7 · Positioning, and what we would send them

**We are not the well-resourced party.** Elohim is one developer with a day job, paying out of pocket
for tooling. In money terms Playnet was better resourced than we are. The correct positioning is
**peer at the same ceiling who found a lever** — the lever being architecture convergence plus a
human+AI collaboration model that dissolves exactly the single-committer bottleneck their PR queue
records. That comparison is the strongest empirical argument this project has, and their ledger is
the control group.

**Readiness gates before any approach** — the operator's judgment is "too early," sharpened into
testable conditions:

1. `epr-rea`'s fold is **called by something real**, with signed records and a revocation filter.
2. At least one economic quantity in the live sidecar is **non-null** (today: 4,132 commitment
   quantities are `null`).
3. One care event is representable end-to-end — the cheap empirical test nobody in this survey ran,
   and which would settle the "we are the observer substrate" claim in about an hour.

**Four things to send them, all gifts, none asking for anything** (they cost us nothing and each is
actionable in minutes):

1. **β is misdocumented** — `*priority-base* = 1000^(1/9)`, so the nine-level span is 1000× and
   level-9/level-6 is 10×; the appendix's "β ≈ 1000" reads as a 1000× *per-level* ratio.
2. **§9 and `EQ-2.10`/`EQ-3.8` specify different balance semantics.** §9's balance is a two-term local
   fold, offline-verifiable; `EQ-2.10`'s is a share of a federation-wide pool that rescales when
   reserves release. The offline-verifiability claim does not hold in their own design.
3. **There is no exit accounting.** Secession is specified for flows and not for property, and credits
   lapse on loss of citizenship — so exit is confiscatory by construction. This is the property that
   most reliably converts a voluntary association into something else.
4. **The tension registry has no ratchet.** `w_a` is governance-set, so a captured or merely tired body
   can legalise any ecological breach by lowering it — *"without touching a line of solver code,"* their
   own boast read adversarially. (Stated as *we did not find one* — §8 was not read completely.)

Plus one question: **what licence covers the document?**

Channel: `coalition@openassociation.org`; insist on Radicle issues/patches (public, signed, archived)
for anything technical rather than SimpleX, which is unattributable by design. **Do not use the word
"partner"** — their own terms read `[operator not configured]`; there is no counterparty to contract
with. Lead with the findings, not a proposal. And note the mirror: being cited by a large, AI-heavy,
church-adjacent project may cost *them* credibility in *their* milieu; a proposal that does not account
for that reads as naive.

The one sentence that earns a second sentence, in the operator's own words:
**"We're looking for friends who need a new social contract that can scale."**

---

## 8 · Corrections and limits of this survey

- **Overclaim corrected:** the fold convergence was initially graded "maximum strength — genuine
  independent arrival." It is a commonplace of event-sourcing. Downgraded in §5.4. The survey warned
  about this seduction (the "fold-mirror" risk) and then committed it — worth remembering as a pattern.
- **β was wrong in our working corpus at two sites**, with an inverted editorial gloss. The corrected
  value is in §3.1; the corpus is discarded, so this paragraph is the record.
- **"The spec has no implementation" was wrong.** It was true of `github.com/interplaynetary/planner`
  and false of the whole — the convex QP lives in the Radicle repo. Beware the staleness trap (§1).
- **Not read:** §8 *"Take back the playnet!"* (231 lines) — which means the "their ecology model is
  excellent" verdict and the register/positioning read both rest on adjacent sections rather than the
  political core. `12-glossary.md` never read whole. The Radicle `/boards/` (issues/patches) never
  inspected, though §7 recommends it as the channel.
- **Single-sourced and decaying:** `/scopes` = `{"count":0}`; the live-head sha and `planner.scm` size;
  all Open Collective figures. Re-check before relying.
- **Falsifiers** — what would overturn the main claims: *if `/scopes` > 0*, the "complete economy with
  no network" reading inverts; *if the live `planner.scm` lacks `*priority-base*`*, the β correction
  rests on one unreplicated read (mitigated: the stale-head `harmony.scm` gives the same value
  independently); *if `graduation/` gets wired*, several rows of the substrate comparison move.
- **Names:** deliberately omitted except Lynn Foster (a public, verifiable, load-bearing signal).
  Several git-author names in the org are probably fixture personas.

---

## 9 · Outputs

Per the mint-pass discipline, surviving adopt-items fold as rows into backlog clusters citing this
survey's slug; takes not worth a cluster row die here in the prose.

**Mint pass completed 2026-08-05.** Two new clusters were minted and one existing cluster extended;
the operator's steering note reframed the largest group before it landed.

| Destination | What went there |
|---|---|
| **[measure-family-borrows-backlog](epr:measure-family-borrows-backlog)** (new, 11 rows) | Every economic take, reframed as **Measure families** rather than currency decisions — hardening `epr-rea`'s fold (row 1, the prerequisite), the closure invariant, labour-time-as-a-family, parallel families toward a lamad credential, soft/hard limit semantics, scarcity/cadence, worst-off aggregation, the SNE rollup, skills-as-conformance, `MR = min`, the ε invariant. Plus the four explicit refusals, recorded so they are not re-proposed. |
| **[design-legibility-borrows-backlog](epr:design-legibility-borrows-backlog)** (new, 7 rows) | The visual grammar — palette registry, tension render primitive, area-preserving surface, show-the-trajectory, two-tone realized/unrealized, a headline metric naming its binding lever, and a retirement discipline for superseded surfaces. Rows 2–3 are render-halves that must land paired with their measure halves. |
| **[arch-workspace-discipline-backlog](epr:arch-workspace-discipline-backlog)** rows 9–10 | The missing root LICENSE (which makes any contamination analysis undefined on our side), and the `.epr-meta` policy-relaxation gap found while building this directory's sovereignty membrane. |
| Outstanding | The four-gift email + the licence question (§7) — not yet written. |

### The steering note that governed the mint (operator, 2026-08-05)

Playnet uses labour-hours; their planner uses SNE/SNLT; free-association uses recognition shares.
**We adopt none as *the* unit.** The EPR substrate stays **agnostic to the measure applied**,
supporting in theory any aggregate the substrate can observe or derive at observation time — which is
part of the **story + value + governance coupling**, and which now has a home in the
[Middot Measure primitive](epr:middot-measure-primitive-design). Every economic take was therefore
re-framed as *a Measure family*, composing with middot rather than competing with it. The worked case:
**observed hours toward a lamad credential** is one family, to be composed in parallel with
skill-practice energy/"attention" and quiz results — three families over one subject, aggregated by a
lens, never collapsed into one unlabeled number.

## 10 · Credit

The ideas above were arrived at by the Playnet / Free-Association collective over roughly two years,
largely by one person, on about fourteen thousand euros. Where we adopt one, we should say so — in
this document and in the code. A commons that does not credit is extraction with better manners, and
their own AGPL §7 attribution term asks for less than we should give voluntarily.

Particular credit: the **soft/hard tension** distinction and the **closure property** are, in our
judgment, original contributions of real quality; the **single domain-keyed palette registry** is the
best small piece of design engineering in the survey; and **`MR = min(A→B, B→A)`** is an elegant
answer to a problem most systems solve with a reputation oracle.
