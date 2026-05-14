# Story-Driven P2P Scaling Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add `stewardedBy` with graduated affinity to genesis content nodes, update the seeder to route content per-conductor, write graduated 1→5 scaling scenarios, and unpin edgenodes from intel-nuc.

**Architecture:** Content nodes get a `stewardedBy` array with humanId + affinity + role. The seeder filters content per-conductor based on highest-affinity steward. A2O scenarios validate each scale step with shefa compute budget assertions. K8s nodeSelector is relaxed to spread conductors across available nodes.

**Tech Stack:** TypeScript (content models, seeder), Gherkin (a2o features), YAML (k8s manifests), Python (content annotation script)

---

### Task 1: Add ContentSteward type to content-node model

**Files:**
- Modify: `elohim-app/src/app/lamad/models/content-node.model.ts`

**Step 1: Read the existing ContentNode interface to find where to add the field**

Run: `grep -n "export interface ContentNode" elohim-app/src/app/lamad/models/content-node.model.ts`

**Step 2: Add the ContentSteward interface and stewardedBy field**

Add before the ContentNode interface:

```typescript
/**
 * ContentSteward - A human's stewardship relationship with content.
 *
 * Stewardship is graduated, not binary. The author typically has highest
 * affinity, but curators, translators, and endorsers all have stewardship
 * relationships with different weights.
 *
 * Affinity determines:
 * - Which conductor is "home" for this content (highest affinity steward)
 * - How REA value flows proportionally back to stewards
 * - Replication priority (higher affinity = earlier sync)
 */
export interface ContentSteward {
  /** Reference to a genesis human ID */
  humanId: string;

  /** Strength of stewardship relationship (0-1) */
  affinity: number;

  /** What kind of stewardship */
  role: StewardshipRole;
}

/**
 * StewardshipRole - How this human relates to the content.
 */
export type StewardshipRole = 'author' | 'curator' | 'translator' | 'endorser' | 'steward';
```

Add to the ContentNode interface:

```typescript
  /** Who stewards this content, with graduated affinity */
  stewardedBy?: ContentSteward[];
```

**Step 3: Verify compilation**

Run: `cd /projects/elohim && pnpm exec tsc --noEmit --project elohim-app/tsconfig.json 2>&1 | grep content-node.model`
Expected: No new errors

**Step 4: Commit**

```bash
git add elohim-app/src/app/lamad/models/content-node.model.ts
git commit -m "feat(lamad): add ContentSteward type with graduated affinity

Stewardship is a property of content, not an administrative manifest.
Each content node can have multiple stewards with different affinity
weights and roles (author, curator, translator, endorser, steward).

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

### Task 2: Write stewardship annotation script for genesis content

**Files:**
- Create: `genesis/scripts/annotate-stewardship.py`

This script adds `stewardedBy` to the 3,525 genesis content JSON files based on tag-to-human mapping rules. It's idempotent — running it twice produces the same result.

**Step 1: Create the annotation script**

```python
#!/usr/bin/env python3
"""
Annotate genesis content with stewardedBy based on tag-to-human mapping.

Each human stewards content that matches their story:
- Matthew: governance, protocol core, family learning
- Susan: family curriculum, relationship content
- Pastor Pete: faith community, pastoral care
- Terrance: tutorials, mentorship, learning paths
- Frank: agriculture, supply chain, local economy

Content with no matching rule defaults to Matthew (founder, backwards compat).
Idempotent: overwrites existing stewardedBy on each run.

Usage: python3 genesis/scripts/annotate-stewardship.py [--dry-run]
"""

import json
import os
import sys
from pathlib import Path
from typing import TypedDict

class Steward(TypedDict):
    humanId: str
    affinity: float
    role: str

# ─── Stewardship Rules ─────────────────────────────────────────────────────
# Each rule: (tag_pattern, stewards)
# First matching rule wins. More specific rules first.
# A content node can match multiple rules — stewards accumulate.

STEWARDSHIP_RULES: list[tuple[set[str], list[Steward]]] = [
    # Assessments — Terrance (tutor) is primary, Susan (homeschool) curates
    ({"assessment"}, [
        {"humanId": "human-terrance-tutor", "affinity": 0.8, "role": "author"},
        {"humanId": "human-susan-partner", "affinity": 0.5, "role": "curator"},
    ]),

    # Faith/pastoral content — Pastor Pete primary
    ({"faith", "pastoral", "spiritual"}, [
        {"humanId": "human-pastor-pete-pastor", "affinity": 0.9, "role": "author"},
    ]),

    # Bible/scripture content — Pastor Pete stewards, Matthew endorses
    ({"fct"}, [
        {"humanId": "human-pastor-pete-pastor", "affinity": 0.7, "role": "steward"},
        {"humanId": "human-matthew-manager", "affinity": 0.3, "role": "endorser"},
    ]),

    # Governance — Matthew primary, Pete endorses community governance
    ({"governance"}, [
        {"humanId": "human-matthew-manager", "affinity": 0.8, "role": "author"},
        {"humanId": "human-pastor-pete-pastor", "affinity": 0.4, "role": "endorser"},
    ]),

    # Family-layer governance — Susan primary
    ({"governance_layer:family"}, [
        {"humanId": "human-susan-partner", "affinity": 0.8, "role": "author"},
        {"humanId": "human-matthew-manager", "affinity": 0.6, "role": "curator"},
    ]),

    # Economic/autonomous entity content — Frank (farmer, producer)
    ({"autonomous-entity", "economic-coordination"}, [
        {"humanId": "human-frank-farmer", "affinity": 0.7, "role": "author"},
        {"humanId": "human-matthew-manager", "affinity": 0.4, "role": "endorser"},
    ]),

    # Community/neighborhood content — Nancy (neighbor) + Pete
    ({"governance_layer:neighborhood"}, [
        {"humanId": "human-nancy-neighbor", "affinity": 0.6, "role": "steward"},
        {"humanId": "human-pastor-pete-pastor", "affinity": 0.4, "role": "endorser"},
    ]),

    # Learning paths, education — Terrance primary
    ({"learning", "tutorial", "path", "education"}, [
        {"humanId": "human-terrance-tutor", "affinity": 0.7, "role": "author"},
    ]),

    # Elohim agent content — Matthew (protocol architect)
    ({"elohim_agents:personal_agent"}, [
        {"humanId": "human-matthew-manager", "affinity": 0.9, "role": "author"},
    ]),

    # Value scanner scenarios — distributed across economy humans
    ({"value-scanner"}, [
        {"humanId": "human-frank-farmer", "affinity": 0.5, "role": "steward"},
        {"humanId": "human-georgina-grocer", "affinity": 0.4, "role": "steward"},
        {"humanId": "human-matthew-manager", "affinity": 0.3, "role": "endorser"},
    ]),
]

# Default steward when no rule matches
DEFAULT_STEWARD: list[Steward] = [
    {"humanId": "human-matthew-manager", "affinity": 1.0, "role": "author"},
]

CONTENT_DIR = Path(__file__).parent.parent / "data" / "lamad" / "content"


def match_stewards(tags: list[str]) -> list[Steward]:
    """Find stewards for a content node based on its tags."""
    tag_set = set(tags)
    matched_stewards: dict[str, Steward] = {}

    for required_tags, stewards in STEWARDSHIP_RULES:
        if required_tags & tag_set:  # any tag matches
            for s in stewards:
                # Keep highest affinity per human
                existing = matched_stewards.get(s["humanId"])
                if existing is None or s["affinity"] > existing["affinity"]:
                    matched_stewards[s["humanId"]] = s

    if not matched_stewards:
        return list(DEFAULT_STEWARD)

    # Sort by affinity descending
    return sorted(matched_stewards.values(), key=lambda s: -s["affinity"])


def annotate_file(filepath: Path, dry_run: bool = False) -> tuple[str, list[Steward]]:
    """Annotate a single content JSON with stewardedBy."""
    with open(filepath) as f:
        data = json.load(f)

    tags = data.get("tags", [])
    stewards = match_stewards(tags)
    data["stewardedBy"] = stewards

    if not dry_run:
        with open(filepath, "w") as f:
            json.dump(data, f, indent=2, ensure_ascii=False)
            f.write("\n")

    return data.get("id", filepath.stem), stewards


def main():
    dry_run = "--dry-run" in sys.argv

    if not CONTENT_DIR.exists():
        print(f"Content directory not found: {CONTENT_DIR}")
        sys.exit(1)

    # Exclude humans/ and graph/ subdirectories
    json_files = [
        f for f in CONTENT_DIR.glob("*.json")
        if f.is_file()
    ]

    print(f"{'DRY RUN: ' if dry_run else ''}Annotating {len(json_files)} content files...")

    steward_counts: dict[str, int] = {}
    for filepath in sorted(json_files):
        content_id, stewards = annotate_file(filepath, dry_run)
        for s in stewards:
            steward_counts[s["humanId"]] = steward_counts.get(s["humanId"], 0) + 1

    print(f"\nStewardship distribution:")
    for human_id, count in sorted(steward_counts.items(), key=lambda x: -x[1]):
        print(f"  {human_id}: {count} content nodes")

    print(f"\nTotal: {len(json_files)} files {'would be ' if dry_run else ''}annotated")


if __name__ == "__main__":
    main()
```

**Step 2: Run dry-run to verify distribution**

Run: `cd /projects/elohim && python3 genesis/scripts/annotate-stewardship.py --dry-run`
Expected: Stewardship distribution across 5+ humans, Matthew as largest share but not 100%

**Step 3: Run the actual annotation**

Run: `cd /projects/elohim && python3 genesis/scripts/annotate-stewardship.py`
Expected: 3,525 files annotated with stewardedBy

**Step 4: Spot-check a few files**

Run: `python3 -c "import json; d=json.load(open('genesis/data/lamad/content/assessment-attachment-style.json')); print(json.dumps(d.get('stewardedBy'), indent=2))"`
Expected: Terrance as author (0.8), Susan as curator (0.5)

Run: `python3 -c "import json; d=json.load(open('genesis/data/lamad/content/governance-organizations-solarpunk-readme.json')); print(json.dumps(d.get('stewardedBy'), indent=2))"`
Expected: Matthew as primary steward

**Step 5: Commit**

```bash
git add genesis/scripts/annotate-stewardship.py
git commit -m "feat(genesis): add stewardship annotation script for content nodes

Maps genesis content to human stewards based on tag rules.
Each steward has graduated affinity (0-1) and role.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

### Task 3: Run annotation and commit stewardship data

**Step 1: Run the annotation**

Run: `cd /projects/elohim && python3 genesis/scripts/annotate-stewardship.py`

**Step 2: Verify the distribution looks reasonable**

Run: `cd /projects/elohim && python3 -c "
import json, os, collections
d = 'genesis/data/lamad/content/'
counts = collections.Counter()
for f in os.listdir(d):
    if f.endswith('.json') and os.path.isfile(d+f):
        data = json.load(open(d+f))
        for s in data.get('stewardedBy', []):
            if s.get('affinity', 0) == max(st['affinity'] for st in data['stewardedBy']):
                counts[s['humanId']] += 1
                break
for h, c in counts.most_common():
    print(f'{h}: {c} (primary steward)')
"`
Expected: Distribution across multiple humans, no single human > 80%

**Step 3: Commit the annotated content**

```bash
git add genesis/data/lamad/content/*.json
git commit -m "data(genesis): annotate 3525 content nodes with stewardedBy

Distributed stewardship across genesis humans based on story roles.
Each content node has graduated affinity coefficients for proportional
REA value attribution.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

### Task 4: Add graduated scaling scenarios to persona-testnet-validation.feature

**Files:**
- Modify: `genesis/a2o/features/deployment/persona-testnet-validation.feature`

**Step 1: Read the current feature file**

The file already has 20-node scenarios. We're adding a new section for graduated 1→5 scaling with per-conductor content distribution. Add BEFORE the existing "Cluster Formation" section.

**Step 2: Add graduated scaling scenarios**

Insert after line 13 (after the Background block):

```gherkin
  # ─── Graduated Scaling (1→5 conductors) ──────────────────────────────────

  @scaling @compute
  Scenario: Step 1 — Single conductor seeds Matthew's stewardship content
    Given 1 conductor is running for "Matthew"
    And content is filtered by stewardedBy for "human-matthew-manager"
    When genesis content is seeded to Matthew's conductor
    Then Matthew's conductor has content where he is highest-affinity steward
    And baseline compute is measured
      | metric | unit        |
      | cpu    | millicores  |
      | memory | megabytes   |
      | time   | seconds     |
    And compute usage is recorded as shefa EconomicEvent

  @scaling @compute
  Scenario: Step 2 — Household replication between Matthew and Susan
    Given 2 conductors are running for "Matthew" and "Susan"
    And each conductor is seeded with its stewardship content
    When household replication activates via spouse relationship
    Then Susan can see Matthew's content at neighborhood reach or above
    And Susan cannot see Matthew's private-reach content
    And Matthew can see Susan's content at neighborhood reach or above
    And compute for 2 conductors is within budget
      | resource | budget | unit       |
      | cpu      | 2000   | millicores |
      | memory   | 4000   | megabytes  |

  @scaling @compute
  Scenario: Step 3 — Cross-cluster bridge to Pastor Pete
    Given 3 conductors are running for "Matthew", "Susan", and "Pastor Pete"
    And each conductor is seeded with its stewardship content
    When cross-cluster discovery activates via congregation_member relationship
    Then Pastor Pete can see community-reach content from Matthew's household
    And Pastor Pete cannot see family-private content
    And content replication follows trust topology not bulk access
    And compute for 3 conductors is within budget
      | resource | budget | unit       |
      | cpu      | 3000   | millicores |
      | memory   | 6000   | megabytes  |

  @scaling @compute
  Scenario: Step 5 — Five conductors with multi-hop content discovery
    Given 5 conductors are running for "Matthew", "Susan", "Pastor Pete", "Terrance", and "Frank"
    And each conductor is seeded with its stewardship content
    When all conductors have discovered their peers
    Then Terrance sees learning content via Susan's learning_partner bridge
    And Frank's economy content is NOT directly visible to Matthew
    And content discovery paths match the relationship graph
    And compute for 5 conductors is within budget
      | resource | budget | unit       |
      | cpu      | 5000   | millicores |
      | memory   | 10000  | megabytes  |
    And per-conductor compute is recorded in shefa vocabulary
```

**Step 3: Verify feature file syntax**

Run: `cd /projects/elohim && npx gherkin-lint genesis/a2o/features/deployment/persona-testnet-validation.feature 2>&1 || echo "No linter — check manually"`

**Step 4: Commit**

```bash
git add genesis/a2o/features/deployment/persona-testnet-validation.feature
git commit -m "feat(a2o): add graduated 1→5 conductor scaling scenarios

Each step tests content distribution by stewardship, replication by
trust topology, and compute budget assertions in shefa vocabulary.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

### Task 5: Update k8s edgenode manifest — relax nodeSelector

**Files:**
- Modify: `genesis/orchestrator/manifests/edgenode/alpha.yaml:104-105`

**Step 1: Read the current nodeSelector**

Run: `grep -n -A1 "nodeSelector" genesis/orchestrator/manifests/edgenode/alpha.yaml`
Expected: `nodeSelector: node-type: operations` at line 104-105

**Step 2: Replace nodeSelector with nodeAffinity preference**

Replace the `nodeSelector` block (lines 104-105):

```yaml
      nodeSelector:
        node-type: operations
```

With a soft affinity that prefers operations nodes but allows scheduling elsewhere:

```yaml
      affinity:
        nodeAffinity:
          preferredDuringSchedulingIgnoredDuringExecution:
            - weight: 80
              preference:
                matchExpressions:
                  - key: node-type
                    operator: In
                    values:
                      - operations
            - weight: 60
              preference:
                matchExpressions:
                  - key: node-type
                    operator: In
                    values:
                      - compute
```

This keeps existing behavior (prefers intel-nuc) but allows k8s to schedule on ethosengine or other nodes when intel-nuc is overcommitted.

**Step 3: Verify YAML syntax**

Run: `python3 -c "import yaml; yaml.safe_load(open('genesis/orchestrator/manifests/edgenode/alpha.yaml'))" && echo "Valid YAML"`
Expected: Valid YAML

**Step 4: Commit**

```bash
git add genesis/orchestrator/manifests/edgenode/alpha.yaml
git commit -m "infra(alpha): relax edgenode nodeSelector to soft affinity

intel-nuc is at 248% CPU overcommit. Changing from hard nodeSelector
(node-type: operations) to preferred affinity allows k8s to schedule
on ethosengine (18+ idle cores) when intel-nuc is saturated.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

### Task 6: Add stewardship-aware seeding to doorway-client

**Files:**
- Modify: `genesis/seeder/src/seed.ts`
- Modify: `genesis/seeder/src/doorway-client.ts`

**Step 1: Read the current seedViaDoorway function signature**

Run: `grep -n "seedViaDoorway\|bulkCreateContent" genesis/seeder/src/seed.ts | head -10`

**Step 2: Add stewardship filter function to seed.ts**

Add a utility function that filters content by conductor's human ID:

```typescript
/**
 * Filter content nodes to those stewarded by a specific human.
 * Returns content where the given humanId is the highest-affinity steward.
 * If no stewardedBy field exists, defaults to the operator (backwards compat).
 */
function filterBySteward(
  content: ContentItem[],
  humanId: string,
  operatorId: string = 'human-matthew-manager',
): ContentItem[] {
  return content.filter((item) => {
    const stewards = (item as Record<string, unknown>).stewardedBy as
      | Array<{ humanId: string; affinity: number }>
      | undefined;

    if (!stewards || stewards.length === 0) {
      // No stewardship annotation — default to operator
      return humanId === operatorId;
    }

    // Find highest-affinity steward
    const primary = stewards.reduce((max, s) => (s.affinity > max.affinity ? s : max), stewards[0]);
    return primary.humanId === humanId;
  });
}
```

**Step 3: Add --conductor-for CLI flag**

In the CLI argument parsing section of seed.ts, add:

```typescript
// After existing arg parsing
const conductorFor = args.find((a) => a.startsWith('--conductor-for='))?.split('=')[1];
```

**Step 4: Wire the filter into the seeding flow**

In the `seedViaDoorway` function, before the bulk create call, add content filtering:

```typescript
// Before bulkCreateContent call
let contentToSeed = allContent;
if (conductorFor) {
  contentToSeed = filterBySteward(allContent, conductorFor);
  console.log(`[stewardship] Filtered to ${contentToSeed.length}/${allContent.length} content nodes for ${conductorFor}`);
}
```

**Step 5: Test with dry run**

Run: `cd /projects/elohim/genesis/seeder && npx tsx src/seed.ts --dry-run --conductor-for=human-terrance-tutor 2>&1 | grep stewardship`
Expected: Shows filtered count (Terrance should get assessment + tutorial content)

**Step 6: Commit**

```bash
git add genesis/seeder/src/seed.ts
git commit -m "feat(seeder): add --conductor-for flag for per-human content routing

When seeding to a specific conductor, filters content to nodes where
the given human is highest-affinity steward. Backwards compatible —
without the flag, seeds all content as before.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Summary

| Task | What | Files | Est. |
|------|------|-------|------|
| 1 | ContentSteward type | `content-node.model.ts` | 3 min |
| 2 | Annotation script | `annotate-stewardship.py` (new) | 10 min |
| 3 | Run annotation + commit data | `content/*.json` (3525 files) | 5 min |
| 4 | Graduated scaling a2o scenarios | `persona-testnet-validation.feature` | 5 min |
| 5 | Relax k8s nodeSelector | `alpha.yaml` | 3 min |
| 6 | Stewardship-aware seeder | `seed.ts` | 10 min |

**Total: ~36 minutes, 6 commits**

## What This Enables Next

With these changes landed:
1. **Run the 1→5 scale test** — seed Matthew's conductor, add Susan, measure, add Pete, measure
2. **Visible compute constraints** — shefa budget assertions in a2o, not hidden in kubectl
3. **Protocol-native distribution** — content reaches humans through trust, not admin access
4. **Foundation for dynamic stewardship** — once explicit works, affinity-based discovery layers on
5. **REA value attribution** — stewardship affinity feeds proportional economic events
