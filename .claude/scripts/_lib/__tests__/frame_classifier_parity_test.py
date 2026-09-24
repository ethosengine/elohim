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

  * the confidence and the rubric answer (native: inside the carried `classification` record;
    Python: top-level in its evidence dict);
  * `ontologyRef` — the fold table's own registry-row CID, carried beside `classificationCid`
    (ruling R-C14, review W2), equal in both hosts and to `frame_atoms.ontology_ref`.

The cases (review W3) cover an ASCII abstain for each guard, a NON-ASCII abstain (a multibyte
prefix, so byte offsets differ from character offsets, plus a Cyrillic homoglyph the fold must
see), and a `drift` (`sovereignty-frame: apex`). A write that declares a legitimate frame must be
silent in both hosts — for sovereignty (`sovereignty-frame:`) and for ownership
(`stewardship-frame:`).

HOW FIRING AND SILENT CASES ARE TOLD APART. The comparisons must be able to fail, so:
  * every firing case must carry a non-empty `frameRef`, a verdict and spans, and no two firing
    cases may share the same `(frameRef, verdict, spans)` — a host answering one constant
    classification, or none, is red;
  * a silent case is silent only because the frame was READ: Python's own classifier returns
    `legitimate` WITH spans for it (the write does carry apex phrases), and the native host cannot
    be silent by failing to read the atom, because an unreadable atom is `Unavailable`, which the
    evaluator clamps to the row's dispatch class — it would fire, not fall silent.

And every `frame-ref:` a policy row declares must equal the CID `frame_atoms.frame_ref` computes
for the atom its `frame:` names.

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
# (rule id, probe file name, content, expected frame verdict)
PROBES = [
    ("sovereignty-ontology-guard", "zz-frame-parity-probe-sovereignty.md",
     FRONT + "The learner is self-sovereign here, and we build digital sovereignty for them.\n",
     "abstain"),
    ("ownership-ontology-guard", "zz-frame-parity-probe-ownership.md",
     FRONT + "Every household should own your data outright; true ownership is the promise.\n",
     "abstain"),
    # Non-ASCII (review W3): a multibyte prefix moves every later byte offset off its character
    # offset, and `self\u2011s\u043evereign` (non-breaking hyphen, Cyrillic о) only matches after
    # the fold. The ASCII `digital sovereignty` keeps the live @3 binding's raw `contains-any`
    # pre-filter satisfied until the operator's rebind to @5 lands (R-C9, R-C13).
    ("sovereignty-ontology-guard", "zz-frame-parity-probe-non-ascii.md",
     FRONT + "Ünïcödé — the learner is self\u2011s\u043evereign, and digital sovereignty follows.\n",
     "abstain"),
    # Drift (review W3): an `apex` marker adjudicates the frame as the apex, confidence 1.
    ("sovereignty-ontology-guard", "zz-frame-parity-probe-drift.md",
     FRONT + "sovereignty-frame: apex\nThe learner is self-sovereign here.\n",
     "drift"),
]
SILENT = [
    ("sovereignty-ontology-guard", "epr:validator-sovereignty-ontology-guard",
     "zz-frame-parity-probe-declared.md",
     FRONT + "sovereignty-frame: bounded\nThe learner is self-sovereign here.\n"),
    ("ownership-ontology-guard", "epr:validator-ownership-ontology-guard",
     "zz-frame-parity-probe-declared-ownership.md",
     FRONT + "stewardship-frame: inalienable\nA person holds true ownership of her own image.\n"),
]

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


def compare(binary: str, rule_id: str, name: str, content: str, expected: str) -> tuple | None:
    """Compare one firing case; returns its `(frameRef, verdict, spans)` fingerprint."""
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
        return None
    want = declared_class(nv.get("policyRef") or "")
    check(nv.get("class") == pv.cls == rs.get("winningClass") == py.get("cls") == want,
          f"class inherited from the row (R-C7): row={want} native={nv.get('class')} "
          f"python={pv.cls}")
    ne, pe = nv.get("evidence") or {}, pv.evidence or {}
    check(ne.get("frameRef") == pe.get("frameRef") and bool(pe.get("frameRef")),
          f"frameRef: {pe.get('frameRef')}")
    check(ne.get("verdict") == pe.get("verdict") == expected,
          f"frame verdict: native={ne.get('verdict')} python={pe.get('verdict')} "
          f"expected={expected}")
    check(ne.get("spans") == pe.get("spans") and bool(pe.get("spans")),
          f"spans: native={ne.get('spans')} python={pe.get('spans')}")
    raw = content.encode("utf-8")
    sliced = [raw[sp["start"]:sp["end"]].decode("utf-8") for sp in pe.get("spans") or []]
    check(all(sliced), f"every span slices whole characters out of the original bytes: {sliced}")
    if not content.isascii():
        check(any(sp["start"] != len(raw[:sp["start"]].decode("utf-8"))
                  for sp in pe.get("spans") or []),
              "non-ASCII case: span offsets are BYTE offsets (they differ from char offsets)")
    check(ne.get("confidence") == pe.get("confidence") and pe.get("confidence") is not None,
          f"confidence: native={ne.get('confidence')} python={pe.get('confidence')}")
    native_answer = ((ne.get("classification") or {}).get("evidence") or {}).get("rubricAnswer")
    check(native_answer == pe.get("rubricAnswer"),
          f"rubricAnswer: native={native_answer} python={pe.get('rubricAnswer')}")
    # w2_evidence_carries_ontology_ref_equal_in_both_hosts (review W2, ruling R-C14)
    check(ne.get("ontologyRef") == pe.get("ontologyRef") == frame_atoms.ontology_ref(repo),
          f"w2 ontologyRef equal in both hosts: native={ne.get('ontologyRef')} "
          f"python={pe.get('ontologyRef')}")
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
    return (pe.get("frameRef"), pe.get("verdict"), json.dumps(pe.get("spans")))


def compare_silent(binary: str, rule_id: str, validator: str, name: str, content: str) -> None:
    rel = f"{PROBE_DIR}/{name}"
    print(f"\n  declared frame — {rule_id} — {rel}")
    assert not (repo / rel).exists(), f"{rel} exists — the probe must never land"
    # Silence must be the READ frame, not an absence of hits: the write carries apex phrases,
    # and Python's own classifier says `legitimate` with the spans it read.
    direct = frame_atoms.classify({"content": content, "is_new": True, "path": rel}, validator,
                                  repo)
    check(direct is not None and direct["verdict"] == "legitimate" and bool(direct["spans"]),
          f"the silent case carries apex phrases the frame declares legitimate: {direct}")
    rs, py = native(binary, rel, content), python(rel, content)
    fired_rs = [v.get("ruleId") for v in rs.get("verdicts") or [] if v.get("ruleId") == rule_id]
    fired_py = [v.rule_id for v in (py.get("dispatches") or []) + (py.get("measures") or [])
                if v.rule_id == rule_id]
    check(fired_rs == fired_py == [],
          f"the declared frame silences {rule_id} in both hosts "
          f"(native={fired_rs} python={fired_py})")
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
    fingerprints = [compare(binary, rule_id, name, content, expected)
                    for rule_id, name, content, expected in PROBES]
    print("\n  firing cases are told apart")
    check(None not in fingerprints and len(set(fingerprints)) == len(fingerprints),
          f"every firing case has its own (frameRef, verdict, spans): {len(set(fingerprints))} "
          f"distinct of {len(fingerprints)}")
    for rule_id, validator, name, content in SILENT:
        compare_silent(binary, rule_id, validator, name, content)
    compare_frame_refs()
    if failures:
        print(f"\n  ❌ {len(failures)} parity failure(s)")
        return 1
    print("\n  frame classifier parity: native == python ✅")
    return 0


if __name__ == "__main__":
    sys.exit(main())
