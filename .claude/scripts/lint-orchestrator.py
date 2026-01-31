#!/usr/bin/env python3
"""
Lint Fixing Orchestrator

Manages the parallel lint-fixing pipeline:
1. Dispatches batches of issues to Haiku agents
2. Parses agent outcomes from output files
3. Updates manifest with results
4. Re-dispatches escalations to stronger models
5. Maintains human backlog for manual review

Usage:
  python lint-orchestrator.py parse-outcomes <output_dir>  # Parse completed agent outputs
  python lint-orchestrator.py update-manifest              # Update manifest from outcomes
  python lint-orchestrator.py show-status                  # Show current status
  python lint-orchestrator.py next-batch <tier> <count>    # Get next batch to dispatch
  python lint-orchestrator.py human-backlog                # Show items needing human review
"""

import json
import os
import re
import sys
from dataclasses import dataclass, asdict
from enum import Enum
from pathlib import Path
from typing import Optional
from datetime import datetime

# Paths
PROJECT_DIR = Path('/projects/elohim')
MANIFEST_PATH = PROJECT_DIR / '.claude' / 'lint-manifest.json'
OUTCOMES_PATH = PROJECT_DIR / '.claude' / 'lint-outcomes.json'
HUMAN_BACKLOG_PATH = PROJECT_DIR / '.claude' / 'lint-human-backlog.md'
TASK_OUTPUT_DIR = Path('/tmp/claude/-projects-elohim/tasks')

# Mapping of task_id -> issue_id for issues dispatched without lint-XXXX in prompt
# This gets populated during dispatch and used during outcome parsing
TASK_ISSUE_MAP_PATH = PROJECT_DIR / '.claude' / 'task-issue-map.json'


class Status(Enum):
    PENDING = 'pending'
    IN_PROGRESS = 'in_progress'
    FIXED = 'fixed'
    ESCALATED = 'escalated'
    FAILED = 'failed'
    SKIPPED = 'skipped'
    HUMAN_REVIEW = 'human_review'


class Tier(Enum):
    MECHANICAL = 'mechanical'
    CONTEXTUAL = 'contextual'
    JUDGMENT = 'judgment'


TIER_MODEL_MAP = {
    'mechanical': 'haiku',
    'contextual': 'sonnet',
    'judgment': 'opus',
}


@dataclass
class Outcome:
    """Parsed outcome from an agent run."""
    issue_id: str
    status: str
    tier: str
    escalate_to: Optional[str] = None
    reason: Optional[str] = None
    changes: Optional[str] = None
    handoff: Optional[str] = None
    task_id: Optional[str] = None
    parsed_at: Optional[str] = None


def load_task_issue_map() -> dict[str, str]:
    """Load task_id -> issue_id mapping."""
    if not TASK_ISSUE_MAP_PATH.exists():
        return {}
    with open(TASK_ISSUE_MAP_PATH) as f:
        return json.load(f)


def save_task_issue_map(mapping: dict[str, str]) -> None:
    """Save task_id -> issue_id mapping."""
    with open(TASK_ISSUE_MAP_PATH, 'w') as f:
        json.dump(mapping, f, indent=2)


def register_dispatch(task_id: str, issue_id: str) -> None:
    """Register a dispatched task for later outcome parsing."""
    mapping = load_task_issue_map()
    mapping[task_id] = issue_id
    save_task_issue_map(mapping)

    # Mark issue as in_progress in manifest
    manifest = load_manifest()
    for item in manifest:
        if item['id'] == issue_id:
            item['status'] = 'in_progress'
            item['taskId'] = task_id
            break
    save_manifest(manifest)


def load_manifest() -> list[dict]:
    """Load the lint manifest."""
    with open(MANIFEST_PATH) as f:
        return json.load(f)


def save_manifest(manifest: list[dict]) -> None:
    """Save the lint manifest."""
    with open(MANIFEST_PATH, 'w') as f:
        json.dump(manifest, f, indent=2)


def load_outcomes() -> dict[str, Outcome]:
    """Load parsed outcomes."""
    if not OUTCOMES_PATH.exists():
        return {}
    with open(OUTCOMES_PATH) as f:
        data = json.load(f)
        outcomes = {}
        for k, v in data.items():
            # Filter to only known Outcome fields
            known_fields = {'issue_id', 'status', 'tier', 'escalate_to', 'reason',
                           'changes', 'handoff', 'task_id', 'parsed_at'}
            filtered = {key: val for key, val in v.items() if key in known_fields}
            outcomes[k] = Outcome(**filtered)
        return outcomes


def save_outcomes(outcomes: dict[str, Outcome]) -> None:
    """Save parsed outcomes."""
    with open(OUTCOMES_PATH, 'w') as f:
        json.dump({k: asdict(v) for k, v in outcomes.items()}, f, indent=2)


def parse_outcome_block(content: str) -> Optional[dict]:
    """Parse a ## Outcome block from agent output."""
    # Find the Outcome block
    outcome_match = re.search(r'## Outcome\s*\n(.*?)(?=\n##|\n\n\n|$)', content, re.DOTALL)
    if not outcome_match:
        return None

    outcome_text = outcome_match.group(1)
    result = {}

    # Parse each field - handle multiple formats:
    # - **status**: value (markdown bold with dash prefix)
    # - **Status:** value (markdown bold with colon)
    # - status: value (plain)
    # Handle various markdown formats:
    # - **status**: value  (colon outside bold)
    # - **Status:** value  (colon inside bold)
    # - status: value (plain)
    # - **FIXED** - ... (status word alone, bold)
    # - Status: COMPLETED
    patterns = {
        'status': r'\*\*status:?\*\*:?\s*(\w+)|status:?\s*(\w+)|\*\*(FIXED|COMPLETED|RESOLVED|ESCALATE|SKIP)\*\*',
        'tier': r'\*\*tier:?\*\*:?\s*(\w+)|tier:?\s*(\w+)',
        'escalate_to': r'\*\*escalate\s*To:?\*\*:?\s*(\w+)|escalate\s*To:?\s*(\w+)',
        'reason': r'\*\*(?:reason|problem|solution):?\*\*:?\s*(.+?)(?=\n\s*[-*\[]|\n\n|$)|(?:reason|problem|solution):?\s*(.+?)(?=\n\s*[-*\[]|\n\n|$)',
        'changes': r'\*\*changes(?:\s+made)?:?\*\*:?\s*(.+?)(?=\n\s*[-*\[]|\n\n|$)|changes(?:\s+made)?:?\s*(.+?)(?=\n\s*[-*\[]|\n\n|$)',
        'handoff': r'\*\*handoff:?\*\*:?\s*(.+?)(?=\n##|$)|handoff:?\s*(.+?)(?=\n##|$)',
    }

    for field, pattern in patterns.items():
        match = re.search(pattern, outcome_text, re.IGNORECASE | re.DOTALL)
        if match:
            # With alternation (|), we need to find the first non-None group
            value = next((g for g in match.groups() if g is not None), None)
            if value:
                result[field] = value.strip()

    # Normalize status values
    if result.get('status'):
        status_lower = result['status'].lower()
        if status_lower in ('fixed', 'resolved', 'done', 'complete', 'completed'):
            result['status'] = 'fixed'
        elif status_lower in ('escalate', 'escalated'):
            result['status'] = 'escalate'
        elif status_lower in ('skip', 'skipped'):
            result['status'] = 'skip'

    return result if result.get('status') else None


def parse_task_output(task_id: str, task_issue_map: dict[str, str] = None) -> Optional[Outcome]:
    """Parse an agent's output file for its outcome."""
    output_path = TASK_OUTPUT_DIR / f'{task_id}.output'
    if not output_path.exists():
        return None

    # The output is JSONL format - parse each line
    content = output_path.read_text()
    issue_id = None
    outcome_data = None

    for line in content.split('\n'):
        if not line.strip():
            continue
        try:
            entry = json.loads(line)

            # Look for issue ID in prompt (user messages)
            if entry.get('type') == 'user':
                msg_content = entry.get('message', {}).get('content', [])
                if isinstance(msg_content, list):
                    for part in msg_content:
                        if isinstance(part, dict) and part.get('type') == 'text':
                            match = re.search(r'lint-\d{4}', part.get('text', ''))
                            if match:
                                issue_id = match.group(0)

            # Look for outcome in assistant messages
            if entry.get('type') == 'assistant':
                msg = entry.get('message', {})
                content_parts = msg.get('content', [])
                if isinstance(content_parts, list):
                    for part in content_parts:
                        if isinstance(part, dict) and part.get('type') == 'text':
                            text = part.get('text', '')
                            if '## Outcome' in text:
                                outcome_data = parse_outcome_block(text)
                                # Also try to find issue ID in the text
                                if not issue_id:
                                    match = re.search(r'lint-\d{4}', text)
                                    if match:
                                        issue_id = match.group(0)
        except json.JSONDecodeError:
            continue

    if not outcome_data:
        return None

    # Fall back to task_issue_map if issue_id not found in content
    if not issue_id and task_issue_map:
        issue_id = task_issue_map.get(task_id)

    return Outcome(
        issue_id=issue_id or 'unknown',
        status=outcome_data.get('status', 'unknown'),
        tier=outcome_data.get('tier', 'unknown'),
        escalate_to=outcome_data.get('escalate_to'),
        reason=outcome_data.get('reason'),
        changes=outcome_data.get('changes'),
        handoff=outcome_data.get('handoff'),
        task_id=task_id,
        parsed_at=datetime.now().isoformat(),
    )


def update_manifest_from_outcomes(outcomes: dict[str, Outcome]) -> dict:
    """Update manifest with parsed outcomes. Returns summary."""
    manifest = load_manifest()
    id_to_index = {item['id']: i for i, item in enumerate(manifest)}

    updated = 0
    for issue_id, outcome in outcomes.items():
        if issue_id in id_to_index:
            idx = id_to_index[issue_id]
            manifest[idx]['status'] = outcome.status
            if outcome.escalate_to:
                manifest[idx]['escalateTo'] = outcome.escalate_to
            if outcome.reason:
                manifest[idx]['outcomeReason'] = outcome.reason
            if outcome.task_id:
                manifest[idx]['lastTaskId'] = outcome.task_id
            updated += 1

    save_manifest(manifest)
    return {'updated': updated, 'total': len(manifest)}


def generate_human_backlog() -> str:
    """Generate markdown backlog of items needing human review."""
    manifest = load_manifest()
    outcomes = load_outcomes()

    human_items = []
    for item in manifest:
        if item.get('status') == 'human_review' or item.get('escalateTo') == 'human':
            human_items.append(item)

    # Also include judgment-tier escalations that weren't auto-resolved
    for item in manifest:
        if item.get('status') == 'escalated' and item.get('escalateTo') == 'judgment':
            # Check if it was retried at judgment tier
            outcome = outcomes.get(item['id'])
            if not outcome or outcome.status != 'fixed':
                human_items.append(item)

    if not human_items:
        return "# Human Review Backlog\n\nNo items pending human review.\n"

    lines = ["# Human Review Backlog", "", f"Generated: {datetime.now().isoformat()}", ""]
    lines.append(f"## Summary: {len(human_items)} items", "")

    for item in human_items:
        lines.extend([
            f"### {item['id']}: {item['ruleId']}",
            f"- **File**: `{item['file']}`",
            f"- **Line**: {item['line']}:{item.get('column', '?')}",
            f"- **Message**: {item['message']}",
        ])
        if item.get('outcomeReason'):
            lines.append(f"- **Escalation Reason**: {item['outcomeReason']}")
        outcome = outcomes.get(item['id'])
        if outcome and outcome.handoff:
            lines.extend(["", "#### Handoff Context", outcome.handoff])
        lines.append("")

    return '\n'.join(lines)


def get_dispatch_batch(tier: str, count: int) -> list[dict]:
    """Get next batch of issues to dispatch at given tier."""
    manifest = load_manifest()

    # For escalated issues, get those escalated TO this tier
    # For pending issues, get those AT this tier
    # Exclude in_progress issues
    candidates = []
    for item in manifest:
        status = item.get('status', 'pending')
        if status == 'in_progress':
            continue  # Skip already-dispatched issues
        if status == 'pending' and item.get('tier') == tier:
            candidates.append(item)
        elif status in ('escalate', 'escalated') and item.get('escalateTo') == tier:
            candidates.append(item)

    return candidates[:count]


def show_status() -> dict:
    """Show current status of the manifest."""
    manifest = load_manifest()

    by_status = {}
    by_tier = {}

    for item in manifest:
        status = item.get('status', 'pending')
        tier = item.get('tier', 'unknown')

        by_status[status] = by_status.get(status, 0) + 1
        by_tier[tier] = by_tier.get(tier, 0) + 1

    # Count escalations
    escalated_to = {}
    for item in manifest:
        if item.get('escalateTo'):
            target = item['escalateTo']
            escalated_to[target] = escalated_to.get(target, 0) + 1

    return {
        'total': len(manifest),
        'by_status': by_status,
        'by_tier': by_tier,
        'escalated_to': escalated_to,
    }


def generate_task_prompt(issue: dict) -> str:
    """Generate the prompt for a lint-fixing agent."""
    return f"""Fix this lint issue:
- File: {issue['file']}
- Line: {issue['line']}, Column: {issue.get('column', '?')}
- Rule: {issue['ruleId']}
- Message: {issue['message']}
- Issue ID: {issue['id']}
- Current Tier: {issue.get('tier', 'unknown')}

Fix hint: {issue.get('fixHint', 'Review the rule documentation and fix accordingly')}

Instructions:
1. Read ±30 lines around line {issue['line']}
2. Assess if this fix is within your tier capability:
   - mechanical: pattern replacement only
   - contextual: needs code understanding
   - judgment: needs architectural decisions
3. If within capability: make the fix
4. If beyond capability: escalate with a clear reason
5. End with structured Outcome block

Your assessment matters. A thoughtful escalation helps the team as much as a good fix.
"""


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(1)

    cmd = sys.argv[1]

    if cmd == 'parse-outcomes':
        # Parse all task outputs in the directory
        if len(sys.argv) > 2:
            output_dir = Path(sys.argv[2])
        else:
            output_dir = TASK_OUTPUT_DIR

        outcomes = load_outcomes()
        task_issue_map = load_task_issue_map()
        new_count = 0
        unknown_count = 0

        # Sort by task_id to process in order (later task IDs = newer)
        output_files = sorted(output_dir.glob('*.output'), key=lambda f: f.stem)

        for output_file in output_files:
            task_id = output_file.stem
            outcome = parse_task_output(task_id, task_issue_map)
            if outcome:
                if outcome.issue_id != 'unknown':
                    # Check if this is a newer task for the same issue
                    existing = outcomes.get(outcome.issue_id)
                    if existing and existing.task_id:
                        if task_id > existing.task_id:
                            # Newer task - update the outcome
                            outcomes[outcome.issue_id] = outcome
                            new_count += 1
                            print(f"Updated: {outcome.issue_id} -> {outcome.status} (was {existing.status})")
                        # else: keep existing (older task)
                    elif task_id not in [o.task_id for o in outcomes.values()]:
                        outcomes[outcome.issue_id] = outcome
                        new_count += 1
                        print(f"Parsed: {outcome.issue_id} -> {outcome.status}")
                else:
                    if task_id not in [o.task_id for o in outcomes.values()]:
                        unknown_count += 1
                        print(f"  Unknown issue ID for task {task_id} (status: {outcome.status})")

        save_outcomes(outcomes)
        print(f"\nParsed {new_count} new outcomes. {unknown_count} unknown. Total: {len(outcomes)}")

    elif cmd == 'register-dispatch':
        # Register a task_id -> issue_id mapping
        if len(sys.argv) < 4:
            print("Usage: register-dispatch <task_id> <issue_id>")
            sys.exit(1)
        task_id = sys.argv[2]
        issue_id = sys.argv[3]
        register_dispatch(task_id, issue_id)
        print(f"Registered: {task_id} -> {issue_id}")

    elif cmd == 'update-manifest':
        outcomes = load_outcomes()
        result = update_manifest_from_outcomes(outcomes)
        print(f"Updated {result['updated']}/{result['total']} manifest entries")

    elif cmd == 'show-status':
        status = show_status()
        print(json.dumps(status, indent=2))

    elif cmd == 'next-batch':
        tier = sys.argv[2] if len(sys.argv) > 2 else 'mechanical'
        count = int(sys.argv[3]) if len(sys.argv) > 3 else 10
        batch = get_dispatch_batch(tier, count)

        print(f"Next {len(batch)} {tier}-tier issues:")
        for item in batch:
            print(f"  {item['id']}: {item['file'].split('/')[-1]}:{item['line']} [{item['ruleId']}]")

        # Also output dispatch prompts
        if batch:
            print("\n--- Task Dispatch Prompts ---")
            for item in batch:
                print(f"\n# {item['id']}")
                print(generate_task_prompt(item))

    elif cmd == 'human-backlog':
        backlog = generate_human_backlog()
        print(backlog)
        # Also write to file
        with open(HUMAN_BACKLOG_PATH, 'w') as f:
            f.write(backlog)
        print(f"\nWritten to {HUMAN_BACKLOG_PATH}")

    elif cmd == 'dispatch-info':
        # Output JSON for programmatic dispatch
        tier = sys.argv[2] if len(sys.argv) > 2 else 'mechanical'
        count = int(sys.argv[3]) if len(sys.argv) > 3 else 10
        batch = get_dispatch_batch(tier, count)

        dispatch_info = []
        for item in batch:
            dispatch_info.append({
                'issue_id': item['id'],
                'model': TIER_MODEL_MAP.get(tier, 'haiku'),
                'prompt': generate_task_prompt(item),
                'file': item['file'],
            })

        print(json.dumps(dispatch_info, indent=2))

    elif cmd == 'batch-summary':
        # Minimal output for lean orchestration
        tier = sys.argv[2] if len(sys.argv) > 2 else 'mechanical'
        count = int(sys.argv[3]) if len(sys.argv) > 3 else 10
        batch = get_dispatch_batch(tier, count)

        print(f"Batch: {len(batch)} {tier}-tier issues")
        for item in batch:
            print(f"  {item['id']}: {item['file'].split('/')[-1]}:{item['line']} [{item['ruleId'].split('/')[-1]}]")

    elif cmd == 'write-batch':
        # Write batch prompts to temp files for external dispatch
        tier = sys.argv[2] if len(sys.argv) > 2 else 'mechanical'
        count = int(sys.argv[3]) if len(sys.argv) > 3 else 10
        batch = get_dispatch_batch(tier, count)

        batch_dir = PROJECT_DIR / '.claude' / 'batch-prompts'
        batch_dir.mkdir(exist_ok=True)

        for item in batch:
            prompt_file = batch_dir / f"{item['id']}.txt"
            prompt_file.write_text(generate_task_prompt(item))

        print(f"Wrote {len(batch)} prompts to {batch_dir}")
        print("Issue IDs:", ' '.join(item['id'] for item in batch))

    elif cmd == 'get-escalations':
        # Find issues that have been escalated and need re-dispatch
        manifest = load_manifest()
        outcomes = load_outcomes()

        escalations = []
        for item in manifest:
            if item.get('status') in ('escalate', 'escalated'):
                escalate_to = item.get('escalateTo', '').lower()
                # Normalize escalation targets
                if escalate_to in ('judgment', 'opus'):
                    target_tier = 'judgment'
                    target_model = 'opus'
                elif escalate_to in ('contextual', 'sonnet'):
                    target_tier = 'contextual'
                    target_model = 'sonnet'
                elif escalate_to == 'human':
                    continue  # Skip human escalations for auto-dispatch
                else:
                    continue  # Unknown tier

                # Get the escalation context from outcomes
                outcome = outcomes.get(item['id'])
                escalation_reason = item.get('outcomeReason') or (outcome.reason if outcome else '')

                escalations.append({
                    'issue_id': item['id'],
                    'target_tier': target_tier,
                    'target_model': target_model,
                    'file': item['file'],
                    'line': item['line'],
                    'ruleId': item['ruleId'],
                    'message': item['message'],
                    'escalation_reason': escalation_reason,
                    'original_tier': item.get('tier', 'unknown'),
                })

        if not escalations:
            print("No pending escalations.")
        else:
            print(f"Found {len(escalations)} pending escalations:\n")
            for esc in escalations:
                print(f"  {esc['issue_id']}: {esc['original_tier']} -> {esc['target_tier']}")
                print(f"    File: {esc['file'].split('/')[-1]}:{esc['line']}")
                print(f"    Rule: {esc['ruleId']}")
                if esc['escalation_reason']:
                    reason_preview = esc['escalation_reason'][:100] + '...' if len(esc['escalation_reason']) > 100 else esc['escalation_reason']
                    print(f"    Reason: {reason_preview}")
                print()

            # Output dispatch commands
            print("\n--- Dispatch Prompts for Escalations ---")
            for esc in escalations:
                prompt = f"""Fix this escalated lint issue (previously escalated from {esc['original_tier']} tier):

- File: {esc['file']}
- Line: {esc['line']}
- Rule: {esc['ruleId']}
- Message: {esc['message']}
- Issue ID: {esc['issue_id']}

Previous escalation reason: {esc['escalation_reason']}

You have judgment-tier capability. Make an architectural decision:
1. If the issue reveals conflicting requirements, propose a resolution
2. If the fix needs a design pattern change, implement it
3. If the issue should be skipped (intentional code), document why
4. End with structured Outcome block
"""
                print(f"\n# {esc['issue_id']} ({esc['target_model']})")
                print(prompt)

    else:
        print(f"Unknown command: {cmd}")
        print(__doc__)
        sys.exit(1)


if __name__ == '__main__':
    main()
