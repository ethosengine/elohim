#!/usr/bin/env python3
"""
Agent catalog audit — read-only quality scan over .claude/agents/*.md.

Agent metadata (name + description) is ALWAYS loaded into context (the
agent-type list at session start). Agents also encode role + tool grants
+ model assignments + body prompts that drift like any other doc surface.

Parallels skill-audit.py but adds agent-specific checks:
  - Vague descriptions (too short, no triggers, no when-language)
  - Trigger overlap with siblings (distinctive-keyword overlap > N)
  - Stale-by-mtime (>90 days)
  - Dead multi-component path citations in the body
  - Imperatives without rationale
  - Tools-declared-but-unreferenced (body doesn't mention the tool)
  - Model declared (opus/sonnet/haiku) sanity-flag

Output: .claude/memory-kit/<YYYY-MM-DD>/agent-audit.md
No agent files modified. No apply step.
"""
from __future__ import annotations

import argparse
import re
import sys
from collections import defaultdict
from dataclasses import dataclass, field
from datetime import date, datetime, timezone
from pathlib import Path

# Bootstrap: locate .claude/scripts/_lib by walking up
_here = Path(__file__).resolve()
for _ in range(8):
    if (_here / ".claude" / "scripts" / "_lib").is_dir():
        sys.path.insert(0, str(_here / ".claude" / "scripts"))
        break
    _here = _here.parent
from _lib import paths as _paths, frontmatter as _fm  # noqa: E402

REPO_ROOT = _paths.repo_root_from_file(__file__)
AGENTS_ROOT = REPO_ROOT / ".claude" / "agents"
DEFAULT_OUT_ROOT = _paths.reports_root(REPO_ROOT)
TODAY = date.today()
NOW = datetime.now(timezone.utc)

# Thresholds (re-tunable)
MIN_DESCRIPTION_CHARS = 80          # agents are richer than skills; bump from 60
STALE_MTIME_DAYS = 90
TRIGGER_OVERLAP_THRESHOLD = 3
DYNAMIC_STOPWORD_FRACTION = 0.25

GENERIC_DESC_PATTERNS = [
    re.compile(r"\bfor working with\b", re.IGNORECASE),
    re.compile(r"\buse this agent\b", re.IGNORECASE),
    re.compile(r"\bhelps? you\b", re.IGNORECASE),
    re.compile(r"\bgeneral(?:ly)? (?:purpose|useful)\b", re.IGNORECASE),
]
TRIGGER_SIGNALS = re.compile(
    r"\b(use|when|trigger|invoke|before|after|whenever|context)\b", re.IGNORECASE
)
QUOTED_TRIGGER_RE = re.compile(r'"[^"]{4,}"|<example>')
WORD_RE = re.compile(r"[a-zA-Z][a-zA-Z\-_/]*")

# Imperative detection (same as claude-md-audit)
IMPERATIVE_PATTERNS = [
    re.compile(r"\b(always|never|must|MUST|do not|don'?t)\b", re.IGNORECASE),
]
RATIONALE_PATTERNS = [
    re.compile(
        r"\b(because|due to|to (avoid|prevent|ensure)|reason:|why:|fixes:)\b",
        re.IGNORECASE,
    ),
]
RATIONALE_WINDOW = 3

# Multi-component path citation: at least one slash → unambiguous file ref
# Bare filenames (e.g. `views.rs`) are too false-positive-prone to flag.
MULTICOMPONENT_PATH_RE = re.compile(
    r"`([a-zA-Z0-9_./\-]+/[a-zA-Z0-9_./\-]+\.(?:rs|ts|tsx|js|jsx|py|toml|md|json|yaml|yml|sh|sql|html|scss|css|feature))`"
)

# Tool grants we know about (used for tools-declared analysis)
KNOWN_TOOLS = {
    "Task", "Bash", "Glob", "Grep", "Read", "Edit", "Write", "TodoWrite",
    "LSP", "WebFetch", "WebSearch", "NotebookEdit", "TaskList", "TaskGet",
    "TaskUpdate", "TaskCreate", "SendMessage", "ExitPlanMode", "EnterPlanMode",
}


@dataclass
class Agent:
    name: str
    path: Path
    rel: str
    description: str
    tools: list[str]
    model: str
    color: str
    body: str
    body_lines: int
    mtime: datetime


@dataclass
class AgentIssue:
    agent: str
    kind: str         # vague | overlap | stale | drifted-factual | over-imperative | tools-mismatch
    detail: str       # human-readable
    items: list[str] = field(default_factory=list)


def gather_agents() -> list[Agent]:
    agents: list[Agent] = []
    if not AGENTS_ROOT.is_dir():
        return agents
    for p in sorted(AGENTS_ROOT.glob("*.md")):
        try:
            text = p.read_text(encoding="utf-8", errors="replace")
            mtime = datetime.fromtimestamp(p.stat().st_mtime, tz=timezone.utc)
        except OSError:
            continue
        fm = _fm.parse(text)
        tools_raw = fm.get("tools", "")
        tools = [t.strip() for t in tools_raw.split(",") if t.strip()] if tools_raw else []
        agents.append(Agent(
            name=fm.get("name", p.stem),
            path=p,
            rel=str(p.relative_to(REPO_ROOT)),
            description=fm.get("description", ""),
            tools=tools,
            model=fm.get("model", ""),
            color=fm.get("color", ""),
            body=fm.body,
            body_lines=len(fm.body.splitlines()),
            mtime=mtime,
        ))
    return agents


def find_vague_descriptions(agents: list[Agent]) -> list[AgentIssue]:
    issues: list[AgentIssue] = []
    for a in agents:
        kinds: list[str] = []
        desc = a.description
        if len(desc) < MIN_DESCRIPTION_CHARS:
            kinds.append(f"too-short ({len(desc)} chars; min {MIN_DESCRIPTION_CHARS})")
        for pat in GENERIC_DESC_PATTERNS:
            if pat.search(desc):
                kinds.append(f"generic-phrase ({pat.pattern})")
                break
        if (not TRIGGER_SIGNALS.search(desc)) and (not QUOTED_TRIGGER_RE.search(desc)):
            kinds.append("no-trigger-language")
        if kinds:
            issues.append(AgentIssue(
                agent=a.name, kind="vague-description",
                detail="; ".join(kinds),
            ))
    return issues


def find_trigger_overlap(agents: list[Agent]) -> list[AgentIssue]:
    # Tokenize each description; build per-agent token sets
    tokens_per: dict[str, set[str]] = {}
    df: dict[str, int] = defaultdict(int)
    for a in agents:
        toks = {w.lower() for w in WORD_RE.findall(a.description) if len(w) > 3}
        tokens_per[a.name] = toks
        for t in toks:
            df[t] += 1
    n = max(1, len(agents))
    stopwords = {t for t, c in df.items() if c / n >= DYNAMIC_STOPWORD_FRACTION}
    issues: list[AgentIssue] = []
    seen: set[tuple[str, str]] = set()
    names = list(tokens_per.keys())
    for i, a_name in enumerate(names):
        a_distinctive = tokens_per[a_name] - stopwords
        for b_name in names[i + 1:]:
            b_distinctive = tokens_per[b_name] - stopwords
            shared = a_distinctive & b_distinctive
            if len(shared) > TRIGGER_OVERLAP_THRESHOLD:
                key = tuple(sorted([a_name, b_name]))
                if key in seen:
                    continue
                seen.add(key)
                issues.append(AgentIssue(
                    agent=f"{a_name} ↔ {b_name}",
                    kind="trigger-overlap",
                    detail=f"{len(shared)} distinctive shared tokens",
                    items=sorted(shared)[:12],
                ))
    return issues


def find_stale(agents: list[Agent]) -> list[AgentIssue]:
    issues: list[AgentIssue] = []
    for a in agents:
        age = (NOW - a.mtime).days
        if age > STALE_MTIME_DAYS:
            issues.append(AgentIssue(
                agent=a.name, kind="stale-mtime",
                detail=f"{age} days since last edit",
            ))
    return issues


def find_dead_paths(agent: Agent) -> list[str]:
    cited = {m.group(1) for m in MULTICOMPONENT_PATH_RE.finditer(agent.body)}
    dead: list[str] = []
    for cit in cited:
        candidate = REPO_ROOT / cit.lstrip("/")
        if not candidate.exists():
            dead.append(cit)
    return sorted(dead)


def find_imperatives_without_rationale(agent: Agent) -> list[tuple[int, str]]:
    """Same shape as claude-md-audit's detector; excludes lines inside code fences."""
    lines = agent.body.splitlines()
    in_fence = False
    flagged: list[tuple[int, str]] = []
    for i, line in enumerate(lines):
        stripped = line.strip()
        if stripped.startswith("```"):
            in_fence = not in_fence
            continue
        if in_fence:
            continue
        if stripped.startswith("#"):
            continue
        if not any(p.search(line) for p in IMPERATIVE_PATTERNS):
            continue
        start = max(0, i - RATIONALE_WINDOW)
        end = min(len(lines), i + RATIONALE_WINDOW + 1)
        window = "\n".join(lines[start:end])
        if any(p.search(window) for p in RATIONALE_PATTERNS):
            continue
        flagged.append((i + 1, line.strip()))
    return flagged


def find_tools_mismatch(agent: Agent) -> list[str]:
    """Tools declared in frontmatter but never referenced in body — possibly over-granted.

    Conservative: only reports tools whose name doesn't appear ANYWHERE in the body.
    A declared tool that's mentioned in passing still counts as "referenced."
    """
    issues: list[str] = []
    body_lower = agent.body.lower()
    for tool in agent.tools:
        if tool not in KNOWN_TOOLS:
            issues.append(f"unknown-tool: `{tool}` not in KNOWN_TOOLS set")
            continue
        # Match tool name word-bounded, case-insensitive
        if not re.search(rf"\b{re.escape(tool.lower())}\b", body_lower):
            issues.append(f"declared-but-unreferenced: `{tool}`")
    return issues


def find_missing_model(agents: list[Agent]) -> list[AgentIssue]:
    issues: list[AgentIssue] = []
    for a in agents:
        if not a.model:
            issues.append(AgentIssue(
                agent=a.name, kind="missing-model",
                detail="no `model:` field in frontmatter (will inherit from parent — usually unintended)",
            ))
    return issues


def render(agents: list[Agent], per_agent: dict[str, dict]) -> str:
    out: list[str] = []
    out.append(f"# Agent Catalog Audit — {TODAY.isoformat()}\n")
    out.append(
        "Read-only diagnostic on `.claude/agents/*.md`. Agent metadata is "
        "always-loaded into context; body content drifts like any doc surface.\n"
    )
    out.append(f"**Agents audited**: {len(agents)}\n")

    # Summary counts
    out.append("\n## Summary\n\n| Issue class | Count |\n|---|---:|\n")
    counts = defaultdict(int)
    for key, rep in per_agent.items():
        if key.startswith("__"):
            continue  # skip special keys like __overlaps__
        if rep["dead_paths"]:
            counts["DRIFTED-FACTUAL (dead multi-component paths)"] += 1
        if rep["imperatives"]:
            counts["OVER-IMPERATIVE (imperatives without rationale)"] += 1
        if rep["tools_issues"]:
            counts["TOOLS-MISMATCH (declared-but-unreferenced or unknown)"] += 1
        if rep["vague_kinds"]:
            counts["VAGUE-DESCRIPTION"] += 1
        if rep["stale"]:
            counts["STALE-MTIME (>90 days)"] += 1
        if rep["missing_model"]:
            counts["MISSING-MODEL"] += 1
    for k in ("DRIFTED-FACTUAL (dead multi-component paths)",
              "OVER-IMPERATIVE (imperatives without rationale)",
              "TOOLS-MISMATCH (declared-but-unreferenced or unknown)",
              "VAGUE-DESCRIPTION",
              "STALE-MTIME (>90 days)",
              "MISSING-MODEL"):
        out.append(f"| {k} | {counts.get(k, 0)} |\n")
    out.append(f"| Trigger-overlap pairs | {counts.get('TRIGGER-OVERLAP-PAIRS', 0)} |\n")

    # Trigger overlap section (cross-agent, separate from per-agent)
    overlaps = per_agent.get("__overlaps__", [])  # type: ignore
    if overlaps:
        out.append(f"\n## Trigger overlap ({len(overlaps)} pairs)\n\n")
        out.append("Distinctive description tokens shared between agents — "
                   "may indicate competing trigger surfaces.\n\n")
        for ov in overlaps:
            out.append(f"- **{ov.agent}** — {ov.detail}: {', '.join(ov.items)}\n")

    # Per-agent
    out.append("\n## Per-agent findings\n")
    for a in agents:
        rep = per_agent.get(a.name, {})
        if not any([rep.get("dead_paths"), rep.get("imperatives"),
                    rep.get("tools_issues"), rep.get("vague_kinds"),
                    rep.get("stale"), rep.get("missing_model")]):
            continue
        out.append(f"\n### `{a.rel}` — {a.body_lines} body lines, model={a.model or '—'}, "
                   f"tools={len(a.tools)}, mtime {a.mtime.date().isoformat()}\n")
        if rep.get("vague_kinds"):
            out.append(f"- **Vague description**: {rep['vague_kinds']}\n")
        if rep.get("stale"):
            out.append(f"- **Stale**: {rep['stale']}\n")
        if rep.get("missing_model"):
            out.append(f"- **Missing model**: {rep['missing_model']}\n")
        if rep.get("dead_paths"):
            out.append(f"- **Dead multi-component path citations** ({len(rep['dead_paths'])}):\n")
            for dp in rep["dead_paths"][:15]:
                out.append(f"  - `{dp}`\n")
            if len(rep["dead_paths"]) > 15:
                out.append(f"  - ...and {len(rep['dead_paths']) - 15} more.\n")
        if rep.get("imperatives"):
            out.append(f"- **Imperatives without rationale** ({len(rep['imperatives'])}):\n")
            for line_no, txt in rep["imperatives"][:8]:
                snippet = txt[:140] + ("…" if len(txt) > 140 else "")
                out.append(f"  - L{line_no}: {snippet}\n")
            if len(rep["imperatives"]) > 8:
                out.append(f"  - ...and {len(rep['imperatives']) - 8} more.\n")
        if rep.get("tools_issues"):
            out.append("- **Tools issues**:\n")
            for ti in rep["tools_issues"]:
                out.append(f"  - {ti}\n")

    out.append("\n---\n")
    out.append("_Read-only. Operator-gated. To act: edit flagged agents in their `.claude/agents/*.md` "
               "files. Dead-path false positives may exist (relative refs, paths-in-prose); spot-check "
               "before bulk-fixing._\n")
    return "".join(out)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output-dir", help=f"Override (default {DEFAULT_OUT_ROOT}/<YYYY-MM-DD>/)")
    args = parser.parse_args()

    agents = gather_agents()
    per_agent: dict[str, dict] = {a.name: {
        "dead_paths": [],
        "imperatives": [],
        "tools_issues": [],
        "vague_kinds": "",
        "stale": "",
        "missing_model": "",
    } for a in agents}

    # Vague descriptions
    for issue in find_vague_descriptions(agents):
        per_agent[issue.agent]["vague_kinds"] = issue.detail
    for issue in find_stale(agents):
        per_agent[issue.agent]["stale"] = issue.detail
    for issue in find_missing_model(agents):
        per_agent[issue.agent]["missing_model"] = issue.detail

    # Per-agent body checks
    for a in agents:
        per_agent[a.name]["dead_paths"] = find_dead_paths(a)
        per_agent[a.name]["imperatives"] = find_imperatives_without_rationale(a)
        per_agent[a.name]["tools_issues"] = find_tools_mismatch(a)

    # Cross-agent trigger overlap
    overlaps = find_trigger_overlap(agents)
    per_agent["__overlaps__"] = overlaps  # type: ignore

    out_dir = Path(args.output_dir) if args.output_dir else DEFAULT_OUT_ROOT / TODAY.isoformat()
    out_dir.mkdir(parents=True, exist_ok=True)
    out_path = out_dir / "agent-audit.md"
    out_path.write_text(render(agents, per_agent), encoding="utf-8")

    # Console summary
    print(f"audited: {len(agents)} agents")
    drifted = sum(1 for a in agents if per_agent[a.name]["dead_paths"])
    imperative = sum(1 for a in agents if per_agent[a.name]["imperatives"])
    tools_mismatch = sum(1 for a in agents if per_agent[a.name]["tools_issues"])
    vague = sum(1 for a in agents if per_agent[a.name]["vague_kinds"])
    stale_count = sum(1 for a in agents if per_agent[a.name]["stale"])
    print(f"  DRIFTED-FACTUAL: {drifted}")
    print(f"  OVER-IMPERATIVE: {imperative}")
    print(f"  TOOLS-MISMATCH:  {tools_mismatch}")
    print(f"  VAGUE-DESCRIPTION: {vague}")
    print(f"  STALE-MTIME:     {stale_count}")
    print(f"  trigger-overlap pairs: {len(overlaps)}")
    print(f"report: {out_path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
