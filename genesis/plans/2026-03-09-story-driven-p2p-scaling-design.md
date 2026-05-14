# Story-Driven P2P Scaling with Per-Human Content Stewardship

**Date:** 2026-03-09
**Status:** Approved
**Scope:** Graduated 1→5 conductor scaling on alpha cluster, driven by a2o persona stories

## Problem

The genesis seeder bulk-loads all content through a single conductor — a god-mode operation that doesn't model real P2P dynamics. Each human should steward specific content based on their story, with content reaching others through protocol replication, not administrative bulk loading. Additionally, the alpha cluster's compute is pinned to an overcommitted intel-nuc (248% CPU limits) while ethosengine sits mostly idle.

We need to scale from 1→5 conductors deliberately, measuring compute at each step through shefa vocabulary, so we know before we request and prevent starving ourselves.

## Architecture Decisions

### Stewardship is a content property, not an administrative manifest
`stewardedBy` lives on each content node JSON — it's a property OF the content, describing who stewards it and with what affinity. Computed manifest views (who stewards what, total attribution) are projections — exactly what shefa does.

### Stewardship has graduated affinity
Stewardship isn't binary. The author typically has highest affinity, but curators, translators, and endorsers all have stewardship relationships with different weights. These weights determine:
- Which conductor is the "home" for that content (highest affinity steward)
- How REA value flows proportionally back to stewards
- Replication priority (higher affinity = earlier sync)

### Explicit stewardship first, then protocol dynamics
Reproducible scenarios require fixed starting conditions. We annotate content with explicit stewardship assignments, run scenarios against those, then layer in dynamic affinity-based discovery as a separate story.

### Compute constraints visible through shefa, not just k8s metrics
Every scale step records compute usage in shefa vocabulary (ComputeResource, EconomicEvent). The a2o scenarios assert against shefa budget, not kubectl output.

## Content Stewardship Model

### stewardedBy Field

Each content node in `genesis/data/lamad/content/` gets a `stewardedBy` array:

```json
{
  "id": "concept-constitutional-governance",
  "title": "Constitutional Governance",
  "stewardedBy": [
    { "humanId": "human-matthew-manager", "affinity": 0.9, "role": "author" },
    { "humanId": "human-pastor-pete-pastor", "affinity": 0.4, "role": "endorser" }
  ]
}
```

### Fields

- **humanId** — reference to a genesis human
- **affinity** — 0 to 1, strength of stewardship relationship
- **role** — informational classification: `author`, `curator`, `translator`, `endorser`, `steward`

### Rules

- `stewardedBy` is an array (content can have multiple stewards)
- Content with no `stewardedBy` defaults to `human-matthew-manager` with affinity 1.0 (backwards compatibility during migration)
- The highest-affinity steward's conductor is the "home" for that content
- Other stewards receive the content via P2P replication, not seeding
- Cold-start humans (Maria, Ronald) have no stewardship — they discover content through the network
- Affinity coefficients are normalized when computing REA value attribution (proportional flow)

### Stewardship Roles

| Role | Typical Affinity | What They Did |
|------|-----------------|---------------|
| author | 0.7-1.0 | Created the original content |
| curator | 0.4-0.7 | Organized, categorized, contextualized |
| translator | 0.5-0.8 | Adapted for a new language/culture (can exceed author for that community) |
| endorser | 0.2-0.5 | Reviewed and attested quality |
| steward | 0.3-0.6 | General ongoing maintenance |

### REA Value Flow

When content generates economic events (care-tokens, learning-tokens, recognition):
- Value flows proportionally to stewards based on normalized affinity
- Example: content with Matthew (0.9) and Pete (0.4) earns 10 care-tokens
  - Normalized: Matthew 0.9/1.3 = 69%, Pete 0.4/1.3 = 31%
  - Matthew receives ~7 tokens, Pete receives ~3
- Uses existing `ValueAttribution` model from rea-bridge
- `stewardship-begin` economic event type already exists

## Graduated Scale Steps

### Step 1: Single Conductor (Matthew)
**Proves:** Content loads, baseline measurement
- Matthew's conductor seeds content where he's highest-affinity steward
- Measure: CPU, memory, content count, seed time
- Assert: content accessible through doorway
- Compute: ~660m CPU request, ~1.2GB RAM

### Step 2: Two Conductors (Matthew + Susan)
**Proves:** Household replication via trust topology
- Susan's conductor seeds her stewardship content separately
- Household content replicates between them (spouse relationship, intimate trust level)
- Assert: Susan sees Matthew's shared-reach content via P2P replication
- Assert: Susan does NOT see Matthew's private-reach content
- Compute: ~1.3 cores request, ~2.4GB RAM

### Step 3: Three Conductors (+ Pastor Pete)
**Proves:** Cross-cluster bridge, reach-gated replication
- Pete discovers Matthew via congregation_member relationship (connection trust level)
- Faith community content replicates to Pete
- Assert: Pete sees community-reach content but NOT family-private content
- Assert: trust topology governs what replicates (not bulk access)
- Compute: ~2.0 cores request, ~3.6GB RAM

### Step 5: Five Conductors (+ Terrance + Frank)
**Proves:** Multi-hop content discovery, economic flows
- Terrance bridges learning and faith clusters (learning_partner with Susan, mentee with Sammy)
- Frank bridges economy cluster (business_partner with Georgina, congregation via Bub→Pete)
- Content flows through relationship bridges, not direct seeding
- Compute budget tracked per-conductor in shefa vocabulary
- Assert: content discovery paths match relationship graph
- Assert: no conductor exceeds its per-node budget
- Assert: cluster has not exceeded 80% of total budget
- Compute: ~3.3 cores request, ~6.0GB RAM

## Compute Budget Visibility

### Per Scale Step

```gherkin
Scenario Outline: Scale to <count> conductors within compute budget
  Given the cluster compute budget is
    | resource | total          | unit       |
    | cpu      | <cpu_budget>   | millicores |
    | memory   | <mem_budget>   | megabytes  |
  And <count> conductors are running for humans <humans>
  When genesis content is distributed by stewardship
  Then compute usage is visible in shefa vocabulary
  And no conductor exceeds 660m CPU request
  And no conductor exceeds 1200 MB memory request
  And the cluster has not exceeded 80% of total budget

  Examples:
    | count | humans                           | cpu_budget | mem_budget |
    | 1     | Matthew                          | 1000       | 2000       |
    | 2     | Matthew, Susan                   | 2000       | 4000       |
    | 3     | Matthew, Susan, Pastor Pete      | 3000       | 6000       |
    | 5     | Matthew, Susan, Pete, Tim, Frank | 5000       | 10000      |
```

### Shefa Economic Events

Each measurement produces:
- `EconomicEvent` with action `use`, resource `compute`, quantity in cpu-milliseconds
- Tracked per-conductor (agent = human ID)
- Aggregated per-cluster
- Budget assertion: `cluster_usage / cluster_budget < 0.8`

## Infrastructure: Unpin from intel-nuc

### Current State
- Node selector likely `node: "operations"` pinning all edgenodes to intel-nuc
- intel-nuc: 8 cores, 16GB — already at 248% CPU limit overcommit

### Target State
- Remove or relax node selector on elohim-alpha edgenode deployments
- Spread conductors to ethosengine (24 cores, 64GB, ~78% idle at request level)
- Optional: use thinkc-p0t (4 cores, 94% idle) for overflow
- Node affinity labels: `elohim-role: conductor` for explicit scheduling control

### Why ethosengine
- 18+ cores idle at request level
- 35GB+ RAM headroom
- Already runs Eclipse Che (development) and ingress (client traffic)
- Adding 5 conductors at request level = 3.3 cores, 6GB — well within headroom

## File Changes

| What | Where | Type |
|------|-------|------|
| Add `stewardedBy` to content nodes | `genesis/data/lamad/content/**/*.json` | Data annotation |
| Update seeder for per-conductor routing | `genesis/seeder/` | Code change |
| Graduated scaling scenarios | `genesis/a2o/features/deployment/persona-testnet-validation.feature` | A2O story |
| Compute budget step definitions | `genesis/a2o/steps/compute-coordination.steps.ts` | Step defs |
| Stewardship model type | `elohim-app/src/app/elohim/models/` (or content-node model) | TypeScript interface |
| Node affinity update | k8s deployment manifests | Infrastructure |

## Dependencies

- Coordination verb interfaces (just landed — `coordination-envelope.model.ts`)
- Persona testnet scripts (merged in PR #207 — `spawn-persona-testnet.sh`, `gen-persona-configs.sh`)
- Existing shefa compute models (`ComputeResource`, `EconomicEvent` with `use` action)
- Existing trust topology (`Reach` levels, `IntimacyLevel`, relationship types)

## What This Enables

With explicit per-human stewardship and graduated scaling:
1. **Reproducible P2P scenarios** — fixed stewardship assignments, deterministic content distribution
2. **Visible compute constraints** — shefa budget assertions in a2o stories, not hidden in k8s
3. **Protocol-native content discovery** — content reaches humans through trust topology, not admin bulk loading
4. **Foundation for dynamic stewardship** — once explicit assignments work, affinity-based discovery (option B) can layer on top
5. **REA value attribution** — stewardship affinity directly feeds proportional value flow
