# Hypha DAO governance-token prior art (VOICE → reach/standing)

> Module-boundary pointer note. Full survey: [`genesis/research/hypha-dao-autonomous-collectives-cross-pollination-2026-06-24.md`](../../../genesis/research/hypha-dao-autonomous-collectives-cross-pollination-2026-06-24.md).
> **Name-collision guard:** `hypha-dao` (DAO/DHO governance tooling, this note) ≠ `hyphacoop` (Distributed Press) ≠ `Pointsnode/hypha-network` (unrelated).

[Hypha DAO](https://github.com/hypha-dao)'s [`voice-token`](https://github.com/hypha-dao/voice-token) is the closest external prior art for the protocol's **reach / standing** as a non-currency governance signal. Its repo line is verbatim: *"Contract for **non-transferable, mintable** token used for voting."* In the product (HVOICE) it is:

- **Earn-only, never bought** — explicitly "to move away from oligarchy/plutocracy models where voice can be purchased." Decouples vote weight from capital.
- **Work-minted** — `HVOICE = USD × 2` per Assignment, tiered by salary band.
- **Decaying** — `DecayPerPeriod` / `DecayPeriod` struct fields are confirmed (decay-*exists*: HIGH); the **specific rates** (~1-year half-life, ~1.4% per lunar phase, inactive after 6 months) are **policy-sourced / MEDIUM**, possibly proposed-not-wired. Effect: influence tracks *recent* contribution, not hoarding.

## The borrow (green)

The **use-it-or-lose-it decay intuition**, re-homed as a **recognition-decay kernel** on reach/standing — already a planned post-v0 direction, **(B) temporal decay/accrual** in [`../../elohim-storage/research/future-distribution-models.md`](../../elohim-storage/research/future-distribution-models.md). Mechanically:

- **Operational-C** — reach/standing is *recomputed on read* from notarized contribution events with a half-life kernel. **Do NOT mint a `VoiceBalance` / `StandingToken` DHT entry** — that makes standing a bank-like ledger and burns the entry-cap budget. The DHT notarizes the underlying contributions; the decay lives in the projection fold. (Same shape as the VSM `CoverageRollup` recompute-on-read.)
- **Borrow the intuition, not the rate** — Hypha's specific half-life may be aspirational; the protocol picks its own.
- **Recency, not tenure** — decay rewards *recent* participation, so do **not** also bolt on a "reward durable commitment" premium (Hypha's +30% defer bonus) that pulls the opposite way. Pick one temporal stance.
- **The half-life is a governed parameter.** Whoever sets it controls whose standing evaporates and how fast — a soft capture surface. Route the parameter to the constitutional / charter layer (subsidiarity) and apply the [rent-extraction test](../../holochain/dna/CLAUDE.md) to the *parameter-setting authority*, not just the entry type.

Justice framing: decay is **non-renewal of a gift**, never punishment — an inactive captor's grip fades on its own. That is justice-as-restored-capability, not retribution.

## The reject (red)

Everything *transferable*, at the "[DHT is notary, not bank](../../holochain/dna/CLAUDE.md)" line:

- **REJECT** HYPHA-as-voting-multiplier ("total Hypha holdings weigh on voting power") — held fungible capital buying governance weight is the exact plutocratic backdoor the [identity-sovereignty guard](../../../.claude/skills/p2p-design-gate/SKILL.md) names. Reach/standing must stay purely *earned*; capital must never weigh.
- **REJECT** a fungible treasury token (HUSD redeemable for ETH/BTC, multisig issue/burn) — that is the "become the bank" rent-extraction failure. The protocol's **REA compute-commitment primitive** (`Mishpat::Commitment` + `delegates-compute`) already gives the *audit properties* of a threshold-multisig (checkable standing, real revocation, notarized authority chain) **without** holding fungible value.
- **DEFER** USD-denominated salary bands + deferral premiums to the post-v0 distribution work, stripped of currency denomination.
- **REJECT** SEEDS / market-traded ReFi currency as a substrate dependency (comparator frame only — "build native, steal patterns").
