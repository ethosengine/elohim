---
title: Hypha DAO — Autonomous Entity & Collective Designs Cross-Pollination
id: hypha-dao-autonomous-collectives-cross-pollination-2026-06-24
status: Capture
date: 2026-06-24
---

# Hypha DAO Cross-Pollination — June 2026

**[Hypha DAO](https://github.com/hypha-dao)** is a decade-deep project building tooling for **DAOs / DHOs** — "Decentralised Human Organisations." It is the closest external mirror we have found for the protocol's *autonomous-entity and collectives* work (the **recursive-Qahal** substrate: [`Collective` / `Membership` / `CollabAgreement`](../docs/content/elohim-protocol/architecture/2026-05-23-multi-collective-collaboration-epr-design.md)). Where the [hyphacoop / Distributed Press survey](hypha-distributed-press-cross-pollination-2026-06-23.md) mirrored the **doorway** (federation/projection), and the [DDS-WG survey](dds-wg-cross-pollination-2026-05-01.md) mirrored **deliberation**, Hypha DAO mirrors the **collective itself** — how a group becomes a legible, governed, value-distributing entity.

This survey is a green-team / red-team: what to borrow, what to defer, what to reject. It was produced against primary sources (the repos, the [Frontiers in Blockchain 2025 peer-reviewed paper](https://www.frontiersin.org/journals/blockchain/articles/10.3389/fbloc.2025.1630402/full), and Hypha's own guides) and adjudicated against the protocol's design gates ([p2p-design-gate](../../.claude/skills/p2p-design-gate/SKILL.md), the [DNA rent-extraction test](../../elohim/holochain/dna/CLAUDE.md), [stewardship-over-sovereignty](../docs/architecture/stewardship-over-sovereignty.md), [justice-as-Mishpat](../docs/architecture/justice-manifesto.md)).

The one-line verdict: **Hypha is the closest philosophical fellow-traveler the protocol has, and its substrate is the cleanest thing to reject.** It independently reached human-at-the-heart, collective-stewardship-as-apex, capital-decoupled-from-voice, non-transferable decaying standing, and fractal/membranic nesting that maps almost 1:1 onto recursive-Qahal — but it builds all of it on a global-consensus token blockchain, which is exactly the trust root the protocol is built to avoid. Borrow the mechanics and the framing; reject the chain and every transferable token.

---

## ⚠️ Name-collision guard (read first)

Three distinct things share the "hypha" name. Conflating them has already cost confusion once and will again:

| Org | What it is | Where it's surveyed |
|---|---|---|
| **`hypha-dao`** (this doc) | DAO/DHO governance tooling on Telos/EOSIO → EVM. `document-graph`, `dao-contracts`, `voice-token`, `dho-web-client`, `hypha-web`. | **this survey** |
| **`hyphacoop`** | Hypha Worker Co-operative (Toronto) — **Distributed Press** + the **Social Inbox** (ActivityPub/fediverse projection). | [hypha-distributed-press-cross-pollination-2026-06-23.md](hypha-distributed-press-cross-pollination-2026-06-23.md) |
| **`Pointsnode/hypha-network`** | Unrelated third party — a "Neural Bus for AGI" on Base L2 (`HyphaEscrow.sol`). **Not Hypha at all.** | not surveyed; do not attribute |

When `research-manifest.json` pins Hypha-DAO repos, it pins `hypha-dao/*` **only** — never `hyphacoop`, never `Pointsnode`.

---

## Legacy vs. live (this changes what "borrow" means)

Hypha has a **clean generational split**, and it matters: the most architecturally interesting things to borrow are from the *prior* generation.

- **Legacy (C++ / EOSIO, the document-graph stack):** `document-graph` (last touched May 2024), `dao-contracts` (Oct 2024), `voice-token` (Mar 2024). This is where the content-addressed graph, the typed governance entities, and the decaying voice token live. **Quiet for 12-26 months.** Funded originally by an **$850k EOS Network Foundation grant**.
- **Live (TypeScript / Solidity, "V3"):** `hypha-web` (TS ~91% / Solidity ~5%, actively developed **Jun 2026**, 8k+ commits) is a *complete rewrite*. V3 reframes around four pillars — **membership, governance, treasury, networking (nested spaces)** — sold as "Organization-in-a-Box" at ~$11/month/space, with the deliberate posture *"we've put blockchain where it belongs — in the background. No one needs to know it's there,"* and a homepage rebrand toward **"AI-native coordination"** that has drifted away from the "DHO" vocabulary the paper and guides still use.

**Implication:** every borrow below is **pattern-level** — an idea proven (or paid for) in Hypha's prior generation — not "adopt the live product." Hypha is blockchain-native throughout; targeted searches found **no** Holochain / agent-centric / local-first exploration anywhere in the org.

---

## Subjects surveyed

### 1. `document-graph` — content-addressed property graph + CQRS projection

A reusable C++ EOSIO **smart-contract library** modeling an on-chain content-addressed property graph from two primitives ([repo](https://github.com/hypha-dao/document-graph), [document.hpp](https://raw.githubusercontent.com/hypha-dao/document-graph/master/include/document_graph/document.hpp), [edge.hpp](https://raw.githubusercontent.com/hypha-dao/document-graph/master/include/document_graph/edge.hpp)):

- **Document** (vertex): `checksum256 hash` (content fingerprint), `creator`, `ContentGroups`, `certificates`, `created_date`. The `hash` is computed over content only and **uniqueness is enforced at the contract level** — identical content collapses to one node (free dedup + tamper-evidence), explicitly compared to **IPFS/IPLD**.
- **Edge** (labeled directional relationship): `id = hash(from_node + to_node + edge_name)` — idempotent typed relationships. "member document → role document" is one edge.
- **`ContentGroups`**: a 3-level schemaless nest with a typed leaf (`FlexValue = variant<asset, string, time_point, name, int64>`) — self-describing, evolvable payload per node, "schema flexibility without migrations."
- **`certificates`**: an attestation slot **excluded from the content hash** (so validation can be appended without changing identity) — but the field is noted "currently unused" *[UNVERIFIED in production]*.

Because EOSIO has no SQL/joins and **RAM is the paid bottleneck**, traversal is served by **eight composite secondary indexes** on edges (`byfromname`, `byfromto`, `bytoname`…), and everything else is pushed **off-chain**: `document-graph-elasticsearch` is a Go stream processor — **chain → dfuse firehose (table deltas) → processor → Elasticsearch** — resumable via a persisted **cursor**, namespaced per contract (`index-prefix`), selectively filtered (`edge-black-list`) ([README](https://raw.githubusercontent.com/hypha-dao/document-graph-elasticsearch/master/README.md)). A **parallel Dgraph projection** (`document-cache`) runs off the *same* source-of-truth — ES for full-text, Dgraph for traversal. This is **CQRS**: a query-hostile truth store with purpose-built read models eagerly reconciled on top.

### 2. `dao-contracts` — governance primitives & deliberation

Roles, Assignments, Badges, Payouts, Proposals, Circles all "belong to a DHO" only via **edges from the root node** — the relationship-link carries membership, no entity owns it ([dao-go pkg](https://pkg.go.dev/github.com/hypha-dao/dao-contracts/dao-go), [explore.joinseeds.earth](https://explore.joinseeds.earth/4.-organisation-tools-daos-and-dhos/doing-a-dho-and-dao)):

- **Roles** — reusable templates ("salary band, max holders, min utility-token %"), on the principle **"separating role from soul"** (role ≠ identity).
- **Assignments** — a member commits to a role for a period and is **compensated**; applied for via proposal.
- **Badges / Multipliers**, **Quests** (single/multi stage·person·payout), **Contributions / Payouts** (USD-denominated recognition).
- **Periods / Claims** — time is chunked into `Period{id, start, end, phase}`; compensation accrues per period and is **pulled** via `ClaimPay()`.
- **Circles** / V3 **"governance spaces"** — semi-autonomous subDAOs with their own *membership, agreements, treasury*; **"fractal and membranic"** — nested layers with semi-permeable boundaries ([Frontiers 2025](https://www.frontiersin.org/journals/blockchain/articles/10.3389/fbloc.2025.1630402/full)).

The lifecycle is **sense-making → proposal → (dissent window) → vote → enact**: *"Posting a proposal without having done the sense-making beforehand? Ninety percent of the time, it fails."* Voting is **consent-and-quorum-gated, not plutocratic** — the paper's term is **"role-based governance that decouples voting power from capital ownership."** The default on the example DHO is an **80/20** method (≥20% quorum, ≥80% unity), *configurable per community* ("multi-voting plugins"). **"Trust-based safeguards"** are social-procedural — sense-making stages + dissent windows + role-based agency — not purely cryptographic.

### 3. `voice-token` & tokenomics

The sharpest object. `voice-token` is verbatim *"Contract for **non-transferable, mintable** token used for voting"* ([repo](https://github.com/hypha-dao/voice-token)). In the product it is **HVOICE**:

- **Earn-only, never bought** — explicitly "to move away from oligarchy/plutocracy models where voice can be purchased."
- **Decays** — `DecayPerPeriod`/`DecayPeriod` fields exist in the `hvoice` Stats struct *(decay-exists: **HIGH** confidence, struct fields confirmed)*; the **specific rates** — ~1-year half-life, ~1.4% per lunar phase (~1 week), inactive after 6 months — are **MEDIUM / policy-sourced** and possibly *proposed-in-policy rather than wired in the contract*. Effect: influence tracks *recent* participation, not hoarding.
- **Work-minted** — `HVOICE = USD × 2` per Assignment, tiered by salary band.

The full set is a deliberate **three-token model**: **HYPHA** (transferable utility token, and a **voting-power multiplier** — "total Hypha holdings… weigh on voting power" *[MEDIUM, single-source]*), **HUSD** (cash token, redeemable for ETH/BTC, issued+burned by the treasury), **HVOICE** (non-transferable voice). Plus **SEEDS** (external, market-traded ReFi currency), **Village Tokens** (fungible), and **NFTs**. Compensation is multi-token with a **deferral lever**: liquid pay (SEEDS+HUSD) or **defer for +30%** (SEEDS+HYPHA). The **treasury** is a badge-gated **multisig** smart contract (threshold treasurers; `redeem`→`redeemed` burns HUSD; batched payouts).

### 4. DHO philosophy & "DAO 3.0"

Hypha's arc is **DAO → DHO → DAO 3.0 ("Adaptable Organization")** ([Frontiers 2025](https://www.frontiersin.org/journals/blockchain/articles/10.3389/fbloc.2025.1630402/full); [SEEDS tokenomics](https://explore.joinseeds.earth/7.-take-action-with-the-commons-organisations/the-hypha-dao-dho/deep-dive/tokenomics)). The DHO move "foregrounds sense-making, community care, and inner development as core governance principles" — *"You need to change yourself first. If you expect the tech to make the system regenerative without that inner shift, it's not going to work."* They reject token-plutocracy **by name** (merging value and governance tokens "ensures that governance is a plutocracy"). The apex is explicitly **collective**: *"leadership without control… shifts emphasis from centralized token-holding to **distributed agency and collective stewardship**"*; *"No shareholders. No board. Just us. Owned by the network."* Consensus is replaced by **aligned dissent**: *"I may not agree, but I understand and support the direction."* The economic thesis is **regenerative coordination** (ReFi), tied to **SEEDS** and the **ReGen Civics Alliance**; adopters include the **Global Ecovillage Network**, **ReFi DAO**, and others.

> **One honest caveat for the framing borrow:** the paper *also* invokes individual-flavored language — "honour human sovereignty," "owned by you," "Maximum freedom." The operative resolution is collective (agency *within* shared governance), but the vocabulary leaks. See the red-team note on the identity seam.

---

## What we'd lift (green team)

Each borrow names its **Elohim home**. Per the [weave-epic "compose-don't-fork" finding](../docs/superpowers/specs/2026-06-20-weave-epic-arc-design.md), **none of these mints a new DHT entry type** — they are projection-mechanics, recognition-fold kernels, vocabulary, or framing.

1. **Content-addressed graph identity + chain-as-projection — the strongest, best-grounded convergence.** Hypha independently arrived at *hash-of-content = node identity* (IPFS/IPLD-compared, uniqueness-enforced, dedup-by-design) and *truth-store + off-chain read model*. These map almost 1:1 onto the protocol's two deepest substrate rules — **CID-as-identity** (p2p-design-gate) and **storage-as-projection, not truth** ([P1 reconciliation controller](../../.claude/memory/project_principle_p1_reconciliation_controller.md)). The borrow is *validation, plus discipline*: treat content-hash dedup as an **explicit entry-cap budget invariant**, not an emergent nicety — an adversary paid for ignoring it in RAM. **Home:** reinforce in [`elohim/holochain/dna/CLAUDE.md`](../../elohim/holochain/dna/CLAUDE.md) (entry-cap / rent-extraction section).

2. **Cursor-resumable, namespaced, selectively-filtered projection mechanics** *(one borrow, surfaced in three themes — consolidate, don't triple-count)*. Hypha's firehose→ES/Dgraph projector hardens exactly the protocol's **P1 "eager reconcile" controller** with operational details worth lifting: a **persisted projection checkpoint** so an `elohim-storage` restart resumes from a known DHT position instead of full re-derive; **per-source namespacing**; **selective projection**. And the **dual-target** pattern (ES *and* Dgraph off one truth) makes explicit doctrine that *multiple purpose-built read-models over one notarized truth is legitimate* — the [facing-lens family](../docs/superpowers/specs/2026-06-20-weave-epic-arc-design.md) (REA-economic lens, operational `WeaveView`) already does this. **Home:** the `elohim-storage` projection path; the EprRouter [heal-on-read](../../.claude/memory/project_epr_router_empties_on_poisoned_scope.md) story is the resilience analog. **Operational-C.**

3. **Typed-variant-in-JSON-leaf as a named "evolve-before-you-mint" discipline.** Hypha's `ContentGroups` (schemaless typed leaf) lets a node payload evolve without a migration. The protocol *already* practices this — `Collective.charter`, `share_allocation_json`, `governance_terms_json`, the VSM-council `{"kind":"council"}` charter value — and it keeps new structure *out* of the ~100-entry-type / 256-link-type budget. The borrow is to **name it as a deliberate pattern** so agents reach for it before proposing a new entry type. **Home:** a category-A2 note in the [p2p-design-gate skill](../../.claude/skills/p2p-design-gate/SKILL.md) ("evolve a JSON leaf on an existing entry before minting a type").

4. **Mandatory sense-making + dissent-window before the vote (the Habermas-shaped lifecycle).** A proposal must pass a deliberation/dissent stage before becoming votable. This fills a real gap: the [constitution's](../docs/content/elohim-protocol/constitution.md) Conflict-Resolution Algorithm flags conflicts "for human deliberation, never auto-resolved" but doesn't specify the *human protocol*. **Home:** a `proposal_state` lifecycle (`sensemaking → open → settled`) on the existing qahal governance flow + a qahal a2o scenario. **BUT** — run the [rent-extraction test](../../elohim/holochain/dna/CLAUDE.md) on the record itself: deliberation *chatter* is Operational-C (reconstructable), but the **settled-proposal outcome is plausibly capture-relevant** (whoever controls "what reached a vote" controls the vote) and likely needs a **B2 signed Attestation**. And name the witness: Mishpat is **El Roi / sortition + public record**, *not* a core team's informal pre-vote — a dissent window that silently decides what is votable is exactly the unaccountable filter [justice-as-Mishpat](../docs/architecture/justice-manifesto.md) exists to resist.

5. **The governance *vocabulary*: Role / Assignment / Badge / Quest / Period / Claim — and "separating role from soul."** Role-≠-soul is the [identity-sovereignty guard](../../.claude/memory/feedback-identity-sovereignty-ontology-guard.md) expressed economically (standing is held *on behalf of*, never owned); `ClaimPay()`-as-pull mirrors stewardship-not-extraction. **Home:** Role/Assignment map onto `Membership.role`; Badges/Quests fold into the existing `ContributionRecord` / Attestation-EPR via **subtype strings** (the weave-arc `attestation:storage-capability` pattern). ⚠️ **Correction (do not misroute):** an Assignment is *labor committed to a role for recognition* — that is `Membership.role` + an REA recognition/`appreciation` event, **not** the [`delegates-compute` commitment](../docs/architecture/rea-compute-commitment-primitive.md). Reserve the compute-commitment primitive for actual *compute-authority* delegation (republish-epr, serve-url, provide-cycles); do not conscript it for labor.

6. **VOICE decay — the use-it-or-lose-it intuition, re-homed as a recognition-decay kernel on reach/standing.** Influence that decays toward recent contribution structurally prevents accumulation-capture; an inactive captor's grip *decays on its own* — no punishment, just non-renewal of a gift (justice-as-restored-capability, not retribution). This is already a planned post-v0 direction — **(B) temporal decay/accrual** in [`future-distribution-models.md`](../../elohim/elohim-storage/research/future-distribution-models.md). Mechanically it is **Operational-C**: reach/standing is *recomputed on read* from notarized contribution events with a half-life kernel — **do NOT mint a `VoiceBalance`/`StandingToken` entry** (that makes standing a bank-like ledger). Borrow the *intuition*, not Hypha's specific rate (which may be policy-proposed, not wired). **Two conditions:** (a) reconcile against tenure-rewarding — decay rewards **recency**, so do not also bolt on a "reward durable commitment" premium that pulls the opposite way; pick recency. (b) **The half-life is a governance knob** — whoever sets it controls whose standing evaporates and how fast. Route the parameter to the **constitutional/charter layer** under [subsidiarity](../docs/content/elohim-protocol/constitution.md), and apply the rent-extraction test to the *parameter-setting authority*, not just the entry type.

7. **Hypha as a cite-able *governance / anti-plutocracy* fellow-traveler.** A peer-reviewed project independently reached "collective stewardship over token-holding" and "decouples voting power from capital" — external corroboration that [stewardship-over-sovereignty](../docs/architecture/stewardship-over-sovereignty.md) and recognition-not-currency are *convergent* design, not protocol idiosyncrasy. ⚠️ **Seam precision:** cite it for **stewardship-over-ownership and anti-plutocracy**, **NOT** as corroboration of the *identity-sovereignty guard* — those sit on different seams (governance-power vs identity-ontology), and Hypha's own "owned by you / human sovereignty" marketing is precisely the leak the identity guard catches. Name the divergence when citing.

---

## Where our paths diverge (red team)

1. **Global-consensus blockchain as the trust root — REJECT (never).** On-chain global graph on Telos/EOS/EVM. A global ledger makes consensus *and RAM rent* a participation gate and imports the silicon-crypto ontology the protocol subordinates to community governance. This is the whole point of agent-centric: **no global consensus, no metered global state.** Hypha is a *validation by negative example* — it hit precisely the no-join and RAM-rent walls that "DHT-as-notary + entry-cap + storage-as-projection" is built to avoid, and its CQRS escape hatch *is* the P1 controller arrived at under duress.

2. **The transferable token layer (HYPHA utility + HUSD cash + the HYPHA voting-multiplier) — REJECT.** HYPHA being **transferable and a voting multiplier** re-introduces the exact capital→voice coupling Hypha *says* it rejects — a plutocratic backdoor the [identity-sovereignty guard](../../.claude/memory/feedback-identity-sovereignty-ontology-guard.md) names. HUSD "redeemable for ETH/BTC" is the crypto-as-money ontology. The protocol's [`commons_pool_tribute > 0` invariant](../../elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/qahal.rs) ("no pure-private extraction") already does the anti-extraction work *without* a fungible token.

3. **On-chain treasury as a fungible-asset bank (multisig issue/retire HUSD) — REJECT the bank; the audit properties already exist natively.** A treasury that holds and moves fungible value is precisely the "[DHT is notary, not bank](../../elohim/holochain/dna/CLAUDE.md)" failure. The **[REA compute-commitment primitive](../docs/architecture/rea-compute-commitment-primitive.md)** (`Mishpat::Commitment` + `delegates-compute`, bounded reciprocity, revocable, on-chain-auditable) is the structurally-correct answer to "distributed authority, no single signer": it gives Hypha's threshold-multisig *audit properties* — checkable standing, real revocation, notarized authority chain — **without a fungible treasury**, and it already exists (no new entry type).

4. **Salary bands + USD-denominated payout / deferral premium — DEFER.** Pricing contribution in USD and paying a liquidity-vs-commitment premium reframes contribution as *labor-for-wages*, not *recognized stewardship* (recognition-not-currency). It is, however, a genuine answer to "how do you sustain compensated work" that the recognition economy under-specifies. **Defer** to the post-v0 [distribution-models](../../elohim/elohim-storage/research/future-distribution-models.md) work — input to direction (A) multi-dimensional weighting, stripped of currency denomination.

5. **SEEDS / external market-traded ReFi currency — REJECT as substrate.** A CoinMarketCap-listed fungible token reintroduces price, speculation, and an external dependency the [hub-optional, laptop-only floor](../../.claude/memory/project_hub_optional_floor.md) cannot assume. [`economic-systems-research.md`](../../elohim/elohim-storage/research/economic-systems-research.md) already surveyed this class under "build native, steal patterns." Comparator/adversary frame only.

6. **The generic typed-document-graph as the *primary* data model — REJECT; keep the typed-leaf subset.** Hypha collapses *everything* (roles, badges, payouts, members) into untyped Documents distinguished only by ContentGroup convention — erasing the A/A2/B/B2/C source-of-truth categorization the p2p-design-gate exists to enforce. The protocol's strongly-typed integrity entries (`Collective`, `Membership`, `CollabAgreement`) carry **scalar invariants at the substrate** (charter ≤16 KiB, `0.0 < tribute ≤ 1.0`, 32-hex salt). ⚠️ **Honest scope:** the *most complex* invariant — `share_allocation_json` summing to 1.0 — runs **coordinator-side, not in the integrity validator** ([`qahal.rs` explicitly delegates it](../../elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/qahal.rs)). So the durable ground for the REJECT is **the A/A2/B/B2/C categorization + the CID-vs-`action_hash` identity split**, which holds regardless of enforcement tier — *not* an overclaim that "every governance invariant is notarized."

7. **Pure content-hash as the *sole* identity for mutable governance entities — REJECT as a universal rule.** Hypha's pure addressing means any edit is a new node — fine for immutable claims, broken for lifecycle mutation (`Membership.withdrawn_at_block_height`, counter-attestation accrual, Commitment revoke/graduate). The protocol already correctly separates **content-CID-as-identity** from **`action_hash`-as-anchor** (the [`dht_anchor_hash` trap](../../.claude/memory/project_mishpat_commitment_cid_is_entry_hash.md)); pure Hypha-style addressing would silently break every bounds-gate. The protocol's CID-vs-anchor split is the correct, more nuanced design.

8. **On-chain composite-index fan-out + inline `certificates` attestation — REJECT on the DHT.** The eight secondary indexes are an EOSIO no-join workaround that would burn the scarce **256 link-type budget** for exactly the **banned `*By*` query-index** purpose; traversal indexes belong in the SQLite projection (where Hypha's own Dgraph path agrees). And inlining attestations *inside* the node conflates asserted-by-self payload with third-party witness — the protocol's gate mandates the opposite (**B2: a separate signed Attestation** notarizes the outcome). ⚠️ **Precision:** Hypha's certificates-excluded-from-hash validates *hash-stability-under-appended-validation*, **not** the B2 private/attested split — it is *not* "Hypha already does B2."

---

## The lossy-bridge invariant (the one hard rule)

If a Hypha DHO ever wants to participate in the protocol, the seam is a **bridge crate** (`bridges/`, the [`valueflows`/`atproto` pattern](../../elohim/holochain/dna/CLAUDE.md)) reading its firehose deltas into a **doorway T4 projection** — never an adoption of Hypha's substrate. Hypha's hREA-shaped Agreement/Agent map onto the protocol's existing [hREA projection](../docs/content/elohim-protocol/architecture/2026-05-23-multi-collective-collaboration-epr-design.md) (§7), but **the mapping is lossy by design, and that is the safety property**:

> **A Hypha → Elohim bridge MUST strip the transferable-token and voting-power-multiplier semantics at the seam.** It projects Hypha's *agreements and agents*, never its *token-weighted governance*. The protocol's `CollabAgreement` tribute-floor (`tribute > 0`, no pure-private extraction) and recognition-not-currency stance reject exactly the HYPHA-as-vote-weight / HUSD-cash shapes — so "maps cleanly" is wrong; it maps *lossily*, and the dropped fields (HYPHA holdings, vote multipliers, cash settlement) are the dangerous ones. This single invariant is the consolidated form of the reject that the substrate, governance, tokenomics, and philosophy lenses each made separately.

---

## The sharpest convergence, and the sharpest divergence

**Convergence (why this survey was worth doing):** Hypha is a non-Holochain, peer-reviewed, decade-deep project that *independently converged* on the protocol's deepest instincts — content-addressing as identity, storage-as-projection, non-transferable earned standing that decays, holonic/fractal nested organizations, collective-stewardship-as-apex, and explicit anti-plutocracy. When an outsider reaches the same conclusions under different constraints, the conclusions are probably structural, not idiosyncratic.

**Divergence (the rhyme that names the whole bet):** both designs say *"the chain layer takes weight as coordination scale grows."* But Hypha's chain layer is **token/stake-weighted consensus on Telos/EVM**; the protocol's [multi-collective spec §4](../docs/content/elohim-protocol/architecture/2026-05-23-multi-collective-collaboration-epr-design.md) is an **elohim-council chain weighted by aggregated proof-of-care** — *"validators … not weighted by staked capital, mined hashes, or treasury yield — they are weighted by aggregated proof-of-care,"* hosted on household nodes, "sociocracy faithfully implemented as substrate." **Same holonic graduation shape; opposite trust root.** Hypha *describes* anti-plutocracy while its token/treasury substrate still leaks capital-as-voice; the protocol forecloses it structurally because care cannot be bought at intimate scale. That contrast — care-weighted council vs. capital-weighted chain — is the single most useful sentence this survey produces for outreach and for design.

---

## Outputs

- **This survey** — `genesis/research/hypha-dao-autonomous-collectives-cross-pollination-2026-06-24.md`.
- **`research-manifest.json` clonable repos** — `document-graph`, `document-graph-elasticsearch`, `voice-token`, `dao-contracts` (pillar tags + the name-collision warning inline). Clone via `research.sh` for future deep grounding.
- **README enrichment** — a new **The Coordination-Scaling Problem (Collectives & Autonomous Entities)** standing-problem section, pointing at the recursive-Qahal substrate and this survey.
- **Module-boundary pointer note** — [`elohim/elohim-token/research/hypha-dao-governance-token-prior-art.md`](../../elohim/elohim-token/research/hypha-dao-governance-token-prior-art.md) (the VOICE-decay / non-transferable-earned-standing lesson).
- **Memory** — `.claude/memory/project_hypha_dao_cross_pollination.md` + MEMORY.md pointer (the durable, re-loadable residue: per-theme verdict, the two hard guards, the name-collision triple).

**Still open (not done here):**
- A **spec stub** for recognition-decay as `future-distribution-models.md` direction (B) — the half-life-as-governed-parameter design, gated through the p2p-design-gate (decay = Operational-C fold, **no** new entry type) — not authored.
- A **proposal-lifecycle / dissent-window** design for the qahal governance flow (with the El-Roi witness + B2-settled-outcome questions answered) — not authored.
- **Outreach** — Hypha is a genuine fellow-traveler (sibling to the [Polity/Ethelo](../../.claude/memory/project_polity_ethelo_outreach_thread.md) and [Canteen](../../.claude/memory/project_canteen_outreach_thread.md) threads); a thank-you / cross-pollination contact is reasonable but **not initiated** — operator's call.

## Method note

Produced via a parallel research workflow (8 grounding agents — 5 Hypha facets web-cited, 3 Elohim facets repo-grounded — then a 5-theme green/red-team with adversarial fact-verification, then a completeness critic). Every Hypha claim carries a confidence grade; `[UNVERIFIED]` / MEDIUM flags are preserved rather than laundered into fact. The adversarial pass caught and corrected: the enforcement-tier overclaim (§6 coordinator-vs-integrity), the Assignment→compute-delegation misroute (green #5), the decay-as-governance-knob omission (green #6), the B2-mechanism slip (§8), the identity-vs-governance seam slip (green #7), and the lossy-bridge consolidation.

## Credit

Hypha DAO — and the broader Hypha / SEEDS / ReGen Civics community — built and operate the DHO tooling, the document-graph, and the voice-token surveyed here, and authored the [Frontiers in Blockchain 2025](https://www.frontiersin.org/journals/blockchain/articles/10.3389/fbloc.2025.1630402/full) account of their DAO → DHO → DAO 3.0 evolution. Their decade of work — and their willingness to publish it openly and reflectively — made this engagement worthwhile.
