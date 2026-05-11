#!/usr/bin/env python3
"""
Skill catalog audit — read-only quality scan over .claude/skills/*/SKILL.md.

Skill metadata (name + description) is ALWAYS loaded into the model's context
(per progressive disclosure: descriptions always-loaded, bodies on-demand).
Weak descriptions and skill overlap directly cost tokens and clutter the
always-on context. This tool surfaces quality issues so the operator can
sharpen, merge, or retire skills.

Three issue classes:
  A. Vague descriptions — too short, no triggers, no when-language
  B. Trigger overlap with siblings — distinctive-keyword overlap > N
  C. Stale-by-mtime — not modified in > 90 days (informational)

Output: .claude/memory-kit/<YYYY-MM-DD>/skill-audit.md
No skill files modified. No apply step.
"""

from __future__ import annotations

import argparse
import re
import sys
from dataclasses import dataclass, field
from datetime import date, datetime
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
SKILLS_ROOT = REPO_ROOT / ".claude" / "skills"
DEFAULT_OUT_ROOT = REPO_ROOT / ".claude" / "memory-kit"

TODAY = date.today()

# Thresholds
MIN_DESCRIPTION_CHARS = 60
STALE_MTIME_DAYS = 90
TRIGGER_OVERLAP_THRESHOLD = 3  # >N distinctive shared words → flag

STOPWORDS = frozenset(
    """
    the a is of to and in for with that this it as on at by from or are be was were
    not can if then but which you your our their they we use when it's skill
    """.split()
)

# Generic phrasing patterns that flag a description as vague (no concrete triggers).
# Note: we DON'T flag "Reference for X" alone — many strong skills open that way
# and immediately follow with concrete quoted triggers.
GENERIC_PATTERNS = [
    re.compile(r"\bfor working with\b", re.IGNORECASE),
    re.compile(r"\buse this skill\b", re.IGNORECASE),
    re.compile(r"\bhelps? you\b", re.IGNORECASE),
    re.compile(r"\bgeneral(?:ly)? (?:purpose|useful)\b", re.IGNORECASE),
]

# Trigger-language signals: any presence of these tokens means "the description
# at least gestures toward when to fire."
TRIGGER_SIGNALS = re.compile(r"\b(use|when|trigger|invoke|before|after|whenever)\b", re.IGNORECASE)

# A description that includes quoted user utterances ("how do I X", "set up Y")
# has concrete triggers regardless of opening phrasing — exempts from "no-triggers".
QUOTED_TRIGGER_RE = re.compile(r'"[^"]{4,}"')

WORD_RE = re.compile(r"[a-zA-Z][a-zA-Z\-_/]*")

# Words that appear in this fraction of descriptions are scaffolding, not
# distinctive triggers — exclude them from overlap analysis.
DYNAMIC_STOPWORD_FRACTION = 0.25


@dataclass
class Skill:
    name: str
    path: Path
    description: str
    body_lines: int
    mtime: datetime


@dataclass
class VagueIssue:
    skill: Skill
    kinds: list[str]  # e.g. ["description-too-short", "description-no-when"]
    hint: str


@dataclass
class OverlapPair:
    a: Skill
    b: Skill
    shared: list[str]


@dataclass
class StaleSkill:
    skill: Skill
    age_days: int


def read_frontmatter(text: str) -> dict[str, str]:
    """Extract simple flat YAML frontmatter as a dict (lowercased keys)."""
    if not text.startswith("---"):
        return {}
    end = text.find("\n---", 3)
    if end < 0:
        return {}
    body = text[3:end].strip()
    out: dict[str, str] = {}
    current_key: str | None = None
    for line in body.splitlines():
        # Skip nested mapping continuations (e.g. "metadata:" sub-keys indented under)
        if line.startswith((" ", "\t")) and current_key:
            continue
        if ":" in line:
            k, _, v = line.partition(":")
            key = k.strip().lower()
            val = v.strip().strip("\"'")
            out[key] = val
            current_key = key
    return out


def load_skill(skill_md: Path) -> Skill | None:
    try:
        text = skill_md.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError):
        return None
    fm = read_frontmatter(text)
    name = fm.get("name", skill_md.parent.name)
    description = fm.get("description", "")
    body_after_fm = text.split("\n---", 1)[-1] if text.startswith("---") else text
    body_lines = sum(1 for _ in body_after_fm.splitlines())
    mtime = datetime.fromtimestamp(skill_md.stat().st_mtime)
    return Skill(name=name, path=skill_md, description=description, body_lines=body_lines, mtime=mtime)


def gather_skills() -> list[Skill]:
    skills: list[Skill] = []
    for child in sorted(SKILLS_ROOT.iterdir()):
        if not child.is_dir():
            continue
        skill_md = child / "SKILL.md"
        if not skill_md.exists():
            continue
        s = load_skill(skill_md)
        if s is not None:
            skills.append(s)
    return skills


# --- A. Vague descriptions ---------------------------------------------------


def diagnose_vague(skill: Skill) -> VagueIssue | None:
    kinds: list[str] = []
    desc = skill.description or ""
    desc_stripped = desc.strip()

    if not desc_stripped:
        kinds.append("description-missing")
    else:
        if len(desc_stripped) < MIN_DESCRIPTION_CHARS:
            kinds.append("description-too-short")
        if not TRIGGER_SIGNALS.search(desc_stripped):
            kinds.append("description-no-when")
        # description-no-triggers fires only if generic phrasing is present
        # AND the description has no quoted user-utterance examples.
        has_quoted_triggers = bool(QUOTED_TRIGGER_RE.search(desc_stripped))
        if not has_quoted_triggers:
            for pat in GENERIC_PATTERNS:
                if pat.search(desc_stripped):
                    kinds.append("description-no-triggers")
                    break

    if not kinds:
        return None

    hint_map = {
        "description-missing": "add a description with concrete triggers",
        "description-too-short": "expand to >=60 chars; name 2-3 representative trigger phrases",
        "description-no-when": 'add "Use when ..." plus 2-3 quoted user-utterance triggers',
        "description-no-triggers": "replace generic framing with concrete user-utterance examples",
    }
    # De-duplicate hints, preserve order
    seen = set()
    hints = []
    for k in kinds:
        h = hint_map.get(k)
        if h and h not in seen:
            seen.add(h)
            hints.append(h)
    return VagueIssue(skill=skill, kinds=kinds, hint="; ".join(hints))


# --- B. Trigger overlap ------------------------------------------------------


def raw_keywords(description: str) -> set[str]:
    """Extract candidate distinctive tokens (pre-corpus-stopword filter)."""
    if not description:
        return set()
    tokens = {t.lower() for t in WORD_RE.findall(description)}
    # Drop static stopwords and tokens shorter than 3 chars (noise)
    return {t for t in tokens if t not in STOPWORDS and len(t) >= 3}


def build_dynamic_stopwords(skills: list[Skill]) -> set[str]:
    """Words appearing in >= DYNAMIC_STOPWORD_FRACTION of descriptions are
    boilerplate scaffolding (e.g. 'reference', 'someone', 'asks') — drop them
    from distinctiveness analysis."""
    if not skills:
        return set()
    counts: dict[str, int] = {}
    for s in skills:
        for tok in raw_keywords(s.description):
            counts[tok] = counts.get(tok, 0) + 1
    threshold = max(2, int(len(skills) * DYNAMIC_STOPWORD_FRACTION))
    return {t for t, c in counts.items() if c >= threshold}


def find_overlap_pairs(skills: list[Skill]) -> tuple[list[OverlapPair], set[str]]:
    dynamic_stops = build_dynamic_stopwords(skills)
    keyword_sets = {
        s.name: raw_keywords(s.description) - dynamic_stops for s in skills
    }
    pairs: list[OverlapPair] = []
    for i, a in enumerate(skills):
        a_kw = keyword_sets[a.name]
        if not a_kw:
            continue
        for b in skills[i + 1 :]:
            b_kw = keyword_sets[b.name]
            if not b_kw:
                continue
            shared = sorted(a_kw & b_kw)
            if len(shared) > TRIGGER_OVERLAP_THRESHOLD:
                pairs.append(OverlapPair(a=a, b=b, shared=shared))
    # Highest-overlap pairs first — most actionable
    pairs.sort(key=lambda p: -len(p.shared))
    return pairs, dynamic_stops


# --- C. Stale-by-mtime -------------------------------------------------------


def find_stale(skills: list[Skill]) -> list[StaleSkill]:
    out: list[StaleSkill] = []
    for s in skills:
        age_days = (TODAY - s.mtime.date()).days
        if age_days > STALE_MTIME_DAYS:
            out.append(StaleSkill(skill=s, age_days=age_days))
    out.sort(key=lambda x: -x.age_days)
    return out


# --- Report ------------------------------------------------------------------


def relpath(p: Path) -> str:
    try:
        return str(p.relative_to(REPO_ROOT))
    except ValueError:
        return str(p)


def render_report(
    skills: list[Skill],
    vague: list[VagueIssue],
    overlaps: list[OverlapPair],
    stale: list[StaleSkill],
    dynamic_stops: set[str],
) -> str:
    out: list[str] = []
    out.append(f"# Skill Audit — {TODAY.isoformat()}")
    out.append("")
    out.append(
        "Generated by `.claude/scripts/memory-kit/skill-audit.py`. Read-only "
        "quality scan over `.claude/skills/*/SKILL.md`. Skill metadata is "
        "always-loaded into the model's context, so weak descriptions and "
        "trigger overlap directly cost tokens and noise. No files are modified; "
        "this report only diagnoses — sharpening / merging / retiring skills "
        "is operator judgment."
    )
    out.append("")
    out.append(f"- Skills audited: **{len(skills)}**")
    out.append(f"- Section A — vague descriptions: **{len(vague)}**")
    out.append(f"- Section B — trigger-overlap pairs: **{len(overlaps)}**")
    out.append(f"- Section C — stale-by-mtime (>{STALE_MTIME_DAYS} days): **{len(stale)}**")
    out.append("")
    out.append("---")
    out.append("")

    # Section A
    out.append("## A. Vague Descriptions")
    out.append("")
    if not vague:
        out.append("_No vague-description issues found._")
        out.append("")
    else:
        out.append(
            "These skills have descriptions that are too short, lack trigger "
            "language, or use generic framing. Sharpening helps the model "
            "decide when to fire and reduces context noise."
        )
        out.append("")
        for i, v in enumerate(vague, start=1):
            out.append(f"### {i}. `{v.skill.name}`")
            out.append("")
            out.append(f"- **Path**: `{relpath(v.skill.path)}`")
            issue_str = ", ".join(f"`{k}`" for k in v.kinds)
            out.append(f"- **Issues**: {issue_str}")
            desc_preview = v.skill.description or "_(missing)_"
            if len(desc_preview) > 240:
                desc_preview = desc_preview[:237] + "..."
            out.append(f"- **Current description** ({len(v.skill.description)} chars): {desc_preview}")
            out.append(f"- **Suggested direction**: {v.hint}")
            out.append("")

    out.append("---")
    out.append("")

    # Section B
    out.append("## B. Trigger Overlap")
    out.append("")
    if not overlaps:
        out.append("_No overlap pairs above threshold._")
        out.append("")
    else:
        out.append(
            f"Pairs sharing more than {TRIGGER_OVERLAP_THRESHOLD} distinctive "
            "keywords in their descriptions. Two skills competing for the same "
            "conversational cue split selection and waste context. **Suggested "
            "action**: review for merge or differentiation (sharpen one toward "
            "a narrower trigger, or fold them together)."
        )
        out.append("")
        if dynamic_stops:
            stop_list = ", ".join(f"`{w}`" for w in sorted(dynamic_stops))
            out.append(
                f"_Corpus-derived scaffolding excluded from overlap analysis "
                f"(words appearing in >={int(DYNAMIC_STOPWORD_FRACTION*100)}% "
                f"of descriptions): {stop_list}._"
            )
            out.append("")
        for i, p in enumerate(overlaps, start=1):
            out.append(f"### {i}. `{p.a.name}` <-> `{p.b.name}` ({len(p.shared)} shared)")
            out.append("")
            out.append(f"- **Shared keywords**: {', '.join(f'`{w}`' for w in p.shared)}")
            out.append(f"- **{p.a.name}** (`{relpath(p.a.path)}`):")
            a_desc = p.a.description if p.a.description else "_(missing)_"
            if len(a_desc) > 200:
                a_desc = a_desc[:197] + "..."
            out.append(f"  > {a_desc}")
            out.append(f"- **{p.b.name}** (`{relpath(p.b.path)}`):")
            b_desc = p.b.description if p.b.description else "_(missing)_"
            if len(b_desc) > 200:
                b_desc = b_desc[:197] + "..."
            out.append(f"  > {b_desc}")
            out.append("")

    out.append("---")
    out.append("")

    # Section C
    out.append("## C. Stale-by-mtime")
    out.append("")
    if not stale:
        out.append(f"_No skills older than {STALE_MTIME_DAYS} days._")
        out.append("")
    else:
        out.append(
            f"Skills not modified in > {STALE_MTIME_DAYS} days. **Informational "
            "only** — a skill may be perfectly load-bearing and just stable. "
            "Worth a glance to confirm the trigger language still matches "
            "current usage patterns."
        )
        out.append("")
        for i, s in enumerate(stale, start=1):
            mtime_str = s.skill.mtime.date().isoformat()
            out.append(
                f"{i}. `{s.skill.name}` — last modified **{mtime_str}** "
                f"({s.age_days} days ago) — `{relpath(s.skill.path)}`"
            )
        out.append("")

    return "\n".join(out)


# --- Main --------------------------------------------------------------------


def main() -> int:
    global TRIGGER_OVERLAP_THRESHOLD, STALE_MTIME_DAYS, SKILLS_ROOT

    parser = argparse.ArgumentParser(description=__doc__.splitlines()[1] if __doc__ else "")
    parser.add_argument(
        "--skills-root",
        type=Path,
        default=SKILLS_ROOT,
        help=f"Skills directory to scan (default: {SKILLS_ROOT})",
    )
    parser.add_argument(
        "--out-root",
        type=Path,
        default=DEFAULT_OUT_ROOT,
        help=f"Output root for dated report directory (default: {DEFAULT_OUT_ROOT})",
    )
    parser.add_argument(
        "--overlap-threshold",
        type=int,
        default=TRIGGER_OVERLAP_THRESHOLD,
        help=f"Distinctive-keyword overlap threshold (default: {TRIGGER_OVERLAP_THRESHOLD})",
    )
    parser.add_argument(
        "--stale-days",
        type=int,
        default=STALE_MTIME_DAYS,
        help=f"Stale-by-mtime threshold in days (default: {STALE_MTIME_DAYS})",
    )
    args = parser.parse_args()

    TRIGGER_OVERLAP_THRESHOLD = args.overlap_threshold
    STALE_MTIME_DAYS = args.stale_days
    SKILLS_ROOT = args.skills_root

    if not SKILLS_ROOT.exists():
        print(f"error: skills root does not exist: {SKILLS_ROOT}", file=sys.stderr)
        return 2

    skills = gather_skills()
    if not skills:
        print(f"error: no SKILL.md files found under {SKILLS_ROOT}", file=sys.stderr)
        return 2

    vague = [v for v in (diagnose_vague(s) for s in skills) if v is not None]
    overlaps, dynamic_stops = find_overlap_pairs(skills)
    stale = find_stale(skills)

    out_dir = args.out_root / TODAY.isoformat()
    out_dir.mkdir(parents=True, exist_ok=True)
    out_path = out_dir / "skill-audit.md"
    out_path.write_text(
        render_report(skills, vague, overlaps, stale, dynamic_stops),
        encoding="utf-8",
    )

    print(
        f"audited {len(skills)} skills | "
        f"vague: {len(vague)} | "
        f"overlap-pairs: {len(overlaps)} | "
        f"stale-by-mtime: {len(stale)} | "
        f"report: {out_path}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
