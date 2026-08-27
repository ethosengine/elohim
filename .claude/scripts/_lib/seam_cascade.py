"""seam_cascade.py — the PIN-KEYED cascade (seam-concern-contract plan, task P4.2, surface 5).

WHAT A CASCADE IS HERE. Policy bindings resolve by MODE 2 (pinned): a binding declares
`policy: <id>@<version>` and that version keeps binding until the binding itself re-declares.
So a registry version bump moves NOTHING — every bound point stays on the old semantics, silently,
forever. The cascade is the worklist that closes that gap: diff each binding's declared pin against
its policy's registry TIP (followed through supersession lineage), and emit ONE fingerprinted
finding per stale pin so the existing flag->agent->canon->stasis machinery can dispatch it.

KEYED ON PINS, NEVER ON BUNDLE OR DNA HASHES — the plan states this as a hard clause and the reason
is structural: a coordinator-zome-only change moves NO DNA hash at all (the hash covers integrity
zomes + modifiers), so a hash-keyed cascade is blind to exactly the hot-swap class it most needs to
see. Nothing in this module reads a bundle digest, a DNA hash, or a build id. The registry pin is
the only key.

TWO CASCADE TRIGGERS:
  1. STALE PIN — binding pins @N, the policy's active tip is @M (M != N, reachable from @N by
     following `superseded_by`). One finding per (binding-site, policy id, N->M).
  2. GRADUATION FAN-OUT — a policy row whose NEW version changes its `scaffold` / `successor` /
     `graduation_trigger` fields relative to its lineage predecessor. That is the C13
     (graduated-authority) event: a scaffold's successor landing means every matrix cell citing
     that class must be re-examined, whether or not any binding is stale. The fan-out is over the
     CELLS (concern x seam), not over the bindings — a graduation invalidates examinations, not
     pins. The live case is fixture-only today (no canon row carries scaffold fields yet), which is
     why the detector is written against the FIELD SET and not against a hardcoded C13 id: when the
     god-mode canonical-head declarer graduates to earned-tier arbitration, the row that records it
     fires this hook with no code change.

SUPERSESSION LINEAGE CROSSES HOMES. C6a graduated from `concerns.yaml@1` to `policies.yaml@2`
(`superseded_by: policies.yaml#c6a-bounded-work@2`), so lineage resolution must read BOTH homes and
tolerate a home-qualified `superseded_by` (`<file>#<id>@<v>`) as well as the bare form (`<id>@<v>`).
A cascade that only read policies.yaml would report C6a's graduation as an unknown policy.

LEDGER SHAPE AND SUPPRESSION — MIRRORED FROM THE MEASURE TIER, NOT INVENTED. Findings are filed to
`.claude/data/cascade-findings.jsonl` in the identical row shape the measure tier uses for
`.claude/data/architecture-findings.jsonl` ({fp, rule, policy, path, detail, first_seen, status}),
through the shared `_lib.findings_ledger` primitives: flock'd read-check-append (two sessions can
never double-file), one line per LIVE fingerprint, and the row DELETED when the pin is re-declared
(`drain`) rather than parked as closed. Re-citation of an already-filed finding is debounced
through the SAME `.claude/data/epr-meta-advice.json` store the resolver hook uses, under a
`cascade-fp:` key prefix — one advisory per ~working session, never per run.

WHO FILES. `placement-audit.py --epr-meta` RENDERS the cascade read-only (it is a scoreboard and
must stay one); `.claude/scripts/seam-audit.py --cascade --file` is the runnable surface that
appends to the ledger and prints the dispatch directives. Deliberate split: an audit that mutates
state as a side effect of being read is how a scoreboard becomes an actor.

FAIL-LOUD, DEGRADE PER ROW. A binding manifest that will not parse is ONE finding naming that file;
every other manifest still loads (the EprRouter poisoned-scope lesson). An unknown policy id in a
binding is already fail-loud at the resolver (`expand_policies` drops the rule with an advisory);
here it surfaces as a `binding-unknown-policy` finding so the cascade does not silently score it
"not stale".
"""
from __future__ import annotations

import hashlib
import json
from pathlib import Path

from _lib import epr_meta as _em
from _lib import findings_ledger as _fl
from _lib import seam_census as _sc

try:
    import yaml
except Exception:  # pragma: no cover
    yaml = None

CASCADE_LEDGER_REL = ".claude/data/cascade-findings.jsonl"

# Fields whose change between two versions of the SAME policy id constitutes a C13 graduation
# event. Deliberately field-keyed, not id-keyed: the hook must fire for whatever row records the
# next scaffold->successor landing, not only for a class we happened to name today.
GRADUATION_FIELDS = ("scaffold", "successor", "successor_criterion", "graduation_trigger")

MANIFEST_FILENAME = ".epr-meta"


# ───────────────────────────── binding discovery ─────────────────────────────

def discover_manifest_paths(repo_root: Path) -> list[Path]:
    """Every authored governance manifest under repo_root, in BOTH forms.

    NEITHER HALF OF THIS IS RE-IMPLEMENTED HERE, on purpose. The bounded walk is
    `seam_census.iter_dirs` — one exclusion vocabulary, including `.claude/worktrees` for the
    correctness reason documented at its definition. WHICH file in a directory *is* the
    manifest is answered by `epr_meta.manifest_for_dir` — the resolver's own rule, so this
    cascade can never watch a form the resolver does not enforce, nor miss one it does.

    Both halves were previously restated here, and both restatements had gone stale:

      - The walk carried a private skip set that never received `.claude/worktrees`, so 80 of
        the 125 discovered manifests were stale copies inside two agent worktrees, reporting
        phantom stale pins against policy versions no resolver in this tree enforces.
      - The form was pinned to the flat `.epr-meta` FILE on the stated ground that the resolver
        "resolves only the flat form". That ceased to be true when the directory form landed:
        `epr_meta.manifest_path` now documents "Directory form wins" and `manifest_for_dir`
        tries it FIRST. The cost was that the REPO-ROOT CHARTER — `.epr-meta/manifest.md`,
        where the constitutional policies bind — was invisible to the stale-pin watch. An
        instrument blind to the charter while reading 24 phantom bindings is not measuring
        the tree.
    """
    return sorted(m for m in (_em.manifest_for_dir(d) for d in _sc.iter_dirs(repo_root))
                  if m is not None)


def bindings_in(path: Path) -> tuple[list[dict], list[str]]:
    """Every `policy: <id>@<version>` binding in ONE manifest -> (bindings, errors).
    A binding is {rule_id, policy_id, version, ref}. Parse errors degrade to an error string;
    they never raise."""
    try:
        cfg = _em.load_meta(path)
    except Exception as e:  # noqa: BLE001
        return [], [f"unreadable manifest: {e}"]
    if not isinstance(cfg, dict) or not cfg:
        return [], []
    rules = cfg.get("rules")
    if not isinstance(rules, list):
        return [], []
    out: list[dict] = []
    errs: list[str] = []
    for i, rule in enumerate(rules):
        if not isinstance(rule, dict):
            continue
        ref = rule.get("policy")
        if not ref:
            continue
        rid = rule.get("id") or f"<rule {i}>"
        if not isinstance(ref, str) or not _em._POLICY_REF_RE.match(ref):
            errs.append(f"rule `{rid}` policy ref {ref!r} is not `<id>@<version>`")
            continue
        pid, ver = ref.rsplit("@", 1)
        out.append({"rule_id": rid, "policy_id": pid, "version": int(ver), "ref": ref})
    return out, errs


# ───────────────────────────── registry lineage ─────────────────────────────

def _norm_superseded_by(value) -> str | None:
    """`superseded_by` may be bare (`c2-monotonic-authority@2`) or home-qualified
    (`policies.yaml#c6a-bounded-work@2`, the ACROSS-HOMES graduation form). Both normalize to
    `<id>@<version>`; anything else is None (unparseable lineage, reported by the caller)."""
    if not isinstance(value, str) or not value.strip():
        return None
    v = value.strip()
    if "#" in v:
        v = v.rsplit("#", 1)[1]
    return v if _em._POLICY_REF_RE.match(v) else None


def canon_rows(repo_root: Path) -> tuple[dict, list[str]]:
    """Both canon homes merged into one `{id@version: row}` index, with each row carrying a
    `__home__` field. Keys collide only if the same id@version exists in both files — which the
    concerns.yaml header explicitly forbids ("two rows keyed the same in two files would make a
    pin ambiguous"), so a collision is reported as an error rather than resolved."""
    rows: dict = {}
    errs: list[str] = []
    policies, perrs = _em.load_policies(repo_root)
    errs.extend(perrs)
    concerns, cerrs = _sc.load_concerns(repo_root)
    errs.extend(cerrs)
    for home, src in (("policies.yaml", policies), ("concerns.yaml", concerns)):
        for key, row in src.items():
            if not isinstance(row, dict):
                continue
            if key in rows:
                errs.append(f"registry row `{key}` exists in BOTH homes — a pin to it is ambiguous")
                continue
            r = dict(row)
            r["__home__"] = home
            rows[key] = r
    return rows, errs


def resolve_tip(rows: dict, ref: str, *, max_hops: int = 16) -> tuple[str | None, list[str], str | None]:
    """Follow `superseded_by` from `ref` to the lineage TIP.
    Returns (tip_ref, path_including_ref, error). `tip_ref` is None when `ref` itself is unknown.
    Cycle- and depth-guarded: lineage is a chain by construction, and a cycle in it is a registry
    defect that must be reported, never spun on."""
    seen: list[str] = []
    cur = ref
    for _ in range(max_hops):
        if cur in seen:
            return cur, seen, f"supersession cycle at `{cur}`"
        seen.append(cur)
        row = rows.get(cur)
        if row is None:
            return (None, seen, None) if len(seen) == 1 else (
                seen[-2], seen, f"`{cur}` is named by supersession lineage but is not in either registry")
        nxt = _norm_superseded_by(row.get("superseded_by"))
        if nxt is None:
            if str(row.get("status", "")).strip() == "superseded":
                return cur, seen, (f"`{cur}` is marked superseded but names no parseable "
                                    f"`superseded_by` — lineage is broken, not absent")
            return cur, seen, None
        cur = nxt
    return cur, seen, f"supersession lineage from `{ref}` exceeded {max_hops} hops"


# ───────────────────────────── findings ─────────────────────────────

def stale_pin_fingerprint(binding_site: str, policy_id: str, from_ref: str, to_ref: str) -> str:
    """STABLE fingerprint: binding-site + policy id + from->to, and nothing else. Prose, line
    numbers and the finding's detail text are excluded on purpose — re-wording a policy's `why`
    must not re-mint every stale-pin finding, exactly as the measure tier fingerprints `rule|path`
    rather than the reason string. The same stale pin therefore keeps ONE identity across runs,
    across sessions, and across edits to the policy body."""
    key = f"{binding_site}|{policy_id}|{from_ref}->{to_ref}"
    return hashlib.sha256(key.encode("utf-8")).hexdigest()[:12]


def graduation_fingerprint(policy_id: str, from_ref: str, to_ref: str, cell: str) -> str:
    """Graduation fan-out fingerprint: one per (policy lineage step, matrix cell). A second
    graduation of the same class re-fans with different refs and therefore different fingerprints —
    correct, because a second graduation is a second re-examination."""
    key = f"graduation|{policy_id}|{from_ref}->{to_ref}|{cell}"
    return hashlib.sha256(key.encode("utf-8")).hexdigest()[:12]


def _finding(kind: str, fp: str, *, policy: str, path: str, detail: str, agent: str) -> dict:
    """The measure tier's row shape, field for field, so both ledgers read the same and the hook
    can be refactored onto the shared primitive without a migration."""
    return {"fp": fp, "rule": kind, "policy": policy, "path": path, "detail": detail,
            "dispatch_agent": agent, "first_seen": _fl.utc_now(), "status": "open"}


def _dispatch_agent(row: dict | None) -> str:
    if isinstance(row, dict):
        a = row.get("dispatch-agent") or row.get("dispatch_agent")
        if isinstance(a, str) and a.strip():
            return a.strip()
    return "general-purpose"


def cascade_data(repo_root: Path, *, matrix: dict | None = None) -> dict:
    """Compute the whole cascade. Never raises; every failure mode degrades to a finding at the
    narrowest granularity that failed. `matrix` (from `_lib.seam_matrix.matrix_data`) is optional:
    without it, graduation events are reported but cannot fan out over cells, so the fan-out list
    is empty and a note says why — a missing input must state itself, never silently score zero."""
    repo_root = Path(repo_root)
    rows, reg_errs = canon_rows(repo_root)

    findings: list[dict] = []
    errors: list[str] = list(reg_errs)
    bindings: list[dict] = []
    stale: list[dict] = []

    for mpath in discover_manifest_paths(repo_root):
        try:
            rel = mpath.relative_to(repo_root).as_posix()
        except ValueError:
            rel = str(mpath)
        found, errs = bindings_in(mpath)
        for e in errs:
            errors.append(f"{rel}: {e}")
        for b in found:
            site = f"{rel}#{b['rule_id']}"
            b = dict(b, site=site, manifest=rel)
            bindings.append(b)
            tip, lineage, lerr = resolve_tip(rows, b["ref"])
            if lerr:
                errors.append(f"{site}: {lerr}")
            if tip is None:
                findings.append(_finding(
                    "cascade-binding-unknown-policy",
                    stale_pin_fingerprint(site, b["policy_id"], b["ref"], "<unknown>"),
                    policy=b["ref"], path=rel,
                    detail=(f"binding `{site}` pins `{b['ref']}`, which exists in NEITHER canon "
                            f"home. The resolver already drops this rule with an advisory; the "
                            f"cascade files it so an unknown pin is never scored 'not stale'."),
                    agent="general-purpose"))
                continue
            if tip == b["ref"]:
                continue
            tip_row = rows.get(tip)
            fp = stale_pin_fingerprint(site, b["policy_id"], b["ref"], tip)
            chain = lineage if lineage and lineage[-1] == tip else lineage + [tip]
            hops = " -> ".join(chain)
            homes = " / ".join(sorted({rows[r]["__home__"] for r in chain if r in rows}))
            detail = (f"STALE PIN: `{site}` pins `{b['ref']}` but the lineage tip is `{tip}` "
                      f"({hops}; home(s): {homes}). Resolution is PINNED (mode 2) — the tip moved, "
                      f"this binding did not, and it keeps the OLD semantics until it re-declares. "
                      f"Re-conform the point to `{tip}` or document a variance.")
            stale.append({"site": site, "manifest": rel, "policy_id": b["policy_id"],
                          "from": b["ref"], "to": tip, "fp": fp, "lineage": chain,
                          "concern": (tip_row or {}).get("concern")})
            findings.append(_finding("cascade-stale-pin", fp, policy=tip, path=rel,
                                     detail=detail, agent=_dispatch_agent(tip_row)))

    # ── graduation events (C13 scaffold -> successor) ──
    graduations = detect_graduations(rows)
    fanout: list[dict] = []
    notes: list[str] = []
    if graduations and matrix is None:
        notes.append("graduation event(s) detected but no matrix was supplied — fan-out over "
                      "cells was NOT computed (pass matrix_data() to compute it)")
    for g in graduations:
        cells = _cells_citing(matrix, g["concern"]) if matrix else []
        if not cells and matrix is not None:
            notes.append(f"graduation {g['from']} -> {g['to']} (class {g['concern']}) fans out "
                          f"over ZERO cells — no registered decision point cites that class yet")
        for cell in cells:
            fp = graduation_fingerprint(g["policy_id"], g["from"], g["to"], cell)
            fanout.append({"fp": fp, "cell": cell, **g})
            findings.append(_finding(
                "cascade-graduation-reexamine", fp, policy=g["to"], path=cell,
                detail=(f"GRADUATION FAN-OUT: `{g['policy_id']}` moved {g['from']} -> {g['to']} and "
                        f"the new version CHANGES {', '.join(g['changed_fields'])}. That is a C13 "
                        f"scaffold->successor event: cell `{cell}` cites class {g['concern']} and "
                        f"its examination was made against the scaffold. Re-examine it against the "
                        f"successor criterion."),
                agent=_dispatch_agent(rows.get(g["to"]))))

    return {
        "n_manifests": len(set(b["manifest"] for b in bindings)),
        "n_bindings": len(bindings),
        "bindings": bindings,
        "stale_pins": stale,
        "graduations": graduations,
        "graduation_fanout": fanout,
        "findings": findings,
        "errors": errors,
        "notes": notes,
    }


def detect_graduations(rows: dict) -> list[dict]:
    """Every lineage step whose successor version CHANGES one of GRADUATION_FIELDS.
    Walks each superseded row to its immediate successor and diffs the field set — so this fires
    on the first landing of a `successor:`/`scaffold:` field just as it does on a change to one
    (an absent field becoming present IS the graduation being recorded)."""
    out: list[dict] = []
    for key, row in sorted(rows.items()):
        nxt_ref = _norm_superseded_by(row.get("superseded_by"))
        if not nxt_ref:
            continue
        nxt = rows.get(nxt_ref)
        if not isinstance(nxt, dict):
            continue
        changed = [f for f in GRADUATION_FIELDS if row.get(f) != nxt.get(f)]
        if not changed:
            continue
        out.append({
            "policy_id": key.rsplit("@", 1)[0],
            "from": key, "to": nxt_ref,
            "concern": nxt.get("concern") or row.get("concern"),
            "changed_fields": changed,
            "from_home": row.get("__home__"), "to_home": nxt.get("__home__"),
        })
    return out


def _cells_citing(matrix: dict | None, concern: str | None) -> list[str]:
    """Every matrix cell id (`<concern>x<seam>`) that CITES `concern` — i.e. is examined at all
    (state conformant / variant / n-a). An `unexamined` cell has no examination to invalidate, so a
    graduation does not fan out over it; it is already on the forecast's side of the ledger."""
    if not matrix or not concern:
        return []
    out = []
    for cell in matrix.get("cells", []):
        if cell.get("concern") != concern:
            continue
        if cell.get("state") in ("conformant", "variant", "n-a"):
            out.append(f"{cell['concern']}x{cell['seam']}")
    return sorted(out)


# ───────────────────────────── filing ─────────────────────────────

def file_findings(repo_root: Path, data: dict) -> dict:
    """Append every NEW finding to the cascade ledger, drain rows whose fingerprint is no longer
    live, and return {new, already, contended, drained, dispatches}. Idempotent by construction:
    a second run files nothing (fingerprints are stable) and drains nothing (the live set did not
    change). The dispatch directives mirror the measure tier's phrasing verbatim in shape so an
    agent reading either ledger acts the same way."""
    repo_root = Path(repo_root)
    ledger = repo_root / CASCADE_LEDGER_REL
    new, already, contended = [], [], []
    for f in data["findings"]:
        r = _fl.append_if_new(ledger, f)
        if r is True:
            new.append(f)
        elif r is False:
            already.append(f)
        else:
            contended.append(f)
    drained = _fl.drain(ledger, {f["fp"] for f in data["findings"]})
    dispatches = [_dispatch_text(f) for f in new]
    for f in already:
        if not _fl.debounced(repo_root, f"cascade-fp:{f['fp']}"):
            dispatches.append(
                f"[cascade] finding {f['fp']} ({f['rule']}) is still live and already filed in "
                f"{CASCADE_LEDGER_REL} (status open) — drain it, do not re-file.")
    return {"new": new, "already": already, "contended": contended,
            "drained": drained, "dispatches": dispatches}


def _dispatch_text(f: dict) -> str:
    agent = f.get("dispatch_agent") or "general-purpose"
    return (
        f"[cascade] NEW finding {f['fp']} ({f['rule']}): {f['detail']} Captured to "
        f"{CASCADE_LEDGER_REL}. DISPATCH NOW (do not derail the current task): launch the "
        f"`{agent}` agent via the Agent tool with run_in_background: true and the prompt "
        f"'Cascade finding {f['fp']}: {f['detail']} Re-conform the bound point to the tip policy "
        f"or record a documented variance in the binding, then drain the ledger row.' If that "
        f"agent type is unavailable this session, use general-purpose with the same prompt. Then "
        f"continue your current task.")


# ───────────────────────────── rendering ─────────────────────────────

def render_cascade(data: dict, *, as_json: bool = False) -> str:
    if as_json:
        return json.dumps(data, indent=1)
    lines: list[str] = []
    lines.append("CASCADE  (plan P4.2 — pin-keyed; never bundle/DNA-hash-keyed)")
    lines.append("=" * 72)
    lines.append(f"Bindings: {data['n_bindings']} across {data['n_manifests']} manifest(s)")
    if data["stale_pins"]:
        lines.append(f"\nSTALE PINS ({len(data['stale_pins'])}) — the tip moved, the binding did not:")
        for s in data["stale_pins"]:
            cls = f" [{s['concern']}]" if s.get("concern") else ""
            lines.append(f"  ⛔ {s['fp']}  {s['site']}")
            lines.append(f"      {s['from']} -> {s['to']}{cls}")
    else:
        lines.append("  ✓ zero stale pins — every binding re-declares at its policy's lineage tip")
    if data["graduations"]:
        lines.append(f"\nGRADUATION EVENTS ({len(data['graduations'])}) — C13 scaffold -> successor:")
        for g in data["graduations"]:
            lines.append(f"  • {g['policy_id']}: {g['from']} -> {g['to']} "
                          f"(class {g['concern']}; changed: {', '.join(g['changed_fields'])})")
        lines.append(f"  fan-out: {len(data['graduation_fanout'])} cell(s) to re-examine")
    else:
        lines.append("  ✓ no graduation events (no lineage step changes scaffold/successor fields)")
    for n in data.get("notes", []):
        lines.append(f"  ⚠ {n}")
    errs = data.get("errors", [])
    lines.append("")
    lines.append(f"FINDINGS: {len(data['findings'])} · registry/binding errors: {len(errs)}")
    for e in errs:
        lines.append(f"  ⛔ {e}")
    if data["findings"]:
        lines.append(f"  → file + dispatch with: python3 .claude/scripts/seam-audit.py "
                      f"--cascade --file")
    return "\n".join(lines)
