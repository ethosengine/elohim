"""SEAL-TIME CHECKS — the Cedar ethic on rails we already own.

No schema language (JSON Schema, pydantic, OWL, SHACL) can express these: every one is a
CROSS-DOCUMENT GRAPH predicate over the policy corpus, not a shape over one document.
Law they encode: an UNANALYZABLE policy is REJECTED, never admitted-as-advisory. Today's
inversion — `_eval_rule`'s `Verdict("inject", "validator `X` not registered (advisory)")`
(epr_meta.py) — turns an unresolvable `deny` into a nudge. That is the hole S5 closes.
"""
from __future__ import annotations

from pathlib import Path

# Stratum ladder: an exception may only narrow FROM a strictly-lower stratum. Referencing
# its own-or-higher stratum lets a narrowing rule re-widen through recursion.
STRATA = ("observation", "plan", "knowledge", "constitutional")
SEVERITY = {"deny": 3, "ask": 2, "inject": 1, "measure": 0, "dispatch": 0}


class SealError(str):
    """A rejection, not a warning — once seal is wired as a gate.

    NOT WIRED YET (verified 2026-08-05): nothing in `.husky/`, `scripts/ci/`, or any
    Jenkinsfile calls `seal_repo`. It must not be wired as-is — `seal_repo(Path('.'))`
    against the live corpus returns 16 errors, most of them `S2-STRATUM … unknown
    stratum None` because no real `.epr-meta` policy carries a `stratum:` field yet.
    Populate strata first, drive the corpus to zero, then make it blocking.
    """


def _err(code: str, subject: str, msg: str) -> SealError:
    return SealError(f"{code} {subject}: {msg}")


def s1_default_deny(policy_set: dict) -> list[SealError]:
    """A policy set with no explicit default arm is unanalyzable: silence means whatever the
    engine happens to do. Cedar/Rego fix the default in the ENGINE; we make it DECLARED."""
    d = policy_set.get("default")
    if d != "deny":
        return [_err("S1-DEFAULT", policy_set.get("id", "?"),
                     f"`default:` must be `deny` (got {d!r}). A policy set whose silence is "
                     f"undeclared cannot be analyzed; declare the default arm.")]
    return []


def s2_exception_stratum(policy: dict) -> list[SealError]:
    """`Except{grant, exceptions}` narrows. Every exception must sit at a STRICTLY LOWER
    stratum than its grant — otherwise the exception can re-derive the grant it narrows,
    and narrow-never-widen stops being syntactic."""
    out: list[SealError] = []
    grant_s = policy.get("stratum")
    if grant_s not in STRATA:
        return [_err("S2-STRATUM", policy.get("id", "?"),
                     f"unknown stratum {grant_s!r}; must be one of {STRATA}")]
    gi = STRATA.index(grant_s)
    for ex in policy.get("exceptions", []) or []:
        es = (ex or {}).get("stratum")
        if es not in STRATA:
            out.append(_err("S2-STRATUM", policy["id"], f"exception stratum {es!r} unknown"))
        elif STRATA.index(es) >= gi:
            out.append(_err("S2-STRATUM", policy["id"],
                            f"exception at `{es}` references its own-or-higher stratum "
                            f"(grant is `{grant_s}`) — a narrowing rule may only except from "
                            f"strictly below, else it can re-widen."))
    return out


def s3_closure_acyclic(policy: dict, acyclic_proofs: dict) -> list[SealError]:
    """A `closure-via` over a relation not PROVEN acyclic is an unbounded traversal — the
    universal-join shape we refuse. The proof is a named artifact (a test or a graph gate),
    not a promise; `max-depth` alone does not substitute (it truncates, it does not prove)."""
    out: list[SealError] = []
    for c in policy.get("closure-via", []) or []:
        rel = (c or {}).get("relation")
        proof = acyclic_proofs.get(rel)
        if not proof:
            out.append(_err("S3-CLOSURE", policy.get("id", "?"),
                            f"closure over relation `{rel}` has no acyclicity proof. Register "
                            f"`{rel}` in the registry's `acyclicRelations` with a `provenBy:` "
                            f"ref (e.g. brit-graph `is_cyclic_directed` gate), or drop it."))
    return out


def s4_witness_rule(policy: dict) -> list[SealError]:
    """Every operator that can produce a verdict must name the rule that emits its
    CheckWitness. An unwitnessed operator produces a decision nobody can explain — and
    every *success* check goes unrecorded, which is the row REA/VF already gets right."""
    ops = [k for k in ("grant", "exceptions", "closure-via", "refer") if k in policy]
    if ops and not policy.get("witness"):
        return [_err("S4-WITNESS", policy.get("id", "?"),
                     f"operators {ops} carry no `witness:` rule id — every verdict must be "
                     f"attributable to a CheckWitness{{rule_id, outcome, fact_cid, observed_at}}.")]
    return []


def s5_validator_resolvable(rules: dict, registered: set[str]) -> list[SealError]:
    """SOUNDNESS INVERSION, closed. Today an unregistered ref downgrades the rule's class to
    `inject` — a `deny` silently becomes advice. At seal it is a REJECTION: an unresolvable
    reference means the policy cannot be evaluated, and an unevaluable policy is not a weaker
    policy, it is an INVALID one."""
    out: list[SealError] = []
    for rid, rule in (rules or {}).items():
        ref = rule.get("validator")
        if ref and ref not in registered:
            out.append(_err("S5-UNRESOLVED", rid,
                            f"validator `{ref}` is not registered. Register it in "
                            f"`REFERENCE_VALIDATORS` (Python) AND `repository_validators.rs` "
                            f"(Rust) — the two registries must agree — or remove the rule. "
                            f"An unevaluable rule is invalid, not advisory."))
    return out


def seal(policy_set: dict, rules: dict, registered: set[str]) -> list[SealError]:
    """Full seal over one corpus. [] == sealable. Intended gate, not yet wired — see SealError."""
    proofs = policy_set.get("acyclicRelations", {}) or {}
    out = s1_default_deny(policy_set) + s5_validator_resolvable(rules, registered)
    for pol in policy_set.get("policies", []) or []:
        out += s2_exception_stratum(pol) + s3_closure_acyclic(pol, proofs) + s4_witness_rule(pol)
    return out


def seal_repo(root: Path) -> list[SealError]:
    """Intended entry point for `.husky/pre-push` and the genesis CI stage.

    No caller wires it yet, and it returns 16 errors against the live corpus today.
    Run it for advice; do not gate on it until those are driven to zero.
    """
    import sys
    sys.path.insert(0, str(Path(root) / ".claude/scripts"))
    from _lib import epr_meta  # noqa: PLC0415
    policies, errs = epr_meta.load_policies(Path(root))
    corpus = {"id": "epr-meta-policies", "default": "deny",
              "policies": list(policies.values())}
    rules = {k: v for k, v in ((p.get("id"), p) for p in policies.values()) if k}
    return [SealError(f"REGISTRY {e}") for e in errs] + \
        seal(corpus, rules, set(epr_meta.REFERENCE_VALIDATORS))
