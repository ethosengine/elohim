# Five-Peer Content Distribution — Genesis Seeds the Garden

**Date:** 2026-03-12
**Status:** Approved
**Scope:** Per-human StatefulSets, stewardship-aware SQLite seeder, Jenkins orchestrated seeding to 5 conductors

## Problem

The P2P scaling plan (2026-03-09) added `stewardedBy` annotations to all 3,525 genesis content nodes and created `filterBySteward()` in the doorway-mode seeder. But none of this actually runs. The genesis pipeline still bulk-loads everything to a single conductor — the "god-mode" operation the design doc called out as the problem.

The `seed-sqlite.ts` (which CI actually uses) has zero stewardship awareness. The alpha cluster runs 2 anonymous replicas with no human identity. Content distribution is a fiction.

## Architecture Decisions

### Per-human StatefulSets, not ordinal mapping

Each human gets their own StatefulSet (1 replica) rather than mapping ordinals within a shared StatefulSet. This enables:

- **Independent lifecycle** — `kubectl scale matthew --replicas=0` takes Matthew offline. The a2o partition healing scenario becomes a literal `kubectl scale`.
- **Heterogeneous resources** — Sammy's guardian-constrained node gets 256MB. Matthew's founder node gets 1GB. The YAML IS the story.
- **Independent failure domains** — Pod crashes in one StatefulSet don't block others.
- **Per-human config** — Each human has their own `P2P_BOOTSTRAP_NODES` reflecting trust topology.
- **Story-driven infrastructure** — Each YAML lives in `genesis/manifests/humans/` alongside account-packages and stewardship data. The file IS the human's infrastructure story.

### Jenkins orchestrates seeding, pods identify themselves

The genesis Jenkinsfile seeds each conductor after deploy. Flow:

1. Deploy all 5 StatefulSets
2. Wait for all pods healthy
3. Query each pod's `/manifest` endpoint to discover its `HUMAN_ID`
4. For each human: `STORAGE_URL=http://{pod} npx tsx src/seed-sqlite.ts --conductor-for={humanId}`
5. Report per-conductor content counts, assert distribution

This keeps genesis as the story orchestrator. Pipeline output shows "Matthew got 1,200 nodes, Pete got 340, Timothy got 280" — aggregated feedback that drives development.

Future exploration: seeder-as-a-k8s-service where pods pull their own content (protocol-native CI/CD). Not now.

### Add stewardship filtering to seed-sqlite.ts

Port `filterBySteward()` from `seed.ts` to `seed-sqlite.ts`. The filter reads `stewardedBy` from each content JSON, finds the highest-affinity steward, and only seeds content where that steward matches the `--conductor-for` value. Content with no stewardship match defaults to Matthew (founder, backwards compat).

## Per-Human StatefulSets

### First Five Humans

| Human | Story Role | Resources | Bootstrap Peers |
|-------|-----------|-----------|----------------|
| Matthew | Founder, governance, protocol core | 1Gi RAM, 500m CPU | Susan (household) |
| Susan | Household, family curriculum | 768Mi RAM, 400m CPU | Matthew (household) |
| Pastor Pete | Faith community, pastoral care | 768Mi RAM, 400m CPU | Matthew (congregation) |
| Timothy | Tutorials, mentorship, learning | 512Mi RAM, 300m CPU | Susan (learning partner) |
| Frank | Agriculture, local economy | 512Mi RAM, 300m CPU | Pete (congregation via Bub) |

### YAML Structure

```
genesis/manifests/humans/
  matthew-manager.yaml     # StatefulSet + headless Service
  susan-spouse.yaml
  pete-pastor.yaml
  timothy-tutor.yaml
  frank-farmer.yaml
```

Each is derived from the current `alpha.yaml` pattern with:
- `HUMAN_ID` env var on the elohim-storage container
- Per-human resource requests/limits
- Per-human `P2P_BOOTSTRAP_NODES` reflecting trust topology
- Same container images (conductor + storage + ws-proxy + happ-installer)
- NodeAffinity already in place — k8s spreads across intel-nuc and ethosengine

The existing `alpha.yaml` StatefulSet (replicas: 2) is replaced by these 5 individual deployments.

## seed-sqlite.ts Changes

Minimal additions:
- `--conductor-for` CLI arg parsing (mirrors `seed.ts` pattern)
- `filterBySteward()` function ported from `seed.ts`
- Filter applied after `loadContentFiles()`, before `transformContent()`
- Stewardship distribution logged: `[stewardship] Filtered to 340/3525 for human-pete-pastor`
- No changes to paths/blobs/account-packages seeding

## Jenkins Seeding Stage

The genesis Jenkinsfile seeding stage changes from single-target to per-conductor loop:

1. **Discover conductors** — query each pod's storage `/manifest` for `HUMAN_ID`
2. **Seed per human** — loop over 5 pods, call `seed-sqlite.ts --conductor-for={humanId}` targeting each pod's storage URL
3. **Report** — per-conductor content counts, total time, stewardship distribution
4. **Assert** — each conductor got content, no conductor got everything, total across 5 covers full corpus

## What This Doesn't Do (Yet)

- **P2P replication verification** — content gets seeded per-conductor, but we don't yet assert that Pete discovers Matthew's community-reach content via protocol replication
- **Reach-gated filtering** — all content a human stewards goes to their conductor regardless of reach level
- **Dynamic stewardship** — stewardship is static from genesis data, not earned through protocol interaction
- **Seeder-as-a-service** — Jenkins orchestrates, pods are passive recipients. Future: pods pull their own content (protocol-native CI/CD)
- **Schema version bridging** — all 5 peers run the same schema version. When feature branches change content schema or wire format, peers at different versions will need to negotiate. This emerges naturally from the branching/merge workflow and the 5-peer setup will be the first time it matters. A real P2P concern that centralized systems never face — schema migration as network-wide negotiation, not a deployment script.

## Dependencies

- Story-driven P2P scaling scaffolding (2026-03-09) — `stewardedBy` data, `filterBySteward()` in `seed.ts`, `conductor-groups.json`
- Dynamic route registry (2026-03-11) — `/manifest` endpoint on storage for conductor identity
- Existing alpha.yaml — template for per-human StatefulSets
- Existing genesis Jenkinsfile — seeding stage to modify

## What This Enables

1. **Real P2P dynamics** — 5 conductors with different content, discovering each other through trust topology
2. **Visible distribution** — pipeline output shows exactly what each human got
3. **Story-driven infrastructure** — each human's YAML tells their infrastructure story
4. **Foundation for replication testing** — once each conductor has its steward's content, we can test whether content flows through relationship bridges
5. **Foundation for heterogeneous testing** — different resource constraints model real-world device diversity
