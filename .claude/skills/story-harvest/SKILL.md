---
name: story-harvest
description: Use when finishing a development branch, after resolving a debugging session, or whenever a failure-then-fix cycle reveals engineering constraints worth preserving as a2o scenarios. Also invocable manually with /story-harvest.
---

# Story Harvest

## Overview

Capture engineering discoveries as a2o scenarios before context fades.

When a bridge engineer tests a design to failure, the failure reveals a constraint — a load limit, a material property, an environmental factor. They document that constraint as a specification. Future bridges are designed and tested against it. The specification is replicable: anyone can test against it.

This skill applies the same loop to software: **bug -> design -> proven capability -> scenario**.

## When This Fires

- **Finishing a branch**: Between "tests pass" and "present options" in the finishing-a-development-branch workflow.
- **After debugging**: When systematic-debugging resolves a root cause and the fix is verified.
- **Manual**: `/story-harvest` at any time.

## The Discovery Loop

```
Failure observed
    |
    v
Constraint revealed ------> specific parameters
    |                        (memory, concurrency, cache size,
    v                         timeout, peer count...)
Design addresses it
    |
    v
Scenario captures it -----> replicable proof the constraint is handled
    |
    v
Future work inherits the guard rail
```

## Process

### Step 1: Gather Context

Review what happened on this branch or debugging session:

```bash
# Branch work
git log --oneline $(git merge-base HEAD dev)..HEAD
git diff --stat $(git merge-base HEAD dev)..HEAD

# Check for captured intent
cat .claude/data/dev-intent.jsonl | tail -5
```

Also consider: what was the user working through in conversation? What broke? What surprised them?

### Step 2: Identify Discoveries

Ask the user (or reason from context):

1. **What failed, or what did you test to its limits?**
   Not "what bug did you fix" but "what constraint did the failure reveal?"

2. **At what specific values did it fail?**
   The parameters matter. "Storage broke" is noise. "Storage with 256MB container limit receiving 30 concurrent requests for a 40-file HTML5 app triggers OOM" is engineering knowledge. Those specific values — 256MB, 30 concurrent, 40 files — become the parameters that define peer diversity presets and operator recommendations.

3. **How does the system now handle that constraint?**
   The design that addresses it. This becomes the capability proof scenario.

4. **What did you need to observe to diagnose it?**
   The diagnostic capability that enabled the discovery. If you couldn't see it, you couldn't design for it.

### Step 3: Assess Value

Not every change yields a story worth capturing. The highest-value discoveries share traits:

**Parameter-bearing stories** are the most valuable. When a discovery carries specific operational values (memory limits, concurrency thresholds, cache sizes, timeout windows, peer counts), those values do double duty: they prove the constraint in the scenario AND they inform operator presets, documentation, and recommended configurations after major releases. A story about "storage with X allocated memory under Y concurrent requests" creates parameters for the diversity of peers — it's how a laptop preset (200MB cache) differs from a home node (2GB) from a network node (10GB).

**Constraint boundaries** — stories that define where one behavior transitions to another — are worth capturing even without exact numbers, because they mark the territory for future measurement.

**Observability stories** — "an operator can see/control X" — preserve the diagnostic capability that made the discovery possible in the first place.

**Skip** stories that are just "feature X works as designed" with no constraint, boundary, or parameter insight. Those are covered by normal acceptance tests.

### Step 4: Classify and Scaffold

Each discovery maps to one of three scenario types:

**Failure Regression** (`@regression`)
"Without X, Y breaks." The original failure preserved as a regression anchor. If the failure ever stops being reproducible, the architecture should be revisited — the constraint may have moved, not disappeared.

```gherkin
@wip @regression
Scenario: Without projection cache, browser load overwhelms storage
  Given the doorway projection cache is disabled
  And elohim-storage is allocated 256MB memory
  When 10 browsers simultaneously load "evolution-of-trust" (40 files each)
  Then elohim-storage receives 300+ concurrent requests
  And some requests fail or storage resource usage spikes dangerously
  # Constraint: P2P storage node with 256MB cannot absorb CDN-pattern traffic.
  # These parameters inform the home_node preset (2GB cache minimum).
```

**Capability Proof** (`@wip`, becomes `@regression` when wired)
"With X enabled, the system handles Y." The bridge that holds.

```gherkin
@wip
Scenario: With projection cache enabled, same load is absorbed
  Given the doorway projection cache is enabled and warm
  When 10 browsers simultaneously load "evolution-of-trust"
  Then zero requests reach elohim-storage
  And all browsers receive complete responses
```

**Observability Gate** (`@wip`)
"An operator can see/control Z." The diagnostic capability that enabled the discovery.

```gherkin
@wip
Scenario: Response headers indicate which cache layer served the request
  When Matthew requests a file from "evolution-of-trust"
  Then the response includes a header indicating the serving layer
```

### Step 5: Place Scenarios

Identify the right feature file:
- Check existing files under `genesis/a2o/features/` for topical fit
- If no file fits, suggest a new file with the appropriate directory and tags
- Follow conventions: `@e2e @{domain}` tags, `@wip` on all new scenarios, named personas, doorway background

Append scenario outlines to the identified file, or present them to the user for placement.

### Step 6: Note Operational Parameters

If the discovery carried specific parameter values, note them explicitly in a comment on the scenario:

```gherkin
# Operational parameters: 256MB memory, 30 concurrent browsers, 40 files/app
# Informs: NodeCapabilities presets (cache_budget_bytes), operator documentation
# Review after: major releases, container sizing changes
```

These comments are the raw material for operator guides and preset updates. They connect the scenario back to the configuration decisions it informs.

## Output

For each discovery, produce:
1. The scenario skeleton (Given/When/Then with specific parameters)
2. Classification tag (`@regression` or `@wip`)
3. A one-line constraint statement as a comment
4. Operational parameter notes where applicable
5. Suggested file placement

If no discoveries are worth capturing, say so. Not every branch reveals a constraint. The skill is advisory, not a gate.

## Integration

This skill is invoked by:
- **finishing-a-development-branch** — between Step 1 (tests pass) and Step 3 (present options)
- **systematic-debugging** — after root cause is identified and fix is verified
- **Manual** — `/story-harvest`

## What This Is NOT

- Not a full Gherkin authoring tool (step definitions come later)
- Not a gate that blocks finishing (advisory only)
- Not a substitute for writing scenarios before implementation (story-first is still the default)
- Not for capturing "feature X works" acceptance tests (those come from the story-first loop)

This skill captures what the story-first loop misses: the **constraints discovered during implementation** that weren't in the original specification.
