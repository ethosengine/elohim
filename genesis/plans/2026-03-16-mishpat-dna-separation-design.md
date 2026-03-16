# Mishpat DNA Separation — Governance Split from Lamad

**Date:** 2026-03-16
**Status:** Approved
**Problem:** Governance entry types (proposals, votes, challenges, appeals, precedents, statements) currently live in the lamad DNA at 83/~100 entry types. Governance is a distinct concern from learning content — it has its own validation rules, its own lifecycle, and its own trust requirements. Keeping it in lamad wastes scarce entry type capacity and couples governance evolution to content evolution.

**Solution:** Create a new Holochain DNA named **Mishpat** (מִשְׁפָּט — justice, judgment, the legal process). Move 7 governance entry types and ~18 link types from lamad into mishpat. Lamad drops from 83 to 76 entry types. Mishpat starts with comfortable headroom at 7/~100.

## DNA Naming

| DNA | Hebrew | Meaning | Domain |
|---|---|---|---|
| Lamad | לָמַד | To learn | Content, paths, mastery, knowledge graph |
| Imagodei | בְּצֶלֶם אֱלֹהִים | Image of God | Identity, relationships, attestations, presence |
| Qahal | קָהָל | Assembly | Community structure, collectives, directory, affinity |
| **Mishpat** | **מִשְׁפָּט** | **Justice, judgment** | **Formal governance — proposals, votes, challenges, appeals, precedents** |
| Infrastructure | — | — | Doorway registry, heartbeats, node discovery |

## What Moves

### Entry Types (7, from lamad → mishpat)

| Entry Type | Classification | Description |
|---|---|---|
| Proposal | A (Notarized) | Formal governance proposals |
| Precedent | A (Notarized) | Binding governance decisions, case law |
| Challenge | A (Notarized) | Formal challenges to content or decisions |
| OpinionStatement | A (Notarized) | Polis-style sensemaking statements |
| ProposalVote | B2 (Agent-Scoped + Attestation) | Private ballot, verifiable tally |
| StatementVote | B2 (Agent-Scoped + Attestation) | Private stance, clustered aggregate |
| GovernanceReaction | B2 (Agent-Scoped + Attestation) | Emotional feedback signals |

### Link Types (~18, from lamad → mishpat)

All governance-related links: ProposalToVotes, AgentToVotes, VoteByPosition, ContextToStatements, AgentToStatements, StatementToVotes, AgentToStatementVotes, ContentToReactions, AgentToReactions, ReactionByType, ReactionToMediation, IdToChallenge, EntityToChallenge, ChallengerToChallenge, ChallengeByStatus, IdToProposal, ProposalByType, ProposerToProposal, ProposalByStatus, IdToPrecedent, PrecedentByScope, PrecedentByStatus.

### Stays in Lamad

`MasteryChallenge` — this is a learning assessment construct (practice pool challenge), not a governance entity. Despite the name, it belongs with content/mastery.

## DNA Structure

```
elohim/holochain/dna/mishpat/
├── dna.yaml
├── zomes/
│   ├── mishpat_integrity/
│   │   ├── Cargo.toml
│   │   └── src/lib.rs          # 7 entry types + ~18 link types + validation
│   └── mishpat/
│       ├── Cargo.toml
│       └── src/lib.rs          # Coordinator: CRUD + cross-DNA bridges
└── workdir/
    └── dna.yaml                # DNA manifest for packaging
```

## Cross-DNA Bridges

Mishpat needs to verify entities in other DNAs. The bridge pattern already exists (imagodei → lamad).

| From (mishpat) | To | Purpose |
|---|---|---|
| Create proposal | lamad | Verify content_id exists |
| Create challenge | lamad | Verify target entity exists |
| Cast vote | imagodei | Verify voter identity + attestation eligibility |
| Issue tally attestation | imagodei | Create Attestation entry for verifiable result |

Pattern:
```rust
let response: ZomeCallResponse = call(
    CallTargetCell::OtherRole("imagodei".into()),
    "imagodei",
    "get_human_by_id".into(),
    None,
    voter_id,
)?;
```

Cross-DNA calls are synchronous within the same conductor — no network hop.

Governance entries themselves live entirely in mishpat. Only identity verification and content existence checks cross the bridge.

## Capacity After Split

| DNA | Before | After | Headroom |
|---|---|---|---|
| Lamad | 83 entry types | ~76 entry types | Comfortable |
| Mishpat | N/A | 7 entry types | Wide open (93 slots) |
| Imagodei | 28 | 28 (unchanged) | Comfortable |
| Infrastructure | 6 | 6 (unchanged) | Wide open |

## What Changes Where

### Holochain layer (new files + removals)
- Create `elohim/holochain/dna/mishpat/` — full DNA with integrity + coordinator zomes
- Remove 7 entry types and ~18 link types from `content_store_integrity/src/lib.rs`
- Remove corresponding coordinator functions from `content_store/src/lib.rs`
- Create DNA manifest `mishpat/dna.yaml`

### Storage layer (reference updates only)
- No migration changes — Sprint 4 already wired dht_anchor_hash on all governance tables
- Update source-of-truth comments to reference "mishpat DNA" instead of "lamad DNA"
- Update governance.rs TODO comments to reference mishpat coordinator functions

### Angular (no changes)
- Frontend talks to elohim-storage HTTP routes, not directly to zomes
- DNA split is invisible to Angular

### CI/CD
- DNA Jenkinsfile needs to build mishpat alongside existing DNAs
- Orchestrator changeset patterns need `elohim/holochain/dna/mishpat/` added

### Documentation
- Update DHT capacity constraints in memory and p2p-design-gate skill
- Update CLAUDE.md architecture section with mishpat DNA
- Update rust-architect agent with mishpat in the DNA list

## What This Does NOT Do

- Does not change the storage layer schema (already done in Sprint 4)
- Does not change Angular code (invisible to frontend)
- Does not add new entry types (only moves existing ones)
- Does not change the HTTP API routes
- Does not implement the signal wiring (separate TODO)

## Success Criteria

- Mishpat DNA builds and packages successfully (WASM)
- Lamad DNA builds without the removed entry types
- Cross-DNA bridge calls compile and resolve
- All 404 existing tests pass
- `dht_anchor_hash` source-of-truth comments reference the correct DNA
- p2p-design-gate skill capacity table reflects the new numbers
