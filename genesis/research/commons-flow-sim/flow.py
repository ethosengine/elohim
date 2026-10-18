#!/usr/bin/env python3
"""Commons flow simulation: one illustrative probe, not proof.

Reads a real Cargo.lock, builds the reliance graph from the workspace's own crates downward,
and flows an inflow (default $1,000) through it under a LENS (a JSON file of declared weights).
Each node splits only its own inbox, once per settlement window: its makers keep their labour
share, a positional (land-like) share pools at its tier commons, the rest splits across its
direct reliances, and any split below the precision floor pools at the tier commons with its
descent recorded instead of travelling further. Tiers keep a share and pass the rest upward
to the planetary commons.

Two invariants are enforced on every split (verify_split): the node's ratio lines sum to
exactly 1, and every ratio carries its reason (a presence, a cite record, the reverse-index
rollup, or descent naming why nothing could be attributed) plus the lens clause that weighted
it. Amounts are derived from ratios only where they are applied. Reasons sort the commons:
'positional' is explained by a community (land-like); 'no-record' and 'below-floor' are
unexplained (nobody yet; nobody at this precision) and pool with descent. A leaf's "rests on"
share is no-record, so leaving cites out never enriches the one who left them out.

Arithmetic is exact (fractions.Fraction), so a 1e-9 share stays visible and conservation is
checked to the last unit, not within a tolerance.

Usage:
    python3 flow.py                                  # elohim/Cargo.lock, lens.example.json
    python3 flow.py --floor 0                        # same lens, no precision floor
    python3 flow.py --bridge-with doorway/doorway-service/Cargo.lock
    python3 flow.py --lock path/to/Cargo.lock --lens my-lens.json --out out/result.json

Design doc: genesis/research/commons-composition-at-scale-2026-10-15.md
Standard library only. No cargo is run; the lockfile is parsed directly.
"""

from __future__ import annotations

import argparse
import json
import os
import sys
from collections import deque
from decimal import Decimal, getcontext
from fractions import Fraction

getcontext().prec = 60

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", "..", ".."))
ROOT = "(entry work)"


# ---------------------------------------------------------------------------
# Lockfile -> reliance graph
# ---------------------------------------------------------------------------

def parse_lock(text: str) -> list[dict]:
    """Parse the [[package]] blocks of a Cargo.lock (v3/v4) without a TOML library."""
    packages: list[dict] = []
    cur: dict | None = None
    in_deps = False
    for raw in text.splitlines():
        line = raw.strip()
        if line.startswith("["):
            if in_deps and line == "]":
                in_deps = False
                continue
            if cur is not None:
                packages.append(cur)
            cur = {"deps": []} if line == "[[package]]" else None
            in_deps = False
            continue
        if cur is None:
            continue
        if in_deps:
            if line == "]":
                in_deps = False
            elif line.startswith('"'):
                cur["deps"].append(line.rstrip(",").strip('"'))
            continue
        if line.startswith("dependencies = ["):
            rest = line[len("dependencies = ["):].strip()
            if rest == "]":
                continue
            in_deps = True
            continue
        for key in ("name", "version", "source"):
            if line.startswith(key + " = "):
                cur[key] = line.split(" = ", 1)[1].strip().strip('"')
    if cur is not None:
        packages.append(cur)
    return [p for p in packages if "name" in p]


def build_graph(packages: list[dict]) -> dict:
    """Collapse workspace crates (no `source`) into one entry work; external crates are nodes.

    A dependency line is `name`, `name version` or `name version (source)`; the bare form is
    unique in the lockfile by cargo's own rule.
    """
    by_key: dict[str, dict] = {}
    by_name: dict[str, list[str]] = {}
    for p in packages:
        key = f"{p['name']} {p.get('version', '?')}"
        by_key[key] = p
        by_name.setdefault(p["name"], []).append(key)

    def resolve(dep: str) -> str:
        parts = dep.split(" ")
        if len(parts) == 1:
            keys = by_name.get(parts[0], [])
            if len(keys) != 1:
                raise ValueError(f"ambiguous or unknown bare dependency {dep!r}: {keys}")
            return keys[0]
        key = f"{parts[0]} {parts[1]}"
        if key not in by_key:
            raise ValueError(f"unknown dependency {dep!r}")
        return key

    workspace = {k for k, p in by_key.items() if "source" not in p}
    if not workspace:
        raise ValueError("no workspace crates (packages without `source`) in lockfile")

    def node_of(key: str) -> str:
        return ROOT if key in workspace else key

    out: dict[str, list[str]] = {ROOT: []}
    for key, p in by_key.items():
        src = node_of(key)
        out.setdefault(src, [])
        for dep in p["deps"]:
            dst = node_of(resolve(dep))
            if dst == src or dst == ROOT:
                continue  # an edge inside the entry work, or back into it
            if dst not in out[src]:
                out[src].append(dst)
    for n in out:
        out[n].sort()

    # Keep only what the entry work actually rests on.
    reach: set[str] = set()
    q = deque([ROOT])
    while q:
        n = q.popleft()
        if n in reach:
            continue
        reach.add(n)
        q.extend(out[n])
    out = {n: [d for d in ds if d in reach] for n, ds in out.items() if n in reach}

    fan_in: dict[str, int] = {n: 0 for n in out}
    for n, ds in out.items():
        for d in ds:
            fan_in[d] += 1
    return {"out": out, "fan_in": fan_in, "workspace": sorted(workspace),
            "packages_total": len(by_key)}


def topo_order(out: dict[str, list[str]]) -> list[str]:
    indeg = {n: 0 for n in out}
    for ds in out.values():
        for d in ds:
            indeg[d] += 1
    q = deque(sorted(n for n, k in indeg.items() if k == 0))
    order: list[str] = []
    while q:
        n = q.popleft()
        order.append(n)
        for d in out[n]:
            indeg[d] -= 1
            if indeg[d] == 0:
                q.append(d)
    if len(order) != len(out):
        stuck = sorted(n for n, k in indeg.items() if k > 0)[:10]
        raise ValueError(f"reliance cycle among external crates, e.g. {stuck}")
    return order


def degrees(out: dict[str, list[str]]) -> dict[str, int]:
    """Shortest-path degree from the entry work: the 'LinkedIn' reading."""
    deg = {ROOT: 0}
    q = deque([ROOT])
    while q:
        n = q.popleft()
        for d in out[n]:
            if d not in deg:
                deg[d] = deg[n] + 1
                q.append(d)
    return deg


def path_counts(out: dict[str, list[str]], order: list[str]) -> dict[str, int]:
    """Distinct entry->node paths: the message count a naive per-path push would pay."""
    paths = {n: 0 for n in out}
    paths[ROOT] = 1
    for n in order:
        for d in out[n]:
            paths[d] += paths[n]
    return paths


# ---------------------------------------------------------------------------
# Lens
# ---------------------------------------------------------------------------

def F(x) -> Fraction:
    return Fraction(str(x))


def load_lens(path: str, floor_override: str | None = None) -> dict:
    with open(path, encoding="utf-8") as fh:
        raw = json.load(fh)
    tiers = {t["id"]: {"keep": F(t["keep"]), "parent": t["parent"]} for t in raw["tiers"]}
    lens = {
        "id": raw["lens"],
        "status": raw.get("status", ""),
        "inflow": F(raw["inflow"]["amount"]),
        "unit": raw["inflow"].get("unit", ""),
        "entry_keep": F(raw["entry"]["keep_for_own_labour"]),
        "upstream_keep": F(raw["upstream"]["keep_for_own_labour"]),
        "split_rule": raw["split_rule"]["rule"],
        "pos_cap": F(raw.get("positional_share", {}).get("cap", "0")),
        "pos_half": F(raw.get("positional_share", {}).get("half_point", 1)),
        "floor": F(floor_override if floor_override is not None
                   else raw["precision_floor"]["amount"]),
        "tiers": tiers,
        "tier_of": raw["tier_of"],
    }
    for k in ("entry_keep", "upstream_keep", "pos_cap"):
        if not (0 <= lens[k] <= 1):
            raise ValueError(f"lens {k} must be in [0, 1]")
    if lens["split_rule"] not in ("equal", "fan_in"):
        raise ValueError("split_rule.rule must be 'equal' or 'fan_in'")
    for tid, t in tiers.items():
        if t["parent"] is None and t["keep"] != 1:
            raise ValueError(f"top tier {tid} must keep everything (keep = 1)")
        if t["parent"] is not None and t["parent"] not in tiers:
            raise ValueError(f"tier {tid} names unknown parent {t['parent']}")
    return lens


# ---------------------------------------------------------------------------
# The local split: everything one node needs, and nothing else
# ---------------------------------------------------------------------------

# Every ratio line carries its reason. The reason's SUBJECT sorts it:
#   labour      a maker: the node's own makers (its presence)          -> kept by the node
#   cite        a maker upstream: a recorded cite of one reliance      -> sent to the reliance
#   positional  a community: the reverse-index rollup (fan-in)         -> tier commons (land-like)
#   no-record   nobody yet: the node rests on more than any record     -> tier commons, with descent
#               names (a leaf's whole "rests on" share)
#   below-floor nobody at this precision: a cite share smaller than    -> tier commons, with descent
#               the declared floor; a cost choice, never a deletion
EXPLAINED = ("labour", "cite", "positional")
UNEXPLAINED = ("no-record", "below-floor")
LENS_CLAUSES = ("entry.keep_for_own_labour", "upstream.keep_for_own_labour", "split_rule",
                "positional_share", "precision_floor")


def ratio_table(node: str, own_deps: list[str], own_fan_in: int,
                dep_fan_in: dict[str, int], is_entry: bool, lens: dict) -> list[dict]:
    """One node's ratio table under one lens version: the carried proof.

    Judged once per (node's cite record, lens version, rollup reading) and reused by every later
    inflow; it holds no amounts. Inputs are local: the node's own direct reliances (its cite
    record), its own fan-in (read from the reverse-index rollup, not by walking the graph) and,
    for the fan_in rule only, its reliances' fan-in from the same rollup.
    """
    keep_clause = "entry.keep_for_own_labour" if is_entry else "upstream.keep_for_own_labour"
    keep = lens["entry_keep"] if is_entry else lens["upstream_keep"]
    pos = Fraction(0)
    if lens["pos_cap"] and own_fan_in:
        pos = lens["pos_cap"] * Fraction(own_fan_in, 1) / (own_fan_in + lens["pos_half"])
    lines: list[dict] = []

    def line(kind, to, ratio, ref, clause):
        if ratio:
            lines.append({"kind": kind, "to": to, "ratio": ratio,
                          "reason": {"ref": ref, "lens_clause": clause}})

    line("positional", "tier", pos, f"rollup:reverse-index fan_in={own_fan_in}", "positional_share")
    line("labour", node, (1 - pos) * keep, f"presence:{node}", keep_clause)
    rests_on = (1 - pos) * (1 - keep)
    if not own_deps:
        line("no-record", "tier", rests_on, f"descent:{node} has no recorded reliance", keep_clause)
    elif rests_on:
        if lens["split_rule"] == "equal":
            weights = {d: 1 for d in own_deps}
        else:
            weights = {d: max(1, dep_fan_in.get(d, 1)) for d in own_deps}
        total_w = sum(weights.values())
        for d in own_deps:
            line("cite", d, rests_on * weights[d] / total_w, f"cite:{node}->{d}", "split_rule")
    return lines


def apply_floor(node: str, table: list[dict], inbox: Fraction, floor: Fraction) -> list[dict]:
    """The settlement-plane step: the only one that needs an amount. A cite line whose amount
    would fall below the floor keeps its ratio and becomes 'below-floor' with its descent; the
    table itself is untouched, so the ratios still sum to exactly 1."""
    out = []
    for l in table:
        if l["kind"] == "cite" and inbox * l["ratio"] < floor:
            out.append({"kind": "below-floor", "to": "tier", "ratio": l["ratio"],
                        "reason": {"ref": f"descent:{node}->{l['to']}",
                                   "lens_clause": "precision_floor"}})
        else:
            out.append(l)
    return out


def split_locally(node: str, inbox: Fraction, own_deps: list[str], own_fan_in: int,
                  dep_fan_in: dict[str, int], is_entry: bool, lens: dict) -> list[dict]:
    """One node's split of its own inbox for one settlement window: table, then floor."""
    return apply_floor(node, ratio_table(node, own_deps, own_fan_in, dep_fan_in, is_entry, lens),
                       inbox, lens["floor"])


def verify_split(node: str, lines: list[dict], own_deps: list[str]) -> None:
    """What any peer can check of one split with only the node's own records and the lens:
    the ratios sum to exactly 1, and every ratio names a reason that resolves locally."""
    total = sum((l["ratio"] for l in lines), Fraction(0))
    if total != 1:
        raise AssertionError(f"{node}: ratios sum to {total}, not exactly 1")
    for l in lines:
        ref, clause = l["reason"]["ref"], l["reason"]["lens_clause"]
        if clause not in LENS_CLAUSES:
            raise AssertionError(f"{node}: ratio cites unknown lens clause {clause!r}")
        if l["kind"] == "cite" and (l["to"] not in own_deps or ref != f"cite:{node}->{l['to']}"):
            raise AssertionError(f"{node}: cite share to {l['to']} has no cite record")
        if l["kind"] == "labour" and ref != f"presence:{node}":
            raise AssertionError(f"{node}: labour share names no presence")
        if not ref:
            raise AssertionError(f"{node}: a ratio with no reason")


# ---------------------------------------------------------------------------
# The run
# ---------------------------------------------------------------------------

def lens_key(lens: dict) -> str:
    """The lens version a table is judged under (inflow and floor are settlement-plane)."""
    return json.dumps([lens["id"], str(lens["entry_keep"]), str(lens["upstream_keep"]),
                       lens["split_rule"], str(lens["pos_cap"]), str(lens["pos_half"])])


def run(lock_path: str, lens: dict, tables: dict | None = None) -> dict:
    with open(lock_path, encoding="utf-8") as fh:
        g = build_graph(parse_lock(fh.read()))
    out, fan_in = g["out"], g["fan_in"]
    order = topo_order(out)
    deg = degrees(out)
    paths = path_counts(out, order)

    inbox = {n: Fraction(0) for n in out}
    inbox[ROOT] = lens["inflow"]
    senders = {n: 0 for n in out}
    kept = {n: Fraction(0) for n in out}
    tier_in = {t: Fraction(0) for t in lens["tiers"]}
    tier_by_kind = {t: {k: Fraction(0) for k in ("positional", "no-record", "below-floor")}
                    for t in lens["tiers"]}
    by_deg: dict[int, dict] = {}
    edges_sent = edges_pooled = nodes_split = ratio_lines = f64_split_misses = 0
    tables = {} if tables is None else tables
    tables_judged = tables_reused = 0
    tables_by_node: dict[str, list[dict]] = {}
    max_den_digits = 0
    residue_log: list[dict] = []

    def tier_for(n: str) -> str:
        return lens["tier_of"]["entry" if n == ROOT else "registry"]

    for n in order:  # one settlement window per hop; order only sequences the windows
        if inbox[n] == 0:
            continue
        nodes_split += 1
        key = (n, tuple(out[n]), fan_in[n], lens_key(lens))
        if key in tables:
            tables_reused += 1  # the carried proof: no re-judgement, only the floor at the edge
        else:
            tables[key] = ratio_table(n, out[n], fan_in[n], fan_in, n == ROOT, lens)
            verify_split(n, tables[key], out[n])  # judged once: sums to 1, every reason resolves
            tables_judged += 1
        lines = apply_floor(n, tables[key], inbox[n], lens["floor"])
        verify_split(n, lines, out[n])  # the cheap check every hop pays
        tables_by_node[n] = lines
        ratio_lines += len(lines)
        if sum(float(l["ratio"]) for l in lines) != 1.0:
            f64_split_misses += 1  # the same check, had the ratios been f64
        t = tier_for(n)
        row = by_deg.setdefault(deg[n], {"nodes_reached": 0, "received": Fraction(0),
                                         "kept_by_makers": Fraction(0),
                                         "positional_to_commons": Fraction(0),
                                         "no_record_to_commons": Fraction(0),
                                         "below_floor_to_commons": Fraction(0)})
        row["nodes_reached"] += 1
        row["received"] += inbox[n]
        for l in lines:
            amt = inbox[n] * l["ratio"]  # amounts are derived here, at the edge
            max_den_digits = max(max_den_digits, len(str(amt.denominator)))
            k = l["kind"]
            if k == "labour":
                kept[n] += amt
                row["kept_by_makers"] += amt
            elif k == "cite":
                inbox[l["to"]] += amt
                senders[l["to"]] += 1
                edges_sent += 1
            else:
                tier_in[t] += amt
                tier_by_kind[t][k] += amt
                row[{"positional": "positional_to_commons", "no-record": "no_record_to_commons",
                     "below-floor": "below_floor_to_commons"}[k]] += amt
                if k == "below-floor":
                    edges_pooled += 1
                if k != "positional":
                    residue_log.append({"from": n, "kind": k, "reason": l["reason"]["ref"],
                                        "amount": amt})

    # Tiers aggregate on the receive side once per window, then pass upward.
    # Each bucket keeps its reason kind as it climbs, so every tier can say what it holds.
    kinds = ("positional", "no-record", "below-floor")
    tier_kept = {t: Fraction(0) for t in lens["tiers"]}
    tier_kept_by_kind = {t: {k: Fraction(0) for k in kinds} for t in lens["tiers"]}
    tier_passed = {t: Fraction(0) for t in lens["tiers"]}
    tier_received = dict(tier_in)
    pending = {t: dict(tier_by_kind[t]) for t in lens["tiers"]}
    for _ in range(len(lens["tiers"]) + 1):
        moved = False
        for tid, t in lens["tiers"].items():
            for kind in kinds:
                amt = pending[tid][kind]
                if not amt:
                    continue
                pending[tid][kind] = Fraction(0)
                k = amt * t["keep"]
                tier_kept[tid] += k
                tier_kept_by_kind[tid][kind] += k
                if t["parent"] is not None and amt - k:
                    tier_passed[tid] += amt - k
                    pending[t["parent"]][kind] += amt - k
                    tier_received[t["parent"]] += amt - k
                    moved = True
        if not moved:
            break

    total = sum(kept.values()) + sum(tier_kept.values())
    conserved = total == lens["inflow"]
    hot = max((n for n in out if n != ROOT), key=lambda n: (fan_in[n], n))
    recipients = [n for n in out if kept[n] > 0]
    reached_zero = [n for n in out if n != ROOT and inbox[n] == 0]
    smallest = min(recipients, key=lambda n: kept[n]) if recipients else None

    floats = [float(kept[n]) for n in out if kept[n]] + [float(v) for v in tier_kept.values() if v]
    float_sums = {"insertion_order": sum(floats), "reversed": sum(reversed(floats)),
                  "ascending": sum(sorted(floats)), "descending": sum(sorted(floats, reverse=True))}

    return {
        "lock": os.path.relpath(lock_path, REPO),
        "graph": {
            "packages_in_lockfile": g["packages_total"],
            "workspace_crates_collapsed_into_entry": len(g["workspace"]),
            "external_nodes_relied_on": len(out) - 1,
            "edges": sum(len(ds) for ds in out.values()),
            "max_degree": max(deg.values()),
            "direct_reliances_of_entry": len(out[ROOT]),
        },
        "out": out, "fan_in": fan_in, "deg": deg, "paths": paths,
        "inbox": inbox, "kept": kept, "senders": senders,
        "by_deg": by_deg, "tier_received": tier_received, "tier_kept": tier_kept,
        "tier_passed": tier_passed, "residue_log": residue_log,
        "tier_kept_by_kind": tier_kept_by_kind,
        "tables_by_node": tables_by_node,
        "work": {
            "nodes_split": nodes_split,
            "ratio_lines_verified": ratio_lines,
            "ratio_tables_judged": tables_judged,
            "ratio_tables_reused": tables_reused,
            "max_amount_denominator_digits": max_den_digits,
            "f64_splits_failing_exact_sum_check": f64_split_misses,
            "edges_sent": edges_sent,
            "edges_pooled_below_floor": edges_pooled,
            "edges_touched": edges_sent + edges_pooled,
            "naive_per_path_pushes": sum(paths.values()) - 1,
            "max_paths_to_one_node": max(paths[n] for n in out if n != ROOT),
            "max_paths_node": max((n for n in out if n != ROOT), key=lambda n: (paths[n], n)),
            "hot_spot": hot,
            "hot_spot_fan_in": fan_in[hot],
            "hot_spot_messages_this_window": senders[hot],
        },
        "recipients": len(recipients),
        "reached_but_below_floor": len(reached_zero),
        "smallest_recipient": smallest,
        "total_accounted": total,
        "conserved": conserved,
        "float_sums": float_sums,
    }


# ---------------------------------------------------------------------------
# Inward: a signal from one upstream back toward the edge (a retraction)
# ---------------------------------------------------------------------------

def inward(res: dict, target_name: str) -> dict:
    """Who rests on `target`, by reverse degree, and what reaching them costs pushed vs pulled.

    Pushed: the upstream (or its steward) sends along every reverse edge into the affected set,
    now. Pulled: each holon re-checks only its own cites at its own cadence; a dependent learns
    one window after the thing it cites marks itself affected, so the farthest one waits as many
    windows as its reverse degree, and the per-window check costs every edge whether or not
    anything was retracted.
    """
    out = res["out"]
    matches = sorted(n for n in out if n.split(" ")[0] == target_name or n == target_name)
    if not matches:
        raise ValueError(f"{target_name!r} is not in the reliance graph")
    target = max(matches, key=lambda n: (res["fan_in"][n], n))
    rev: dict[str, list[str]] = {n: [] for n in out}
    for u, ds in out.items():
        for d in ds:
            rev[d].append(u)
    rdeg = {target: 0}
    q = deque([target])
    while q:
        n = q.popleft()
        for u in rev[n]:
            if u not in rdeg:
                rdeg[u] = rdeg[n] + 1
                q.append(u)
    affected = set(rdeg)
    pushed = sum(1 for u, ds in out.items() for d in ds if d in affected)
    by: dict[int, int] = {}
    for n, d in rdeg.items():
        if n != target:
            by[d] = by.get(d, 0) + 1
    return {"target": target, "dependents_by_reverse_degree": dict(sorted(by.items())),
            "entry_reverse_degree": rdeg.get(ROOT), "pushed_messages": pushed,
            "pulled_checks_per_window": sum(len(ds) for ds in out.values()),
            "pulled_windows_to_farthest": max(rdeg.values()),
            "value_through_target": res["inbox"][target],
            "inflow_share": res["inbox"][target] / res["inbox"][ROOT]}


# ---------------------------------------------------------------------------
# Mixing depth: does attribution stop distinguishing its route, as ancestry does?
# ---------------------------------------------------------------------------

def mixing(res: dict) -> dict:
    """For each node, the composition of its inbox by first-degree branch (which direct reliance
    of the entry the value came through), against the background composition of all value at
    degree >= 2. Total-variation distance 0 means the node's attribution no longer says which
    route it came by: the identical-ancestors analogue. Floats are used: this is a reading, not
    an account."""
    out, inbox, deg = res["out"], res["inbox"], res["deg"]
    order = topo_order(out)
    comp: dict[str, dict[str, float]] = {n: {} for n in out}
    for b in out[ROOT]:
        comp[b] = {b: 1.0}
    for n in order:
        if n == ROOT or not inbox[n] or not comp[n]:
            continue
        sent = {d: float(inbox[n] * r) for d, r in
                ((l["to"], l["ratio"]) for l in res["tables_by_node"].get(n, []) if l["kind"] == "cite")}
        for d, amt in sent.items():
            if deg[d] < 2:
                continue
            for b, w in comp[n].items():
                comp[d][b] = comp[d].get(b, 0.0) + w * amt
    for n in out:
        if deg.get(n, 0) >= 2:
            tot = sum(comp[n].values())
            comp[n] = {b: v / tot for b, v in comp[n].items()} if tot else {}
    bg: dict[str, float] = {}
    for n in out:
        if deg.get(n, 0) >= 2 and comp[n]:
            w = float(inbox[n])
            for b, v in comp[n].items():
                bg[b] = bg.get(b, 0.0) + v * w
    z = sum(bg.values()) or 1.0
    bg = {b: v / z for b, v in bg.items()}
    rows: dict[int, list[tuple[float, float, int]]] = {}
    for n in out:
        if deg.get(n, 0) >= 2 and comp[n]:
            keys = set(bg) | set(comp[n])
            tv = 0.5 * sum(abs(comp[n].get(k, 0.0) - bg.get(k, 0.0)) for k in keys)
            rows.setdefault(deg[n], []).append((tv, float(inbox[n]), len(comp[n])))
    summary = {}
    for d, xs in sorted(rows.items()):
        w = sum(x[1] for x in xs) or 1.0
        summary[d] = {"nodes": len(xs), "tv_weighted": sum(x[0] * x[1] for x in xs) / w,
                      "tv_min": min(x[0] for x in xs),
                      "branches_mean": sum(x[2] for x in xs) / len(xs)}
    return {"branches": len(out[ROOT]), "by_degree": summary}


# ---------------------------------------------------------------------------
# Reporting
# ---------------------------------------------------------------------------

def dec(x: Fraction, places: int = 6) -> str:
    d = Decimal(x.numerator) / Decimal(x.denominator)
    if x != 0 and abs(d) < Decimal("0.0001"):
        return f"{d:.3E}"
    return f"{d:,.{places}f}"


def frac(x: Fraction) -> str:
    return f"{x.numerator}/{x.denominator}" if x.denominator != 1 else str(x.numerator)


def bridge(res_a: dict, res_b: dict) -> dict:
    """Presences relied on by both independently grown networks (keyed by crate name)."""
    def by_name(res):
        acc: dict[str, Fraction] = {}
        for n, v in res["kept"].items():
            if n == ROOT or not v:
                continue
            acc[n.split(" ")[0]] = acc.get(n.split(" ")[0], Fraction(0)) + v
        return acc
    a, b = by_name(res_a), by_name(res_b)
    shared = sorted(set(a) & set(b), key=lambda k: (-min(a[k], b[k]), k))

    def versions(res, name):
        return {n.split(" ")[1] for n, v in res["kept"].items() if n != ROOT and v and n.split(" ")[0] == name}
    # In the valueflows ledger's words: the same name and version on both sides is an
    # IdentityShape mapping; the same project at different versions is a Reconciliation the
    # bridge must declare (two heads of one presence); one side only stays unreconciled, placed.
    identity = [k for k in shared if versions(res_a, k) == versions(res_b, k)]
    return {
        "identity_shape": len(identity), "needs_reconciliation": len(shared) - len(identity),
        "only_a": len(set(a) - set(b)), "only_b": len(set(b) - set(a)),
        "a_presences": len(a), "b_presences": len(b), "shared_presences": len(shared),
        "a_to_shared": sum((a[k] for k in shared), Fraction(0)),
        "b_to_shared": sum((b[k] for k in shared), Fraction(0)),
        "top_shared": [{"crate": k, "from_a": a[k], "from_b": b[k]} for k in shared[:10]],
    }


def summary_text(res: dict, lens: dict, top: int, br: dict | None, br_lock: str | None,
                 sig: dict | None = None) -> str:
    L: list[str] = []
    u = lens["unit"]
    L.append(f"COMMONS FLOW SIMULATION  lens={lens['id']}")
    L.append(f"  {lens['status']}")
    L.append(f"  lockfile {res['lock']}: {res['graph']['packages_in_lockfile']} packages; "
             f"{res['graph']['workspace_crates_collapsed_into_entry']} workspace crates are the entry work; "
             f"{res['graph']['external_nodes_relied_on']} external crates relied on; "
             f"{res['graph']['edges']} reliance edges; deepest shortest-path degree "
             f"{res['graph']['max_degree']}")
    L.append(f"  inflow {dec(lens['inflow'], 2)} {u}; entry keeps {frac(lens['entry_keep'])}, "
             f"upstream keeps {frac(lens['upstream_keep'])}, split={lens['split_rule']}, "
             f"positional cap {frac(lens['pos_cap'])} (half at {frac(lens['pos_half'])} dependents), "
             f"precision floor {dec(lens['floor'], 2)} {u}")
    L.append("")
    L.append("BY DEGREE (shortest path from the entry work). Commons columns are what nodes at")
    L.append("that degree sent to their tier, by the reason kind of the ratio.")
    L.append(f"  {'deg':>3} {'reached':>7} {'received':>14} {'kept by makers':>14} "
             f"{'positional':>12} {'no-record':>12} {'below-floor':>12}")
    for d in sorted(res["by_deg"]):
        r = res["by_deg"][d]
        L.append(f"  {d:>3} {r['nodes_reached']:>7} {dec(r['received'], 4):>14} "
                 f"{dec(r['kept_by_makers'], 4):>14} {dec(r['positional_to_commons'], 4):>12} "
                 f"{dec(r['no_record_to_commons'], 4):>12} {dec(r['below_floor_to_commons'], 4):>12}")
    nodes_at = {}
    for n, dg in res["deg"].items():
        nodes_at[dg] = nodes_at.get(dg, 0) + 1
    L.append("  graph size by degree (all nodes, reached or not): " +
             ", ".join(f"{d}:{nodes_at[d]}" for d in sorted(nodes_at)))
    L.append("")
    L.append(f"TOP {top} RECIPIENTS (kept for their makers' labour)")
    ranked = sorted((n for n in res["kept"] if n != ROOT and res["kept"][n]),
                    key=lambda n: (-res["kept"][n], n))
    for n in ranked[:top]:
        L.append(f"  {dec(res['kept'][n], 4):>12} {u}  {n:<34} degree {res['deg'][n]}  "
                 f"fan-in {res['fan_in'][n]}")
    L.append(f"  entry work keeps {dec(res['kept'][ROOT], 4)} {u}")
    if res["smallest_recipient"]:
        s = res["smallest_recipient"]
        L.append(f"  smallest recipient: {s} keeps {dec(res['kept'][s], 12)} {u} "
                 f"(exact {frac(res['kept'][s])})")
    L.append(f"  recipients {res['recipients']} (incl. entry); reached nothing because every split "
             f"toward them fell below the floor: {res['reached_but_below_floor']}")
    L.append("")
    L.append("DISSOLVED INTO THE COMMONS, BY TIER")
    for tid in lens["tiers"]:
        L.append(f"  {tid:<24} received {dec(res['tier_received'][tid], 6):>14}  "
                 f"kept {dec(res['tier_kept'][tid], 6):>14}  passed up {dec(res['tier_passed'][tid], 6):>14}")
    L.append("  what each tier keeps, by reason: explained (positional: a community) vs unexplained")
    L.append("  (no-record: nobody yet; below-floor: nobody at this precision)")
    for tid in lens["tiers"]:
        kk = res["tier_kept_by_kind"][tid]
        L.append(f"  {tid:<24} explained {dec(kk['positional'], 6):>12}  "
                 f"unexplained {dec(kk['no-record'] + kk['below-floor'], 6):>12} "
                 f"(no-record {dec(kk['no-record'], 6)}, below-floor {dec(kk['below-floor'], 6)})")
    expl = sum(res["kept"].values(), Fraction(0)) + sum(
        (res["tier_kept_by_kind"][t]["positional"] for t in lens["tiers"]), Fraction(0))
    L.append(f"  whole inflow: explained {dec(expl, 6)} {u} (makers' labour + positional), "
             f"unexplained {dec(lens['inflow'] - expl, 6)} {u}")
    placed = sum(res["tier_kept"].values(), Fraction(0)) - sum(
        (res["tier_kept_by_kind"][t]["positional"] for t in lens["tiers"]), Fraction(0))
    unplaced = lens["inflow"] - expl - placed
    L.append(f"  account: explained {dec(expl / lens['inflow'], 6)} + unexplained but placed with descent "
             f"{dec(placed / lens['inflow'], 6)} + unplaced {dec(unplaced / lens['inflow'], 6)} = "
             f"{frac((expl + placed + unplaced) / lens['inflow'])}; closed: {unplaced == 0}")
    top_tier = [t for t, v in lens["tiers"].items() if v["parent"] is None][0]
    share = res["tier_kept"][top_tier] / lens["inflow"]
    L.append(f"  {top_tier} holds {dec(res['tier_kept'][top_tier], 9)} {u} = {dec(share, 9)} of the inflow "
             f"(exact {frac(res['tier_kept'][top_tier])})")
    L.append("")
    w = res["work"]
    L.append("WORK")
    L.append(f"  nodes that split: {w['nodes_split']}  edges touched: {w['edges_touched']} "
             f"(sent {w['edges_sent']}, pooled below floor {w['edges_pooled_below_floor']})")
    L.append(f"  a naive per-path push would send {w['naive_per_path_pushes']:,} messages; "
             f"most paths into one node: {w['max_paths_to_one_node']:,} ({w['max_paths_node']})")
    L.append(f"  hot spot: {w['hot_spot']} with fan-in {w['hot_spot_fan_in']}; "
             f"messages it receives this window: {w['hot_spot_messages_this_window']}")
    L.append(f"  ratio tables judged this run: {w['ratio_tables_judged']} (one per node under this lens "
             f"version and rollup reading); reused from an earlier judgement: {w['ratio_tables_reused']}")
    L.append(f"  ratio lines verified (sum exactly 1, every ratio a resolvable reason): "
             f"{w['ratio_lines_verified']}; largest amount denominator: "
             f"{w['max_amount_denominator_digits']} digits; the same splits held as f64 would fail an "
             f"exact sum-to-1 check {w['f64_splits_failing_exact_sum_check']} times of {w['nodes_split']}")
    L.append("")
    L.append("CONSERVATION")
    L.append(f"  makers {dec(sum(res['kept'].values()), 9)} + commons {dec(sum(res['tier_kept'].values()), 9)} "
             f"= {frac(res['total_accounted'])} {u}; exact match to inflow: {res['conserved']}")
    fs = res["float_sums"]
    L.append("  the same shares summed as f64 in four orders: " +
             ", ".join(f"{k} {v!r}" for k, v in fs.items()))
    if sig is not None:
        L.append("")
        L.append(f"INWARD SIGNAL (a retraction at {sig['target']})")
        L.append("  dependents by reverse degree: " + ", ".join(
            f"{d}:{c}" for d, c in sig["dependents_by_reverse_degree"].items())
            + f"; the entry work is {sig['entry_reverse_degree']} hops back")
        L.append(f"  value that rested on it in this flow: {dec(sig['value_through_target'], 6)} {u} "
                 f"({dec(sig['inflow_share'], 9)} of the inflow)")
        L.append(f"  pushed: {sig['pushed_messages']} messages along reverse edges, now; pulled: no "
                 f"messages, {sig['pulled_checks_per_window']} own-cite checks per window across all "
                 f"holons, farthest dependent informed after {sig['pulled_windows_to_farthest']} windows")
    if br is not None:
        L.append("")
        L.append(f"BRIDGE with {br_lock} (same lens, same inflow, independent network)")
        L.append(f"  presences reached: this {br['a_presences']}, other {br['b_presences']}, "
                 f"relied on by both {br['shared_presences']}")
        L.append(f"  shared presences at identical versions (IdentityShape): {br['identity_shape']}; "
                 f"same project, different versions (Reconciliation to declare): {br['needs_reconciliation']}; "
                 f"unreconciled and placed in their own network: this {br['only_a']}, other {br['only_b']}")
        L.append(f"  recognition reaching shared presences: from this {dec(br['a_to_shared'], 4)} {u}, "
                 f"from other {dec(br['b_to_shared'], 4)} {u}")
        for row in br["top_shared"][:8]:
            L.append(f"    {row['crate']:<24} this {dec(row['from_a'], 4):>10}  other {dec(row['from_b'], 4):>10}")
    return "\n".join(L)


def to_json(res: dict, lens: dict, br: dict | None, br_lock: str | None) -> dict:
    def j(x: Fraction) -> dict:
        return {"exact": frac(x), "decimal": dec(x, 12)}
    ranked = sorted((n for n in res["kept"] if res["kept"][n]), key=lambda n: (-res["kept"][n], n))
    return {
        "lens": lens["id"], "status": lens["status"], "lock": res["lock"],
        "inflow": j(lens["inflow"]), "unit": lens["unit"], "floor": j(lens["floor"]),
        "graph": res["graph"],
        "by_degree": {str(d): {k: (j(v) if isinstance(v, Fraction) else v) for k, v in r.items()}
                      for d, r in sorted(res["by_deg"].items())},
        "recipients": [{"node": n, "kept": j(res["kept"][n]), "degree": res["deg"][n],
                        "fan_in": res["fan_in"][n]} for n in ranked],
        "tiers": {t: {"received": j(res["tier_received"][t]), "kept": j(res["tier_kept"][t]),
                      "passed_up": j(res["tier_passed"][t]),
                      "kept_by_reason": {k: j(v) for k, v in res["tier_kept_by_kind"][t].items()}}
                  for t in lens["tiers"]},
        "unexplained_descent": [{"from": r["from"], "kind": r["kind"], "reason": r["reason"],
                                 "amount": j(r["amount"])} for r in res["residue_log"]],
        "work": res["work"],
        "conservation": {"total": j(res["total_accounted"]), "exact": res["conserved"]},
        "float_sums": res["float_sums"],
        "bridge": None if br is None else {
            "with": br_lock, "a_presences": br["a_presences"], "b_presences": br["b_presences"],
            "shared_presences": br["shared_presences"], "a_to_shared": j(br["a_to_shared"]),
            "identity_shape": br["identity_shape"], "needs_reconciliation": br["needs_reconciliation"],
            "only_a": br["only_a"], "only_b": br["only_b"],
            "b_to_shared": j(br["b_to_shared"]),
            "top_shared": [{"crate": r["crate"], "from_a": j(r["from_a"]), "from_b": j(r["from_b"])}
                           for r in br["top_shared"]]},
    }


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--lock", default=os.path.join(REPO, "elohim", "Cargo.lock"))
    ap.add_argument("--lens", default=os.path.join(HERE, "lens.example.json"))
    ap.add_argument("--floor", default=None, help="override the lens's precision floor")
    ap.add_argument("--bridge-with", default=None, help="a second Cargo.lock to bridge against")
    ap.add_argument("--out", default=os.path.join(HERE, "out", "result.json"))
    ap.add_argument("--top", type=int, default=12)
    ap.add_argument("--signal-from", default=None,
                    help="a crate name: report an inward signal (retraction) from it to the edge")
    ap.add_argument("--mixing", action="store_true",
                    help="report how route composition converges with degree (ancestry analogue)")
    ap.add_argument("--inflows", type=int, default=1,
                    help="repeat the inflow N times under the same lens to count table reuse")
    a = ap.parse_args(argv)

    def path(p: str) -> str:
        return p if os.path.isabs(p) else os.path.join(REPO, p) if not os.path.exists(p) else p

    lens = load_lens(path(a.lens), a.floor)
    tables: dict = {}
    res = run(path(a.lock), lens, tables)
    judged, reused = res["work"]["ratio_tables_judged"], res["work"]["ratio_tables_reused"]
    for _ in range(a.inflows - 1):
        again = run(path(a.lock), lens, tables)
        judged += again["work"]["ratio_tables_judged"]
        reused += again["work"]["ratio_tables_reused"]
        if not again["conserved"]:
            res["conserved"] = False
    br = br_lock = None
    if a.bridge_with:
        res_b = run(path(a.bridge_with), lens)
        br, br_lock = bridge(res, res_b), res_b["lock"]
    sig = inward(res, a.signal_from) if a.signal_from else None
    print(summary_text(res, lens, a.top, br, br_lock, sig))
    if a.mixing:
        m = mixing(res)
        print(f"\nMIXING (route composition over {m['branches']} first-degree branches vs background; "
              f"total-variation 0 = route no longer distinguishable)")
        for d, r in m["by_degree"].items():
            print(f"  degree {d}: nodes {r['nodes']:>4}  TV value-weighted {r['tv_weighted']:.3f}  "
                  f"TV min {r['tv_min']:.3f}  branches reaching a node, mean {r['branches_mean']:.1f}")
    if a.inflows > 1:
        print(f"\nREPEATED INFLOWS: {a.inflows} inflows under one lens version judged {judged} ratio "
              f"tables and reused them {reused} times; each reuse paid only the sum-and-resolve "
              f"check and the floor at the edge")
    os.makedirs(os.path.dirname(os.path.abspath(a.out)), exist_ok=True)
    with open(a.out, "w", encoding="utf-8") as fh:
        json.dump(to_json(res, lens, br, br_lock), fh, indent=1)
    print(f"\nwrote {os.path.relpath(os.path.abspath(a.out), REPO)}")
    return 0 if res["conserved"] else 1


if __name__ == "__main__":
    sys.exit(main())
