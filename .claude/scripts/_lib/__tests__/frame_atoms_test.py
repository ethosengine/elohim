"""Frame atoms — the values guards' phrase lists and markers, lifted into content-addressable data.

WHY THIS EXISTS. The sovereignty and ownership ontology guards each carry a hand-rolled phrase
list and frame marker(s), written twice: once in Python (`_lib/epr_meta.py`) and once in Rust
(`elohim/eprfs/epr-cli/src/repository_validators.rs`). Lane C of the post-station-4 sprint moves
that vocabulary into S2 frame atoms under `elohim/sdk/schemas/v1/frames/` so both hosts read ONE
list (ruling R-C2). This test proves the atoms are well-formed and that the migration a later
task makes (deleting the in-code constants) is LOSSLESS: the atoms carry exactly the phrases and
markers the guards enforce today, pinned here as literals so a silent drift in either host or
in the atoms reds this file.

Run: python3 .claude/scripts/_lib/__tests__/frame_atoms_test.py  (exit 0 = pass)
Bespoke assert-based harness — matches this __tests__ dir's convention (pytest is not installed).
"""
import json
import re
import sys
from pathlib import Path

_here = Path(__file__).resolve()
REPO = None
for _ in range(8):
    if (_here / ".claude" / "scripts" / "_lib").is_dir():
        sys.path.insert(0, str(_here / ".claude" / "scripts"))
        REPO = _here
        break
    _here = _here.parent
if REPO is None:  # pragma: no cover - only if the tree is moved
    print("FAIL: could not locate repo root", file=sys.stderr)
    sys.exit(1)

FRAMES = REPO / "elohim/sdk/schemas/v1/frames"
SCHEMA = FRAMES / "frame.schema.json"
ONTOLOGY = FRAMES / "frame-ontology.json"
ATOM_FILES = {
    "frame-sovereignty-apex": FRAMES / "frame-sovereignty-apex.json",
    "frame-ownership-inalienable": FRAMES / "frame-ownership-inalienable.json",
}
VALIDATOR_REGISTRY = REPO / "elohim/sdk/schemas/v1/registries/governance-validators.json"
RUST_PROVIDER = REPO / "elohim/eprfs/epr-cli/src/repository_validators.rs"

# The guards' vocabulary as it stands before the migration — pinned literally, not read back from
# the code under test, so a change on either side is a visible red rather than a silent agreement.
PINNED_SOV_PHRASES = {
    "self-sovereign", "self sovereign", "self-sovereignty", "self sovereignty",
    "true data sovereignty", "full data sovereignty", "sovereign identity",
    "digital sovereignty", "fully sovereign",
}
PINNED_OWN_PHRASES = {
    "data ownership", "own your data", "owns their data", "owns your data",
    "true ownership", "outright ownership",
    "ownership rights", "ownership of the commons", "owns the commons",
}
PINNED_SOV_MARKERS = ["sovereignty-frame:"]
PINNED_OWN_MARKERS = ["stewardship-frame:", "sovereignty-frame:"]

REQUIRED_ATOM_KEYS = {
    "id", "version", "validator", "apex_concept", "family", "polarity", "cost_class", "binding",
    "linguistic_definition", "reason_clause", "rubric", "recall_signal", "cites", "established_by",
}
LIFECYCLE_KEYS = {"status", "contentHash", "superseded_by", "retire-when"}

_passed = 0
_failures: list[str] = []


def check(name: str, cond: bool, detail: str = "") -> None:
    global _passed
    if cond:
        _passed += 1
        print(f"  ok   {name}")
    else:
        _failures.append(name)
        print(f"  FAIL {name}")
        if detail:
            print(f"       {detail}")


def _has_float(value) -> bool:
    if isinstance(value, float):
        return True
    if isinstance(value, dict):
        return any(_has_float(v) for v in value.values())
    if isinstance(value, list):
        return any(_has_float(v) for v in value)
    return False


def load(path: Path):
    return json.loads(path.read_text())


def rust_phrases(fn_name: str) -> set[str]:
    """The `PHRASES` const inside a named guard fn in the Rust provider."""
    src = RUST_PROVIDER.read_text()
    m = re.search(rf"fn {fn_name}\b.*?const PHRASES: &\[&str\] = &\[(.*?)\];", src, re.S)
    return set(re.findall(r'"([^"]*)"', m.group(1))) if m else set()


def rust_markers(fn_name: str) -> list[str]:
    src = RUST_PROVIDER.read_text()
    m = re.search(rf"fn {fn_name}\b(.*?)\n}}\n", src, re.S)
    return re.findall(r'post\.contains\("([^"]+)"\)', m.group(1)) if m else []


# ── atoms_validate_against_schema_required_keys ─────────────────────────────────────────────
print("atoms_validate_against_schema_required_keys")
for path in (SCHEMA, ONTOLOGY, *ATOM_FILES.values()):
    check(f"{path.name} exists and parses as JSON", path.is_file() and bool(load(path)) if path.is_file() else False)
if _failures:
    print(f"\n  {_passed} passed, {len(_failures)} FAILED ❌")
    sys.exit(1)

schema = load(SCHEMA)
atoms = {aid: load(p) for aid, p in ATOM_FILES.items()}
try:
    import jsonschema  # type: ignore
except ImportError:  # pragma: no cover - environment dependent
    jsonschema = None

for aid, atom in atoms.items():
    if jsonschema is not None:
        errors = sorted(jsonschema.Draft202012Validator(schema).iter_errors(atom), key=str)
        check(f"{aid} validates against frame.schema.json", not errors,
              "; ".join(e.message for e in errors[:5]))
    missing = REQUIRED_ATOM_KEYS - set(atom)
    check(f"{aid} carries every required key", not missing, f"missing: {sorted(missing)}")
    check(f"{aid} carries no lifecycle keys", not (LIFECYCLE_KEYS & set(atom)),
          f"lifecycle keys belong on the registry row: {sorted(LIFECYCLE_KEYS & set(atom))}")
    check(f"{aid} id matches its file name", atom.get("id") == aid)
    check(f"{aid} is a defeater / incriminating / high-fp-cost / binding-local",
          (atom.get("family"), atom.get("polarity"), atom.get("cost_class"), atom.get("binding"))
          == ("defeater", "incriminating", "high-fp-cost", "binding-local"))
    check(f"{aid} rubric.apex_answer is apex", atom.get("rubric", {}).get("apex_answer") == "apex")
    rs = atom.get("recall_signal", {})
    check(f"{aid} recall_signal parameters are the ruled values",
          (rs.get("min_net_new"), rs.get("scan_cap_bytes"), rs.get("cosine_floor_permille"),
           rs.get("probe_min_net_new_bytes")) == (1, 262144, 350, 400))
    # Ruling R-C6: governed declarations carry no floats — the atom's identity is the registry-row
    # CID recipe, whose canonical JSON refuses a float (CPython and Rust render them differently).
    check(f"{aid} carries no float anywhere (R-C6)", not _has_float(atom))
    cites = atom.get("cites", [])
    stale = [c for c in cites if not (REPO / c).is_file()]
    check(f"{aid} cites resolve to files on disk", bool(cites) and not stale, f"missing: {stale}")

check("schema itself refuses unknown keys (additionalProperties false)",
      schema.get("additionalProperties") is False)
if jsonschema is not None:
    bogus = dict(atoms["frame-sovereignty-apex"], status="active")
    check("schema refuses a lifecycle key smuggled into an atom",
          not jsonschema.Draft202012Validator(schema).is_valid(bogus))

registry_refs = {row["ref"] for row in load(VALIDATOR_REGISTRY)["validators"]}
check("sovereignty atom binds the sovereignty validator",
      atoms["frame-sovereignty-apex"].get("validator") == "epr:validator-sovereignty-ontology-guard")
check("ownership atom binds the ownership validator",
      atoms["frame-ownership-inalienable"].get("validator") == "epr:validator-ownership-ontology-guard")
check("every atom's validator ref is declared in governance-validators.json",
      all(a.get("validator") in registry_refs for a in atoms.values()))

frames_sov = {f["id"] for f in atoms["frame-sovereignty-apex"]["rubric"]["legitimate_frames"]}
frames_own = {f["id"] for f in atoms["frame-ownership-inalienable"]["rubric"]["legitimate_frames"]}
check("sovereignty legitimate frames are adversary | bounded | bridge-legibility",
      frames_sov == {"adversary", "bounded", "bridge-legibility"}, f"got {sorted(frames_sov)}")
# Ownership's four frames are the deliberated @2/@3 set (policies.yaml): bridge-legibility is the
# sovereignty-specific wallet frame, and ownership's own outward frame is external-legibility.
check("ownership legitimate frames are adversary | bounded | external-legibility | inalienable",
      frames_own == {"adversary", "bounded", "external-legibility", "inalienable"},
      f"got {sorted(frames_own)}")

ontology = load(ONTOLOGY)
check("ontology id/version", (ontology.get("id"), ontology.get("version")) == ("frame-ontology", 1))
check("ontology cites both atoms at @1",
      set(ontology.get("cites", [])) == {"frame-sovereignty-apex@1", "frame-ownership-inalienable@1"})
norm = ontology.get("normalization", {})
check("ontology normalizes NFKC + lowercase", norm.get("nfkc") is True and norm.get("lowercase") is True)
confusables = norm.get("confusables", {})
needed = {"а": "a", "е": "e", "о": "o", "р": "p", "с": "c", "у": "y", "х": "x", "і": "i", "ѕ": "s", "ј": "j"}
wrong = {k: confusables.get(k) for k, v in needed.items() if confusables.get(k) != v}
check("confusable table folds the Cyrillic homoglyphs а е о р с у х і ѕ ј", not wrong, f"wrong: {wrong}")
check("every confusable maps one non-ASCII char to ASCII",
      all(len(k) == 1 and ord(k) > 127 and v.isascii() and v for k, v in confusables.items()))
check("testimony exemption is the public_observer prefix + `testimony` frontmatter key",
      ontology.get("testimony_exempt") == {
          "path_prefixes": ["genesis/docs/content/elohim-protocol/public_observer/"],
          "frontmatter_key": "testimony",
      })
check("the public_observer prefix exists on disk",
      (REPO / "genesis/docs/content/elohim-protocol/public_observer").is_dir())

# ── phrase_lists_match_deleted_constants ───────────────────────────────────────────────────
print("phrase_lists_match_deleted_constants")
sov_atom = atoms["frame-sovereignty-apex"]["recall_signal"]["phrases"]
own_atom = atoms["frame-ownership-inalienable"]["recall_signal"]["phrases"]
check("sovereignty atom phrases == the pinned nine", set(sov_atom) == PINNED_SOV_PHRASES and len(sov_atom) == 9,
      f"diff: {sorted(set(sov_atom) ^ PINNED_SOV_PHRASES)}")
check("ownership atom phrases == the pinned nine", set(own_atom) == PINNED_OWN_PHRASES and len(own_atom) == 9,
      f"diff: {sorted(set(own_atom) ^ PINNED_OWN_PHRASES)}")
check("atom phrases are lowercase (the fold target)", all(p == p.lower() for p in sov_atom + own_atom))

from _lib import epr_meta  # noqa: E402

py_sov = getattr(epr_meta, "_SOV_APEX_PHRASES", None)
py_own = getattr(epr_meta, "_OWN_APEX_PHRASES", None)
# Once the Python mirror deletes its constants the pins above carry the proof alone.
if py_sov is not None:
    check("Python _SOV_APEX_PHRASES == pinned", set(py_sov) == PINNED_SOV_PHRASES)
if py_own is not None:
    check("Python _OWN_APEX_PHRASES == pinned", set(py_own) == PINNED_OWN_PHRASES)
r_sov, r_own = rust_phrases("sovereignty_guard"), rust_phrases("ownership_guard")
if r_sov:
    check("Rust sovereignty_guard PHRASES == pinned", r_sov == PINNED_SOV_PHRASES)
if r_own:
    check("Rust ownership_guard PHRASES == pinned", r_own == PINNED_OWN_PHRASES)

# ── markers_match_current_guards ───────────────────────────────────────────────────────────
print("markers_match_current_guards")
check("sovereignty atom markers", atoms["frame-sovereignty-apex"]["recall_signal"]["markers"] == PINNED_SOV_MARKERS)
check("ownership atom markers (honours both)",
      atoms["frame-ownership-inalienable"]["recall_signal"]["markers"] == PINNED_OWN_MARKERS)
if hasattr(epr_meta, "_SOV_FRAME_MARKER"):
    check("Python sovereignty marker == pinned", [epr_meta._SOV_FRAME_MARKER] == PINNED_SOV_MARKERS)
if hasattr(epr_meta, "_OWN_FRAME_MARKER"):
    check("Python ownership markers == pinned",
          [epr_meta._OWN_FRAME_MARKER, epr_meta._SOV_FRAME_MARKER] == PINNED_OWN_MARKERS)
rm_sov, rm_own = rust_markers("sovereignty_guard"), rust_markers("ownership_guard")
if rm_sov:
    check("Rust sovereignty_guard markers == pinned", rm_sov == PINNED_SOV_MARKERS, f"got {rm_sov}")
if rm_own:
    check("Rust ownership_guard markers == pinned", rm_own == PINNED_OWN_MARKERS, f"got {rm_own}")

print()
if _failures:
    print(f"  {_passed} passed, {len(_failures)} FAILED ❌")
    sys.exit(1)
print(f"  {_passed} frame-atom assertions passed ✅")
