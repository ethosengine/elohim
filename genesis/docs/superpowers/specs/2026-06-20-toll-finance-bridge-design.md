---
title: "Toll / Fiat-Interop / Commons-Pool — the doorway Role-2 finance-bridge (Wave-4 design of the Doorway Membrane arc)"
id: toll-finance-bridge-design
status: Draft
class: protocol-canonical
domain: D8
sprint: vision-deferred
cites:
  - epr-reachability-economics | KEYSTONE — the Role-2 finance-bridge this spec elaborates; §9 open questions = the forks; the purely-compensatory invariant (toll != availability) | sha256:19e359867f22af5a | path: genesis/docs/superpowers/specs/2026-05-29-epr-reachability-economics.md
  - doorway-membrane-prosocial-routing-design | the membrane arc whose §7.1 binding canon this composes (zero new entry types; facilitate-not-bank; bridge = external-system interop) | sha256:10ba2875185c52b0 | path: genesis/docs/superpowers/specs/2026-06-20-doorway-membrane-prosocial-routing-design.md
  - per-substrate-limitarian-governor-design | the limitarian ceiling that caps the market; §9 substrate_signal gate blocks toll-reward concentration capping (built for attention/token only) | sha256:5d10a556e2ec7a14 | path: genesis/docs/superpowers/specs/2026-06-09-per-substrate-limitarian-governor-design.md
  - vision-gap-limit-governor-stub | the limit-governor stub (un-blessed) + the conceptual-only donut floor/ceiling governor — the upstream gate on the capped-by property | sha256:14ea8f3e81cd87c8 | path: genesis/docs/superpowers/plans/2026-06-14-vision-gap-limit-governor-stub.md
  - mutual-storage-replication-dwelling-hub-design | commons-pool + donut split ratios + COMMONS_MIN_FLOOR + proposed->active graduation on ProvideAnnounce | sha256:1acbeeec8b7a3956 | path: genesis/docs/superpowers/specs/2026-05-28-mutual-storage-replication-dwelling-hub-design.md
  - non-commons-provide-commitments-design | the replicates-* capacity-variant Commitment — commons-pool membership / opt-in hosting backing | sha256:936b660644fde390 | path: genesis/docs/superpowers/specs/2026-06-13-non-commons-provide-commitments-design.md
  - rea-compute-commitment-primitive | rea-compute-commitment-primitive | sha256:3ea123e3a9796449 | path: genesis/docs/architecture/rea-compute-commitment-primitive.md
refines: genesis/docs/superpowers/specs/2026-05-29-epr-reachability-economics.md
# DESIGN-ONLY / vision-deferred. NO doc-level requires_env — but implementation is gated on TWO real
# upstream items (see §5): the limitarian governor's `substrate_signal` field (to cap toll-reward
# concentration) and the donut floor/ceiling governor (conceptual-only). Plus a fiat-rail choice + the
# bridges/fiat crate (neither exists). v1 of the membrane ships NOTHING here (membrane §7.1).
---

# Toll / Fiat-Interop / Commons-Pool — Role-2 Finance-Bridge (Wave-4 design)

> **One line:** the elaboration of the keystone's **Role-2 finance-bridge** — a `bridges/fiat` crate that
> lets non-stewarding clients pay a **toll** to reach uncontracted commons content; tolls become **traffic
> rewards** routed to the peers/stewards who bore the cost and to **commons pools**, bounded by the
> limit-governor. The doorway only *facilitates* — it routes value through, never banks it, and is never
> the toll authority. **Design-only:** the chain composes from existing primitives (zero new DHT entry
> types) but implementation is gated on a fiat rail + the limit-governor's `substrate_signal` (§5).

## Provenance

The keystone `epr-reachability-economics` (§3) names two doorway roles: Role-1 (projection cache, built)
and **Role-2 (resolver / finance-bridge)** — and says Role-2 "is the next epic and deserves its own P2P
design gate." The membrane spec §7.1 stubbed it as a named fast-follow with the binding economic canon
(zero new DHT entry types; doorway facilitates-not-banks; doorway rewarded via core rails; bridge =
external-system interop). This spec is that gate + that elaboration. It is **the mechanism**; the keystone
is **the frame** — kept separate deliberately (the keystone asked for it).

## 1. The model

A non-stewarding client (search engine, bot, browser — no standing, no projection contract) requests
commons content lacking an explicit projection contract:

1. **Head/name→CID resolution is free** — commons heads are visible to anonymous visitors.
2. At the **bytes** step, the Role-2 resolve path reaches a serving peer whose **peer-side standing
   check** finds no standing → it signals "payment required."
3. The **doorway translates** that into an HTTP **402** + a payment challenge (honest pricing of the
   externality a web2 browser never sees — *not* a paywall).
4. The client pays an external micro-transaction; **`bridges/fiat`** translates the confirmation into an
   **epoch-aggregated REA `EconomicEvent`** (never per-toll) and **routes** the value out, split by
   donut/constitutional **steward / collective / commons** ratios, to the parties that bore the cost —
   recognized as **`appreciation` EconomicEvents** `bounded_by` the serving peer's `replicates-*`
   Commitment.
5. The reward funds peers who **opt-in to host cached EPRs**, building **commons pools**
   (`replicates-commons` capacity pledge above `COMMONS_MIN_FLOOR_PCT`).

The whole emergent market is **subordinate to and capped by the limit-governor** (limitarian ceiling +
donut floor/ceiling).

**⚠ Load-bearing correctness invariant (keystone §4, lines 94–95): the toll is purely COMPENSATORY — it
does NOT increase availability.** Only the in-kind `replicates-*` path adds replicas. The commons pool
*funds reward*; it is **not** the replication mechanism. "Tolls build commons pools" must never be read as
"tolls add replicas." Demand drives replication via the in-kind path; money is honest pricing of an
externality. (This is the single most likely misreading and the spec forecloses it.)

**New (genuinely):** the `bridges/fiat` crate; the fiat↔REA *conversion* concept (accept external payment
→ mint epoch-aggregated `EconomicEvent` → route to `appreciation`); the chain wired as ONE flow; the
conversion rate/policy (config); the `Verdict::Toll` variant + 402 render seam.
**Composes from canon:** REA `EconomicEvent` (Weave #3), `appreciation` EconomicEvent, `replicates-*`
capacity Commitment, `delegates-compute` primitive, donut split ratios (`mutual-storage-replication`
§5), peer-side frontier enforcement, the membrane bridge-mount pattern (`valueflows`).

## 2. P2P Design Gate output

**Zero new DHT entry types** — the claim holds across all entities. Two are pre-blessed by membrane §4·6/§4·7;
three are where a new type could sneak in and are reasoned through. Every economic attribution joins on
**`agent_cid` directly, NOT `AgentPeerBinding`** (it is `STAGE1_SIGNATURE_SENTINEL`, self-asserted — forbidden
for economic attribution until a cross-signed proof lands).

| Entity | Classification | New DHT type? |
|---|---|---|
| **Toll receipt** | Notarized-A **reuse** — REA `EconomicEvent` ("fiat is another resource flow"). Per-toll detail stays **Operational-C**, surfaced only as an **epoch-aggregated** EconomicEvent (never per-toll — granular-data-on-DHT anti-pattern). | **No** |
| **Commons-pool membership / opt-in hosting offer** | Notarized-A **reuse** — `Mishpat::Commitment` `replicates-*` capacity variant; graduates `proposed → active` on first `ProvideAnnounce`. **The "offer" is a Commitment in `proposed` state — NOT a new REA Intent/Offer type** (resisting the REA-textbook reflex is the gate's whole job). | **No** |
| **`Verdict::Toll` variant** | **Not a DHT entity** — Operational-C. An in-memory `guard::Verdict` variant + an HTTP **402** render seam (sibling to the 429/`Challenge` arm). Edge-local, reconstructable, never notarized. | **No** |
| **Fiat↔REA conversion record** | Operational-C — the bridge's local translation ledger (the `valueflows` `TranslationPoint` observation pattern). **Canonical** record = the reused epoch-aggregated `EconomicEvent`; the bridge ledger is fire-and-forget provenance. Conversion rate/policy = **config**, not a notarized entity. | **No** |
| **Traffic reward / settlement payout** | Notarized-A **reuse** — `appreciation` EconomicEvent (provider→peer mutual credit), `bounded_by` the peer's `replicates-*` Commitment. Split *ratios* = donut/constitutional config. | **No** |

**Discipline to enforce at build:** the opt-in "offer" stays a `proposed` Commitment, and per-toll detail
stays Operational-C — the two spots where an over-eager designer would otherwise mint a new entry type.

## 3. The `bridges/fiat` crate

A **bridge** = interop with a NON-PROTOCOL/external system (here, fiat payments) ↔ the canonical EPR-REA
substrate — distinct from membrane *edge functions* (TLS/DNS/CDN/`guard`), which operate over the *same*
substrate and are not bridges.

**Structure** (mirrors `valueflows`' three-crate split — leaf crate, no elohim-workspace dependency):
- `fiat-types` — stable enums + conversion-policy schema (no async-graphql/hyper).
- `fiat-bridge` — `pub async fn handle_request(req, ctx) -> Result<Response, BridgeError>` + `BridgeContext { pool }`; **mounted by `doorway-service`** on one route, exactly as `atproto`/`activitypub` mount. Translate fns pure; the Operational-C conversion observation is fire-and-forget (`spawn_blocking`).
- `fiat-tests`.

**Inbound (toll paid):** external payment confirmation → mint the **epoch-aggregated** `EconomicEvent` (on `agent_cid`).
**Outbound (reward settled):** route the collected value by donut ratios → `appreciation` EconomicEvents to serving peer / author-steward / collective / commons pool, each `bounded_by` the recipient's `replicates-*` Commitment.

**Invariants (binding):**
- **Routes value through, never banks it** (external validation: x402's facilitator-never-banks pattern).
- **Doorway rewarded via core rails, not a separate system** — operators are peers first; their
  projection/edge/bridge-facilitation compute is recognized on the *same* `delegates-compute` +
  `appreciation` rails every contribution uses. Only a *parallel/duplicated* toll-accrual mechanism for
  doorways is out of scope.
- **The doorway is never the toll authority** — the economic frontier is enforced **peer-side** (the
  serving peer's standing check). A doorway-resident gatekeeper recreates a platform gatekeeper (keystone
  §9 Q3). Same architectural fact as the `http-reach-enforcement-gap` backlog (fine-grained reach runs
  only on the P2P resolve path).

## 4. Genuine forks (operator decisions — to lock at the implementation `/plan`, when upstream lands)

These do not need resolution to hold the design; they are surfaced with a recommendation each.

- **A — Fiat rail.** L402/Lightning (402 + macaroon + LN invoice; *cacheable/attenuable* credentials —
  pay-once-per-endpoint + spend-cap caveats = a natural limit-governor seam) · x402/stablecoin (402 +
  `X-PAYMENT`; facilitator verifies+settles, never banks; reusable sessions) · Interledger/Web-Monetization
  (streaming micropayments; *probabilistic revenue sharing* = the model for capacity-weighted commons-pool
  splits) · Stripe-classic (familiar but operator-banks-custody is the anti-pattern canon rejects).
  *Recommendation surface:* L402 or x402 for cacheable-credential amortization + facilitator-never-banks;
  Web Monetization's recursive split for the commons-pool payout.
- **B — Settlement split mechanics & where computed.** Keystone §9 Q4: a boutique read may split across
  server(`serve-blob`)/author-steward/collective/commons per donut ratios — is multi-party settlement the
  intent or v1-overkill? Where computed: `bridges/fiat` (config-driven donut ratios) vs structurally
  (payment-pointer recursion). **The split ratios ARE the donut floor/ceiling expressing itself at settlement.**
- **C — Head-visible/bytes-metered + peer-side frontier boundary.** Working assumption (keystone §9 Q1/Q3):
  heads free to all; toll meters only bytes; doorway *issues* the 402, the **peer enforces** the standing
  check. Operator confirms the line + that there is no doorway-resident toll authority.
- **D — Crawler/bot policy.** Pay-per-crawl (Cloudflare Pay-Per-Crawl shape: default-block → 402 → flat
  per-request price) vs a **commons free-tier** for heads/low-volume. *Positive-sum framing:* a commons
  reward is the counter-incentive a pure block-wall lacks (avoids the IP-rotation/UA-spoof arms race).
- **E — `Verdict::Toll` integration with the Wave-2 guard stage.** Cleanest framing (verified against
  `guard.rs`): **orthogonal axes.** `guard.assess` is rate-only, content-blind, source-keyed, early. The
  toll decision needs three things `assess` lacks: (a) no projection contract, (b) non-stewarding/no
  standing, (c) a *bytes* request. So `Verdict::Toll { challenge }` extends the **shared enum** but is
  **emitted on the Role-2 resolve/serve path, not by `assess`** — the membrane keeps emitting only the four
  rate verdicts. A toll-payer is still bannable for flooding (can't buy past abuse); you can't rate-limit
  out of the toll. Operator confirms: extend the shared enum vs a separate serve-path verdict type.
- **F — Doorway governability (deferred-named).** Keystone §9 Q5: full design of recoverable/governable
  infrastructure under the governable≠seizable gate. Named, not this spec's mechanics.

## 5. Upstream gates / blockers (honest — design-only/eventual)

- **Limit-governor (the "capped by" property) — PARTIALLY BLOCKED.**
  - *Limitarian ceiling* — **built/shipped** (`validate_ratifies_limit_gradient`, `elohim-core::measure`
    concentration curve) but governs only the per-agent **token/attention** balance. Binding **toll-reward
    concentration** to it is **GATED on the `substrate_signal` field** (limitarian §9, Cluster-#3:
    "the governor cannot compute until `substrate_signal` exists"). So "capped by the limit-governor" is
    satisfiable today only for the attention/token substrate; for toll-reward flow it is blocked.
  - *Donut floor/ceiling governor* — **conceptual only** (the "donut ratios" exist as a split concept,
    keystone §4; floor reuses `dignity_floor`; the ecological ceiling is unwired). No whole-system donut
    governor exists.
  - *Personal "respect-your-own-limit" stub* — un-blessed (`stub-greenlight-to-expand`).
- **Fiat rail / custody** — no rail chosen (Fork A); no `bridges/fiat` crate; the internal value unit
  (Unyt-inspired mutual credit / hREA ledger above a future external settlement layer) is the target —
  **fiat does NOT settle peers directly**; it is recorded as a resource flow and reward runs as
  mutual-credit/`appreciation` (real exchange rates "will come from the Unyt protocol" — placeholder today).
- **Membrane self-scoping** — §7.1 states **"v1 ships nothing here"**; mutual-aid recognition (§7) is the
  only economic layer that ships.
- **Verdict:** the *flow* can be designed now and even built against existing primitives; the literal
  "capped by the limit-governor" property is **blocked on `substrate_signal`** (toll-reward concentration)
  and the **un-blessed donut governor** (floor/ceiling half). Design now; flag both as gates; do not assume
  them built.

## 6. Scope

**This spec covers now:** the wired `toll → 402 → bytes → peer reward → commons pool` chain over existing
primitives (§1); the P2P-gate pre-classification (§2, zero new DHT types, `agent_cid` attribution); the
`bridges/fiat` crate shape (§3); the six forks (§4); the `Verdict::Toll` orthogonal-axis contour.

**Deferred to the implementation `/plan` (when upstream lands):** the crate implementation + the chosen
fiat rail + custody wiring; binding toll-reward to the limitarian ceiling (**blocked on `substrate_signal`**);
the donut floor/ceiling governor (conceptual-only / stub un-blessed). Per the membrane, v1 ships nothing
here.

## 7. Non-goals

- Building anything in this wave — this is the design + gate the keystone required; implementation is
  deferred behind the §5 gates.
- A new DHT entry type (any of `TollReceipt`/`PaymentIntent`/`HostingOffer` is an anti-pattern — reuse
  `EconomicEvent` + `replicates-*` Commitment; the opt-in offer is a `proposed` Commitment, not an Intent).
- A doorway-resident toll authority or a doorway-banked toll-accrual system (the doorway facilitates and
  routes; the peer enforces; value flows to peers/commons).
- Reading the toll as availability — it is **purely compensatory** (§1 invariant); replication is the
  in-kind `replicates-*` path only.
- Folding inline into `epr-reachability-economics` — that keystone is the frame; this is the mechanism,
  and the keystone itself asks for a separate gate.
