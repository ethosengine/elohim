"""Tests for measure-fold (derivation key, fold cache round-trip, clippy canonicalization).
Run: python3 .claude/scripts/__tests__/measure_fold_test.py (exit 0 = pass)."""
import importlib.util
import json
import sys
import tempfile
from pathlib import Path

here = Path(__file__).resolve()
for _ in range(8):
    if (here / ".claude" / "scripts" / "_lib").is_dir():
        sys.path.insert(0, str(here / ".claude" / "scripts"))
        break
    here = here.parent

_spec = importlib.util.spec_from_file_location("measure_fold", here / ".claude" / "scripts" / "measure-fold.py")
mf = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(mf)

_p = 0


def check(label, cond):
    global _p
    assert cond, f"FAIL: {label}"
    _p += 1
    print(f"  ✅ {label}")


# ── derivation key: determinism + sensitivity ────────────────────────────────
SUBJ = {"crate": "steward/node", "tree": "a" * 40}
ENV = {"rustc": "rustc 1.85.0", "clippy": "clippy 0.1.85", "cargo_lock": "b" * 64, "clippy_toml": None, "rustflags": ""}

k1 = mf.derivation_key("clippy-pedantic@1", SUBJ, ENV)
k2 = mf.derivation_key("clippy-pedantic@1", dict(reversed(list(SUBJ.items()))), dict(reversed(list(ENV.items()))))
check("same input -> same key (key-order independent)", k1 == k2)
check("key is a full sha256 hex", len(k1) == 64 and all(c in "0123456789abcdef" for c in k1))

check("tree change -> different key", mf.derivation_key("clippy-pedantic@1", {**SUBJ, "tree": "c" * 40}, ENV) != k1)
check("crate change -> different key", mf.derivation_key("clippy-pedantic@1", {**SUBJ, "crate": "crates/elohim-sdk"}, ENV) != k1)
check("rustc change -> different key", mf.derivation_key("clippy-pedantic@1", SUBJ, {**ENV, "rustc": "rustc 1.86.0"}) != k1)
check("cargo_lock change -> different key", mf.derivation_key("clippy-pedantic@1", SUBJ, {**ENV, "cargo_lock": "d" * 64}) != k1)
check("clippy_toml presence -> different key", mf.derivation_key("clippy-pedantic@1", SUBJ, {**ENV, "clippy_toml": "e" * 64}) != k1)
check("rustflags change -> different key", mf.derivation_key("clippy-pedantic@1", SUBJ, {**ENV, "rustflags": '--cfg getrandom_backend="custom"'}) != k1)
check("measure version bump -> different key", mf.derivation_key("clippy-pedantic@2", SUBJ, ENV) != k1)
check("canonical json is compact + sorted", mf.canonical({"b": 1, "a": [2, 3]}) == '{"a":[2,3],"b":1}')

# ── fold cache round-trip ────────────────────────────────────────────────────
FINDINGS = [
    {"file": "src/b.rs", "line": 3, "endLine": 3, "code": "clippy::needless_continue", "level": "warning", "message": "m", "rendered": "r"},
    {"file": "src/a.rs", "line": 9, "endLine": 9, "code": "clippy::must_use_candidate", "level": "warning", "message": "m", "rendered": "r"},
    {"file": "src/a.rs", "line": 9, "endLine": 9, "code": "clippy::needless_continue", "level": "warning", "message": "m", "rendered": "r"},
]
fold = mf.make_fold("clippy-pedantic@1", SUBJ, ENV, k1, FINDINGS)
check("fold schema", fold["schema"] == "measure-fold@1")
check("fold evidence tier is claimed", fold["evidence"] == "claimed")
check("fold counts total", fold["counts"]["total"] == 3)
check("fold counts byCode", fold["counts"]["byCode"] == {"clippy::must_use_candidate": 1, "clippy::needless_continue": 2})

with tempfile.TemporaryDirectory() as td:
    mf.FOLD_ROOT = Path(td)
    p = mf.write_fold("clippy-pedantic", fold)
    check("fold path is <measure-id>/<first16>.json", p.parent.name == "clippy-pedantic" and p.name == k1[:16] + ".json")
    check("cache miss before write (other key)", mf.load_fold("clippy-pedantic", "f" * 64) is None)
    got = mf.load_fold("clippy-pedantic", k1)
    check("cache round-trip is byte-faithful", got == json.loads(p.read_text()) and got["findings"] == FINDINGS)
    # a key/body mismatch (hand-edited or truncated fold) must read as a MISS, never a false hit
    p.write_text(json.dumps({**fold, "derivationKey": "0" * 64}))
    check("body/key mismatch -> miss (no false hit)", mf.load_fold("clippy-pedantic", k1) is None)
    p.write_text("{not json")
    check("corrupt fold -> miss", mf.load_fold("clippy-pedantic", k1) is None)

# ── clippy --message-format=json canonicalization ────────────────────────────
STREAM = "\n".join(
    json.dumps(m)
    for m in [
        {"reason": "compiler-artifact", "target": {"name": "x"}},
        {"reason": "build-finished", "success": True},
        {
            "reason": "compiler-message",
            "message": {
                "level": "warning",
                "code": {"code": "clippy::must_use_candidate"},
                "message": "this method could have a `#[must_use]` attribute",
                "spans": [{"file_name": "src/z.rs", "line_start": 40, "line_end": 42, "is_primary": True}],
                "rendered": "\n".join(f"line{i}" for i in range(20)),
            },
        },
        {
            "reason": "compiler-message",
            "message": {
                "level": "warning",
                "code": {"code": "clippy::needless_continue"},
                "message": "this `continue` expression is redundant",
                "spans": [
                    {"file_name": "src/other.rs", "line_start": 1, "line_end": 1, "is_primary": False},
                    {"file_name": "src/a.rs", "line_start": 12, "line_end": 14, "is_primary": True},
                ],
                "rendered": "warning: redundant",
            },
        },
        {  # a plain rustc warning surfaced by clippy — kept
            "reason": "compiler-message",
            "message": {
                "level": "warning",
                "code": {"code": "unused_variables"},
                "message": "unused variable: `x`",
                "spans": [{"file_name": "src/a.rs", "line_start": 5, "line_end": 5, "is_primary": True}],
                "rendered": "warning: unused",
            },
        },
        {  # note-level chatter — dropped
            "reason": "compiler-message",
            "message": {"level": "note", "code": None, "message": "…", "spans": [], "rendered": "note"},
        },
    ]
) + "\nSome non-JSON cargo chatter line\n"

got = mf.canonicalize_clippy(STREAM)
check("artifact/build-finished/note dropped", len(got) == 3)
check(
    "sorted by (file, line, code)",
    [(f["file"], f["line"], f["code"]) for f in got]
    == [("src/a.rs", 5, "unused_variables"), ("src/a.rs", 12, "clippy::needless_continue"), ("src/z.rs", 40, "clippy::must_use_candidate")],
)
check("primary span wins over non-primary", got[1]["file"] == "src/a.rs" and got[1]["endLine"] == 14)
check("rendered truncated to ~10 lines", len(got[2]["rendered"].splitlines()) == 10)
check("canonicalization is deterministic", mf.canonicalize_clippy(STREAM) == got)

# ── --diff-only changed-line filter ──────────────────────────────────────────
touched = {"steward/node/src/a.rs": {13, 14, 15}}
kept = mf.on_changed_lines(got, "steward/node", touched)
check("diff-only keeps the overlapping multi-line finding", [f["code"] for f in kept] == ["clippy::needless_continue"])
check("diff-only drops untouched files", not mf.on_changed_lines(got, "steward/node", {"steward/node/src/z.rs": {1}}))

# ── rustflags rule (hard repo rule, worth a regression pin) ──────────────────
check("storage gets the custom getrandom backend", mf.rustflags_for("elohim/elohim-storage") == '--cfg getrandom_backend="custom"')
check("doorway gets empty RUSTFLAGS", mf.rustflags_for("doorway/doorway-service") == "")
check("crates/* get empty RUSTFLAGS", mf.rustflags_for("crates/elohim-sdk") == "")

# ── registry resolution ──────────────────────────────────────────────────────
REG = {
    "measures-version": 1,
    "measures": [
        {"id": "clippy-pedantic", "version": 1, "family": "clippy", "allows": ["needless_continue"]},
        {"id": "clippy-standard", "version": 1, "family": "clippy"},
    ],
}
check("resolve by id@version", mf.resolve_measure(REG, "clippy-pedantic@1")["ref"] == "clippy-pedantic@1")
check("resolve by bare id when unambiguous", mf.resolve_measure(REG, "clippy-standard")["family"] == "clippy")
try:
    mf.resolve_measure(REG, "clippy-pedantic@9")
    check("unknown version rejected", False)
except SystemExit:
    check("unknown version rejected", True)

with tempfile.TemporaryDirectory() as td:
    try:
        mf.load_registry(Path(td) / "missing.yaml")
        check("missing registry -> clear SystemExit", False)
    except SystemExit as e:
        check("missing registry -> clear SystemExit", "measure registry not found" in str(e))

print(f"\n  {_p} assertions passed ✅")
