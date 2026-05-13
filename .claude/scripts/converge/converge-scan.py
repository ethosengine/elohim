#!/usr/bin/env python3
"""
Phase 1 of /converge: deterministic theme detection across the dev corpus.

v2: Classical IR stack — TF-IDF over filename + section headings, bigram
extraction, document-frequency auto-stopwords, TextRank co-occurrence
graph centrality. Pure stdlib. ~10x signal-over-noise vs v1's
filename-token frequency.

Reads:
  - Active plans (filename + section headings)
  - Memory topic files (filename + frontmatter description + headings)
  - Sprint-result files (title + section headings)
  - The latest dated memory-kit reports (cleanup-backlog-refresh.md,
    dedupe-clusters.md, sprint-digest.md, path-update-proposals.md) for
    item assignment

Outputs: .claude/memory-kit/<YYYY-MM-DD>/convergence-themes.md

Phase 2 (synthesis subagent) consumes this — see memory-kit SKILL.md.
"""

from __future__ import annotations

import argparse
import math
import re
from collections import Counter, defaultdict
from dataclasses import dataclass, field
from datetime import date
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
MEMORY_KIT_ROOT = REPO_ROOT / ".claude" / "memory-kit"
MEMORY_TOPICS_ROOT = Path("/projects/.claude-config/projects/-projects-elohim/memory")
PLAN_GLOBS = [
    "genesis/docs/plans/*.md",
    "genesis/docs/superpowers/plans/*.md",
]
SPRINT_GLOBS = [
    ".claude/shifts/*.md",
    ".claude/shifts/*/sprint-result.md",
]

# Tokens to forcibly exclude — bookkeeping vocabulary that auto-DF won't catch
# (because it appears in moderate-frequency contexts but is never a "theme")
BASE_STOPWORDS = {
    "plan", "design", "spec", "kickoff", "prompt", "session", "review",
    "draft", "phase", "task", "tasks", "step", "steps", "addendum",
    "notes", "todo", "tdd", "impl", "implementation", "doc", "ready",
    "wip", "the", "and", "for", "with", "from", "into", "out", "off",
    "ups", "down", "use", "using", "via", "between", "across", "this",
    "that", "these", "those", "what", "when", "where", "how", "why",
    "make", "made", "build", "built", "test", "tests", "fix", "fixed",
    "elohim", "protocol", "sprint", "shift", "deliver", "shifts",
    "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
    # numerics / version fragments
    "v1", "v2", "v3", "v4", "v5", "rev", "dev", "main",
    # generic English / negations / modal-verbs that survive tokenization
    "not", "yes", "all", "any", "are", "can", "did", "had", "has", "have",
    "her", "him", "his", "its", "let", "may", "now", "one", "our", "she",
    "two", "way", "you", "your", "but", "get", "got", "him", "his", "etc",
    "per", "yet", "off", "set", "see", "say", "saw", "say", "ago", "due",
}

FILENAME_TOKEN_RE = re.compile(r"[a-z0-9]+")
DATE_PREFIX_RE = re.compile(r"^\d{4}-\d{2}-\d{2}-")
HEADING_RE = re.compile(r"^#+\s+(.+?)$", re.MULTILINE)
FRONTMATTER_RE = re.compile(r"^---\n(.+?)\n---", re.DOTALL)


# ---------- Document representation ----------


def strip_frontmatter_and_code(text: str) -> tuple[str, str]:
    """Return (frontmatter_body, content_without_frontmatter_or_code)."""
    fm = FRONTMATTER_RE.search(text)
    fm_body = fm.group(1) if fm else ""
    content = text[fm.end():] if fm else text
    # Drop fenced code blocks
    content = re.sub(r"```.*?```", "", content, flags=re.DOTALL)
    # Drop inline code
    content = re.sub(r"`[^`]+`", "", content)
    return fm_body, content


def extract_doc_text(path: Path) -> str:
    """Pull tokenizable text from a doc: filename + frontmatter description + headings.

    Filename tokens get triple-weight (repeated 3 times in the output) since
    titles are the strongest topical signal.
    """
    try:
        text = path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError):
        return ""
    fm_body, content = strip_frontmatter_and_code(text)

    name = DATE_PREFIX_RE.sub("", path.stem)
    name_tokens = " ".join(FILENAME_TOKEN_RE.findall(name.lower()))

    # Frontmatter description / name field, if present
    fm_desc = ""
    for line in fm_body.splitlines():
        if line.lower().startswith(("name:", "description:", "title:")):
            _, _, val = line.partition(":")
            fm_desc += " " + val.strip().strip("\"'")

    # Section headings — the topical spine of the document
    headings = " ".join(HEADING_RE.findall(content))

    # Triple-weight the filename (titles beat body content for topicality)
    return f"{name_tokens} {name_tokens} {name_tokens} {fm_desc} {headings}".lower()


def tokenize(text: str) -> list[str]:
    return FILENAME_TOKEN_RE.findall(text.lower())


def tokenize_with_bigrams(text: str) -> list[str]:
    """Return unigrams plus bigrams (joined with single space).

    Stopwords are filtered at the unigram level; bigrams are kept even if
    they contain stopwords (since "phase 12" or "cross stack" are valid
    themes despite "phase" being stopworded as a unigram).
    """
    tokens = tokenize(text)
    out = []
    for tok in tokens:
        if len(tok) >= 3 and tok not in BASE_STOPWORDS and not tok.isdigit():
            out.append(tok)
    # Bigrams from the raw token sequence (so stopword unigrams still form bigrams)
    for i in range(len(tokens) - 1):
        a, b = tokens[i], tokens[i + 1]
        if len(a) >= 2 and len(b) >= 2 and not a.isdigit() and not b.isdigit():
            # Both halves of the bigram must contribute SOMETHING — drop pure-bookkeeping bigrams
            if a in BASE_STOPWORDS and b in BASE_STOPWORDS:
                continue
            out.append(f"{a} {b}")
    return out


# ---------- TF-IDF ----------


@dataclass
class DocTerms:
    doc_id: str  # human-readable doc identifier
    counts: Counter  # term -> count


def compute_doc_terms(docs: list[tuple[str, str]]) -> list[DocTerms]:
    """For each (doc_id, text), return the term Counter."""
    out = []
    for doc_id, text in docs:
        terms = tokenize_with_bigrams(text)
        if terms:
            out.append(DocTerms(doc_id, Counter(terms)))
    return out


def compute_df(doc_terms: list[DocTerms]) -> Counter:
    """Document frequency per term."""
    df: Counter = Counter()
    for dt in doc_terms:
        for term in dt.counts:
            df[term] += 1
    return df


def auto_stopwords(df: Counter, n_docs: int, df_cap_pct: float) -> set[str]:
    """Terms appearing in more than df_cap_pct of documents are corpus-derived noise."""
    cap = max(2, int(n_docs * df_cap_pct))
    return {term for term, count in df.items() if count > cap}


def compute_tfidf(doc_terms: list[DocTerms], df: Counter, n_docs: int,
                  exclude: set[str]) -> dict[str, dict[str, float]]:
    """Return {term: {doc_id: tfidf_score}}.

    IDF uses smoothed log: log((n_docs + 1) / (df + 1)) + 1 (so IDF is positive).
    """
    out: dict[str, dict[str, float]] = defaultdict(dict)
    for dt in doc_terms:
        # Length normalization (sqrt of total terms in doc)
        norm = math.sqrt(sum(dt.counts.values())) or 1.0
        for term, tf in dt.counts.items():
            if term in exclude:
                continue
            idf = math.log((n_docs + 1) / (df[term] + 1)) + 1
            out[term][dt.doc_id] = (tf / norm) * idf
    return out


def aggregate_tfidf(tfidf: dict[str, dict[str, float]]) -> dict[str, float]:
    """Per-term aggregate TF-IDF score — sum across all docs."""
    return {term: sum(scores.values()) for term, scores in tfidf.items()}


# ---------- TextRank co-occurrence ----------


def build_cooccurrence(tfidf: dict[str, dict[str, float]],
                       candidates: set[str]) -> dict[tuple[str, str], float]:
    """For each pair of candidate terms appearing in the same doc, accumulate
    weighted co-occurrence. Weight = sqrt(tfidf_a * tfidf_b) per shared doc."""
    # Invert: doc_id -> [(term, tfidf), ...] for candidates only
    doc_to_terms: dict[str, list[tuple[str, float]]] = defaultdict(list)
    for term, doc_scores in tfidf.items():
        if term not in candidates:
            continue
        for doc_id, score in doc_scores.items():
            doc_to_terms[doc_id].append((term, score))

    edges: dict[tuple[str, str], float] = defaultdict(float)
    for doc_id, terms_in_doc in doc_to_terms.items():
        n = len(terms_in_doc)
        for i in range(n):
            ti, si = terms_in_doc[i]
            for j in range(i + 1, n):
                tj, sj = terms_in_doc[j]
                key = tuple(sorted([ti, tj]))
                edges[key] += math.sqrt(si * sj)
    return edges


def textrank(candidates: set[str], edges: dict[tuple[str, str], float],
             damping: float = 0.85, iterations: int = 30,
             tol: float = 1e-4) -> dict[str, float]:
    """Standard PageRank-style iteration over the co-occurrence graph."""
    # Build adjacency: term -> [(neighbor, weight)]
    adj: dict[str, list[tuple[str, float]]] = defaultdict(list)
    for (a, b), w in edges.items():
        adj[a].append((b, w))
        adj[b].append((a, w))

    n = len(candidates)
    if n == 0:
        return {}
    score = {c: 1.0 / n for c in candidates}
    out_weight = {c: sum(w for _, w in adj[c]) or 1.0 for c in candidates}

    for _ in range(iterations):
        new_score = {c: (1 - damping) / n for c in candidates}
        for c in candidates:
            for nbr, w in adj[c]:
                new_score[nbr] += damping * score[c] * (w / out_weight[c])
        delta = sum(abs(new_score[c] - score[c]) for c in candidates)
        score = new_score
        if delta < tol:
            break
    return score


# ---------- Item assignment (consume memory-kit reports) ----------


@dataclass
class Theme:
    keyword: str  # unigram or bigram (bigram has space)
    tfidf_score: float = 0.0
    centrality: float = 0.0
    plans: set[str] = field(default_factory=set)
    backlog_items: list[str] = field(default_factory=list)
    dedupe_clusters: list[str] = field(default_factory=list)
    open_questions: list[str] = field(default_factory=list)
    path_renames: list[str] = field(default_factory=list)

    def total_signal(self) -> int:
        return (
            len(self.plans) * 3
            + len(self.backlog_items) * 2
            + len(self.dedupe_clusters)
            + len(self.open_questions)
            + len(self.path_renames)
        )

    def composite_score(self) -> float:
        """Final ranking signal: TF-IDF importance × TextRank centrality × signal density."""
        return math.sqrt(self.tfidf_score) * (1 + self.centrality * 100) * (1 + self.total_signal() * 0.05)

    def source_type_count(self) -> int:
        return sum([
            1 if self.plans else 0,
            1 if self.backlog_items else 0,
            1 if self.dedupe_clusters else 0,
            1 if self.open_questions else 0,
            1 if self.path_renames else 0,
        ])


def make_theme_matcher(keyword: str) -> re.Pattern:
    """Build a word-boundary regex for a unigram or bigram theme."""
    parts = re.split(r"\s+", keyword.strip())
    pattern = r"\b" + r"[\s\-_]+".join(re.escape(p) for p in parts) + r"\b"
    return re.compile(pattern, re.IGNORECASE)


def parse_backlog_refresh(text: str) -> list[tuple[str, str]]:
    out = []
    pattern = re.compile(
        r"^##\s+\d+\.\s+`([^`]+)`\s*$\n+\*\*What's unfinished:\*\*\s*(.+?)(?=\n\n)",
        re.MULTILINE | re.DOTALL,
    )
    for m in pattern.finditer(text):
        out.append((m.group(1).strip(), m.group(2).strip()))
    return out


def parse_dedupe_clusters(text: str) -> list[str]:
    out = []
    pattern = re.compile(r"^#{2,3}\s+Cluster\s+(\d+).+?(?=\n#{2,3}|\Z)", re.MULTILINE | re.DOTALL)
    for m in pattern.finditer(text):
        excerpt = m.group(0).strip()
        if len(excerpt) > 800:
            excerpt = excerpt[:800] + "..."
        out.append(excerpt)
    return out


def parse_open_questions(text: str) -> list[str]:
    pattern = re.compile(
        r"^##+\s+(?:Aggregated\s+)?Open\s+Questions.*?(?=\n##+\s|\Z)",
        re.MULTILINE | re.DOTALL | re.IGNORECASE,
    )
    m = pattern.search(text)
    if not m:
        return []
    section = m.group(0)
    questions = re.findall(r"^[\t ]*[-*]\s+(.+?)$", section, re.MULTILINE)
    return [q.strip() for q in questions if len(q.strip()) > 10]


def parse_path_update(text: str) -> list[str]:
    out = []
    for line in text.splitlines():
        if "→" in line or "->" in line:
            stripped = line.strip().lstrip("-* ").strip()
            if stripped:
                out.append(stripped)
    return out[:200]


def assign_to_themes(themes: dict[str, Theme], plans: list[Path],
                     backlog: list[tuple[str, str]],
                     dedupe: list[str],
                     questions: list[str],
                     renames: list[str]) -> None:
    matchers = {kw: make_theme_matcher(kw) for kw in themes}

    for plan in plans:
        # Plan title (filename) match
        plan_text = plan.stem.replace("-", " ").replace("_", " ")
        for kw, mat in matchers.items():
            if mat.search(plan_text):
                rel = plan.relative_to(REPO_ROOT) if plan.is_relative_to(REPO_ROOT) else plan
                themes[kw].plans.add(str(rel))

    for path, unfinished in backlog:
        for kw, mat in matchers.items():
            if mat.search(path) or mat.search(unfinished):
                themes[kw].backlog_items.append(path)

    for excerpt in dedupe:
        for kw, mat in matchers.items():
            if mat.search(excerpt):
                themes[kw].dedupe_clusters.append(excerpt[:200])

    for q in questions:
        for kw, mat in matchers.items():
            if mat.search(q):
                themes[kw].open_questions.append(q)

    for rename in renames:
        for kw, mat in matchers.items():
            if mat.search(rename):
                themes[kw].path_renames.append(rename)


def find_canonical_plan(theme_kw: str, all_plans: list[Path]) -> Path | None:
    matcher = make_theme_matcher(theme_kw)
    candidates = []
    for plan in all_plans:
        plan_text = plan.stem.replace("-", " ").replace("_", " ")
        if matcher.search(plan_text):
            candidates.append((plan.stat().st_mtime, plan))
    if not candidates:
        return None
    candidates.sort(reverse=True)
    return candidates[0][1]


# ---------- Output ----------


def render_themes(themes: list[Theme], all_plans: list[Path]) -> str:
    today = date.today().isoformat()
    out = []
    out.append(f"# Convergence Themes — {today}")
    out.append("")
    out.append(
        "Generated by `.claude/scripts/memory-kit/converge-scan.py` (v2 — TF-IDF + bigrams + auto-stopwords + TextRank centrality). "
        "Phase 1 of `/converge`. Phase 2 (synthesis subagent dispatch) consumes this."
    )
    out.append("")
    out.append(f"- Themes surfaced: **{len(themes)}**")
    out.append("- Composite score = √(TF-IDF aggregate) × (1 + TextRank centrality × 100) × (1 + signal × 0.05)")
    out.append("")
    out.append("---")
    out.append("")

    for i, theme in enumerate(themes, start=1):
        canonical = find_canonical_plan(theme.keyword, all_plans)
        canonical_rel = (
            canonical.relative_to(REPO_ROOT)
            if canonical and canonical.is_relative_to(REPO_ROOT)
            else canonical
        )
        out.append(f"## {i}. Theme: **{theme.keyword}**")
        out.append("")
        out.append(
            f"- TF-IDF aggregate: {theme.tfidf_score:.3f}"
            f" | TextRank centrality: {theme.centrality:.4f}"
            f" | Item signal: {theme.total_signal()}"
            f" | Composite: {theme.composite_score():.2f}"
        )
        out.append("")
        out.append(f"**Canonical plan candidate**: `{canonical_rel}`" if canonical else "**Canonical plan candidate**: _none found_")
        out.append("")
        if theme.plans:
            out.append(f"**Plans matching this theme**: {len(theme.plans)}")
            for p in sorted(theme.plans)[:8]:
                out.append(f"- `{p}`")
            if len(theme.plans) > 8:
                out.append(f"- ...and {len(theme.plans) - 8} more")
            out.append("")
        if theme.backlog_items:
            out.append(f"**BACKLOG items**: {len(theme.backlog_items)}")
            for p in theme.backlog_items[:8]:
                out.append(f"- `{p}`")
            if len(theme.backlog_items) > 8:
                out.append(f"- ...and {len(theme.backlog_items) - 8} more")
            out.append("")
        if theme.dedupe_clusters:
            out.append(f"**Dedupe clusters touching this theme**: {len(theme.dedupe_clusters)}")
            out.append("")
        if theme.open_questions:
            out.append(f"**Open questions touching this theme**: {len(theme.open_questions)}")
            for q in theme.open_questions[:4]:
                out.append(f"- {q}")
            if len(theme.open_questions) > 4:
                out.append(f"- ...and {len(theme.open_questions) - 4} more")
            out.append("")
        if theme.path_renames:
            out.append(f"**Path renames touching this theme**: {len(theme.path_renames)}")
            out.append("")
        out.append("---")
        out.append("")

    if not themes:
        out.append("_No themes surfaced. Try lowering --min-composite or running memory-kit scans first._")

    return "\n".join(out)


# ---------- Main ----------


def latest_memory_kit_date() -> date | None:
    if not MEMORY_KIT_ROOT.exists():
        return None
    dates = []
    for entry in MEMORY_KIT_ROOT.iterdir():
        if entry.is_dir():
            try:
                dates.append(date.fromisoformat(entry.name))
            except ValueError:
                continue
    return max(dates) if dates else None


def gather_plans() -> list[Path]:
    plans = []
    for pattern in PLAN_GLOBS:
        plans.extend(REPO_ROOT.glob(pattern))
    return sorted(plans)


def gather_corpus() -> list[tuple[str, str]]:
    """Build the (doc_id, text) corpus for theme discovery.

    Includes plans (filename + headings), memory entries (filename + frontmatter
    + headings), sprint files (filename + headings).
    """
    docs = []
    for plan in gather_plans():
        text = extract_doc_text(plan)
        if text.strip():
            docs.append((str(plan.relative_to(REPO_ROOT)), text))
    if MEMORY_TOPICS_ROOT.exists():
        for mem in sorted(MEMORY_TOPICS_ROOT.glob("*.md")):
            if mem.name == "MEMORY.md":
                continue
            text = extract_doc_text(mem)
            if text.strip():
                docs.append((mem.name, text))
    for pattern in SPRINT_GLOBS:
        for sprint in sorted(REPO_ROOT.glob(pattern)):
            text = extract_doc_text(sprint)
            if text.strip():
                docs.append((str(sprint.relative_to(REPO_ROOT)), text))
    return docs


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reports-date", help="Date dir under .claude/memory-kit/ (default: latest)")
    parser.add_argument("--df-cap-pct", type=float, default=0.40,
                        help="Auto-stopword: terms in >this fraction of docs are noise (default 0.40)")
    parser.add_argument("--min-composite", type=float, default=0.5,
                        help="Min composite score for a theme to surface (default 0.5)")
    parser.add_argument("--min-source-types", type=int, default=2,
                        help="Min number of report types touching the theme (default 2)")
    parser.add_argument("--top", type=int, default=12,
                        help="Cap themes to top N by composite score (default 12)")
    parser.add_argument("--out-dir", help="Output dir (default: same as reports-date)")
    args = parser.parse_args()

    reports_date = (
        date.fromisoformat(args.reports_date) if args.reports_date
        else latest_memory_kit_date()
    )
    if reports_date is None:
        print("error: no dated memory-kit directories found; run memory-kit tools first",
              file=__import__("sys").stderr)
        return 1
    reports_dir = MEMORY_KIT_ROOT / reports_date.isoformat()
    if not reports_dir.exists():
        print(f"error: reports dir not found: {reports_dir}", file=__import__("sys").stderr)
        return 1

    out_dir = Path(args.out_dir) if args.out_dir else reports_dir
    out_dir.mkdir(parents=True, exist_ok=True)

    # ----- Theme discovery (TF-IDF + bigrams + auto-stopwords + TextRank) -----
    corpus = gather_corpus()
    n_docs = len(corpus)
    doc_terms = compute_doc_terms(corpus)
    df = compute_df(doc_terms)
    auto_stops = auto_stopwords(df, n_docs, args.df_cap_pct)
    excluded = auto_stops | BASE_STOPWORDS

    tfidf = compute_tfidf(doc_terms, df, n_docs, excluded)
    tfidf_agg = aggregate_tfidf(tfidf)

    # Restrict candidate set BEFORE building the graph (cheap pruning)
    # Boost bigrams: phrasal themes (e.g. "alpha cluster", "cross stack") have
    # naturally lower TF-IDF aggregate than unigrams (they appear less often)
    # but carry stronger topical signal — boost their score for candidate pool ranking
    def candidate_key(item: tuple[str, float]) -> float:
        term, score = item
        is_bigram = " " in term
        return -(score * 1.6 if is_bigram else score)
    sorted_candidates = sorted(tfidf_agg.items(), key=candidate_key)
    candidate_terms = {term for term, _ in sorted_candidates[:400]}

    edges = build_cooccurrence(tfidf, candidate_terms)
    centrality = textrank(candidate_terms, edges)

    # Build Theme objects from candidates
    themes: dict[str, Theme] = {}
    for term in candidate_terms:
        themes[term] = Theme(
            keyword=term,
            tfidf_score=tfidf_agg.get(term, 0.0),
            centrality=centrality.get(term, 0.0),
        )

    # ----- Item assignment from memory-kit reports -----
    def read_report(name: str) -> str:
        p = reports_dir / name
        return p.read_text(encoding="utf-8") if p.exists() else ""

    backlog = parse_backlog_refresh(read_report("cleanup-backlog-refresh.md"))
    dedupe = parse_dedupe_clusters(read_report("dedupe-clusters.md"))
    questions = parse_open_questions(read_report("sprint-digest.md"))
    renames = parse_path_update(read_report("path-update-proposals.md"))

    plans = gather_plans()
    assign_to_themes(themes, plans, backlog, dedupe, questions, renames)

    # ----- Final filter + rank -----
    candidates = [
        t for t in themes.values()
        if t.composite_score() >= args.min_composite
        and t.source_type_count() >= args.min_source_types
    ]
    candidates.sort(key=lambda t: -t.composite_score())
    ranked = candidates[: args.top]

    output = render_themes(ranked, plans)
    out_path = out_dir / "convergence-themes.md"
    out_path.write_text(output, encoding="utf-8")

    print(f"reports date: {reports_date}")
    print(f"corpus: {n_docs} docs (plans+memory+sprint)")
    print(f"vocab: {len(tfidf_agg)} terms after auto-stopwords ({len(auto_stops)} terms df-filtered)")
    print(f"candidate pool: {len(candidate_terms)} (top by aggregate TF-IDF)")
    print(f"co-occurrence edges: {len(edges)}")
    print(f"themes surfaced (composite>={args.min_composite}, ≥{args.min_source_types} source types): {len(ranked)}")
    print(f"output: {out_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
