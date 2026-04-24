# Holochain Network Upgrades: The Elohim Coordination Model

> **STATUS (2026-04-24)**: The `lineage` field recommended throughout this
> doc is currently **omitted from every dna.yaml**. Holochain 0.6 gates the
> field behind the `unstable-migration` cargo feature; the stable `hc dna
> pack` rejects it as an unknown field. All alpha-network manifests now
> declare only `name`, `integrity`, `coordinator`. Upgrade history is
> reconstructed from git + network_seed rollover until the feature
> stabilizes upstream (or we adopt a holonix flavor that enables it).
>
> This is a real regression — see the `rna/` module (currently on the
> backburner) and a scheduled brainstorming session on first-class upgrade
> paths. The policy below describes the intended end-state; today's
> mechanics are git-history + `_alpha` → `_alpha2` seed rollover.

## The Problem

In Holochain, the DNA hash IS the network identity. Any change to:
- Entry struct definitions (adding/removing/renaming fields)
- Link types
- Validation logic
- Constants used in validation

...produces a completely new DNA hash, which means a **completely new network**.

```
DNA v1 (hash: abc123)     DNA v2 (hash: def456)
┌─────────────────┐       ┌─────────────────┐
│   Network A     │       │   Network B     │
│  [users, data]  │       │  [empty]        │
└─────────────────┘       └─────────────────┘
        ↑                         ↑
   No bridge between these networks
```

**Result**: Every integrity zome change wipes all data. Users on v1 can't see v2 users. The networks are completely separate.

---

## Is This a Feature or a Bug?

### The "Feature" Argument

1. **Trustless validation** - Every agent knows everyone has identical rules
2. **No rug-pull upgrades** - Rules can't change without conscious migration
3. **Immutable history** - Old entries validated under v1 stay valid under v1
4. **No central authority** - No one can decree "upgrade now"

### The "Bug" Argument

1. **Every production system needs upgrades** - This is unavoidable reality
2. **Blockchains solved this** - Hard forks, soft forks, coordinated upgrades exist
3. **Massive developer burden** - Every app builds its own migration infrastructure
4. **Discourages iteration** - Fear of breaking changes stifles development

### The Honest Answer

It's a trade-off optimized for **adversarial, trustless networks**:

```
Easy upgrades    = Someone has power to change rules
Hard upgrades    = Rules are actually binding
```

The question is: **who is this designed for?**

- Fully decentralized adversarial networks: Benefit from rigidity
- Stewarded networks (like Elohim): Pay the cost without the full benefit

---

## What Holochain Provides for Bridging

### Bridge Calls (Conductor-Level)

A conductor can run multiple DNAs, and zomes can make "bridge calls":

```rust
// In DNA v2 coordinator
#[hdk_extern]
pub fn migrate_from_v1(id: String) -> ExternResult<Content> {
    let old: Content = call_bridge("lamad-v1", "content_store", "get_content_by_id", id)?;
    create_content_v2(transform(old))
}
```

### The Limitations

1. **Users must have both DNAs installed** - Bridge only works locally
2. **No network-level bridge** - v2 can't query the v1 DHT directly
3. **No coordination mechanism** - No "upgrade signal" propagates through network

### What Migration Looks Like Today

```
Day 0:  Everyone on v1
        v1: [A] [B] [C] [D]

Day 1:  Some migrate, network fragments
        v1: [A] [B]          v2: [C] [D]
        (A,B can't see C,D and vice versa)

Day 30: Hopefully everyone migrated?
        v1: (dead)           v2: [A] [B] [C] [D]
```

---

## The Elohim Solution: Stewarded Coordination

### The Insight

The Elohim nodes - constitutional AI stewards - can BE the coordination mechanism.

```
┌─────────────────────────────────────────────────────────────┐
│                    Elohim Network                            │
│                                                              │
│   ┌──────────┐    ┌──────────┐    ┌──────────┐             │
│   │ Elohim 1 │────│ Elohim 2 │────│ Elohim 3 │             │
│   │ (family) │    │(community)│    │ (global) │             │
│   └────┬─────┘    └────┬─────┘    └────┬─────┘             │
│        │               │               │                    │
│   Consensus: "DNA v2 ready, begin migration"                │
│        │               │               │                    │
│        ▼               ▼               ▼                    │
│   ┌─────────────────────────────────────────────┐          │
│   │         Migration Period                     │          │
│   │   - All Elohim run v1 AND v2                │          │
│   │   - Export user data from v1                │          │
│   │   - Import into v2                          │          │
│   │   - Hosted users seamlessly transitioned    │          │
│   └─────────────────────────────────────────────┘          │
│        │               │               │                    │
│        ▼               ▼               ▼                    │
│   All users now on v2, v1 sunset                           │
└─────────────────────────────────────────────────────────────┘
```

### Why This Works

1. **Elohim aren't a central authority** - They're constitutional stewards bound by protocol values. They can't change rules, only facilitate transitions.

2. **Users already trust Elohim** - The model assumes Elohim steward presences, facilitate learning, synthesize knowledge. Migration is another stewardship function.

3. **DNA still enforces rules** - Elohim can't sneak in bad rules. New DNA is transparent, auditable. Users can reject by running their own v1 node.

4. **Hosted users get seamless experience** - They don't manage infrastructure. Elohim handles complexity.

### The Governance Flow

```
1. NEED IDENTIFIED
   └─► Elohim observe: "Network needs feature X, requires integrity change"

2. PROPOSAL
   └─► New DNA v2 developed, published, auditable
   └─► Community can inspect the changes

3. ELOHIM CONSENSUS
   └─► Elohim nodes agree: "v2 is safe, aligns with protocol values"
   └─► This IS the coordination signal

4. MIGRATION WINDOW
   └─► Elohim run both versions simultaneously
   └─► User data migrated progressively
   └─► Hosted users transitioned transparently

5. SUNSET
   └─► v1 deprecated
   └─► Network unified on v2
```

---

## Centralized vs Elohim Coordination

| Aspect | Centralized Platform | Elohim Coordination |
|--------|---------------------|---------------------|
| Decision maker | Company decides, users comply | Elohim propose, users can exit |
| Rule transparency | Hidden in ToS | DNA code is public, auditable |
| User alternatives | None - upgrade or leave | Can run own v1 node forever |
| Trust model | Trust the company | Trust the code + distributed stewards |
| Upgrade mechanism | Forced, immediate | Coordinated, gradual, consensual |

---

## What We're Actually Building

We're not building a trustless adversarial network. We're building a **stewarded commons**:

- **Transparent, immutable rules** (DNA as constitution)
- **Distributed but coordinated stewards** (Elohim network)
- **User sovereignty with convenience** (hosted but exportable)
- **Upgrade path** that doesn't require global social coordination

The Elohim make Holochain's weakness (coordination) into a strength that aligns with our governance model.

---

## Practical Implications

### For Development

1. **Freeze integrity zome early** - Entry types are comprehensive, stabilize them
2. **Use metadata_json fields** - Extend without schema changes
3. **Iterate on coordinators** - These changes are free, no migration needed
4. **Build export/import now** - Migration infrastructure is core, not afterthought

### For Deployment

1. **Elohim nodes run both DNAs during transition**
2. **Seeder doubles as migration tool** - Same pattern: export → transform → import
3. **Browser clients connect to v2** - Elohim handles bridging internally
4. **v1 sunset after migration complete**

### Safe vs Breaking Changes

| Change Type | DNA Hash Changes? | Migration Required? |
|-------------|-------------------|---------------------|
| Add entry field | Yes | Yes |
| Remove entry field | Yes | Yes |
| Add link type | Yes | Yes |
| Change validation | Yes | Yes |
| Add coordinator function | No | No |
| Change coordinator logic | No | No |
| Add to metadata_json | No | No |

---

## Forward-Compat Policy (Wave 1 §7)

Each DNA manifest declares `lineage: []` at top level. The current hash is the
genesis ancestor. The policy below governs when you add to the lineage vs keep
the same hash.

### Additive changes — no hash bump, no lineage entry

- New **optional** fields on entry structs, annotated with `#[serde(default)]`
- New link types (additive only — never repurpose an existing one)
- New coordinator functions
- Changes inside coordinator-only logic
- Extensions to `metadata_json` or similar free-form fields

Rule: old and new agents must both be able to deserialize entries written by
the other side. `#[serde(default)]` ensures forward-compat when a newer field
is absent; skipping unknown fields (serde default) ensures backward-compat.

### Breaking changes — bump DNA hash, add previous hash to `lineage`

- Removed or renamed fields on entry structs
- Changed validation (tighter OR looser — either way, the network's rules changed)
- Repurposed link types (same tag, different semantics)
- Changed entry-type enum variants
- Any change to `integrity` zome constants used in validation

When you make a breaking change:

1. Capture the previous DNA hash (the one this replaces): `hc dna hash workdir/<name>.dna`
2. Prepend it to the DNA's `lineage: []` list: `lineage: ["uhC0k<old-hash>"]`
3. Document the migration path in NETWORK_UPGRADES.md (the bridge/seeder story)
4. The stewarded migration flow (elohim nodes running both DNAs) applies

### Why lineage matters (even though Holochain doesn't enforce it yet)

`lineage` is honored by later tooling (`hc` CLI, Launcher, Moss group admin
UIs) to show "this DNA supersedes X, Y, Z." It documents the ancestry even
when users are only running the current version. Future DHT bridging tools
will use it to route cross-version queries. Not setting it today means we
cannot retroactively reconstruct the chain.

### Network seed upgrade ladder

Network seeds are stability contracts. The suffix declares intent:

| Suffix | Meaning |
|---|---|
| `_alpha` | Will be reset on any breaking change. Not production. |
| `_beta` | Breaking changes migrate; no silent resets. |
| (none) | Production — breaking changes require governance + lineage + migration. |

Transitions are one-way (alpha → beta → prod). A reset means bumping `_alpha`
to `_alpha2` (or similar) with the old hash in `lineage` so the history
remains discoverable.

---

## Open Questions

1. **Migration tooling** - What's the exact export/import pipeline?
2. **Elohim consensus mechanism** - How do Elohim agree on upgrade readiness?
3. **User notification** - How are users informed of upcoming migrations?
4. **Rollback strategy** - What if v2 has critical bugs post-migration?
5. **Independent users** - How do self-hosted users participate in coordination?

---

## Conclusion

The "DNA hash = network identity" design creates separation, not bridges. But for a stewarded network like Elohim, this is solvable:

**The Elohim ARE the bridge.**

They span both networks during transition, handle migration complexity, and provide the coordination signal that pure peer-to-peer networks lack.

This transforms Holochain's architectural constraint into alignment with the Elohim Protocol's governance philosophy: distributed stewardship with transparent, immutable rules.
