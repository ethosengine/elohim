# P2P Design Enforcement — Shift-Left Architecture Guards

**Date:** 2026-03-16
**Status:** Approved
**Problem:** AI agents (and developers generally) default to relational-DB patterns when designing new features. By the time anyone notices, the design is already framed around REST routes and SQL tables instead of DHT entry types and content addressing. Rework is expensive.

**Evidence:** A scheduling feature design conversation produced three REST route options (A/B/C) before anyone asked "what's the DHT entry type?" The CID was proposed as a column value in a relational table rather than the entity's identity. This pattern recurs because the codebase's working examples are relational-first.

## Design

Four enforcement layers, ordered by when they fire in the development lifecycle. Layer 0 does 90% of the work; layers 1-3 are defense-in-depth.

### Layer 0: CLAUDE.md Hard Rule

A mandatory instruction in root `CLAUDE.md` that compels invocation of the `p2p-design-gate` skill during brainstorming step 3 (before proposing design approaches). CLAUDE.md is always loaded into conversation context and cannot be skipped.

**Why this is Layer 0:** The brainstorming plugin skill cannot be modified (it's a third-party plugin). The superpowers system checks for applicable skills on user message arrival, but doesn't re-check mid-skill. The CLAUDE.md instruction bridges this gap — it tells the agent "you are inside brainstorming, and before you propose approaches, you must invoke this skill."

### Layer 1: P2P Design Gate Skill

A project-level Claude Code skill at `.claude/skills/p2p-design-gate/SKILL.md` containing:

1. **Entity classification decision tree** — forces the agent to categorize every new data entity before proposing designs:
   - **Notarized** (content, economic events, attestations, relationships) → DHT entry type required, `dht_anchor_hash NOT NULL` in storage, source of truth is Holochain
   - **Agent-scoped** (preferences, schedules, session state) → Private source-chain entry, linked to content by EntryHash, storage projection for fast query only
   - **Operational** (cache, projections, temp state) → SQLite-only acceptable, must document why no notarization

2. **Content address strategy** — forces a decision on entity identity:
   - Content-derived (CID-based ID)
   - Agent-scoped composite (agent + content + type tuple)
   - Arbitrary slug/UUID (must justify)

3. **API design order** — enforces the correct sequence:
   - What Holochain coordinator function creates/reads this?
   - What post-commit signal projects it to storage?
   - What HTTP route exposes the projection? (last question, not first)

4. **Anti-pattern catalog** — specific regressions already caught, with P2P-native alternatives:
   - UUID primary key for notarized entity → EntryHash IS the identity
   - REST route as design starting point → Start with DHT entry type
   - CID bolted onto relational FK → Entity IS content-addressed
   - Standalone table for agent-scoped state → Link from agent to content entry

### Layer 2: Plan Document Hook

A `PostToolUse` hook (`p2p-plan-audit.py`) that fires on Edit/Write to `genesis/plans/*.md`.

**Scan strategy:** Pattern + antidote. For each red-flag pattern, check whether an "antidote" pattern exists within ~10 lines. If flag found without antidote, emit informational warning.

| Red Flag | Antidote (nearby) | Warning |
|---|---|---|
| `PRIMARY KEY` | `dht_anchor_hash` | New table needs DHT anchor for notarized entities |
| `UUID` | `source chain`, `DHT`, `EntryHash` | UUID without P2P anchoring discussion |
| `API endpoint`, `REST route`, `GET /`, `POST /` | `entry type`, `coordinator function`, `zome` | Route designed without Holochain entry type |
| `CREATE TABLE`, `new table`, `schema` | `source of truth`, `DHT`, `projection`, `operational` | Schema without source-of-truth declaration |
| `entity_id.*CID`, `store.*CID.*as` | `content address`, `EntryHash` | CID as FK — should entity BE content-addressed? |

**Non-blocking.** Emits `additionalContext` warnings. Agent sees the warning and adds the missing section to the plan.

### Layer 3: Schema/Migration Hook

A `PostToolUse` hook (`p2p-schema-audit.py`) that fires on Edit/Write to Rust files matching:

- `*/migrations/**/up.sql` — New diesel migrations
- `*/db/models.rs` — Storage model structs
- `*/views.rs` — API boundary types
- `*/routes/*.rs` — Route handlers

**Checks by file type:**

**Migrations:** `CREATE TABLE` for notarized entity patterns without `dht_anchor_hash` column.

**Models:** New `pub struct` with `id: String` without corresponding Holochain entry type.

**Views:** New View struct exposing `id` without `dht_anchor_hash` field.

**Routes:** New handler function without reference to coordinator function or projection pattern.

**Non-blocking.** Last line of defense — by this point, the plan should have addressed these concerns. If the warning fires, it means either the plan was incomplete or the implementation drifted from it.

## Files Created/Modified

| File | Action | Purpose |
|---|---|---|
| `CLAUDE.md` | Modify | Add P2P Design Gate section with hard invocation rule |
| `.claude/skills/p2p-design-gate/SKILL.md` | Create | Decision tree, anti-pattern catalog, entity classification |
| `.claude/hooks/p2p-plan-audit.py` | Create | Plan document red-flag scanner |
| `.claude/hooks/p2p-schema-audit.py` | Create | Migration/model/route/view scanner |
| `.claude/settings.json` | Modify | Register both new hooks in PostToolUse array |

## What This Does NOT Do

- Does not modify the brainstorming plugin (third-party, read-only)
- Does not add ESLint or Clippy rules (those are syntax-level; this is architecture-level)
- Does not add pre-commit hooks (these fire during authoring, which is earlier)
- Does not block code writes (hooks are informational, not gates)
- Does not enforce retroactively on existing tables/models (only on new writes)

## Success Criteria

The scheduling conversation, replayed with this system in place, would go:

1. User asks for scheduling feature
2. Brainstorming skill invoked
3. CLAUDE.md rule triggers `p2p-design-gate` before step 3
4. Agent completes decision tree: "Schedule is agent-scoped state → private source-chain entry → linked to content by EntryHash → storage projection for query"
5. Proposed approaches all start from this P2P framing
6. REST route is the last design decision, shaped by the entry type
7. Plan document passes Layer 2 hook cleanly
8. Implementation passes Layer 3 hook cleanly
