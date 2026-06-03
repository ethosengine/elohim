"""subject_routing — the shared cascade-resolver + class-reconciler for subject-routed decomposition.

Read by BOTH gates (the brainstorm/plan FRONT gate via `_lib.subject_routing`, and decompose.py at the
BACK fire), so the single `class:` field flows front->back and the decompose router is multi-flow instead
of single-flow.

Parent constitution: <repo>/.claude/subject-routing.yaml (hand-tuned). Sub-trees may add their own to
cascade (one repo -> mono-repo -> submodule). The discriminator is DELIVERABLE-TARGET, never vocabulary.

Pure-stdlib except PyYAML (already a repo dep; memory-kit scripts use it). No side effects on import.
"""
from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path

try:
    import yaml
except ImportError:  # pragma: no cover - yaml is a repo dep
    yaml = None

MANIFEST_NAME = "subject-routing.yaml"

# Where a LANDED change lives decides the class (the discriminator). A path is product, process, or
# neutral. architecture/applications + horizons are product-vision; a2o/features/<pillar> is product
# (a learner scenario); a2o harness (scripts/steps/src) is process.
_PRODUCT_MARKERS = (
    "app/",
    "elohim/",
    "doorway/",
    "steward/",
    "sophia/",
    "genesis/docs/content/elohim-protocol/architecture/",
    "genesis/a2o/features/",
)
_PROCESS_MARKERS = (
    ".claude/",
    "genesis/docs/placement.md",
    "genesis/docs/_state/",
    "genesis/docs/claude.md",
    "genesis/orchestrator/",
    "genesis/a2o/claude.md",
    "genesis/a2o/scripts/",
    "genesis/a2o/steps/",
    "genesis/a2o/src/",
    "genesis/data/timeline/",
    # META/CONTRACT surfaces that live in the doc/architecture tree but are process surfaces
    # (the routing contract + the canonical map/index) — a change to these is process-meta even
    # though they sit under architecture/. Checked BEFORE the broad architecture/ product marker.
    "elohim-protocol/architecture/map.md",
    "elohim-protocol/architecture/index.md",
)


@dataclass
class Signal:
    id: str
    severity: str  # "hard" | "advisory"
    detail: str = ""


@dataclass
class Routing:
    """Merged routing config (root + any cascaded sub-tree manifests)."""

    classes: dict = field(default_factory=dict)
    artifact_kind: dict = field(default_factory=dict)
    gate_signals: list = field(default_factory=list)
    default_class: str = "protocol-canonical"
    sources: list = field(default_factory=list)  # the manifest paths merged, root-first


def _norm(path: str) -> str:
    # Strip a leading "./" or "/" PREFIX only — never a bare leading "." (that would corrupt ".claude/").
    p = str(path).strip().lower()
    if p.startswith("./"):
        p = p[2:]
    elif p.startswith("/"):
        p = p[1:]
    return p


def path_kind(path: str) -> str:
    """Classify a single target path: 'product' | 'process' | 'neutral'."""
    p = _norm(path)
    # root CLAUDE.md (process gospel) — bare 'claude.md' at repo root
    if p in ("claude.md",):
        return "process"
    # PROCESS markers are checked FIRST: they include specific meta-surfaces (MAP.md, INDEX.md,
    # PLACEMENT.md, a2o harness) that must win over the broad architecture/ product marker.
    for m in _PROCESS_MARKERS:
        if m in p:
            return "process"
    # architecture/applications + horizons are product (vision archetypes), still product.
    for m in _PRODUCT_MARKERS:
        if m in p:
            return "product"
    return "neutral"


def classify(targets, declared_class: str | None = None) -> str:
    """Infer class from where the LANDED change lives (the discriminator).

    Any product target -> protocol-canonical. All-process (>=1 process, 0 product) -> process-meta.
    No informative target -> provisional. `declared_class` is NOT trusted here (gate_check compares it);
    it only breaks a pure-neutral tie.
    """
    kinds = [path_kind(t) for t in (targets or [])]
    if "product" in kinds:
        return "protocol-canonical"
    if "process" in kinds:
        return "process-meta"
    if declared_class in ("protocol-canonical", "process-meta"):
        return declared_class
    return "provisional"


def reconcile(declared_class: str, residue_targets) -> str:
    """BACK-fire reconciliation: resolve `provisional` (or confirm a stamp) from actual residue."""
    inferred = classify(residue_targets, declared_class=declared_class)
    if declared_class in (None, "", "provisional"):
        return inferred
    return declared_class  # keep the stamp; gate_check surfaces a contradiction separately


def gate_check(fields: dict, targets) -> list:
    """Deterministic fail-loud mis-class detectors (mirrors subject-routing.yaml gate_signals)."""
    signals: list = []
    targets = targets or []
    kinds = [path_kind(t) for t in targets]
    domain = (fields.get("domain") or "").strip()
    has_product = "product" in kinds
    has_process = "process" in kinds
    cls = (fields.get("class") or "").strip()
    artifact_kind = (fields.get("artifact_kind") or "").strip()

    # vocab-vs-target-mismatch: a product domain stamped while every target is process (the D4 collision).
    if domain and has_process and not has_product:
        signals.append(
            Signal(
                "vocab-vs-target-mismatch",
                "hard",
                f"domain:{domain} set but all targets are process — drop domain:, set class:process-meta "
                f"(see history/2026-06-02-d4-name-collision)",
            )
        )

    if cls == "process-meta":
        for t in targets:
            tl = _norm(t)
            # forced-a2o-leg: only the pillar scenario tree, NOT the a2o harness.
            if "genesis/a2o/features/" in tl:
                signals.append(Signal("forced-a2o-leg", "hard", t))
            # forced-architecture-seed: a PRODUCT architecture SEED on process work. path_kind already
            # excludes the meta-surfaces (MAP.md / INDEX.md) that legitimately live under architecture/,
            # so amending the map is NOT a violation. Scoped OFF for a run-output ceremony-rewrite.
            if (
                path_kind(t) == "product"
                and "elohim-protocol/architecture/" in tl
                and "applications" not in tl
                and "horizons" not in tl
                and artifact_kind != "run-output"
            ):
                signals.append(Signal("forced-architecture-seed", "hard", t))

    if (fields.get("status") or "").strip() == "vision" and fields.get("_spawned_sprint"):
        signals.append(Signal("archetype-scheduled-a-sprint", "advisory", "archetype graduated to impl"))

    return signals


def targets_from_frontmatter(fm_fields: dict) -> list:
    """Extract the deliverable-TARGET paths the discriminator reads — i.e. where the landed change lives.

    `proposed_amendments` is the deliverable target (the files the spec amends / lands residue in). `cites`
    is prior-art LINEAGE (what it composes from) and `derived_from` is the dogfood breadcrumb — BOTH are
    excluded, because a cited/dogfooded product seed is lineage, not residue (conflating them is exactly
    the D4 name-collision this whole machine fixes). A spec with no `proposed_amendments` declares no
    target; the caller trusts its stamped `class:` (or leaves it provisional).
    """
    v = fm_fields.get("proposed_amendments")
    return list(v) if isinstance(v, list) else []


def _merge_into(base: Routing, raw: dict, source: str) -> None:
    if not isinstance(raw, dict):
        return
    base.sources.append(source)
    if "default_class" in raw:
        base.default_class = raw["default_class"]
    for cls, spec in (raw.get("classes") or {}).items():
        base.classes[cls] = spec  # nearest-wins on a class home-remap
    for kind, spec in (raw.get("artifact_kind") or {}).items():
        base.artifact_kind[kind] = spec
    existing = {s.get("id") for s in base.gate_signals if isinstance(s, dict)}
    for sig in raw.get("gate_signals") or []:
        if isinstance(sig, dict) and sig.get("id") not in existing:
            base.gate_signals.append(sig)


def find_repo_root(start: Path) -> Path:
    here = start.resolve()
    for _ in range(12):
        if (here / ".git").exists() or (here / ".claude" / MANIFEST_NAME).exists():
            return here
        if here.parent == here:
            break
        here = here.parent
    return start.resolve()


def load_routing(start_path, repo_root=None) -> Routing:
    """Walk UP from start_path to repo root, merging every `.claude/subject-routing.yaml` on the path.

    Root (farthest) is the base; nearer sub-tree manifests override (union classes, nearest-wins
    home-remaps) — the one-repo -> mono-repo -> submodule cascade. Returns a merged Routing.
    """
    start = Path(start_path).resolve()
    root = Path(repo_root).resolve() if repo_root else find_repo_root(start)
    # Collect manifests from root (farthest) down to nearest, so nearer merges LAST (wins).
    chain: list[Path] = []
    here = start if start.is_dir() else start.parent
    while True:
        m = here / ".claude" / MANIFEST_NAME
        if m.is_file():
            chain.append(m)
        if here == root or here.parent == here:
            break
        here = here.parent
    # also the repo-root manifest if not already (when start is outside a .claude subtree)
    root_m = root / ".claude" / MANIFEST_NAME
    if root_m.is_file() and root_m not in chain:
        chain.append(root_m)
    merged = Routing()
    for m in reversed(chain):  # root-first so nearer wins
        if yaml is None:
            break
        try:
            raw = yaml.safe_load(m.read_text(encoding="utf-8"))
        except Exception:
            continue
        _merge_into(merged, raw, str(m))
    if merged.default_class:
        pass
    return merged
