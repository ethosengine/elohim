"""Cross-host parity for the values guards' frame classifier — the reader of habit
`guards-sense-frames` (check (a)).

The native evaluator (`epr govern`, `epr-cli/src/frames.rs`) is the decision authority; the Python
twin (`epr_meta.resolve_write` over `_lib/frame_atoms.py`) mirrors it (ruling R-C1). For a FIXED
probe write under `genesis/docs/content/elohim-protocol/` — at a path that does not exist, sent
`--new --content-stdin`, so nothing lands — both hosts must agree on:

  * the decision, the winning rule id, and the rule's class — Python's validator returns no class
    of its own and INHERITS the rule's declared class (ruling R-C7), so the class is also checked
    against the class the policy row declares, for BOTH guard rows;
  * the frame the verdict names (`frameRef`), the frame verdict, and the evidence spans (byte
    offsets into the original text);
  * the reason line, verdict-level and evidence-level. The one token allowed to differ is the
    classification short CID: the native evaluator mints it and Python spells `unminted`, because
    there is no second DAG-CBOR encoder in Python (ruling R-C4 amended). The native side must
    mint one; the Python side must not.

A write that declares its frame (`sovereignty-frame:`) must be silent in both hosts. And every
`frame-ref:` a policy row declares must equal the CID `frame_atoms.frame_ref` computes for the
atom its `frame:` names.

Skip convention (`cite_cid_parity_test.py`): if no `epr` binary resolves, print a loud SKIP line and
exit 0 — parity is unprovable without the native evaluator, not failed.

Run: python3 .claude/scripts/_lib/__tests__/frame_classifier_parity_test.py (exit 0 = pass or skip).
"""
from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

here = Path(__file__).resolve()
repo = here
for _ in range(8):
    if (repo / ".claude" / "scripts" / "_lib").is_dir():
        sys.path.insert(0, str(repo / ".claude" / "scripts"))
        break
    repo = repo.parent

from _lib import epr_client, epr_meta, frame_atoms  # noqa: E402

PROBE_DIR = "genesis/docs/content/elohim-protocol"
FRONT = "---\ntitle: Frame classifier parity probe\n---\n\n# Probe\n\n"
PROBES = {
    "sovereignty-ontology-guard": (
        "zz-frame-parity-probe-sovereignty.md",
        FRONT + "The learner is self-sovereign here, and we build digital sovereignty for them.\n",
    ),
    "ownership-ontology-guard": (
        "zz-frame-parity-probe-ownership.md",
        FRONT + "Every household should own your data outright; true ownership is the promise.\n",
    ),
}
SILENT = ("zz-frame-parity-probe-declared.md",
          FRONT + "sovereignty-frame: bounded\nThe learner is self-sovereign here.\n")

failures: list[str] = []


def check(ok: bool, what: str) -> None:
    print(f"  {'✅' if ok else '❌'} {what}")
    if not ok:
        failures.append(what)


def native(binary: str, rel: str, content: str) -> dict:
    done = subprocess.run(
        [binary, "govern", "--repo", str(repo), "--path", rel, "--new", "--content-stdin"],
        input=content, capture_output=True, text=True, timeout=60,
    )
    if done.returncode != 0:
        raise SystemExit(f"❌ `epr govern` did not run for {rel} (exit {done.returncode}): "
                         f"{done.stderr.strip()[:400]}")
    return json.loads(done.stdout)


def python(rel: str, content: str) -> dict:
    target = repo / rel
    return epr_meta.resolve_write(
        target, {"path": str(target), "content": content, "is_new": True,
                 "is_new_subdir": False}, repo)


def declared_class(policy_ref: str) -> str | None:
    rule_id, _, version = policy_ref.partition("@")
    policies, _errs = epr_meta.load_policies(repo)
    for row in policies.values() if isinstance(policies, dict) else policies:
        if row.get("id") == rule_id and str(row.get("version")) == version:
            return row.get("class")
    return None


def unminted(text: str, minted: str | None) -> str:
    """The reason with the native classification short CID spelled the way Python spells it."""
    if not minted:
        return text
    return text.replace(f"classification {frame_atoms.short_cid(minted)}",
                        f"classification {frame_atoms.UNMINTED}")


def compare(binary: str, rule_id: str, name: str, content: str) -> None:
    rel = f"{PROBE_DIR}/{name}"
    assert not (repo / rel).exists(), f"{rel} exists — the probe must never land"
    print(f"\n  {rule_id} — {rel}")
    rs, py = native(binary, rel, content), python(rel, content)
    check(rs.get("decision") == py.get("decision"),
          f"decision: native={rs.get('decision')} python={py.get('decision')}")
    check(rs.get("ruleId") == py.get("rule_id") == rule_id,
          f"rule id: native={rs.get('ruleId')} python={py.get('rule_id')}")
    nv = next((v for v in rs.get("verdicts") or [] if v.get("ruleId") == rule_id), None)
    pv = next((v for v in (py.get("dispatches") or []) + (py.get("measures") or [])
               if v.rule_id == rule_id), None)
    if nv is None or pv is None:
        check(False, f"both hosts fired {rule_id} (native={nv is not None} python={pv is not None})")
        return
    want = declared_class(nv.get("policyRef") or "")
    check(nv.get("class") == pv.cls == rs.get("winningClass") == py.get("cls") == want,
          f"class inherited from the row (R-C7): row={want} native={nv.get('class')} "
          f"python={pv.cls}")
    ne, pe = nv.get("evidence") or {}, pv.evidence or {}
    check(ne.get("frameRef") == pe.get("frameRef") and bool(pe.get("frameRef")),
          f"frameRef: {pe.get('frameRef')}")
    check(ne.get("verdict") == pe.get("verdict"),
          f"frame verdict: native={ne.get('verdict')} python={pe.get('verdict')}")
    check(ne.get("spans") == pe.get("spans") and bool(pe.get("spans")),
          f"spans: native={ne.get('spans')} python={pe.get('spans')}")
    minted = ne.get("classificationCid")
    check(isinstance(minted, str) and minted.startswith("bafy") and pe.get("classificationCid") is None,
          f"classification minted natively only (R-C4): native={minted} "
          f"python={pe.get('classificationCid')}")
    check(unminted(ne.get("reason") or "", minted) == pe.get("reason"),
          f"evidence reason: {pe.get('reason')}")
    check(unminted(nv.get("reason") or "", minted) == pv.reason,
          "verdict reason line identical in both hosts (classification token aside)")
    check(unminted(rs.get("reason") or "", minted) == py.get("reason"),
          "decision reason identical in both hosts (classification token aside)")


def compare_silent(binary: str) -> None:
    name, content = SILENT
    rel = f"{PROBE_DIR}/{name}"
    print(f"\n  declared frame — {rel}")
    rs, py = native(binary, rel, content), python(rel, content)
    fired_rs = [v.get("ruleId") for v in rs.get("verdicts") or []
                if v.get("ruleId") == "sovereignty-ontology-guard"]
    fired_py = [v.rule_id for v in (py.get("dispatches") or []) + (py.get("measures") or [])
                if v.rule_id == "sovereignty-ontology-guard"]
    check(fired_rs == fired_py == [],
          f"`sovereignty-frame:` silences the guard in both hosts (native={fired_rs} python={fired_py})")
    check(rs.get("decision") == py.get("decision"),
          f"decision: native={rs.get('decision')} python={py.get('decision')}")


def compare_frame_refs() -> None:
    print("\n  policies.yaml frame-ref pins")
    atoms = {atom["id"]: (atom, ref) for atom, ref in frame_atoms.load_frames(repo).values()}
    policies, _errs = epr_meta.load_policies(repo)
    rows = [r for r in (policies.values() if isinstance(policies, dict) else policies)
            if "frame-ref" in r or "frame" in r]
    check(bool(rows), f"{len(rows)} policy row(s) declare a frame")
    for row in rows:
        frame_id, _, version = str(row.get("frame", "")).partition("@")
        atom, ref = atoms.get(frame_id, (None, None))
        check(atom is not None and str(atom["version"]) == version,
              f"{row.get('id')}@{row.get('version')} frame {row.get('frame')} names an atom")
        check(row.get("frame-ref") == ref == (frame_atoms.frame_ref(atom) if atom else None),
              f"{row.get('id')}@{row.get('version')} frame-ref {row.get('frame-ref')} == "
              f"frame_atoms.frame_ref(atom) {ref}")


def main() -> int:
    binary = epr_client.resolve_binary()
    if binary is None:
        print("  ⏭  SKIP — no `epr` binary resolves (EPR_BIN, PATH, the cargo pool); the frame "
              "classifier's native↔Python parity is UNPROVEN, not passed. Build elohim-epr-cli.")
        return 0
    print(f"  epr: {binary}")
    for rule_id, (name, content) in PROBES.items():
        compare(binary, rule_id, name, content)
    compare_silent(binary)
    compare_frame_refs()
    if failures:
        print(f"\n  ❌ {len(failures)} parity failure(s)")
        return 1
    print("\n  frame classifier parity: native == python ✅")
    return 0


if __name__ == "__main__":
    sys.exit(main())
