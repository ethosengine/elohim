---
name: generalize-permissions
description: Cluster near-duplicate entries in .claude/settings.json + settings.local.json allow list into broader patterns under a safety taxonomy. Propose bulk collapses (10 entries → 1 pattern), user approves per-cluster. Run standalone when the allowlist is getting bloated.
---

# Generalize Permissions

Reduce Claude Code permission-prompt pain by replacing many literal
allowlist entries with fewer broader patterns. Safety is preserved via
the taxonomy in `genesis/agentic/data/safety-taxonomy.json`.

## When to invoke

Run `/generalize-permissions` when the allowlist is large or you've
been approving near-duplicate entries frequently.

## Procedure

1. **Load current allowlists.**

   Read:
   - `.claude/settings.json` (committed, durable — the permanent palette)
   - `.claude/settings.local.json` (local, shift-scoped additions)

2. **Run the generalization algorithm.**

   ```bash
   node --input-type=module -e "
     import('./genesis/agentic/generalize.mjs').then(async ({ clusterAndPropose }) => {
       const { readFileSync, existsSync } = await import('node:fs');
       const tax = JSON.parse(readFileSync('genesis/agentic/data/safety-taxonomy.json', 'utf8'));
       const durable = JSON.parse(readFileSync('.claude/settings.json', 'utf8')).permissions?.allow ?? [];
       const local = existsSync('.claude/settings.local.json')
         ? JSON.parse(readFileSync('.claude/settings.local.json', 'utf8')).permissions?.allow ?? []
         : [];
       const all = [...durable, ...local];
       const proposals = clusterAndPropose(all, tax);
       console.log(JSON.stringify(proposals, null, 2));
     });
   "
   ```

3. **Present each proposal to the user for review.**

   Group by safety tier. For each cluster, show:
   - Proposed pattern
   - The N literal entries it would replace
   - Safety classification

   Ask: *"Apply this generalization? [y/n/skip]"* per cluster. Bulk-approve
   groups where the user says "all broadly-safe proposals".

4. **Apply approved proposals.**

   - Remove the absorbed literal entries from whichever file they came from.
   - Add the single generalized pattern to `.claude/settings.json` (durable),
     unless it originated solely from `settings.local.json` shift-scoped
     entries, in which case it stays local and tagged.

5. **Summarize and return.**

   Print: `"<N> clusters applied, <M> literal entries collapsed to <P>
   patterns. Allowlist is now <X> entries (was <Y>)."`

## Safety invariants

- NEVER apply a generalization without explicit user approval (blanket
  "all broadly-safe" approval is fine; silent apply is not).
- NEVER generalize across the `never_wildcard` list in the taxonomy.
- NEVER promote a shift-scoped local entry to durable without the user
  explicitly approving the promotion.
- Back up the settings files (copy to `.bak` with timestamp) before
  writing modifications. If anything goes wrong, restore.

## Exit criteria

Allowlist has fewer entries than when you started, every removal is
justified by an approved generalization, and no novel patterns were
introduced outside the safety taxonomy.
