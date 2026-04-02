# Elohim Token: EPR-Native Minting Design

## Context

The Elohim Protocol needs a default economic rail — a single fungible medium of exchange minted from witnessed care, work, and exchange. Not proof-of-work (burn electricity), not proof-of-stake (proof of inequality): **proof of witnessed contribution**.

The protocol already has:
- EPR Heads carrying three-leg coupling (lamad/shefa/qahal) with stewardship allocations and recognition policies
- A recognition pipeline that fires REA economic events on content delivery
- A `tokens_minted` field on economic event views (currently unpopulated)
- Internal token types (care, time, learning, steward, creator, infrastructure) with decay rates
- ResourceNature, CommonsPool, and AttributionClaim primitives in the shefa pillar

What's missing: the token itself — the fungible default rail that makes witnessed contribution liquid, exchangeable, and ultimately competitive with fiat currency.

### Theory of Value

The token's value derives from the network's demonstrated capacity to observe and record real contribution. Value is minted when the network verifiably witnesses that care, work, or exchange actually happened, attested by three-leg coupling: governance context (qahal), knowledge context (lamad), and the economic event itself (shefa REA observation).

The key structural mechanism is the **ResponsibilityDemandParam** — as you accumulate, more is demanded of you. Power coupled with responsibility. Not friction (punishment), not taxation (confiscation) — obligation (stewardship).

See: `elohim/elohim-token/research/theory-of-value.md` for full thesis, empirical basis (Ariely/Norton 92% consensus, Robeyns limitarianism, Beer's cybernetic governance).

---

## Architecture

### Two-Tier Minting

#### Tier 1: Micro-Mint (Deterministic, Per-Event)

When the existing recognition pipeline fires an REA economic event, a deterministic micro-amount of elohim-token is minted as part of that same event.

**Mint amount** is a function of:
- The event's `recognitionPolicy` weights (already defined per EPR Document: onView, onComplete, onAttest, onTeach)
- The content's stewardship allocations (already in EPR Head's `shefa` context)
- A network-wide `mintRate` parameter (qahal-governed; initial value determined during implementation based on projected network size and target circulation velocity)

**Distribution** follows the EPR's allocation ratios — the same path recognition already flows. The token IS the recognition, made fungible.

**Integration point:** Inject a mint step into the existing `recognition_pipeline_service.rs` between the "settle" stage and completion. The `tokens_minted` field on `EconomicEventComputeView` (currently `Option<f64>`, unpopulated) gets populated.

#### Tier 2: Elohim Discernment Mint (Periodic, Wisdom-Driven)

Elohim agents evaluate accumulated REA event patterns at a qahal-governed cadence (daily or weekly). They discern contribution that individual events can't capture:

- **Consistency**: the nurse who always stays late with dying patients
- **Cascading impact**: a tutorial that unlocked learning for hundreds
- **Cross-domain stewardship**: the repair tech whose documented methods spread globally
- **Community resilience**: the depot operator whose coordination prevented three emergencies

Discernment mints are issued as separate economic events with:
- Elohim agent attestation (cryptographic proof of evaluation)
- Reasoning traces (explainability — why this contribution was discerned)
- Constitutional context (which governance layer's values were applied)

The word is "discernment," not "amplification." The elohim aren't turning up the volume — they're seeing what's actually there.

### EPR Coupling

Every mint event is structurally coupled to an EPR. There is no free-floating token creation.

```
EPR Head (Tier 1, ~500B, DHT-gossipped)
  └─ shefa: { stewards[], allocations[] }
       │
       ▼  content delivered to a peer
Recognition Pipeline (existing)
  └─ fires REA EconomicEvent
       │     content_id: "epr-{cid}"
       │     action: "use" | "cite" | "produce"
       │     lamad_event_type: "content-view" | "mastery-advance" | ...
       │
       ▼  NEW: token mint step
Token Mint Service
  └─ calculates micro-mint amount from:
       │     recognitionPolicy weights (EPR Document Tier 2)
       │     network mintRate (qahal-governed)
       │
       ▼  distributes per allocation ratios
Token Ledger
  └─ credits steward balances
       └─ ResponsibilityDemandParam checked on accumulation
```

**Non-EPR physical economy events** (Lisa brings compost, Tom repairs the chipper-shredder) are REA economic events linked to a community EPR — the depot's stewardship-context ContentNode. The depot itself is content in the protocol; its existence, rules, and resource inventory are EPR-addressable. Physical contributions are economic events linked to that community EPR.

---

## ResponsibilityDemandParam

### The Curve

Initialized from the 92% consensus (Ariely/Norton, 2011): wealthiest 10-20x the poorest, healthy middle class, nobody destitute. Stored as `ResponsibilityDemandConfig` — a qahal-governed DHT entry type.

| Holdings relative to network median | Effect |
|--------------------------------------|--------|
| Below dignity floor | No demands. Commons pool supports you. Transfers in are frictionless. |
| Floor to median | Normal circulation. Minimal obligations. The economy breathes. |
| Median to 10x | Increasing governance visibility. Stewardship expectations grow. |
| 10x to 20x | Significant responsibility. Must demonstrate active stewardship. Transfer friction as obligation (not fees — requirements before transfer clears). |
| Above 20x | Extreme responsibility. Elohim scrutiny. Constitutional justification required for large transfers. Self-limiting through weight of obligation. |

**Key principle:** Friction is obligation, not taxation. The network doesn't take your tokens. It demands that you DO something with your position — steward content, fund infrastructure, mentor, govern.

### Context-Aware Curves (Robeyns Insight)

The curve shape is contextual to each governance layer's social contract health:

```
Social contract strength:  HIGH ◄──────────────► LOW
                          Norway                 Lagos
Curve tightness:          TIGHT                  LOOSE
Accumulation tolerance:   LOW                    HIGH
Protocol role:            Supplement             IS the safety net
```

- In Norway: strong social contract provides healthcare, education, housing. Individual accumulation above modest thresholds serves no survival need. Tight curve.
- In the US: damaged social contract drives individual accumulation as self-insurance. Looser curve — the protocol compensates for what society doesn't provide.
- In Lagos: extraction economy, minimal social contract. The protocol IS the safety net. Maximum circulation required, loosest curve for individuals, tightest obligations for any entity approaching concentration.

**Structural flow from gradient:** The difference between contexts creates micro-arbitrage — tokens naturally flow from where they're least needed (high social contract) toward where they're most needed (low social contract). Not charity. Structural economics. Millions of tiny flows driven by curve differentials, automatically collecting what love demands and routing it where it's needed most.

Elohim sense social contract health per governance layer and recommend curve parameters. Qahal ratifies changes through consent process.

### Story-Memory Decay

Tokens carry provenance — a `provenance_event_id` linking back to the REA event that minted them.

- Elohim periodically scan the token supply and evaluate provenance vitality: is the original contribution still referenced? Still building value? Still circulating in the graph?
- Dormant provenance tokens decay gently at elohim-recommended, qahal-ratified rates
- Active provenance tokens persist indefinitely
- Dignity floor is exempt — tokens below the floor never decay regardless of provenance

This isn't fixed demurrage. It's a living judgment about which stories the network is still telling. Like commits getting squashed — something gets lost if you don't use it.

---

## DHT Entry Types

Four new entry types in the Elohim DNA (4 of ~27 available slots):

| Entry Type | Category | Fields |
|-----------|----------|--------|
| `TokenMintEvent` | A (notarized) | amount, provenance_event_id, mint_tier (micro \| discernment), source_epr_id, constitutional_context, elohim_attestation (Tier 2 only), reasoning_trace (Tier 2 only) |
| `TokenTransfer` | A (notarized) | from_agent, to_agent, amount, provenance_chain (which mints back this), responsibility_check_result, governance_layer |
| `TokenBalance` | B (agent-scoped) | agent_id, balance, governance_layer, social_contract_context, last_activity_timestamp, provenance_vitality_score |
| `ResponsibilityDemandConfig` | A (constitutional) | governance_layer, breakpoints[], obligation_thresholds[], social_contract_health_score, decay_parameters, ratified_by, ratified_at |

---

## Protocol Schema

New schemas in `elohim/sdk/schemas/v1/objects/`:

- `token-mint-event.schema.json`
- `token-transfer.schema.json`
- `token-balance.schema.json`
- `responsibility-demand-config.schema.json`

These generate TypeScript types via `pnpm run schema:codegen:ts` and Rust types via `pnpm run schema:codegen:rs`, following the existing IoC pattern.

---

## Codebase Placement

### Rust Services (`elohim/elohim-storage/src/services/`)

| Service | Purpose |
|---------|---------|
| `token_mint_service.rs` | Micro-mint calculation from recognition policy + mint rate. Discernment mint creation with elohim attestation. |
| `token_ledger_service.rs` | Balance tracking, transfer validation, curve enforcement, story-memory decay evaluation. |
| `responsibility_demand_service.rs` | Curve evaluation per governance layer, social contract health sensing, obligation checking on transfers. |

### Database (`elohim/elohim-storage/migrations/`)

New migration creating:
- `token_mint_events` table
- `token_transfers` table
- `token_balances` table
- `responsibility_demand_configs` table

### API (`elohim/elohim-storage/src/api/`)

- `token.rs` — routes for balance queries, transfer creation, mint history, curve status

### Integration

- `recognition_pipeline_service.rs` — inject mint step after "settle" stage
- `rea_projection.rs` — project TokenMintEvent and TokenTransfer from DHT post-commit signals

### Integrity Zome

- `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/token_validation.rs` — validation rules for mint events (must link to valid REA event), transfers (must pass curve check), and config changes (must have qahal governance proof)

### Research & Spec

- `elohim/elohim-token/research/` — theory of value, empirical basis, Tithereum lineage (already exists)
- `elohim/elohim-token/README.md` — overview (already exists)

---

## Settlement Bridge Interface

The token lives natively on Holochain. A settlement bridge enables global consensus and fiat conversion on a future chain (decision deferred).

### What crosses the bridge

| Data | Purpose |
|------|---------|
| Token amount | How much is being bridged |
| Provenance hash | Merkle root of the REA event chain backing these tokens |
| Constitutional context | Governance layer, social contract health, curve parameters |
| Elohim signature | Cryptographic proof elohim evaluated and approved the crossing |

### What stays on Holochain (never crosses)

- Full REA event history
- Individual story-memory provenance chains
- Governance deliberation records
- Agent identity details beyond DID

### Bridge trait (chain-agnostic)

```rust
trait SettlementBridge {
    fn bridge_out(
        &self,
        amount: f64,
        provenance_root: Hash,
        constitutional_ctx: ConstitutionalContext,
        elohim_sig: ElohimSignature,
    ) -> Result<BridgeReceipt>;

    fn bridge_in(&self, receipt: BridgeReceipt) -> Result<TokenMintEvent>;

    fn verify_provenance(&self, root: Hash) -> Result<ProvenanceProof>;
}
```

The provenance hash means any settlement chain can verify that tokens were backed by real witnessed events, even without seeing the full event history. This is what makes the token different from every other crypto — it carries proof of care, not proof of computation.

ResponsibilityDemandParam still applies to bridge-out operations — large bridge-outs trigger the curve's obligation requirements before the bridge clears.

---

## What's New vs What Exists

| Component | Status | Work Required |
|-----------|--------|---------------|
| EPR Head with shefa context | ✅ Built | None |
| Recognition pipeline (5-stage) | ✅ Built | Inject mint step |
| Economic event with `tokens_minted` | ✅ Built (unpopulated) | Populate the field |
| REA event → content_id linking | ✅ Built | None |
| Stewardship allocations + ratios | ✅ Built | None |
| Token type system (care, time, etc.) | ✅ Built | Internal tokens become evidence; elohim-token is the derived rail |
| Shefa domain manifest | ✅ Built | Add token signals/observations |
| Token Mint Service | ❌ New | Calculate mint from policy + rate |
| Token Ledger Service | ❌ New | Balance tracking, curve enforcement |
| ResponsibilityDemandParam | ❌ New | Curve evaluation, context-aware |
| Elohim Discernment Service | ❌ New | Pattern evaluation, attestation minting |
| Story-memory decay | ❌ New | Provenance vitality scanning |
| Settlement bridge interface | ❌ New | Trait definition, chain-agnostic |
| DHT entry types (4) | ❌ New | Integrity zome validation |
| Protocol schemas (4) | ❌ New | JSON Schema + codegen |

---

## Verification

1. Schema validation: `pnpm run schema:validate` passes with new token schemas
2. Codegen: `pnpm run schema:codegen:ts` and `pnpm run schema:codegen:rs` generate token types
3. Rust build: `RUSTFLAGS="" cargo build` in elohim-storage compiles with new services
4. Unit tests: mint calculation determinism, curve enforcement, decay mechanics
5. Integration test: EPR content delivery → recognition pipeline → micro-mint → balance update
6. Grep verification: no references to "amplification" in token context (use "discernment")

---

## Key References

- `elohim/elohim-token/research/theory-of-value.md` — core thesis, ResponsibilityDemandParam, empirical basis
- `elohim/elohim-token/research/wealth-inequality-in-america.md` — 92% consensus curve initializer
- `elohim/elohim-token/research/tithereum-enlightenment-neo-restoration.md` — intellectual lineage, Tie velocity constraints
- `elohim/elohim-storage/research/economic-systems-research.md` — Drips, Unyt, hREA, EAE consilience
- `genesis/docs/content/elohim-protocol/protocol-specification.md` — EPR Head structure, coupling rules
- `elohim/elohim-storage/src/services/recognition_pipeline_service.rs` — existing pipeline to extend
- `app/elohim-app/src/app/elohim/models/protocol-core.model.ts` — existing token types, decay rates
- `app/elohim-app/src/app/elohim/models/rea-bridge.model.ts` — REA primitives, CommonsPool
