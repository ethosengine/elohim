#!/usr/bin/env python3
"""
Deterministic duplicate-candidate scanner for memory topic files.

Scans all `*.md` topic files (excluding MEMORY.md index) under the memory
directory, computes TF-IDF cosine similarity across all pairs, and clusters
pairs above a similarity threshold. The output is a report listing each
cluster with similarity scores, member entries, and the top discriminative
shared terms — interpretable evidence the operator can review.

This tool is the candidate-detection layer for the `merge` lifecycle
primitive specified in:
  genesis/docs/superpowers/specs/2026-05-10-memory-lifecycle-design.md

Scope is read-only by design. The merge itself is a future tool.

Output: .claude/memory-kit/<YYYY-MM-DD>/dedupe-clusters.md
"""

from __future__ import annotations

import argparse
import math
import re
import sys
from collections import Counter, defaultdict
from dataclasses import dataclass, field
from datetime import date
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
MEMORY_ROOT = Path("/projects/.claude-config/projects/-projects-elohim/memory")
TODAY = date.today()

# Small built-in stopword list — common English glue and a few markdown noise words.
STOPWORDS = {
    "the", "a", "an", "is", "of", "to", "and", "in", "for", "with", "that",
    "this", "it", "as", "on", "at", "by", "from", "or", "are", "be", "was",
    "were", "not", "can", "if", "then", "but", "which", "you", "your", "our",
    "their", "they", "we", "i", "me", "my", "us", "him", "her", "his", "hers",
    "its", "no", "yes", "do", "does", "did", "done", "have", "has", "had",
    "will", "would", "should", "could", "may", "might", "must", "shall",
    "so", "than", "too", "very", "also", "any", "all", "some", "each",
    "more", "most", "less", "other", "such", "only", "own", "same",
    "what", "when", "where", "who", "whom", "how", "why", "while", "into",
    "out", "up", "down", "over", "under", "again", "further", "once",
    "here", "there", "now", "just", "like", "see", "use", "used", "using",
    "via", "per", "vs", "etc", "eg", "ie", "fix", "set", "get", "new", "old",
    "one", "two", "three", "four", "five", "first", "second", "still", "yet",
    "between", "across", "within", "without", "before", "after", "during",
    "about", "above", "below", "off", "because",
}

# Markdown / structural noise we want to strip before tokenizing.
CODE_FENCE_RE = re.compile(r"```.*?```", re.DOTALL)
INLINE_CODE_RE = re.compile(r"`[^`]*`")
FRONTMATTER_RE = re.compile(r"^---\n.*?\n---\n", re.DOTALL)
URL_RE = re.compile(r"https?://\S+")
WORD_RE = re.compile(r"[a-z][a-z0-9_-]{2,}")  # lowercase alphanum, length >= 3


@dataclass
class Doc:
    path: Path
    name: str  # short description from frontmatter `name:` field
    description: str  # frontmatter description (one-line)
    tokens: list[str] = field(default_factory=list)
    tf: Counter = field(default_factory=Counter)
    norm: float = 0.0


@dataclass
class Cluster:
    members: list[Doc]
    similarity: float  # representative score (max pairwise within cluster)
    pair_scores: list[tuple[str, str, float]]  # (name_a, name_b, score)
    top_terms: list[tuple[str, float]]  # (term, contribution)


def parse_frontmatter(text: str) -> tuple[dict[str, str], str]:
    """Return (frontmatter_dict, body_without_frontmatter)."""
    m = FRONTMATTER_RE.match(text)
    if not m:
        return {}, text
    raw = m.group(0)
    fm: dict[str, str] = {}
    for line in raw.strip("-\n").splitlines():
        if ":" in line:
            k, _, v = line.partition(":")
            fm[k.strip().lower()] = v.strip().strip("\"'")
    return fm, text[m.end():]


def strip_noise(body: str) -> str:
    body = CODE_FENCE_RE.sub(" ", body)
    body = INLINE_CODE_RE.sub(" ", body)
    body = URL_RE.sub(" ", body)
    return body


def tokenize(text: str) -> list[str]:
    text = text.lower()
    text = strip_noise(text)
    tokens = WORD_RE.findall(text)
    return [t for t in tokens if t not in STOPWORDS]


def load_docs(memory_dir: Path) -> list[Doc]:
    docs: list[Doc] = []
    for path in sorted(memory_dir.glob("*.md")):
        if path.name == "MEMORY.md":
            continue
        try:
            text = path.read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError):
            continue
        fm, body = parse_frontmatter(text)
        tokens = tokenize(body)
        if not tokens:
            continue
        docs.append(
            Doc(
                path=path,
                name=fm.get("name", path.stem),
                description=fm.get("description", ""),
                tokens=tokens,
                tf=Counter(tokens),
            )
        )
    return docs


def compute_idf(docs: list[Doc], df_cap: float) -> dict[str, float]:
    """Compute IDF and apply df-based stopword filtering.

    Terms appearing in >df_cap fraction of documents become noise (idf=0).
    """
    n = len(docs)
    df: Counter = Counter()
    for d in docs:
        for term in d.tf.keys():
            df[term] += 1
    idf: dict[str, float] = {}
    cap = df_cap * n
    for term, freq in df.items():
        if freq > cap:
            idf[term] = 0.0  # too common — silently drop
        else:
            # smoothed idf
            idf[term] = math.log((n + 1) / (freq + 1)) + 1.0
    return idf


def compute_tfidf_norms(docs: list[Doc], idf: dict[str, float]) -> None:
    """Replace doc.tf with tf-idf weights and compute L2 norm."""
    for d in docs:
        weighted = Counter()
        for term, count in d.tf.items():
            w = count * idf.get(term, 0.0)
            if w > 0:
                weighted[term] = w
        d.tf = weighted
        d.norm = math.sqrt(sum(v * v for v in weighted.values()))


def cosine(a: Doc, b: Doc) -> tuple[float, list[tuple[str, float]]]:
    """Return (cosine_similarity, top_contributing_terms)."""
    if a.norm == 0.0 or b.norm == 0.0:
        return 0.0, []
    # iterate the smaller vocab for efficiency
    if len(a.tf) > len(b.tf):
        a, b = b, a
    contributions: list[tuple[str, float]] = []
    dot = 0.0
    for term, wa in a.tf.items():
        wb = b.tf.get(term, 0.0)
        if wb > 0:
            prod = wa * wb
            dot += prod
            contributions.append((term, prod))
    sim = dot / (a.norm * b.norm)
    contributions.sort(key=lambda x: -x[1])
    return sim, contributions


def cluster_pairs(
    pairs: list[tuple[int, int, float, list[tuple[str, float]]]],
    docs: list[Doc],
) -> list[Cluster]:
    """Union-find clustering on pairs above threshold."""
    parent = list(range(len(docs)))

    def find(i: int) -> int:
        while parent[i] != i:
            parent[i] = parent[parent[i]]
            i = parent[i]
        return i

    def union(i: int, j: int) -> None:
        ra, rb = find(i), find(j)
        if ra != rb:
            parent[ra] = rb

    for i, j, _, _ in pairs:
        union(i, j)

    groups: dict[int, list[int]] = defaultdict(list)
    for idx in range(len(docs)):
        groups[find(idx)].append(idx)

    # Map cluster-root -> pair scores within
    cluster_pair_scores: dict[int, list[tuple[int, int, float, list[tuple[str, float]]]]] = defaultdict(list)
    for i, j, score, terms in pairs:
        cluster_pair_scores[find(i)].append((i, j, score, terms))

    clusters: list[Cluster] = []
    for root, member_idxs in groups.items():
        if len(member_idxs) < 2:
            continue
        member_pairs = cluster_pair_scores.get(root, [])
        # Aggregate top terms across all pairs in cluster (sum contributions)
        term_totals: Counter = Counter()
        for _, _, _, terms in member_pairs:
            for term, contrib in terms:
                term_totals[term] += contrib
        top_terms = term_totals.most_common(8)
        max_sim = max((p[2] for p in member_pairs), default=0.0)
        members = [docs[i] for i in member_idxs]
        pair_scores = [
            (docs[i].path.stem, docs[j].path.stem, s) for (i, j, s, _) in member_pairs
        ]
        clusters.append(
            Cluster(
                members=members,
                similarity=max_sim,
                pair_scores=sorted(pair_scores, key=lambda x: -x[2]),
                top_terms=top_terms,
            )
        )
    clusters.sort(key=lambda c: -c.similarity)
    return clusters


def render_report(
    clusters: list[Cluster],
    threshold: float,
    total_docs: int,
    total_pairs: int,
) -> str:
    out: list[str] = []
    out.append(f"# Memory Dedupe Clusters — {TODAY.isoformat()}")
    out.append("")
    out.append(
        "Generated by `.claude/scripts/memory-kit/dedupe-memory-scan.py`. "
        "**Read-only report.** This surfaces likely-duplicate clusters of "
        "memory entries — the candidate-detection layer for the `merge` "
        "primitive ("
        "`genesis/docs/superpowers/specs/2026-05-10-memory-lifecycle-design.md`)."
    )
    out.append("")
    out.append(f"- Memory entries scanned: **{total_docs}**")
    out.append(f"- Pairs above threshold: **{total_pairs}**")
    out.append(f"- Clusters found: **{len(clusters)}**")
    out.append(f"- Similarity threshold: **{threshold:.2f}** (cosine over TF-IDF)")
    out.append("")
    out.append("---")
    out.append("")

    if not clusters:
        out.append("_No clusters above threshold. Memory corpus is well-deduplicated by this measure._")
        out.append("")
        return "\n".join(out)

    out.append("## Clusters")
    out.append("")
    for i, c in enumerate(clusters, start=1):
        out.append(f"### Cluster {i} — similarity {c.similarity:.3f} ({len(c.members)} entries)")
        out.append("")
        out.append("**Entries**:")
        for d in c.members:
            try:
                rel = d.path.relative_to(REPO_ROOT)
                rel_str = str(rel)
            except ValueError:
                rel_str = str(d.path)
            desc = d.description or d.name
            # one-line trim
            desc = re.sub(r"\s+", " ", desc).strip()
            if len(desc) > 160:
                desc = desc[:157] + "..."
            out.append(f"- `{rel_str}` — {desc}")
        out.append("")
        if len(c.pair_scores) > 1:
            out.append("**Pairwise scores**:")
            for a, b, s in c.pair_scores[:6]:
                out.append(f"- {s:.3f} — `{a}` ↔ `{b}`")
            if len(c.pair_scores) > 6:
                out.append(f"- ...and {len(c.pair_scores) - 6} more pairs")
            out.append("")
        if c.top_terms:
            terms_str = ", ".join(f"`{t}`" for t, _ in c.top_terms)
            out.append(f"**Top shared terms**: {terms_str}")
            out.append("")
        # Suggested action heuristic
        if c.similarity >= 0.80:
            action = "review for merge — strong overlap; likely the same concept from different angles"
        elif c.similarity >= 0.65:
            action = "review and possibly compact one into the other — substantial overlap"
        else:
            action = "review for merge candidacy — moderate overlap; verify the entries are truly duplicative before merging"
        out.append(f"**Suggested action**: {action}")
        out.append("")
        out.append("---")
        out.append("")

    return "\n".join(out)


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Detect duplicate-candidate clusters in memory topic files.",
    )
    parser.add_argument(
        "--threshold",
        type=float,
        default=0.55,
        help="Cosine-similarity threshold for clustering (default: 0.55)",
    )
    parser.add_argument(
        "--top",
        type=int,
        default=None,
        help="Cap report at top-N clusters (default: report all)",
    )
    parser.add_argument(
        "--df-cap",
        type=float,
        default=0.70,
        help="Document-frequency stopword cap; terms in >cap fraction of docs are dropped (default: 0.70)",
    )
    parser.add_argument(
        "--memory-dir",
        type=Path,
        default=MEMORY_ROOT,
        help=f"Memory directory to scan (default: {MEMORY_ROOT})",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=None,
        help=f"Output directory (default: {REPO_ROOT}/.claude/memory-kit/<today>)",
    )
    args = parser.parse_args()

    memory_dir: Path = args.memory_dir
    if not memory_dir.exists():
        print(f"error: memory directory does not exist: {memory_dir}", file=sys.stderr)
        return 2

    out_dir: Path = args.output_dir or (REPO_ROOT / ".claude" / "memory-kit" / TODAY.isoformat())
    out_dir.mkdir(parents=True, exist_ok=True)

    docs = load_docs(memory_dir)
    if len(docs) < 2:
        print(f"only {len(docs)} memory entries — need at least 2 to compare", file=sys.stderr)
        return 1

    idf = compute_idf(docs, args.df_cap)
    compute_tfidf_norms(docs, idf)

    # Pairwise cosine
    above_threshold: list[tuple[int, int, float, list[tuple[str, float]]]] = []
    n = len(docs)
    for i in range(n):
        for j in range(i + 1, n):
            sim, contributions = cosine(docs[i], docs[j])
            if sim >= args.threshold:
                # Keep only top-12 contributing terms per pair (interpretability)
                above_threshold.append((i, j, sim, contributions[:12]))

    clusters = cluster_pairs(above_threshold, docs)
    if args.top is not None:
        clusters = clusters[: args.top]

    report = render_report(clusters, args.threshold, n, len(above_threshold))
    out_path = out_dir / "dedupe-clusters.md"
    out_path.write_text(report, encoding="utf-8")

    print(f"scanned: {n} memory entries")
    print(f"pairs above threshold ({args.threshold:.2f}): {len(above_threshold)}")
    print(f"clusters: {len(clusters)}")
    print(f"report: {out_path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
