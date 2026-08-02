#!/usr/bin/env python3
"""Concern-canon validator fixtures (seam-concern-contract plan, task P4.1).

Three validators were registered so standing prose nags become evaluated gates:
`epr:validator-heal-fills-never-moves` (C2, via `c2-monotonic-authority@2`),
`epr:validator-bounded-work` (C6a, via `c6a-bounded-work@2`), and
`epr:validator-dna-hash-neutrality` (registered machinery with NO canon class — the ceremony
tested DNA-hash-neutrality as a class and declined it).

WHAT THIS FILE ASSERTS, AND WHY IT IS NOT A UNIT TEST OF THREE FUNCTIONS: every case runs a
synthetic write through the REAL ladder — `evaluate()` over a merged rule set whose policy
bindings are expanded from the on-disk `.claude/epr-meta/policies.yaml` — and asserts the
CLASS-LADDER VERDICT, not the validator's bool. That is what the plan asks for and it is the
stronger assertion: it fails loud if the predicate stops firing, if the registered row's `class`
is promoted or demoted, if the binding's pin goes stale against the registry, or if the row's
contentHash drifts (a pin mismatch rewrites the rule to `ask`/pin-mismatch, which flips every
expectation here). Three POSITIVE fixtures (the verdict must fire, at `inject`) and three
NEGATIVE ones (a nearby, legitimate diff that must stay silent) — each failing loud either way,
because a validator that fires on everything is exactly as useless as one that fires on nothing.

Run: python3 .claude/scripts/_lib/__tests__/concern_validators_test.py   (exit 0 = pass)
"""
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from _lib import epr_meta  # noqa: E402

ROOT = epr_meta.find_repo_root(Path(__file__).resolve())
POLICIES, POLICY_ERRS = epr_meta.load_policies(ROOT)

_failures: list[str] = []
_passed = 0


def check(label: str, cond: bool, detail: str = "") -> None:
    global _passed
    if cond:
        _passed += 1
        print(f"  ✅ {label}")
    else:
        _failures.append(label)
        print(f"  ❌ {label}{(' — ' + detail) if detail else ''}")


def bound(rule_id: str, policy_ref: str, when: dict | None = None) -> dict:
    """A merged rule set holding ONE policy binding, expanded through the real registry —
    the same path `resolve_write` drives."""
    rule = {"id": rule_id, "policy": policy_ref}
    if when:
        rule["when"] = when
    merged = {"rules": {rule_id: rule}, "validators": {}, "sources": [str(ROOT)]}
    errs = epr_meta.expand_policies(merged, POLICIES)
    if errs:
        _failures.append(f"expand_policies({policy_ref}): {errs}")
    return merged


def inline(rule_id: str, validator: str, when: dict) -> dict:
    """A merged rule set holding ONE inline validator rule (the shape
    `elohim/holochain/dna/.epr-meta` uses — registered machinery with no canon row)."""
    return {"rules": {rule_id: {"id": rule_id, "class": "inject", "when": when,
                                "validator": validator, "why": "fixture"}},
            "validators": {}, "sources": [str(ROOT)]}


def verdict(merged: dict, write: dict):
    return epr_meta.combine(epr_meta.evaluate(merged, write))


def tmp_write(tmp: Path, rel: str, pre: str | None, post: str) -> dict:
    """Materialize the PRE-image on disk (validators read the prior state from the path) and
    return the write dict carrying the POST content — the resolver's own Edit synthesis shape.
    `tmp` carries a `.git` marker (see below) so `find_repo_root` treats it as a repo root and
    the validators' repo-relative path classification is exercised for real, not short-circuited
    to a basename by a rootless temp dir."""
    target = tmp / rel
    target.parent.mkdir(parents=True, exist_ok=True)
    if pre is not None:
        target.write_text(pre)
    return {"path": str(target), "content": post, "is_new": pre is None}


# ── registry preconditions (a stale pin or a demoted class would make every case below lie) ──
check("policies.yaml loads without errors", not POLICY_ERRS, f"{POLICY_ERRS}")
for ref, want_validator in (("c2-monotonic-authority@2", "epr:validator-heal-fills-never-moves"),
                            ("c6a-bounded-work@2", "epr:validator-bounded-work")):
    pol = POLICIES.get(ref) or {}
    check(f"{ref} is an active inject row naming {want_validator}",
          pol.get("class") == "inject" and pol.get("validator") == want_validator
          and pol.get("status") == "active",
          f"got {{class: {pol.get('class')}, validator: {pol.get('validator')}, "
          f"status: {pol.get('status')}}}")
check("c2-monotonic-authority@1 is superseded by @2 (lineage, not deletion)",
      (POLICIES.get("c2-monotonic-authority@1") or {}).get("superseded_by")
      == "c2-monotonic-authority@2")
for name in ("epr:validator-heal-fills-never-moves", "epr:validator-bounded-work",
             "epr:validator-dna-hash-neutrality"):
    check(f"{name} is registered in REFERENCE_VALIDATORS",
          name in epr_meta.REFERENCE_VALIDATORS)

# The storage manifest must PIN @2 — the whole point of a declared dependency is that someone
# re-declared it. A silent revert to @1 would leave the detector unbound and this suite green
# without it, which is the exact stale-pin blindness the cascade exists to close.
_storage_meta = (ROOT / "elohim/elohim-storage/src/.epr-meta").read_text()
check("elohim-storage binding re-declares c2-monotonic-authority@2",
      "policy: c2-monotonic-authority@2" in _storage_meta)
check("elohim-storage binds c6a-bounded-work@2", "policy: c6a-bounded-work@2" in _storage_meta)


with tempfile.TemporaryDirectory() as _td:
    TMP = Path(_td)
    # `find_repo_root` anchors on a `.git` marker; without one the fixture tree has no root and
    # every repo-relative classification collapses to a basename — which would make the
    # canonical-channel case below pass or fail for the wrong reason.
    (TMP / ".git").mkdir()

    # ══ 1. heal-fills-never-moves (C2) ═══════════════════════════════════════════════════════
    M_C2 = bound("heal-fills-never-moves", "c2-monotonic-authority@2")

    # POSITIVE — a net-new `StampMode::Declare` call site in a file that is NOT a canonical
    # channel. This is the 2026-07-11 resurrection class's entry shape: widening `Declare`.
    pre = ("fn heal(row: &Row) {\n"
           "    stamp_declared_head_mode(conn, ctx, id, head, None, None,\n"
           "        StampMode::GapFill, None);\n"
           "}\n")
    post = pre.replace("StampMode::GapFill", "StampMode::Declare")
    v = verdict(M_C2, tmp_write(TMP, "src/p2p/projection_reconcile.rs", pre, post))
    check("C2 POSITIVE: net-new StampMode::Declare outside a canonical channel fires at inject",
          v is not None and v.cls == "inject" and v.rule_id == "heal-fills-never-moves",
          f"got {v}")

    # NEGATIVE — the SAME added line inside a canonical channel. `content_diesel.rs` is where the
    # deliberate-declare wrapper and the guard arm live; firing there would make the advisory
    # unignorable noise on the one file that legitimately owns the mode.
    v = verdict(M_C2, tmp_write(TMP, "elohim/elohim-storage/src/db/content_diesel.rs", pre, post))
    check("C2 NEGATIVE: the same Declare inside a canonical channel stays silent",
          v is None, f"got {v}")

    # NEGATIVE — `CollectiveStampMode::Declare` on the collectives plane is a DIFFERENT enum that
    # merely contains the substring. A word-boundary miss here would fire on every collectives
    # edit forever; this case is the regression that keeps the boundary.
    pre2 = "fn reconcile() {\n    let m = CollectiveStampMode::GapFill;\n}\n"
    post2 = "fn reconcile() {\n    let m = CollectiveStampMode::Declare;\n}\n"
    v = verdict(M_C2, tmp_write(TMP, "src/reconcile/controller.rs", pre2, post2))
    check("C2 NEGATIVE: CollectiveStampMode::Declare (a different enum) never counts",
          v is None, f"got {v}")

    # POSITIVE (arm b) — the stamp guard is touched (`canonical_move_verdict`, the same arbiter
    # fn the real elohim-storage registry names — see its seam-registry.yaml row of the same
    # name) in a crate that DOES register a contract test for this file, and the post content
    # cites none of them. This is the "blind first fix" shape (defect #3): a guard edit with no
    # cited proof is exactly how the who-writes rewrite froze forward adoption.
    (TMP / "guarded_crate/src").mkdir(parents=True, exist_ok=True)
    (TMP / "guarded_crate" / "seam-registry.yaml").write_text(
        "crate: fixture-guarded\n"
        "crateRoot: .\n"
        "seamLocus: fixture\n"
        "decisionPoints:\n"
        "  - name: canonical_move_verdict\n"
        "    sourceLocation:\n"
        "      file: src/guard.rs\n"
        "    contractTests:\n"
        "      - path: guarded_crate/src/guard.rs\n"
        "        testName: canonical_move_verdict_is_antisymmetric_and_total\n"
        "        kind: unit\n")
    # The rule's own `when.contains-any` gate (`stamp_declared_head` / `StampMode` /
    # `projection_reconcile` / `GapFill`) must ALSO match or the validator never runs at all —
    # touching `StampMode::GapFill` alongside the arbiter fn keeps this a realistic guard-touch.
    pre_guard = "fn heal(row: &Row) {\n    let _ = row;\n}\n"
    post_guard = ("fn heal(row: &Row) {\n"
                  "    let _ = row;\n"
                  "    let verdict = canonical_move_verdict(row);\n"
                  "    let _ = StampMode::GapFill;\n"
                  "}\n")
    v = verdict(M_C2, tmp_write(TMP, "guarded_crate/src/guard.rs", pre_guard, post_guard))
    check("C2 POSITIVE arm(b): stamp-guard touch citing none of its registered contract "
          "tests fires at inject",
          v is not None and v.cls == "inject" and v.rule_id == "heal-fills-never-moves",
          f"got {v}")

    # NEGATIVE (arm b, the census-owned gap) — the SAME stamp-guard touch, but no
    # `seam-registry.yaml` ancestor registers ANY contract test for this file: `tests` comes
    # back empty, and an empty registry is an honest gap the CENSUS owns, not a finding this
    # validator invents. Firing here would punish a file the registry has simply never reached.
    v = verdict(M_C2, tmp_write(TMP, "unguarded_crate/src/guard.rs", pre_guard, post_guard))
    check("C2 NEGATIVE arm(b): the same stamp-guard touch with no registry registering any "
          "test for the file stays silent (skip-when-registry-registers-none)",
          v is None, f"got {v}")

    # ══ 2. bounded-work (C6a) ════════════════════════════════════════════════════════════════
    M_C6A = bound("bounded-work", "c6a-bounded-work@2")
    # The detector only speaks inside a REGISTERED seam (a `seam-registry.yaml` at an ancestor),
    # so the fixture builds one — proving the precision gate itself, not just the shape match.
    (TMP / "crate").mkdir(parents=True, exist_ok=True)
    (TMP / "crate" / "seam-registry.yaml").write_text(
        "crate: fixture\ncrateRoot: .\nseamLocus: fixture\ndecisionPoints: []\n")

    # POSITIVE — a retry ladder with no budget anywhere near it. The forcing instance
    # (`74fbdf2d7`) had NO loop token in the diff at all; this is that shape.
    pre = "async fn publish(item: Item) -> Result<()> {\n    send(item).await\n}\n"
    post = ("async fn publish(item: Item) -> Result<()> {\n"
            "    let max_retries = 20;\n"
            "    for _ in 0..max_retries {\n"
            "        if send(item.clone()).await.is_ok() { return Ok(()); }\n"
            "    }\n"
            "    Err(Error::Exhausted)\n"
            "}\n")
    v = verdict(M_C6A, tmp_write(TMP, "crate/src/publish.rs", pre, post))
    check("C6a POSITIVE: a new retry ladder with no budget fires at inject",
          v is not None and v.cls == "inject" and v.rule_id == "bounded-work", f"got {v}")

    # NEGATIVE — the same ladder WITH a declared budget in the window. The guarantee is a
    # declared budget, so declaring one must silence the advisory or the rule teaches nothing.
    post_ok = ("async fn publish(item: Item) -> Result<()> {\n"
               "    let deadline = Instant::now() + Duration::from_secs(30);\n"
               "    let max_retries = 20;\n"
               "    for _ in 0..max_retries {\n"
               "        if Instant::now() > deadline { break; }\n"
               "        if send(item.clone()).await.is_ok() { return Ok(()); }\n"
               "    }\n"
               "    Err(Error::Exhausted)\n"
               "}\n")
    v = verdict(M_C6A, tmp_write(TMP, "crate/src/publish_ok.rs", pre, post_ok))
    check("C6a NEGATIVE: the same ladder with a declared deadline stays silent",
          v is None, f"got {v}")

    # NEGATIVE (precision gate) — the offending shape OUTSIDE any registered seam. The detector
    # is deliberately scoped to crates whose decision points are enumerated.
    v = verdict(M_C6A, tmp_write(TMP, "unregistered/src/publish.rs", pre, post))
    check("C6a NEGATIVE: the same ladder outside a registered seam stays silent",
          v is None, f"got {v}")

    # NEGATIVE (comment suppression) — the SAME unbudgeted retry ladder as the POSITIVE fixture
    # above, but the author declared the budget in a `bounded-work: <budget>` comment near it.
    # The guarantee IS a declared budget, so declaring one this way must silence the advisory or
    # the marker teaches nothing.
    post_marked = ("async fn publish(item: Item) -> Result<()> {\n"
                   "    // bounded-work: 20 attempts, bounded by the caller's cancellation token\n"
                   "    let max_retries = 20;\n"
                   "    for _ in 0..max_retries {\n"
                   "        if send(item.clone()).await.is_ok() { return Ok(()); }\n"
                   "    }\n"
                   "    Err(Error::Exhausted)\n"
                   "}\n")
    v = verdict(M_C6A, tmp_write(TMP, "crate/src/publish_marked.rs", pre, post_marked))
    check("C6a NEGATIVE: the same ladder with a `bounded-work: <budget>` comment stays silent",
          v is None, f"got {v}")

    # ══ 3. dna-hash-neutrality (registered machinery, no canon row) ══════════════════════════
    M_DNA = inline("dna-hash-neutrality", "epr:validator-dna-hash-neutrality", {"write": "*"})
    # This validator classifies by REPO-RELATIVE path, so its fixtures must live under the real
    # repo root — a tmpdir would resolve outside the DNA tree and classify as neutral.
    dna = "elohim/holochain/dna/elohim/zomes"

    # POSITIVE — an integrity-zome source edit: hash-moving, therefore reinstall-only, therefore
    # the partition trap. (Path-classified; the file need not exist.)
    v = verdict(M_DNA, {"path": str(ROOT / dna / "content_store_integrity/src/lib.rs"),
                        "content": "pub fn validate() {}\n", "is_new": True})
    check("DNA POSITIVE: an integrity-zome .rs edit is labelled hash-moving at inject",
          v is not None and v.cls == "inject" and v.rule_id == "dna-hash-neutrality", f"got {v}")

    # NEGATIVE — the coordinator zome beside it: `update_coordinators` hot-swap, no re-key, no
    # churn. Firing here would nag on the cheap path and train the reader past the real one.
    v = verdict(M_DNA, {"path": str(ROOT / dna / "content_store/src/lib.rs"),
                        "content": "pub fn get_head() {}\n", "is_new": True})
    check("DNA NEGATIVE: a coordinator-zome edit is neutral and stays silent",
          v is None, f"got {v}")

    # NEGATIVE — a doc living inside an integrity zome directory compiles into nothing.
    v = verdict(M_DNA, {"path": str(ROOT / dna / "content_store_integrity/README.md"),
                        "content": "# notes\n", "is_new": True})
    check("DNA NEGATIVE: a README inside an integrity zome moves no hash",
          v is None, f"got {v}")

    # POSITIVE — a `dna.yaml` edit to the hashed `integrity:` slice (the `network_seed`
    # modifier). Read-only against the REAL on-disk manifest (never written to): `is_new: False`
    # so `_prior_text` reads the actual file, proving the diff against real content instead of
    # an invented pre-image.
    _dna_yaml_path = ROOT / "elohim/holochain/dna/elohim/dna.yaml"
    _dna_yaml_pre = _dna_yaml_path.read_text()
    check("DNA fixture precondition: dna.yaml has both integrity: and coordinator: keys",
          "\nintegrity:" in _dna_yaml_pre and "\ncoordinator:" in _dna_yaml_pre)

    post_modifiers = _dna_yaml_pre.replace('"elohim_lamad_alpha"', '"elohim_lamad_beta"')
    v = verdict(M_DNA, {"path": str(_dna_yaml_path), "content": post_modifiers, "is_new": False})
    check("DNA POSITIVE: a dna.yaml edit to the hashed `integrity:` slice (modifiers) is "
          "labelled hash-moving",
          v is not None and v.cls == "inject" and v.rule_id == "dna-hash-neutrality", f"got {v}")

    # NEGATIVE — the validator DOES distinguish inside dna.yaml itself: a `coordinator:`-only
    # edit (here, the coordinator zome's wasm path) leaves `_dna_modifier_block` byte-identical,
    # so the hot-swappable slice moved and the hashed slice did not — neutral, stays silent.
    post_coordinator_only = _dna_yaml_pre.replace(
        "target/wasm32-unknown-unknown/release/content_store.wasm",
        "target/wasm32-unknown-unknown/release/content_store_v2.wasm")
    check("DNA fixture precondition: the coordinator-only replacement actually changed the post",
          post_coordinator_only != _dna_yaml_pre)
    v = verdict(M_DNA, {"path": str(_dna_yaml_path), "content": post_coordinator_only,
                        "is_new": False})
    check("DNA NEGATIVE: a dna.yaml edit outside the hashed slice (coordinator-only) stays "
          "silent", v is None, f"got {v}")


print()
if _failures:
    print(f"{len(_failures)} FAILURE(S): {_failures}")
    sys.exit(1)
print(f"{_passed} concern-validator assertions passed ✅")
