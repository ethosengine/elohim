"""Frame atoms — the Python mirror of the native frame classifier (`epr-cli/src/frames.rs`).

The values guards (sovereignty, ownership) no longer carry phrase lists in code. Their vocabulary
lives in content-addressed frame atoms under `elohim/sdk/schemas/v1/frames/`, which both hosts
read at runtime (ruling R-C2). The native evaluator is the authority and mints the
`FrameClassification` CID; this module mirrors its verdict, spans and reason line so the Python
advice agrees with it, and mints NO classification CID (ruling R-C4 amended: no second DAG-CBOR
encoder in Python) — `classificationCid` is always `None` here.

What it does compute, with the stdlib only:
  * `frame_ref(atom)` — the atom's CID by the policy-registry row recipe: sha256 over the exact
    canonical bytes `epr_meta.policy_content_hash` hashes, spelled as a CIDv1 dag-cbor/sha2-256
    base32-lower string (`b` + base32lower(0x01 0x71 0x12 0x20 || digest)).
  * the shadow text — per character: lowercase, then NFKC, then the ontology's confusable table,
    each folded character remembering the ORIGINAL character's UTF-8 byte span, so spans index the
    bytes the author wrote;
  * net-new lines only — post and prior split on `\\n`, diffed as multisets; the write fires when
    the added lines carry at least `min_net_new` more phrase hits than the removed ones;
  * the verdict — `drift` on a marker whose value is `apex` (confidence 1), `legitimate` on any
    other declared marker, `abstain` on hits with no marker (`min(1, hits / 3)`);
  * the scan cap — beyond `scan_cap_bytes` of the document the fold is skipped (lowercase-only
    substring scan) and `unscanned-tail` is recorded.
"""
from __future__ import annotations

import base64
import hashlib
import json
import os
import unicodedata
from pathlib import Path

FRAMES_REL = "elohim/sdk/schemas/v1/frames"
ONTOLOGY_FILE = "frame-ontology.json"
SCHEMA_FILE = "frame.schema.json"
UNSCANNED_TAIL = "unscanned-tail"
APEX_ANSWER = "apex"
# Spelled where `classification <short>` sits in the reason line when no native CID was minted.
UNMINTED = "unminted"

# The repository this module ships in: .claude/scripts/_lib/frame_atoms.py -> parents[3].
_LIB_REPO_ROOT = Path(__file__).resolve().parents[3]

# The strict shape (mirrors `FrameAtom` / `FrameOntology`, `deny_unknown_fields`).
_ATOM_KEYS = {
    "id", "version", "validator", "apex_concept", "family", "polarity", "cost_class", "binding",
    "linguistic_definition", "reason_clause", "rubric", "recall_signal", "cites", "established_by",
}
_SIGNAL_KEYS = {"phrases", "markers", "min_net_new", "scan_cap_bytes", "cosine_floor_permille",
                "probe_min_net_new_bytes"}
_ONTOLOGY_KEYS = {"id", "version", "cites", "normalization", "testimony_exempt"}


class FrameAtomError(Exception):
    """An atom or the ontology could not be read. A guard that meets one never passes."""


def repo_root() -> Path:
    """`CLAUDE_PROJECT_DIR` when it carries the frames dir, else this package's own repository
    (hook tests run in a temp project dir that has no atoms)."""
    pd = os.environ.get("CLAUDE_PROJECT_DIR")
    if pd and (Path(pd) / FRAMES_REL).is_dir():
        return Path(pd)
    return _LIB_REPO_ROOT


def _read(path: Path) -> dict:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as exc:
        raise FrameAtomError(f"cannot read {path}: {exc}") from exc
    if not isinstance(value, dict):
        raise FrameAtomError(f"{path} is not a JSON object")
    return value


def _strict(path: Path, value: dict, keys: set[str]) -> None:
    if set(value) != keys:
        raise FrameAtomError(f"{path}: keys differ from the frame shape — "
                             f"missing {sorted(keys - set(value))}, unknown {sorted(set(value) - keys)}")


def frame_ref(atom: dict) -> str:
    """The atom's CID: the registry-row recipe, spelled as CIDv1 dag-cbor / sha2-256."""
    from _lib import epr_meta  # lazy: epr_meta's guards import this module

    digest = bytes.fromhex(epr_meta.policy_content_hash(atom).split(":", 1)[1])
    raw = bytes([0x01, 0x71, 0x12, 0x20]) + digest
    return "b" + base64.b32encode(raw).decode("ascii").lower().rstrip("=")


def load_frames(root: Path | None = None) -> dict[str, tuple[dict, str]]:
    """Every frame atom, keyed by the validator ref it binds → (atom, frame_ref)."""
    frames_dir = (root or repo_root()) / FRAMES_REL
    try:
        paths = sorted(p for p in frames_dir.glob("*.json")
                       if p.name not in (ONTOLOGY_FILE, SCHEMA_FILE))
    except OSError as exc:
        raise FrameAtomError(f"cannot list {frames_dir}: {exc}") from exc
    if not paths:
        raise FrameAtomError(f"no frame atoms under {frames_dir}")
    frames: dict[str, tuple[dict, str]] = {}
    for path in paths:
        atom = _read(path)
        _strict(path, atom, _ATOM_KEYS)
        if not isinstance(atom.get("recall_signal"), dict):
            raise FrameAtomError(f"{path}: recall_signal is not an object")
        _strict(path, atom["recall_signal"], _SIGNAL_KEYS)
        ref = atom["validator"]
        if ref in frames:
            raise FrameAtomError(f"two frame atoms bind validator `{ref}`")
        frames[ref] = (atom, frame_ref(atom))
    return frames


def load_ontology(root: Path | None = None) -> dict:
    path = (root or repo_root()) / FRAMES_REL / ONTOLOGY_FILE
    ontology = _read(path)
    _strict(path, ontology, _ONTOLOGY_KEYS)
    return ontology


def short_cid(cid: str) -> str:
    """`bafyreig…rn6e` — the spelling `epr` and the surfacing hook print for a short CID."""
    return f"{cid[:8]}…{cid[-4:]}" if len(cid) > 14 else cid


def reason_line(atom: dict, frame: str, classification: str | None, verdict: str) -> str:
    """The one reason-line format both hosts render (`frames::reason_line`)."""
    minted = short_cid(classification) if classification else UNMINTED
    return f"{atom['reason_clause']} · frame {short_cid(frame)} · classification {minted} · {verdict}"


class _Folder:
    def __init__(self, ontology: dict):
        norm = ontology["normalization"]
        self.lowercase = bool(norm.get("lowercase"))
        self.nfkc = bool(norm.get("nfkc"))
        self.confusables = {k: v for k, v in norm.get("confusables", {}).items() if len(k) == 1}

    def shadow(self, text: str, base: int, cap: int) -> tuple[str, list[tuple[int, int]], bool]:
        """Fold `text` (starting at document byte `base`); per folded char, the original span."""
        folded: list[str] = []
        origin: list[tuple[int, int]] = []
        beyond = False
        offset = base
        for original in text:
            width = len(original.encode("utf-8"))
            span = (offset, offset + width)
            lowered = original.lower() if self.lowercase else original
            if offset >= cap:
                beyond = True
                for c in lowered:
                    folded.append(c)
                    origin.append(span)
            else:
                for low in lowered:
                    normalized = unicodedata.normalize("NFKC", low) if self.nfkc else low
                    for n in normalized:
                        for c in self.confusables.get(n, n):
                            folded.append(c)
                            origin.append(span)
            offset += width
        return "".join(folded), origin, beyond


def _lines(text: str) -> list[tuple[int, str]]:
    """`(byte offset, line)` for every `\\n`-separated line — `\\n` only, like the native host."""
    out, start = [], 0
    for line in text.split("\n"):
        out.append((start, line))
        start += len(line.encode("utf-8")) + 1
    return out


def _line_delta(prior: str, post: str) -> tuple[list[tuple[int, str]], list[tuple[int, str]]]:
    remaining: dict[str, list[tuple[int, str]]] = {}
    for offset, line in _lines(prior):
        remaining.setdefault(line, []).append((offset, line))
    added = []
    for offset, line in _lines(post):
        bucket = remaining.get(line)
        if bucket:
            bucket.pop(0)
        else:
            added.append((offset, line))
    removed = sorted(item for bucket in remaining.values() for item in bucket)
    return added, removed


def _scan(lines, phrases, folder: _Folder, cap: int) -> dict:
    hits, spans, found, beyond_cap = 0, [], [], False
    for base, line in lines:
        if not line:
            continue
        folded, origin, beyond = folder.shadow(line, base, cap)
        beyond_cap = beyond_cap or beyond
        for phrase in phrases:
            at = folded.find(phrase)
            while at != -1:
                hits += 1
                spans.append((origin[at][0], origin[at + len(phrase) - 1][1]))
                if phrase not in found:
                    found.append(phrase)
                at = folded.find(phrase, at + len(phrase))
    spans.sort()
    return {"hits": hits, "spans": spans, "phrases": sorted(found), "beyond_cap": beyond_cap}


def _declared_markers(post: str, markers: list[str]) -> list[tuple[str, str]]:
    low = post.lower()
    found = []
    for marker in markers:
        at = low.find(marker)
        while at != -1:
            value = low[at + len(marker):].split("\n", 1)[0]
            found.append((marker, value.strip().strip("\"'").strip()))
            at = low.find(marker, at + len(marker))
    return found


def _prior(write: dict) -> str:
    if write.get("is_new"):
        return ""
    if write.get("prior_content") is not None:
        return write["prior_content"]
    try:
        return Path(write["path"]).read_text(errors="replace")
    except (OSError, KeyError, TypeError):
        return ""  # can't read prior state — fail toward surfacing (the guard is advisory)


def classify(write: dict, validator: str, root: Path | None = None) -> dict | None:
    """Mirror of `frames::classify` → `{frameRef, verdict, spans, confidence, reason,
    classificationCid: None}`, or `None` when the write adds no net-new apex phrases.
    Raises `FrameAtomError` when the atom or ontology cannot be read."""
    frames = load_frames(root)
    if validator not in frames:
        raise FrameAtomError(f"no frame atom binds validator `{validator}`")
    atom, ref = frames[validator]
    folder = _Folder(load_ontology(root))
    signal = atom["recall_signal"]
    post = write.get("content") or ""
    added, removed = _line_delta(_prior(write), post)
    gained = _scan(added, signal["phrases"], folder, signal["scan_cap_bytes"])
    lost = _scan(removed, signal["phrases"], folder, signal["scan_cap_bytes"])
    if gained["hits"] < lost["hits"] + signal["min_net_new"]:
        return None
    net_new = gained["hits"] - lost["hits"]

    markers = _declared_markers(post, signal["markers"])
    apex = next((v for _, v in markers if v in (APEX_ANSWER, atom["rubric"]["apex_answer"])), None)
    if apex is not None:
        verdict, confidence, rubric_answer = "drift", 1.0, apex
    elif markers:
        verdict, confidence, rubric_answer = "legitimate", 1.0, markers[0][1] or None
    else:
        verdict, confidence, rubric_answer = "abstain", min(1.0, net_new / 3.0), None

    matched = list(gained["phrases"])
    matched += [m for m in signal["markers"] if any(d == m for d, _ in markers)]
    if gained["beyond_cap"]:
        matched.append(UNSCANNED_TAIL)

    return {
        "frameRef": ref,
        "verdict": verdict,
        "spans": [{"start": s, "end": e} for s, e in gained["spans"]],
        "confidence": confidence,
        "reason": reason_line(atom, ref, None, verdict),
        "classificationCid": None,
        "netNew": net_new,
        "matchedRecallSignal": matched,
        "rubricAnswer": rubric_answer,
    }
