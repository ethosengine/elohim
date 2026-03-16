# P2P Design Enforcement Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Prevent relational-DB design drift by enforcing P2P-native thinking at brainstorming, plan-writing, and code-authoring stages.

**Architecture:** Four-layer enforcement: CLAUDE.md hard rule (Layer 0) compels invocation of a P2P design gate skill (Layer 1) during brainstorming. A plan-document hook (Layer 2) scans written plans for anti-patterns. A schema hook (Layer 3) catches migration/model regressions at code time.

**Tech Stack:** Claude Code skills (markdown), Python hooks (PostToolUse), JSON settings

---

### Task 1: Create the P2P Design Gate Skill

**Files:**
- Create: `.claude/skills/p2p-design-gate/SKILL.md`

**Step 1: Create the skill directory and file**

Write `.claude/skills/p2p-design-gate/SKILL.md` with this content:

```markdown
---
name: p2p-design-gate
description: Mandatory gate for any feature design involving data entities (tables, models, routes, sync messages). Forces P2P-native thinking — DHT entry types, content addressing, source-of-truth classification — before proposing design approaches. Use when brainstorming any feature that creates, stores, references, or syncs data entities.
metadata:
  author: elohim-protocol
  version: 1.0.0
---

# P2P Design Gate

This skill is MANDATORY during brainstorming before proposing design approaches for any feature involving data entities. It prevents relational-DB drift by forcing P2P-native framing first.

## When This Fires

During brainstorming, between step 2 (clarifying questions) and step 3 (propose approaches), when the feature involves ANY of:
- New database tables or migrations
- New model structs (Rust or TypeScript)
- New API routes or endpoints
- New sync protocol messages
- New entity types or references between entities

## The Decision Tree

Complete ALL applicable sections before proposing design approaches. Present your answers to the user for validation.

### 1. Entity Classification

For each new data entity in the feature, classify it:

**Notarized** — Truth lives on Holochain DHT, storage is a projection:
- Content, economic events, attestations, relationships, commitments
- REQUIRES: Holochain entry type in integrity zome
- REQUIRES: `dht_anchor_hash NOT NULL` in any storage table
- REQUIRES: Post-commit signal → storage projection flow
- Source of truth: DHT. Storage is a fast index.

**Agent-Scoped** — Private to one human, not shared on DHT:
- Preferences, schedules, session state, personal progress
- REQUIRES: Private source-chain entry (not public DHT)
- REQUIRES: Link from agent pubkey to content EntryHash
- Storage projection for fast query only
- Never synced to other peers as standalone entity

**Operational** — No P2P identity needed:
- Cache entries, temp state, projections, build artifacts
- SQLite-only is acceptable
- MUST document why this doesn't need notarization
- If you can't articulate why, it's probably notarized or agent-scoped

### 2. Content Address Strategy

For each entity, how is it identified?

**Content-derived (CID)** — Identity IS a hash of the content:
- Immutable content, blobs, EPR heads
- Use CIDv1 (`bafkrei...`) as canonical format
- Slug is an alias resolved through EPR, not the primary key

**Agent-scoped composite** — Identity is (agent + content + type) tuple:
- The entity doesn't have standalone identity
- It's a relationship between an agent and content
- Example: schedule = (agent, content_entry_hash, schedule_type)

**Slug/UUID** — Arbitrary human-readable identifier:
- MUST justify why content addressing doesn't apply
- Acceptable for: app-scoped config, operational tables
- NOT acceptable for: anything that syncs between peers

### 3. API Design Order

Answer these IN ORDER. Do not skip to HTTP routes.

1. **What Holochain coordinator function creates/reads this?**
   - If notarized: define the zome function signature
   - If agent-scoped: define the private entry + link pattern
   - If operational: skip to step 3

2. **What post-commit signal projects it to storage?**
   - Signal type name and payload
   - Storage upsert handler
   - Reconciliation strategy if projection diverges from DHT

3. **What HTTP route exposes the projection?**
   - This is the LAST question, not the first
   - Route structure follows from the entry type, not the other way around

### 4. Anti-Pattern Check

Before presenting your design, verify it doesn't match these known regressions:

| Anti-Pattern | Symptom | P2P-Native Alternative |
|---|---|---|
| UUID primary key for notarized entity | `id TEXT PRIMARY KEY` without DHT anchor | EntryHash IS the identity; project with `dht_anchor_hash NOT NULL` |
| REST route as starting point | "Option A: `/api/v1/thing`" as first design decision | Start with DHT entry type; route is an afterthought |
| CID as relational foreign key | "store CID in entity_id column" | Entity IS content-addressed; CID is identity, not metadata |
| Standalone table for agent state | `CREATE TABLE schedules (id TEXT PK)` | Private source-chain entry linked to content by EntryHash |
| Three address formats undefined | Plan mentions "id" without specifying CID vs slug vs hash | Declare which address format and why |
| Missing source-of-truth declaration | New table with no discussion of DHT vs storage authority | Every table declares: notarized (DHT+projection), agent-scoped (chain+projection), or operational |

## Output Format

Present your completed decision tree to the user as:

```
P2P DESIGN GATE — [Feature Name]

Entity: [name]
  Classification: notarized | agent-scoped | operational
  Identity: CID | composite (agent + content + type) | slug (justified)
  DHT entry type: [name] or N/A
  Source of truth: DHT | source-chain | SQLite-only
  Storage projection: [table name] or N/A
  HTTP route: [route] (serves projection of above)

Anti-pattern check: PASS | [specific concern]
```

Then proceed with brainstorming step 3 (propose approaches), ensuring all approaches honor the classifications above.
```

**Step 2: Verify skill appears in skill list**

Run: `ls -la .claude/skills/p2p-design-gate/`
Expected: `SKILL.md` exists

**Step 3: Commit**

```bash
git add .claude/skills/p2p-design-gate/SKILL.md
git commit -m "feat: add p2p-design-gate skill — forces P2P-native design thinking during brainstorming"
```

---

### Task 2: Add CLAUDE.md Hard Rule (Layer 0)

**Files:**
- Modify: `CLAUDE.md:132-152` (Development Workflow section)

**Step 1: Add P2P Design Gate section after Story-First Default**

Insert after the "Exploration Fallback" section (line 152), before "Critical Gotchas" (line 154):

```markdown

### P2P Design Gate (MANDATORY)

Before proposing design approaches for ANY feature involving data entities (tables, models, routes, sync messages), invoke the `p2p-design-gate` skill. This gates brainstorming step 3 — no approaches may be proposed until the skill's decision tree is completed and the user has validated the entity classifications.

**This rule exists because** AI agents default to relational-DB patterns (UUID primary keys, REST-first design, CID-as-column). The protocol requires P2P-native thinking: DHT entry types first, content addressing for identity, storage as projection not truth.

**The skill forces you to answer:**
1. Is this entity notarized (DHT), agent-scoped (source-chain), or operational (SQLite-only)?
2. Is identity content-derived (CID), agent-composite, or slug (must justify)?
3. What coordinator function creates it? What signal projects it? (Answer BEFORE designing the HTTP route.)

**If you're about to write "Option A: `GET /api/v1/thing`" without having answered these questions, STOP and invoke the skill.**
```

**Step 2: Verify CLAUDE.md is valid**

Run: `head -n 170 CLAUDE.md | tail -n 25`
Expected: New section visible between Exploration Fallback and Critical Gotchas

**Step 3: Commit**

```bash
git add CLAUDE.md
git commit -m "docs: add P2P Design Gate rule to CLAUDE.md — mandatory skill invocation during brainstorming"
```

---

### Task 3: Create Plan Document Hook (Layer 2)

**Files:**
- Create: `.claude/hooks/p2p-plan-audit.py`

**Step 1: Write the hook**

Create `.claude/hooks/p2p-plan-audit.py` following the same pattern as `epr-link-check.py`:

```python
#!/usr/bin/env python3
"""
Post-edit P2P design audit hook for Claude Code.
Scans plan documents (genesis/plans/*.md) for relational-DB anti-patterns
that suggest P2P-native design thinking was skipped.

Non-blocking: emits informational warnings via additionalContext.

Hook Type: PostToolUse
Matcher: Edit|Write
"""

import json
import os
import re
import sys


# Each rule: (flag_pattern, antidote_pattern, warning_message)
# Flag fires if flag_pattern found without antidote_pattern within 10 lines
RULES = [
    (
        r'(?i)PRIMARY\s+KEY',
        r'(?i)dht_anchor_hash|anchor.hash|DHT.anchor',
        'PRIMARY KEY without dht_anchor_hash — notarized entities need a DHT anchor column',
    ),
    (
        r'(?i)\bUUID\b',
        r'(?i)source.chain|DHT|EntryHash|operational|content.address',
        'UUID identity without P2P anchoring discussion — should this be content-addressed?',
    ),
    (
        r'(?i)(?:API\s+endpoint|REST\s+route|GET\s+/|POST\s+/|PUT\s+/|DELETE\s+/)',
        r'(?i)entry.type|coordinator.function|zome|DHT|source.chain',
        'API route without entry type — routes should follow from DHT design, not precede it',
    ),
    (
        r'(?i)(?:CREATE\s+TABLE|new\s+table|add\s+(?:a\s+)?table|schema\b)',
        r'(?i)source.of.truth|DHT|projection|operational|notarized|agent.scoped',
        'New storage schema without source-of-truth declaration',
    ),
    (
        r'(?i)entity_id.*(?:CID|cid|bafk)|store.*(?:CID|cid).*(?:as|in)',
        r'(?i)content.address(?:ed)?|EntryHash|identity.IS',
        'CID used as foreign key — should the entity BE content-addressed instead?',
    ),
]

ANTIDOTE_WINDOW = 10  # lines before/after to search for antidote


def scan_plan(lines: list[str]) -> list[str]:
    """Scan plan lines for P2P anti-patterns."""
    warnings = []

    for i, line in enumerate(lines):
        for flag_re, antidote_re, message in RULES:
            if not re.search(flag_re, line):
                continue

            # Check window around the flag for the antidote
            window_start = max(0, i - ANTIDOTE_WINDOW)
            window_end = min(len(lines), i + ANTIDOTE_WINDOW + 1)
            window = ' '.join(lines[window_start:window_end])

            if not re.search(antidote_re, window):
                warnings.append(f'  L{i + 1}: {message}')

    # Deduplicate (same warning can fire on adjacent lines)
    return list(dict.fromkeys(warnings))


def main():
    try:
        data = json.load(sys.stdin)

        tool_input = data.get('tool_input', {})
        file_path = tool_input.get('file_path', '')

        if not file_path:
            sys.exit(0)

        # Only audit plan documents
        if '/plans/' not in file_path or not file_path.endswith('.md'):
            sys.exit(0)

        try:
            with open(file_path) as f:
                lines = f.readlines()
        except (FileNotFoundError, PermissionError):
            sys.exit(0)

        warnings = scan_plan(lines)

        if warnings:
            basename = os.path.basename(file_path)
            msg = (
                f'[P2P DESIGN AUDIT] {basename}:\n'
                + '\n'.join(warnings[:10])
                + '\n  Reference: .claude/skills/p2p-design-gate/SKILL.md'
            )
            result = {
                'hookSpecificOutput': {
                    'hookEventName': 'PostToolUse',
                    'additionalContext': msg,
                }
            }
            print(json.dumps(result))

        sys.exit(0)

    except json.JSONDecodeError:
        sys.exit(0)
    except Exception as e:
        print(f'p2p-plan-audit hook error: {e}', file=sys.stderr)
        sys.exit(1)


if __name__ == '__main__':
    main()
```

**Step 2: Make executable**

Run: `chmod +x .claude/hooks/p2p-plan-audit.py`

**Step 3: Commit**

```bash
git add .claude/hooks/p2p-plan-audit.py
git commit -m "feat: add p2p-plan-audit hook — scans plan docs for relational anti-patterns"
```

---

### Task 4: Create Schema/Migration Hook (Layer 3)

**Files:**
- Create: `.claude/hooks/p2p-schema-audit.py`

**Step 1: Write the hook**

Create `.claude/hooks/p2p-schema-audit.py`:

```python
#!/usr/bin/env python3
"""
Post-edit P2P schema audit hook for Claude Code.
Scans Rust migrations, models, views, and routes for P2P anti-patterns.

Non-blocking: emits informational warnings via additionalContext.

Hook Type: PostToolUse
Matcher: Edit|Write
"""

import json
import os
import re
import sys
from fnmatch import fnmatch

# Paths this hook cares about
AUDIT_PATTERNS = [
    '*/migrations/**/up.sql',
    '*/migrations/**/down.sql',
    '*/db/models.rs',
    '*/views.rs',
    '*/routes/*.rs',
]

# Entity name patterns that should be notarized (DHT-backed)
NOTARIZED_ENTITIES = [
    'content', 'economic_event', 'rea_commitment', 'human',
    'human_relationship', 'attestation', 'learning_path',
    'stewardship', 'agreement', 'economic_resource',
]


def matches_audit_path(relative_path: str) -> str | None:
    """Check if file matches any audit pattern. Returns the type."""
    for pattern in AUDIT_PATTERNS:
        if fnmatch(relative_path, pattern):
            if 'migrations' in relative_path:
                return 'migration'
            if 'models.rs' in relative_path:
                return 'model'
            if 'views.rs' in relative_path:
                return 'view'
            if 'routes/' in relative_path:
                return 'route'
    return None


def audit_migration(lines: list[str]) -> list[str]:
    """Audit SQL migration for P2P anti-patterns."""
    warnings = []
    full_text = ''.join(lines).lower()

    # Find CREATE TABLE statements
    for m in re.finditer(
        r'create\s+table\s+(?:if\s+not\s+exists\s+)?(\w+)',
        full_text,
    ):
        table_name = m.group(1)

        # Check if this looks like a notarized entity
        is_notarized = any(entity in table_name for entity in NOTARIZED_ENTITIES)

        if is_notarized and 'dht_anchor_hash' not in full_text:
            line_num = full_text[: m.start()].count('\n') + 1
            warnings.append(
                f'  L{line_num}: CREATE TABLE {table_name} — notarized entity pattern '
                f'without dht_anchor_hash column. Add dht_anchor_hash TEXT NOT NULL '
                f'or document why this is operational-only.'
            )

        # Check for entity_id without content addressing context
        if 'entity_id' in full_text and 'cid' not in full_text and 'hash' not in full_text:
            warnings.append(
                f'  entity_id column without CID/hash reference — '
                f'should entity_id be a content address?'
            )

    return warnings


def audit_model(lines: list[str]) -> list[str]:
    """Audit Rust model structs for P2P anti-patterns."""
    warnings = []

    for i, line in enumerate(lines):
        # New pub struct with id: String
        if re.search(r'pub\s+struct\s+(\w+)', line):
            struct_name_match = re.search(r'pub\s+struct\s+(\w+)', line)
            struct_name = struct_name_match.group(1) if struct_name_match else 'Unknown'

            # Look ahead for id field and dht_anchor_hash
            lookahead = ''.join(lines[i : min(len(lines), i + 30)])
            has_id = bool(re.search(r'pub\s+id\s*:', lookahead))
            has_anchor = 'dht_anchor_hash' in lookahead

            if has_id and not has_anchor:
                is_notarized = any(
                    entity in struct_name.lower()
                    for entity in NOTARIZED_ENTITIES
                )
                if is_notarized:
                    warnings.append(
                        f'  L{i + 1}: struct {struct_name} has id field without '
                        f'dht_anchor_hash — notarized entity needs DHT anchor'
                    )

    return warnings


def audit_view(lines: list[str]) -> list[str]:
    """Audit View structs at the API boundary."""
    warnings = []

    for i, line in enumerate(lines):
        if re.search(r'pub\s+struct\s+(\w+View)', line):
            struct_match = re.search(r'pub\s+struct\s+(\w+View)', line)
            view_name = struct_match.group(1) if struct_match else 'Unknown'

            lookahead = ''.join(lines[i : min(len(lines), i + 30)])
            has_id = bool(re.search(r'pub\s+id\s*:', lookahead))
            has_anchor = 'dht_anchor_hash' in lookahead

            if has_id and not has_anchor:
                is_notarized = any(
                    entity in view_name.lower()
                    for entity in NOTARIZED_ENTITIES
                )
                if is_notarized:
                    warnings.append(
                        f'  L{i + 1}: {view_name} exposes id without dht_anchor_hash — '
                        f'clients cannot verify DHT provenance'
                    )

    return warnings


def audit_route(lines: list[str]) -> list[str]:
    """Audit route handlers for missing DHT context."""
    warnings = []

    for i, line in enumerate(lines):
        if re.search(r'pub\s+async\s+fn\s+(\w+)', line):
            fn_match = re.search(r'pub\s+async\s+fn\s+(\w+)', line)
            fn_name = fn_match.group(1) if fn_match else 'Unknown'

            # Look at surrounding context for DHT references
            window_start = max(0, i - 5)
            window_end = min(len(lines), i + 20)
            window = ''.join(lines[window_start:window_end]).lower()

            is_crud = any(
                verb in fn_name.lower()
                for verb in ['create', 'update', 'delete', 'put', 'post']
            )

            has_dht_context = any(
                term in window
                for term in [
                    'coordinator', 'zome', 'dht', 'entry_type',
                    'post_commit', 'projection', 'operational',
                ]
            )

            if is_crud and not has_dht_context:
                warnings.append(
                    f'  L{i + 1}: {fn_name} — write handler without DHT context. '
                    f'Is this a projection write or direct storage?'
                )

    return warnings


def main():
    try:
        data = json.load(sys.stdin)

        tool_input = data.get('tool_input', {})
        file_path = tool_input.get('file_path', '')

        if not file_path:
            sys.exit(0)

        project_dir = os.environ.get('CLAUDE_PROJECT_DIR', '/projects/elohim')

        try:
            relative_path = os.path.relpath(file_path, project_dir)
        except ValueError:
            relative_path = file_path

        file_type = matches_audit_path(relative_path)
        if not file_type:
            sys.exit(0)

        try:
            with open(file_path) as f:
                lines = f.readlines()
        except (FileNotFoundError, PermissionError):
            sys.exit(0)

        auditors = {
            'migration': audit_migration,
            'model': audit_model,
            'view': audit_view,
            'route': audit_route,
        }

        auditor = auditors.get(file_type)
        if not auditor:
            sys.exit(0)

        warnings = auditor(lines)

        if warnings:
            basename = os.path.basename(file_path)
            msg = (
                f'[P2P SCHEMA AUDIT] {basename}:\n'
                + '\n'.join(warnings[:10])
                + '\n  Reference: .claude/skills/p2p-design-gate/SKILL.md'
            )
            result = {
                'hookSpecificOutput': {
                    'hookEventName': 'PostToolUse',
                    'additionalContext': msg,
                }
            }
            print(json.dumps(result))

        sys.exit(0)

    except json.JSONDecodeError:
        sys.exit(0)
    except Exception as e:
        print(f'p2p-schema-audit hook error: {e}', file=sys.stderr)
        sys.exit(1)


if __name__ == '__main__':
    main()
```

**Step 2: Make executable**

Run: `chmod +x .claude/hooks/p2p-schema-audit.py`

**Step 3: Commit**

```bash
git add .claude/hooks/p2p-schema-audit.py
git commit -m "feat: add p2p-schema-audit hook — scans Rust migrations/models/views/routes for P2P anti-patterns"
```

---

### Task 5: Register Hooks in settings.json

**Files:**
- Modify: `.claude/settings.json:51-75` (PostToolUse section)

**Step 1: Add both hooks to the PostToolUse Edit|Write array**

Add two new entries to the `hooks` array inside the `PostToolUse` matcher for `Edit|Write`, after the existing `epr-link-check.py` entry:

```json
{
  "type": "command",
  "command": "python3 \"$CLAUDE_PROJECT_DIR/.claude/hooks/p2p-plan-audit.py\"",
  "timeout": 3000
},
{
  "type": "command",
  "command": "python3 \"$CLAUDE_PROJECT_DIR/.claude/hooks/p2p-schema-audit.py\"",
  "timeout": 3000
}
```

**Step 2: Verify JSON is valid**

Run: `python3 -c "import json; json.load(open('.claude/settings.json'))"`
Expected: No output (valid JSON)

**Step 3: Commit**

```bash
git add .claude/settings.json
git commit -m "feat: register p2p-plan-audit and p2p-schema-audit hooks in settings.json"
```

---

### Task 6: Smoke Test All Layers

**Step 1: Test the plan hook against an existing plan with known anti-patterns**

Create a temporary test plan:
```bash
cat > /tmp/test-plan.md << 'EOF'
# Test Scheduling Feature

## Schema
CREATE TABLE schedules (
  id TEXT PRIMARY KEY,
  entity_id TEXT NOT NULL
);

## API Design
GET /api/v1/schedules — list all schedules
POST /api/v1/schedules — create a schedule
EOF
```

Run the hook manually:
```bash
echo '{"tool_input":{"file_path":"/projects/elohim/genesis/plans/test-plan.md"}}' | \
  CLAUDE_PROJECT_DIR=/projects/elohim python3 .claude/hooks/p2p-plan-audit.py
```

Expected: JSON output with warnings about PRIMARY KEY without dht_anchor_hash, API routes without entry types, and CREATE TABLE without source-of-truth.

**Step 2: Test the schema hook against a test migration**

```bash
echo '{"tool_input":{"file_path":"/projects/elohim/elohim/elohim-storage/migrations/2026-99-test/up.sql"}}' | \
  CLAUDE_PROJECT_DIR=/projects/elohim python3 .claude/hooks/p2p-schema-audit.py
```

Expected: Exit 0 (file doesn't exist, handled gracefully).

**Step 3: Test hook against existing migration to verify no false positives**

```bash
echo '{"tool_input":{"file_path":"/projects/elohim/elohim/elohim-storage/migrations/2026-01-08-000000_initial/up.sql"}}' | \
  CLAUDE_PROJECT_DIR=/projects/elohim python3 .claude/hooks/p2p-schema-audit.py
```

Expected: May produce warnings for existing tables (this is expected — existing tables predate the enforcement). Verify warnings are sensible, not noise.

**Step 4: Verify settings.json loads without error**

```bash
python3 -c "import json; d = json.load(open('.claude/settings.json')); print(f'PostToolUse hooks: {len(d[\"hooks\"][\"PostToolUse\"][0][\"hooks\"])}')"
```

Expected: Hook count increased by 2 from current count (currently 4, should be 6).

**Step 5: Clean up test file**

```bash
rm -f /tmp/test-plan.md
```

**Step 6: Final commit**

```bash
git add -A
git commit -m "test: verify p2p enforcement hooks work correctly"
```

(Only commit if any fixes were needed during testing. If all passed clean, skip this commit.)
