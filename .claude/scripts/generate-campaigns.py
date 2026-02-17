#!/usr/bin/env python3
"""
Generate Campaign Tasks from Lint Manifest

Reads .claude/lint-manifest.json, groups issues by ruleId into campaigns,
splits large groups by module, and outputs structured campaign data for
the team-based quality orchestrator.

Usage:
  python3 generate-campaigns.py                    # Full JSON output
  python3 generate-campaigns.py --summary          # Human-readable summary
  python3 generate-campaigns.py --tier mechanical   # Filter by tier
  python3 generate-campaigns.py --agent quality-sweep  # Filter by agent type
  python3 generate-campaigns.py --task-descriptions # Output TaskCreate-ready descriptions
  python3 generate-campaigns.py --project doorway   # Filter by project
"""

import json
import re
import sys
from collections import defaultdict
from pathlib import Path

PROJECT_DIR = Path('/projects/elohim')
MANIFEST_PATH = PROJECT_DIR / '.claude' / 'lint-manifest.json'

# Campaign size limits (files per campaign)
MAX_FILES_HAIKU = 40
MAX_FILES_SONNET = 25

# Tier to agent mapping
TIER_AGENT_MAP = {
    'mechanical': 'quality-sweep',
    'contextual': 'quality-deep',
    'sonnet': 'quality-deep',
    'judgment': 'quality-architect',
}

TIER_MODEL_MAP = {
    'mechanical': 'haiku',
    'contextual': 'sonnet',
    'sonnet': 'sonnet',
    'judgment': 'opus',
}


def load_manifest():
    with open(MANIFEST_PATH) as f:
        return json.load(f)


def extract_module(filepath):
    """Extract the module name from a file path.

    Handles multiple project path conventions:
    - elohim-app/doorway-app: first dir after src/app/
    - sophia: package name from packages/<name>/
    - doorway: first dir after src/
    """
    # Angular apps: src/app/<module>/
    match = re.search(r'/src/app/([^/]+)/', filepath)
    if match:
        return match.group(1)

    # sophia monorepo: packages/<package>/
    match = re.search(r'/packages/([^/]+)/', filepath)
    if match:
        return match.group(1)

    # Rust: src/<module>/
    match = re.search(r'/doorway/src/([^/]+)', filepath)
    if match:
        mod = match.group(1)
        return mod.replace('.rs', '') if mod.endswith('.rs') else mod

    return 'root'


def slugify(rule_id, module=None, chunk_idx=None):
    """Create a campaign ID slug."""
    slug = re.sub(r'[^a-z0-9]+', '-', rule_id.lower().split('/')[-1]).strip('-')
    if module:
        slug = f"{slug}--{module}"
    if chunk_idx is not None:
        slug = f"{slug}--{chunk_idx}"
    return slug


def group_issues_by_rule(issues):
    """Group pending issues by ruleId."""
    groups = defaultdict(list)
    for issue in issues:
        if issue.get('status') == 'pending':
            groups[issue['ruleId']].append(issue)
    return dict(groups)


def build_file_map(issues):
    """Build {filepath: [line1, line2, ...]} from a list of issues."""
    file_map = defaultdict(list)
    for issue in issues:
        file_map[issue['file']].append(issue['line'])
    # Sort lines within each file, deduplicate
    return {f: sorted(set(lines)) for f, lines in sorted(file_map.items())}


def detect_project(filepath):
    """Detect which project a filepath belongs to."""
    if '/doorway-app/' in filepath:
        return 'doorway-app'
    if '/doorway/' in filepath:
        return 'doorway'
    if '/sophia/' in filepath:
        return 'sophia'
    if '/elohim-app/' in filepath:
        return 'elohim-app'
    return 'unknown'


def build_campaign(rule_id, issues, module=None, chunk_idx=None):
    """Build a single campaign dict."""
    tier = issues[0].get('tier', 'sonnet')
    agent_type = TIER_AGENT_MAP.get(tier, 'quality-deep')
    file_map = build_file_map(issues)
    project = issues[0].get('project', detect_project(issues[0]['file']))

    return {
        'campaign_id': slugify(rule_id, module, chunk_idx),
        'rule_id': rule_id,
        'tier': tier,
        'agent_type': agent_type,
        'model': TIER_MODEL_MAP.get(tier, 'sonnet'),
        'project': project,
        'issue_count': len(issues),
        'file_count': len(file_map),
        'fix_hint': issues[0].get('fixHint', 'Review the rule documentation and fix accordingly'),
        'files': file_map,
        'issue_ids': [i['id'] for i in issues],
    }


def split_by_module(issues):
    """Group issues by module (first path component after src/app/)."""
    modules = defaultdict(list)
    for issue in issues:
        mod = extract_module(issue['file'])
        modules[mod].append(issue)
    return dict(modules)


def chunk_list(items, size):
    """Split a list into chunks of given size."""
    for i in range(0, len(items), size):
        yield items[i:i + size]


def generate_campaigns(issues):
    """Generate all campaigns from the manifest."""
    campaigns = []
    by_rule = group_issues_by_rule(issues)

    for rule_id, rule_issues in sorted(by_rule.items(), key=lambda x: -len(x[1])):
        tier = rule_issues[0].get('tier', 'sonnet')
        max_files = MAX_FILES_HAIKU if tier == 'mechanical' else MAX_FILES_SONNET

        unique_files = set(i['file'] for i in rule_issues)

        if len(unique_files) <= max_files:
            # Single campaign for this rule
            campaigns.append(build_campaign(rule_id, rule_issues))
        else:
            # Split by module
            by_module = split_by_module(rule_issues)
            for module, mod_issues in sorted(by_module.items()):
                mod_files = set(i['file'] for i in mod_issues)
                if len(mod_files) <= max_files:
                    campaigns.append(build_campaign(rule_id, mod_issues, module=module))
                else:
                    # Further split into chunks
                    file_list = sorted(mod_files)
                    for idx, chunk_files in enumerate(chunk_list(file_list, max_files)):
                        chunk_set = set(chunk_files)
                        chunk_issues = [i for i in mod_issues if i['file'] in chunk_set]
                        campaigns.append(build_campaign(
                            rule_id, chunk_issues, module=module, chunk_idx=idx
                        ))

    # Sort: mechanical first, then contextual, then sonnet, then judgment
    tier_order = {'mechanical': 0, 'contextual': 1, 'sonnet': 2, 'judgment': 3}
    campaigns.sort(key=lambda c: (tier_order.get(c['tier'], 9), -c['issue_count']))

    return campaigns


def format_task_description(campaign):
    """Format a campaign as a TaskCreate-ready description."""
    file_lines = []
    for filepath, lines in campaign['files'].items():
        # Use relative path for readability — handle all projects
        rel = filepath.replace('/projects/elohim/', '')
        line_str = ','.join(str(l) for l in lines)
        file_lines.append(f"- {rel}:{line_str}")

    files_block = '\n'.join(file_lines)

    return f"""Campaign: {campaign['rule_id']} ({campaign['issue_count']} issues, {campaign['file_count']} files)
Tier: {campaign['tier']}
Fix pattern: {campaign['fix_hint']}

Files:
{files_block}

Instructions:
1. For each file listed above:
   a. Read the file
   b. Fix ALL instances of {campaign['rule_id']} at the listed lines
   c. The post-edit hook will lint-check your changes automatically
   d. Write the corrected file
2. Move to the next file. Do NOT stop between files.
3. When all files are done, mark this task as completed via TaskUpdate.
4. If any file requires reasoning beyond your tier, note it in your
   completion message but continue with the remaining files.
5. After completing, check TaskList for the next available campaign.
   Claim it by setting yourself as owner via TaskUpdate.

Issue IDs: {', '.join(campaign['issue_ids'])}"""


def print_summary(campaigns):
    """Print human-readable summary."""
    by_tier = defaultdict(list)
    for c in campaigns:
        by_tier[c['tier']].append(c)

    total_issues = sum(c['issue_count'] for c in campaigns)
    total_files = sum(c['file_count'] for c in campaigns)

    print(f"=== Campaign Summary ===")
    print(f"Total: {len(campaigns)} campaigns, {total_issues} issues, {total_files} file-touches")
    print()

    tier_order = ['mechanical', 'contextual', 'sonnet', 'judgment']
    for tier in tier_order:
        tier_campaigns = by_tier.get(tier, [])
        if not tier_campaigns:
            continue

        agent = TIER_AGENT_MAP.get(tier, 'quality-deep')
        model = TIER_MODEL_MAP.get(tier, 'sonnet')
        tier_issues = sum(c['issue_count'] for c in tier_campaigns)

        print(f"--- {tier.upper()} ({agent} / {model}) ---")
        print(f"  Campaigns: {len(tier_campaigns)}, Issues: {tier_issues}")
        for c in tier_campaigns:
            print(f"  {c['campaign_id']:50s}  {c['issue_count']:4d} issues  {c['file_count']:3d} files")
        print()


def main():
    issues = load_manifest()
    campaigns = generate_campaigns(issues)

    # Parse arguments
    args = sys.argv[1:]

    tier_filter = None
    agent_filter = None
    project_filter = None
    summary_mode = False
    task_mode = False

    i = 0
    while i < len(args):
        if args[i] == '--summary':
            summary_mode = True
        elif args[i] == '--tier' and i + 1 < len(args):
            tier_filter = args[i + 1]
            i += 1
        elif args[i] == '--agent' and i + 1 < len(args):
            agent_filter = args[i + 1]
            i += 1
        elif args[i] == '--project' and i + 1 < len(args):
            project_filter = args[i + 1]
            i += 1
        elif args[i] == '--task-descriptions':
            task_mode = True
        i += 1

    # Apply filters
    if tier_filter:
        campaigns = [c for c in campaigns if c['tier'] == tier_filter]
    if agent_filter:
        campaigns = [c for c in campaigns if c['agent_type'] == agent_filter]
    if project_filter:
        campaigns = [c for c in campaigns if c.get('project') == project_filter]

    # Output
    if summary_mode:
        print_summary(campaigns)
    elif task_mode:
        for c in campaigns:
            print(f"=== {c['campaign_id']} ===")
            print(format_task_description(c))
            print()
    else:
        print(json.dumps(campaigns, indent=2))


if __name__ == '__main__':
    main()
