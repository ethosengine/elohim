"""Authority-in-integrity source-shape probe — is standing authenticated where every validator runs?

WHY THIS EXISTS. `arch-authority-in-integrity-backlog.md` (2026-09-19, whole-DNA audit) found one
systemic placement error across five DNAs: the rules that decide WHO may claim standing over WHOM
(a stewardship grant, a Steward role, a portal host, a content server, a node registration, a peer
status) live in COORDINATOR zomes, which are hot-swappable application code — `update_coordinators`
lets any peer replace them without a network event. An integrity zome that never consults
`action.author()` cannot refuse a counterfeit no matter how well-behaved the coordinator is.

This is the FIRST runnable check the `authority-in-integrity` habit names (its `first_move`,
option 1: "a source-shape test in the DNA workspace ... which is deterministic, runs in the
existing DNA gate, and fails today"). It measures three known-open SHAPES, textually, across the
five LIVE integrity zomes (elohim/imagodei/infrastructure/node-registry/mishpat — NOT lamad-v1,
NOT hrea, which are archived/placeholder and carry no live standing):

  L1  an unconditional open link arm — `FlatOp::Link(OpLink::CreateLink { .. })` (or
      `DeleteLink`) destructured with `{ .. }` and returned `Ok(ValidateCallbackResult::Valid)`
      without ever looking at `link_type`, the link's base, or its author. A link's base is
      exactly where an ownership claim is encoded (`AgentToPeerStatus`, `StewardToGrant`,
      `PortalHosts`, ...), so this arm lets ANY agent link ANY base to ANY target.
  L2  the dispatch swallows the authenticator — `OpEntry::CreateEntry { app_entry, .. }` or
      `OpEntry::UpdateEntry { app_entry, .. }` discards `action` at the point the entry is handed
      to its validator, so nothing beneath that dispatch can ever bind an author, however well
      the validator itself is written.
  L3  a `_ => Ok(ValidateCallbackResult::Valid)` catch-all INSIDE a `match app_entry { ... }`
      block (the per-entry-type dispatch a create/update validator uses) — every entry type not
      given an explicit arm validates unconditionally, silently, including future ones.

WHAT THIS IS NOT. This is a textual probe over Rust source, normalising whitespace and tolerating
arms split across lines — it is not a parser and does not track Rust's full grammar (macros,
strings and nested match arms are not modelled beyond a brace-depth scan for L3's containing
`match app_entry` span). A GREEN here means the three known-open SHAPES named above are gone from
these files — it does NOT mean authority is proven. The behavioural proof is the a2o scenario
named in the backlog (`genesis/a2o/features/trust/counterfeit-standing-is-refused.feature`, not
written yet): a peer commits a counterfeit through a direct source-chain write and a second,
honest conductor refuses the op. Grade what is textually there, not what the backlog narrates —
`mishpat_integrity` already dispatches `OpLink::CreateLink` through a real per-link-type match
(`{ link_type, action }`, not `{ .. }`) with one link type (`CommitmentByState`) genuinely
validated, so it is expected to score DIFFERENTLY from the other four DNAs on L1, even though its
`validate_create_link` still falls open (`_ => Ok(Valid)`) for every other link type — a
declared-classification gap (D6: "open by declaration, not by fallthrough"), not the L1 shape.

Run: python3 .claude/scripts/_lib/__tests__/authority_in_integrity_source_shape_test.py
     (exit 0 = no known-open shape found; exit 1 = at least one DNA still carries one — expected
     today, per authority-in-integrity's first_move)
     --json prints the same findings as machine-readable JSON instead of the human report.
Bespoke assert-based harness — matches this __tests__ dir's convention (pytest is not installed).
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

_here = Path(__file__).resolve()
REPO = None
for _ in range(8):
    if (_here / ".claude" / "scripts" / "_lib").is_dir():
        REPO = _here
        break
    _here = _here.parent
if REPO is None:  # pragma: no cover - only if the tree is moved
    print("FAIL: could not locate repo root", file=sys.stderr)
    sys.exit(1)

DNA_ROOT = REPO / "elohim" / "holochain" / "dna"

# The five LIVE integrity zomes this habit's invariant covers. lamad-v1 is a v1 archive held for
# healing migration (elohim/holochain/dna/CLAUDE.md), and hrea/ is a workdir-only placeholder with
# no zomes — neither carries live standing, so neither belongs in this census.
DNAS: dict[str, str] = {
    "elohim": "content_store_integrity",
    "imagodei": "imagodei_integrity",
    "infrastructure": "infrastructure_integrity",
    "node-registry": "node_registry_integrity",
    "mishpat": "mishpat_integrity",
}

# ── L1: an unconditional open link arm ────────────────────────────────────────
# Literal `{ .. }` destructure — the shape that consults nothing. A real per-link-type match
# (mishpat's `OpLink::CreateLink { link_type, action }`) does not match this pattern, by design:
# grade what is actually there, not the invariant's worst case.
L1_RE = re.compile(
    r"FlatOp::Link\s*\(\s*OpLink::(CreateLink|DeleteLink)\s*\{\s*\.\.\s*\}\s*\)"
    r"\s*=>\s*\{?\s*Ok\s*\(\s*ValidateCallbackResult::Valid\s*\)",
)

# ── L2: the dispatch swallows the authenticator ───────────────────────────────
# `OpEntry::CreateEntry`/`UpdateEntry` destructured with `{ app_entry, .. }` — `action` never
# bound at the dispatch site, so no validator beneath it can ever see who authored the op.
L2_RE = re.compile(
    r"OpEntry::(CreateEntry|UpdateEntry)\s*\{\s*app_entry\s*,\s*\.\.\s*\}",
)

# ── L3: catch-all entry validity inside the per-entry-type dispatch ───────────
# Scoped to spans that open with `match app_entry {` (the create/update entry-type dispatch,
# whether inline as in imagodei/infrastructure or in a standalone validate_create_entry /
# validate_update_entry function as in elohim/node-registry/mishpat) — never a bare grep for the
# phrase anywhere in the file, which would also catch top-level FlatOp fallthroughs that are a
# different, ungraded shape (see the mishpat note in the module docstring).
MATCH_APP_ENTRY_RE = re.compile(r"match\s+app_entry\s*(\{)")
L3_RE = re.compile(r"_\s*=>\s*Ok\s*\(\s*ValidateCallbackResult::Valid\s*\)")


def _line_of(text: str, pos: int) -> int:
    return text.count("\n", 0, pos) + 1


def _match_app_entry_spans(text: str) -> list[tuple[int, int]]:
    """Every `match app_entry { ... }` span, as (open_brace_pos, close_brace_pos) — found by a
    plain brace-depth walk from each opening brace, since these files nest ordinary Rust blocks
    (arm bodies, nested calls) inside the arms and a regex alone cannot find the matching close."""
    spans: list[tuple[int, int]] = []
    for m in MATCH_APP_ENTRY_RE.finditer(text):
        start = m.start(1)
        depth = 0
        i = start
        while i < len(text):
            if text[i] == "{":
                depth += 1
            elif text[i] == "}":
                depth -= 1
                if depth == 0:
                    spans.append((start, i))
                    break
            i += 1
    return spans


def scan_file(path: Path) -> dict[str, list[dict]]:
    text = path.read_text()
    findings: dict[str, list[dict]] = {"L1": [], "L2": [], "L3": []}

    for m in L1_RE.finditer(text):
        findings["L1"].append({
            "line": _line_of(text, m.start()),
            "kind": m.group(1),
            "excerpt": re.sub(r"\s+", " ", m.group(0)).strip(),
        })

    for m in L2_RE.finditer(text):
        findings["L2"].append({
            "line": _line_of(text, m.start()),
            "kind": m.group(1),
            "excerpt": re.sub(r"\s+", " ", m.group(0)).strip(),
        })

    for start, end in _match_app_entry_spans(text):
        span_text = text[start:end + 1]
        for m in L3_RE.finditer(span_text):
            findings["L3"].append({
                "line": _line_of(text, start + m.start()),
                "kind": "match app_entry catch-all",
                "excerpt": re.sub(r"\s+", " ", m.group(0)).strip(),
            })

    for key in findings:
        findings[key].sort(key=lambda f: f["line"])
    return findings


def main() -> int:
    as_json = "--json" in sys.argv[1:]
    report: dict[str, dict] = {}
    any_hit = False
    missing: list[str] = []

    for dna, zome in sorted(DNAS.items()):
        candidates = sorted((DNA_ROOT / dna).glob(f"zomes/{zome}/src/lib.rs"))
        if not candidates:
            missing.append(f"{dna}: no zomes/{zome}/src/lib.rs under {DNA_ROOT / dna}")
            continue
        path = candidates[0]
        rel = path.relative_to(REPO).as_posix()
        findings = scan_file(path)
        counts = {k: len(v) for k, v in findings.items()}
        if any(counts.values()):
            any_hit = True
        report[dna] = {"path": rel, "counts": counts, "findings": findings}

    if missing:
        if as_json:
            print(json.dumps({"error": "missing zome files", "detail": missing}, indent=2))
        else:
            print("FAIL: could not locate every declared integrity zome:")
            for m in sorted(missing):
                print(f"  - {m}")
        return 1

    if as_json:
        print(json.dumps({
            "caveat": (
                "textual probe of Rust source, not a parser; a green here means the three known-"
                "open shapes (L1 unconditional link arm, L2 dispatch-discards-action, L3 entry "
                "catch-all) are gone from these files, NOT that authority is proven — the "
                "behavioural proof is genesis/a2o/features/trust/counterfeit-standing-is-refused"
                ".feature, which does not exist yet"
            ),
            "dnas": report,
            "any_hit": any_hit,
        }, indent=2, sort_keys=True))
        return 1 if any_hit else 0

    print("authority-in-integrity source-shape probe")
    print("=" * 60)
    for dna in sorted(report):
        info = report[dna]
        counts = info["counts"]
        mark = "❌" if any(counts.values()) else "✅"
        print(f"\n{mark} {dna}  ({info['path']})")
        print(f"   L1 open-link-arm={counts['L1']}  L2 dispatch-swallows-action={counts['L2']}  "
              f"L3 entry-catch-all={counts['L3']}")
        for level in ("L1", "L2", "L3"):
            for f in info["findings"][level]:
                print(f"     {level} {info['path']}:{f['line']}  {f['excerpt']}")

    total = sum(sum(info["counts"].values()) for info in report.values())
    print()
    print("-" * 60)
    print(f"TOTAL known-open-shape hits: {total} across {len(report)} DNAs")
    print(
        "CAVEAT: textual probe of Rust source, not a parser. GREEN means the three known-open\n"
        "shapes are gone from these files — it does NOT mean authority is proven. The behavioural\n"
        "proof is genesis/a2o/features/trust/counterfeit-standing-is-refused.feature (not written\n"
        "yet): a peer commits a counterfeit by direct source-chain write and a second, honest\n"
        "conductor refuses the op."
    )
    if any_hit:
        print()
        print("FAILED ❌ — this is the runnable red behind habits `authority-in-integrity`.")
        return 1
    print()
    print("PASSED ✅ — no known-open shape found in the five live integrity zomes.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
