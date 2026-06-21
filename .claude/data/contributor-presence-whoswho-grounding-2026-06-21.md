---
title: Who's-Who / Contributor-Presence Grounding — the inspirer-attribution backbone
written: 2026-06-21
author: fork:contributor-presence-grounding
status: grounding
---

# Who's-Who / Contributor-Presence Grounding

**Directive:** ground the "networking / who's-who of inspirers" skill — what attribution machinery
is actually wired, and which REAL contributors/libraries this codebase leans on — so the skill is
built on the substrate, not invented.

## 0. Canonical prior art — COMPOSE from these, do NOT re-derive

This is the protocol's own legacy design; the who's-who / contributor-presence model is already
documented. Build by composing these:

- **`ContributorPresence` zome design comment** — `imagodei/zomes/imagodei_integrity/src/lib.rs:46-57`
  (mirror in `content_store_integrity`): *"recognition placeholder for absent contributors …
  recognition accumulates even while unclaimed … contributors can claim their presence and receive
  accumulated recognition. Lifecycle: UNCLAIMED → STEWARDED → CLAIMED … recognition before
  registration."* This IS the design.
- **`elohim/holochain/dna/imagodei/STEWARDSHIP_PHILOSOPHY.md`** — graduated stewardship + dignity.
  The **Layer-1 sacred posture (§4) is already canon here**: *"Inalienable rights (always
  preserved)," "Never Binary — no one is exiled," "Dignity first,"* rights that *"cannot be disabled
  even by constitutional-level coordinators."* The network's posture toward the person is
  unconditional by design.
- **`genesis/plans/2026-03-13-recognition-pipeline-plan.md`** — accrual → claim mechanics. **Check
  here FIRST** for whether opt-out/decline is already specced.
- **`genesis/docs/content/elohim-protocol/imagodei.md`** — the imagodei pillar doc.
- **`genesis/data/timeline/backlog/witnessed-records-reach-flywheel.md`** — witnessed records + the
  reach flywheel (the Layer-1 "always witness + credit" engine).
- **`genesis/plans/2026-03-29-sprint-B-imagodei-domain-manifest.md`** + `elohim/sdk/domains/imagodei/CLAUDE.md` — the SDK/domain surface.
- **Reach enum** `private → … → commons` (level 7) — `elohim/sdk/schemas/v1/enums/reach.schema.json`,
  `elohim/epr/src/reach.rs`. **"Flows to the commons" = reach `commons`** — likely not net-new, just
  a distribution at that reach level.

**Implication:** treat the "net-new" in §2/§4/§6 (opt-out ceremonial signal + value-receipt →
commons) as **verify-first, not build-fresh** — reconcile against the recognition-pipeline-plan +
stewardship philosophy; much of it may already exist.

## 1. Verdict

**The machinery is purpose-built and LIVE-WIRED for exactly this use case. The gap is DATA
(no real inspirer presences seeded) + a small net-new flow (an opt-out ceremonial signal + a
value-receipt → commons redirect) + a thin consult/credit skill.** Crucially, the *unconditional
recognition* layer (the sacred posture, §4) is already wired and needs no change.

`ContributorPresence`'s own doc comment (imagodei integrity zome): *"recognition before
registration — a key feature for citing authors, speakers, and contributors who aren't yet on the
network."* That IS the who's-who-of-inspirers. The citation/provenance links, per-content
recognition accrual (in absence), the claim lifecycle, and value-transfer-on-claim are already
fields on the type.

## 2. Machinery (what's wired — file:line)

| Capability | Where | Note |
|---|---|---|
| `ContributorPresence` DHT entry type | `imagodei/zomes/imagodei_integrity/src/lib.rs:499` **and** `elohim/zomes/content_store_integrity/src/lib.rs:1215` | created in coordinators (`imagodei` zome :1758, `content_store` :6564) — **no new entry type needed** |
| Storage projection + query | `elohim-storage/src/db/contributor_presences.rs` (`ContributorPresenceQuery`), `db/models.rs:276`, `db/diesel_schema.rs` | the "who's who" is a query/view over this |
| ts-rs view | `elohim-views/src/imagodei.rs:82` `ContributorPresenceView` | camelCase wire shape exists |
| HTTP/API + services | `api/presence.rs`, `services/presence_service.rs`, `services/recognition_pipeline_service.rs`, `services/economic_event_service.rs`, `api/lamad.rs`, `api/mishpat_recognition.rs` | consult + recognition pipeline already surfaced |
| **Provenance / inspiration link** | `ContributorPresence.establishing_content_ids_json` + relationship-type enum `derived_from` / `source_of` (`sdk/schemas/v1/enums/relationship-type.schema.json`) | inspiration link is **NOT net-new** — `derived_from`/`source_of` covers EPR→source; `establishing_content_ids` is the reverse index |
| **Unconditional recognition/witnessing (Layer 1, §4)** | `recognition_by_content_json`, `citation_count`, `affinity_total`, `recognition_score`, `unique_engagers`, `endorsements_json` | accrues **whether or not the contributor is present or has consented** — the sacred posture is the wired default; **needs no change** |
| **Claim lifecycle** | `presence_state` ∈ {unclaimed, stewarded, claimed}; `claim_verification_method`, `claimed_agent_id`, `steward_id` | a real person can claim a presence accrued in their absence and *receive* its value |
| **Value transfer (claim path)** | `claim_recognition_transferred_value` / `_unit` + `reach_earning.rs` + `economic_event_service.rs` (shefa REA / hREA) | accrued recognition converts to REA value on claim |
| **Opt-out ceremonial signal + value-receipt → commons (NET-NEW)** | — no `opted-out` flag on `presence_state`; no FeedbackSignal kind for it; no flow routing declined-receipt value to a commons pool | the only net-new piece; **must not touch Layer 1** (see §4) |
| Reach back-prop (the credit traversal) | spec: graph-substrate-design §"social reach nervous system" — "provenance + back-prop are traversals"; `reach_earning.rs` is the authoring-time earner | the reverse reach-flow to inspirers rides the same provenance edges |

## 3. The inspirer-attribution flow (composes existing primitives)

1. An author cites an inspirer → a `ContributorPresence` (`unclaimed`) is established, with the
   citing content in `establishing_content_ids` and a `derived_from`/`source_of` link.
2. Recognition accrues per citation and per **learning-path completion / common-value distribution**
   (`recognition_pipeline_service`, `recognition_by_content`) — **unconditionally, whether or not
   the contributor is present** — reach/value back-props along the provenance edges. (Layer 1, §4.)
3. *Receipt* of the accrued material value resolves one of two ways (Layer 2, §4): **claim** (real
   person receives) or **opt-out** (declines receipt → value flows to commons). **Either way the
   witnessing/credit of step 2 continues unchanged.**

## 4. The sacred posture — TWO layers that must not be conflated

The differentiator and the design invariant. There are two distinct layers; only the second is
optional.

**Layer 1 — the sacred posture toward the contributor's imagodei (UNCONDITIONAL; never changes).**
The network *always* witnesses and credits the contributor's presence. Recognition accrues, the
person is seen, their contribution is honored — regardless of presence, consent, or opt-out. The
inherent dignity/standing of the person *in relation to the rest of the network* is sacred and
invariant. **Nothing — including opt-out — diminishes it.** This is the wired default (recognition
accrues unconditionally). It is what separates elohim from **Pirate Bay** (take, don't account) and
makes it **Bookshop.org / fair-trade / Kiva / Bandcamp** (always account, always credit). You
cannot consent the planet — so you *honor* the planet, unconditionally.

**Layer 2 — value RECEIPT (the only optional thing).** Where the accrued *material* value goes:
- **Claim** — a verified real person claims → accrued value transfers to them
  (`claim_recognition_transferred_value`). Bandcamp's "here is what your work earned."
- **Opt-out** — receipt is declined; accrued value **flows to the commons** (redirected, never
  deleted). **The Layer-1 witnessing/credit continues unchanged.**

**Opt-out is a ceremonial flag, not an erasure.** Functionally it (a) emits a **signal** to the
network — expressive dissent ("I don't endorse this use"), naturally a **FeedbackSignal kind**,
which **may inform policy** (qahal/mishpat) — and (b) redirects value-*receipt* to the commons. It
does **not** touch the sacred posture toward the contributor's imagodei. You are always seen and
credited; you may decline to *receive*, and you may *signal* — neither un-sees you.

**Net-new (small; compose, don't fork; must not touch Layer 1):** an `opted-out` ceremonial flag
expressed as a **FeedbackSignal kind** (→ optional qahal/mishpat policy hook) + the value-receipt →
commons REA redirect (an `EconomicEvent` distributing declined value to a commons agent/pool).
Layer-1 recognition already accrues unconditionally and stays untouched.

## 5. The REAL inspirer seed list (from repo evidence)

All get a `ContributorPresence` and are **witnessed + credited unconditionally** (Layer 1). Cohorts
differ only in provenance cleanliness and how a claim would be verified — never in whether to honor.

**Libraries / projects (cohort 1 — clean provenance, seed first):**
- **Khan Academy / Perseus** — Sophia is forked from Perseus (`sophia/package.json` `@khanacademy/*`).
- **Holochain** — `hdk`/`hdi`/`holo_hash`/`@holochain/client` (DHT substrate). Holochain Foundation.
- **libp2p** — Protocol Labs (Track-2 transport).
- **iroh / iroh-blobs / iroh-gossip** — n0 / number0 (blob + gossip transport).
- **Automerge** — Ink & Switch / Martin Kleppmann (the CRDT engine; `automerge-sync` skill).
- **ValueFlows + hREA** — Lynn Foster & Bob Haugen (REA vocabulary; `bridges/valueflows`, shefa).
- **REA accounting model** — William McCarthy (Resource-Event-Agent origin).
- **Unyt** — mutual-credit lineage (`rea-economics` skill).
- **gitoxide / gix** — Sebastian Thiel (`elohim/rakia/elohim/brit`).
- **Eclipse Che / devworkspaces** — the dev substrate (`che-devworkspaces` submodule).
- (Lower-priority tooling cohort: Angular, Lit, Tauri, Diesel, Vitest.)

**Named individuals (cohort 2 — honored the same; the claim path lets them *receive*):**
- **Linus Torvalds** — the Linux mental model `concept-mapping` leans on.
- The authors behind cohort 1 (Brock & Harris-Braun; Kleppmann; Foster & Haugen; McCarthy; Thiel).

The **Canteen / creator-payments outreach** (Stephen Lewis / the Beer arc / creator-services-
bridge) is the channel to **invite claims** — so creators can *receive* what they've already
accrued — not a precondition for honoring them.

## 6. The thin skill (when built)

`who's-who` (networking) = **consult** (query `ContributorPresence` + traverse
`establishing_content_ids` / `derived_from` from an EPR or learning-path to its source presences)
**+ credit** (recognition accrues unconditionally per citation and on path-completion via
`recognition_pipeline_service`; reach/value back-props; *receipt* resolves via claim or
opt-out→commons). It composes `ContributorPresence` (wired) + the provenance relationship (wired) +
REA distribution (wired) + the net-new opt-out signal/redirect (§4). `concept-mapping` feeds it
(each analogy names an inspirer); `app-port` should credit the prior-art it leans on.

**Prerequisites, in order:** (1) the **opt-out ceremonial FeedbackSignal + value-receipt → commons
redirect** (the net-new — must leave Layer-1 recognition untouched); (2) the **cohort-1 seeding
pass** (libraries, as presences linked to the content/specs that lean on them — bounded, safe);
(3) cohort-2 individuals, witnessed the same way, with the Canteen outreach inviting claims.
