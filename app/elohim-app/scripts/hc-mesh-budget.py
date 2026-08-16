#!/usr/bin/env python3
"""hc-mesh-budget — Q11 atomic timing budget probe for the local hc-mesh quiesce
measure (genesis/docs/superpowers/plans/2026-08-16-minutes-quiesce-fixture-trust-
swarm-plan.md §4 Q11, §5 falsifiable targets).

Q11's rule (program plan §6, composed by this plan): the quiesce prediction
must be recorded BEFORE the measured run, not fitted after it. This script is
the falsifiable half of that rule —

    quiesce ≈ Σ (expected_count × unit_cost_ms ÷ parallelism) over the
              ~8 convergence-path atoms, in seconds

— and the residual (predicted − measured) is a NAMED design concern, not
noise to shrug off. A residual that keeps landing on the same atom says which
atom's assumption (count model or parallelism constant) is wrong.

THE 8 ATOMS (label vocabulary is `ConvergenceAtom` in
elohim/elohim-storage/src/metrics.rs, exported on the `elohim_atom_duration_ms`
HistogramVec, label `atom`; read-only reference, never edited by this script):

    put_record | inventory_page | head_batch_per_id | head_record_verify
    | adopt_declare | shard_fetch | manifest_persist | digest_fold

USAGE

  predict — before a measured run. Scrapes each mesh peer's /metrics for the
  observed atom histograms (elohim_atom_duration_ms_sum / _count per atom),
  pulls live reconcile/drain gauges from /p2p/status for the count model where
  derivable, applies the documented parallelism table, and emits + appends a
  markdown budget table to the run ledger.

    ./hc-mesh-budget.py predict [--rows 3439] [--from-file metrics.txt ...]

  settle — after the run, once the wall-clock verdict is known (e.g. from
  hc-mesh-quiesce.sh's own "wall=Ns" line). Appends the measured time and the
  residual (absolute + %) against the LAST predict recorded by this script.

    ./hc-mesh-budget.py settle --measured-secs 312

  Peers/ports mirror hc-mesh-monitor.py's PEERS list (storage HTTP 8090+i for
  matthew/jessica/james). Unreachable peers are reported, not fatal — the
  table still computes from whichever peers answered (falling back to the
  documented crude cost table below for atoms with zero live observations).

  --from-file <path> [<path> ...] reads one or more saved `/metrics` text
  dumps instead of live-scraping (repeatable; label each with
  --from-file-name to name it in the per-peer breakdown). This is what makes
  the script testable while the mesh is down — see the synthetic self-test
  invoked by this file's own doc-comment demo, or run:

    ./hc-mesh-budget.py predict --from-file /tmp/fixture-metrics.txt --rows 100

  Ledger: .claude/data/quiesce-budget-ledger.md (repo-root-relative, appended
  by both subcommands — the shift-visible prediction/residual record). State
  sidecar: .claude/data/quiesce-budget-last-predict.json (holds the most
  recent predict's total, for settle to diff against; overwritten each
  predict — the intended flow is one predict immediately before one measured
  run, then one settle).

Stdlib only (urllib + re + json), matching the other hc-mesh-* rails.
"""

from __future__ import annotations

import argparse
import datetime
import json
import math
import os
import re
import sys
import urllib.request

# ── Mesh topology (mirrors hc-mesh-monitor.py's PEERS) ─────────────────────

PEERS = [
    {"name": "matthew", "http": 8090},
    {"name": "jessica", "http": 8091},
    {"name": "james", "http": 8092},
]

PROBE_TIMEOUT = float(os.environ.get("MESH_BUDGET_PROBE_TIMEOUT", "2.0"))

REPO_ROOT = os.path.abspath(
    os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "..")
)
LEDGER_PATH = os.path.join(REPO_ROOT, ".claude", "data", "quiesce-budget-ledger.md")
STATE_PATH = os.path.join(
    REPO_ROOT, ".claude", "data", "quiesce-budget-last-predict.json"
)

ATOMS = [
    "put_record",
    "inventory_page",
    "head_batch_per_id",
    "head_record_verify",
    "adopt_declare",
    "shard_fetch",
    "manifest_persist",
    "digest_fold",
]

# ── Assumption tables (documented, crude-on-purpose per Q11's directive:
#    "crude is fine — the point is a falsifiable prediction, refined by
#    residuals"). Every number here is a candidate residual explanation. ──

# Fallback unit cost (ms) used ONLY when a peer's histogram has zero
# observations for that atom (fresh boot, or --from-file fixture with no
# data). Anchored to the bucket span comment beside ATOM_DURATION_MS in
# metrics.rs (0.1ms pure-fold floor .. 30s saturated-conductor ceiling):
#   - put_record: one kademlia put + local ack, no conductor round-trip.
#   - inventory_page: paged DB query + serialize, cross-peer HTTP.
#   - head_batch_per_id: DERIVED share of a batched extern (cheap per id).
#   - head_record_verify: single RPC + in-wasm signature/hash verify.
#   - adopt_declare: one candidate decision, in-wasm.
#   - shard_fetch: network race across peers + disk read, the priciest I/O
#     atom short of a conductor call.
#   - manifest_persist: small durable write + read-back.
#   - digest_fold: pure fold, sub-millisecond by the metrics.rs doc comment.
FALLBACK_UNIT_COST_MS = {
    "put_record": 50.0,
    "inventory_page": 150.0,
    "head_batch_per_id": 20.0,
    "head_record_verify": 80.0,
    "adopt_declare": 40.0,
    "shard_fetch": 120.0,
    "manifest_persist": 15.0,
    "digest_fold": 0.5,
}

# Effective parallelism per atom across the WHOLE mesh (not per-peer) — a
# single documented constant, per the task's instruction to state a "per-atom
# effective parallelism ... as a documented constant table":
#   - put_record: each of the 3 peers runs its own publish drain ticker
#     independently and concurrently -> 3.
#   - inventory_page: each peer inventories the other 2 concurrently over
#     HTTP -> 6 (3 peers x 2 remote targets), network-bound not CPU-bound.
#   - head_batch_per_id: HeadBatchResolver's own documented batch fanout (2)
#     x 3 peers running their own reconcile loop concurrently -> 6.
#   - head_record_verify: bounded concurrent-RPC fanout (assumed similar
#     order to ADOPT_CONTEST_FANOUT-style bounds elsewhere in this crate) x
#     3 peers -> 12. Least-confident constant in this table; a fast atom
#     that dominates the residual should make you revisit this first.
#   - adopt_declare: serialized against a peer's OWN chain head (source-chain
#     writes are single-writer), so only cross-peer concurrency counts -> 3.
#   - shard_fetch: swarm fetch races span multiple peers x candidate shard
#     holders concurrently -> 9 (3 peers x 3 candidate sources, generous).
#   - manifest_persist: per-peer local DB write, single-writer per peer,
#     3 peers concurrent -> 3.
#   - digest_fold: pure CPU fold, trivially parallel across the 3 peers'
#     own reconcile loops -> 3 (not higher: sub-ms cost makes the constant
#     nearly irrelevant to the total either way).
PARALLELISM = {
    "put_record": 3,
    "inventory_page": 6,
    "head_batch_per_id": 6,
    "head_record_verify": 12,
    "adopt_declare": 3,
    "shard_fetch": 9,
    "manifest_persist": 3,
    "digest_fold": 3,
}

PAGE_SIZE = 2000  # projection_reconcile's rotating inventory-window page size
                  # (documented beside PROJECTION_RECONCILE_KNOWN_GAPS in
                  # metrics.rs: "~2000-row page").
VERIFY_FRACTION = 0.3     # assumed share of pending ids needing a full
                          # ContentHeadRecord fetch beyond the batched hash.
BLOB_FRACTION = 0.1       # assumed share of rows that carry a blob (shard
                          # fetch / manifest persist only apply to these).


# ── Scraping ─────────────────────────────────────────────────────────────


def http_get_text(url):
    try:
        with urllib.request.urlopen(url, timeout=PROBE_TIMEOUT) as r:
            return r.read().decode("utf-8", "replace")
    except Exception:
        return None


def http_get_json(url):
    try:
        with urllib.request.urlopen(url, timeout=PROBE_TIMEOUT) as r:
            return json.loads(r.read())
    except Exception:
        return None


_HIST_SUM_RE = re.compile(
    r'^elohim_atom_duration_ms_sum\{atom="([a-z_]+)"\}\s+(\S+)'
)
_HIST_COUNT_RE = re.compile(
    r'^elohim_atom_duration_ms_count\{atom="([a-z_]+)"\}\s+(\S+)'
)


def parse_atom_histograms(text):
    """-> {atom: (sum_ms, count)} for whatever atoms appear in `text`."""
    out = {}
    if not text:
        return out
    sums, counts = {}, {}
    for line in text.splitlines():
        m = _HIST_SUM_RE.match(line)
        if m:
            try:
                sums[m.group(1)] = float(m.group(2))
            except ValueError:
                pass
            continue
        m = _HIST_COUNT_RE.match(line)
        if m:
            try:
                counts[m.group(1)] = float(m.group(2))
            except ValueError:
                pass
    for atom in ATOMS:
        if atom in sums and atom in counts:
            out[atom] = (sums[atom], counts[atom])
    return out


def collect_from_live_peers():
    """Scrape /metrics + /p2p/status from every reachable mesh peer.

    Returns (per_peer_hist, reachable_names, unreachable_names, pending_total,
    drain_backlog_total) — the last two are the live count-model inputs.
    """
    per_peer_hist = {}
    reachable, unreachable = [], []
    pending_total = 0.0
    drain_backlog_total = 0.0
    any_status = False

    for peer in PEERS:
        base = f"http://127.0.0.1:{peer['http']}"
        metrics_text = http_get_text(f"{base}/metrics")
        status = http_get_json(f"{base}/p2p/status")
        if metrics_text is None and status is None:
            unreachable.append(peer["name"])
            continue
        reachable.append(peer["name"])
        if metrics_text is not None:
            per_peer_hist[peer["name"]] = parse_atom_histograms(metrics_text)
        if isinstance(status, dict):
            any_status = True
            pr = status.get("projectionReconcile")
            if isinstance(pr, dict) and isinstance(pr.get("pending"), (int, float)):
                pending_total += pr["pending"]
            drain = status.get("drain")
            if isinstance(drain, dict):
                total = drain.get("total") or 0
                published = drain.get("published") or 0
                drain_backlog_total += max(0, total - published)

    return {
        "per_peer_hist": per_peer_hist,
        "reachable": reachable,
        "unreachable": unreachable,
        "pending_total": pending_total if any_status else None,
        "drain_backlog_total": drain_backlog_total if any_status else None,
    }


def collect_from_files(paths):
    """Read one or more saved /metrics text dumps as if they were peers
    peer-0, peer-1, ... (or the file's basename). No /p2p/status available
    from a flat text dump, so the live count-model inputs are None — the
    caller falls back to --rows scaling, which is the point: this path
    exists so the script is testable with the mesh down."""
    per_peer_hist = {}
    reachable = []
    for path in paths:
        name = os.path.splitext(os.path.basename(path))[0]
        try:
            with open(path, "r") as f:
                text = f.read()
        except OSError as e:
            print(f"warn: could not read --from-file {path}: {e}", file=sys.stderr)
            continue
        per_peer_hist[name] = parse_atom_histograms(text)
        reachable.append(name)
    return {
        "per_peer_hist": per_peer_hist,
        "reachable": reachable,
        "unreachable": [],
        "pending_total": None,
        "drain_backlog_total": None,
    }


# ── Count model (expected remaining work per atom) ──────────────────────


def expected_counts(rows, peers_n, pending_total, drain_backlog_total):
    """Per-atom expected count, preferring live reconcile/drain surfaces
    where a plausible mapping exists, falling back to --rows x documented
    per-row multiplier otherwise. Returns {atom: (count, source)} where
    source is "live" or "rows-model" for the table's own legibility.

    Live mappings (only used when the surface returned a real number):
      - put_record        <- drain_backlog_total (queued-but-unpublished
                              records still owed by the mesh's drain tickers)
      - inventory_page     <- ceil(pending_total / PAGE_SIZE) x remote edges
                              (peers x (peers-1))
      - head_batch_per_id  <- pending_total (one batched share per pending id)
      - head_record_verify <- pending_total x VERIFY_FRACTION
      - adopt_declare      <- pending_total (one decision per pending id)
      - digest_fold        <- pending_total (one fold per id a peer heals)

    No live surface identified for shard_fetch / manifest_persist (blob-swarm
    counters record what ALREADY happened, not what remains) — these always
    use the rows-model. That gap is itself worth naming if the residual
    concentrates there.
    """
    remote_edges = peers_n * max(0, peers_n - 1)
    out = {}

    def rows_model(atom):
        if atom == "put_record":
            return rows * 1.0
        if atom == "inventory_page":
            return math.ceil(rows / PAGE_SIZE) * 2 * remote_edges  # 2 streams
        if atom == "head_batch_per_id":
            return rows * max(0, peers_n - 1)
        if atom == "head_record_verify":
            return rows * max(0, peers_n - 1) * VERIFY_FRACTION
        if atom == "adopt_declare":
            return rows * max(0, peers_n - 1)
        if atom == "shard_fetch":
            return rows * BLOB_FRACTION * max(0, peers_n - 1)
        if atom == "manifest_persist":
            return rows * BLOB_FRACTION * peers_n
        if atom == "digest_fold":
            return rows * peers_n
        raise AssertionError(atom)

    for atom in ATOMS:
        if atom == "put_record" and drain_backlog_total is not None:
            out[atom] = (max(drain_backlog_total, 0.0), "live:drain_backlog")
        elif atom == "inventory_page" and pending_total is not None:
            out[atom] = (
                math.ceil(max(pending_total, 0.0) / PAGE_SIZE) * remote_edges,
                "live:pending/page",
            )
        elif atom == "head_batch_per_id" and pending_total is not None:
            out[atom] = (max(pending_total, 0.0), "live:pending")
        elif atom == "head_record_verify" and pending_total is not None:
            out[atom] = (max(pending_total, 0.0) * VERIFY_FRACTION, "live:pending*frac")
        elif atom == "adopt_declare" and pending_total is not None:
            out[atom] = (max(pending_total, 0.0), "live:pending")
        elif atom == "digest_fold" and pending_total is not None:
            out[atom] = (max(pending_total, 0.0), "live:pending")
        else:
            out[atom] = (rows_model(atom), "rows-model")
    return out


def pooled_unit_cost_ms(per_peer_hist, atom):
    """Pool sum/count across all peers that reported this atom's histogram;
    fall back to the documented crude constant if nobody has observed it
    yet (fresh boot / synthetic fixture with no data for this atom)."""
    total_sum, total_count = 0.0, 0.0
    for hist in per_peer_hist.values():
        if atom in hist:
            s, c = hist[atom]
            total_sum += s
            total_count += c
    if total_count > 0:
        return total_sum / total_count, "observed", total_count
    return FALLBACK_UNIT_COST_MS[atom], "fallback", 0.0


# ── Table rendering ──────────────────────────────────────────────────────


def build_table(rows_n, peers_n, collected, source_label):
    counts = expected_counts(
        rows_n,
        peers_n,
        collected["pending_total"],
        collected["drain_backlog_total"],
    )
    lines = []
    lines.append("| atom | expected count | unit cost ms | parallelism | contribution s | count source | cost source |")
    lines.append("|---|---:|---:|---:|---:|---|---|")
    total_s = 0.0
    per_atom = {}
    for atom in ATOMS:
        count, count_source = counts[atom]
        unit_ms, cost_source, observed_n = pooled_unit_cost_ms(
            collected["per_peer_hist"], atom
        )
        parallelism = PARALLELISM[atom]
        contribution_s = (count * unit_ms / 1000.0) / max(1, parallelism)
        total_s += contribution_s
        per_atom[atom] = {
            "count": count,
            "count_source": count_source,
            "unit_ms": unit_ms,
            "cost_source": cost_source,
            "observed_n": observed_n,
            "parallelism": parallelism,
            "contribution_s": contribution_s,
        }
        lines.append(
            f"| {atom} | {count:.1f} | {unit_ms:.2f} | {parallelism} "
            f"| {contribution_s:.2f} | {count_source} "
            f"| {cost_source}{f' (n={observed_n:.0f})' if cost_source == 'observed' else ''} |"
        )
    lines.append(f"| **TOTAL** |  |  |  | **{total_s:.2f}** |  |  |")
    return "\n".join(lines), total_s, per_atom


# ── Ledger I/O ────────────────────────────────────────────────────────────


def append_ledger(block):
    os.makedirs(os.path.dirname(LEDGER_PATH), exist_ok=True)
    is_new = not os.path.exists(LEDGER_PATH)
    with open(LEDGER_PATH, "a") as f:
        if is_new:
            f.write(
                "# Quiesce atomic-timing-budget ledger (Q11)\n\n"
                "Append-only. `predict` records a falsifiable prediction "
                "BEFORE a measured hc-mesh quiesce run; `settle` records the "
                "measured wall-clock and residual after. Generated by "
                "`app/elohim-app/scripts/hc-mesh-budget.py` — see its header "
                "for the assumption tables (parallelism, fallback unit costs, "
                "count-model derivations).\n\n"
            )
        f.write(block)
        f.write("\n\n")


def write_state(total_s, per_atom, rows_n, peers_n, reachable, unreachable):
    os.makedirs(os.path.dirname(STATE_PATH), exist_ok=True)
    with open(STATE_PATH, "w") as f:
        json.dump(
            {
                "predicted_total_secs": total_s,
                "per_atom": per_atom,
                "rows": rows_n,
                "peers_n": peers_n,
                "reachable": reachable,
                "unreachable": unreachable,
                "timestamp": datetime.datetime.now(datetime.timezone.utc)
                .isoformat()
                .replace("+00:00", "Z"),
            },
            f,
            indent=2,
        )


def read_state():
    try:
        with open(STATE_PATH) as f:
            return json.load(f)
    except (OSError, json.JSONDecodeError):
        return None


# ── Subcommands ───────────────────────────────────────────────────────────


def cmd_predict(args):
    if args.from_file:
        collected = collect_from_files(args.from_file)
        source_label = f"--from-file ({', '.join(args.from_file)})"
    else:
        collected = collect_from_live_peers()
        source_label = "live scrape (localhost:8090-8092)"

    reachable = collected["reachable"]
    unreachable = collected["unreachable"]
    peers_n = len(PEERS) if not args.from_file else max(1, len(reachable))

    ts = datetime.datetime.now(datetime.timezone.utc).isoformat().replace(
        "+00:00", "Z"
    )
    table_md, total_s, per_atom = build_table(
        args.rows, peers_n, collected, source_label
    )

    header = (
        f"## PREDICT @ {ts}\n\n"
        f"- source: {source_label}\n"
        f"- rows: {args.rows}\n"
        f"- peers: {peers_n} (reachable: {', '.join(reachable) or 'none'}"
        f"{'; UNREACHABLE: ' + ', '.join(unreachable) if unreachable else ''})\n"
        f"- pending_total (live): {collected['pending_total']}\n"
        f"- drain_backlog_total (live): {collected['drain_backlog_total']}\n\n"
    )
    footer = f"\n**Predicted quiesce time: {total_s:.2f}s ({total_s/60:.2f}min)**\n"

    block = header + table_md + footer
    print(block)
    append_ledger(block)
    write_state(total_s, per_atom, args.rows, peers_n, reachable, unreachable)
    print(f"\n(appended to {LEDGER_PATH}; state -> {STATE_PATH})", file=sys.stderr)
    return 0


def cmd_settle(args):
    state = read_state()
    ts = datetime.datetime.now(datetime.timezone.utc).isoformat().replace(
        "+00:00", "Z"
    )
    measured = args.measured_secs

    if state is None:
        block = (
            f"## SETTLE @ {ts}\n\n"
            f"- measured: {measured:.2f}s\n"
            f"- **no prior PREDICT found in state ({STATE_PATH})** — residual "
            f"cannot be computed. Run `predict` before the next measured run.\n"
        )
        print(block)
        append_ledger(block)
        print(f"\n(appended to {LEDGER_PATH})", file=sys.stderr)
        return 1

    predicted = state["predicted_total_secs"]
    residual_abs = predicted - measured
    residual_pct = (residual_abs / measured * 100.0) if measured else float("nan")

    # Per-atom breakdown sorted by absolute contribution, largest first — the
    # atoms most worth reviewing first if the residual is large.
    per_atom = state.get("per_atom", {})
    ranked = sorted(
        per_atom.items(), key=lambda kv: kv[1]["contribution_s"], reverse=True
    )
    ranked_lines = "\n".join(
        f"  - {atom}: predicted {v['contribution_s']:.2f}s "
        f"(count={v['count']:.1f} [{v['count_source']}], "
        f"cost={v['unit_ms']:.2f}ms [{v['cost_source']}], "
        f"parallelism={v['parallelism']})"
        for atom, v in ranked
    )

    verdict = "OVER-predicted" if residual_abs > 0 else "UNDER-predicted"
    block = (
        f"## SETTLE @ {ts}\n\n"
        f"- predicted (from PREDICT @ {state.get('timestamp')}): "
        f"{predicted:.2f}s\n"
        f"- measured: {measured:.2f}s\n"
        f"- residual: {residual_abs:+.2f}s ({residual_pct:+.1f}%) — {verdict}\n"
        f"- **residual = named design concern**: state which atom's count "
        f"model or parallelism constant is the leading suspect (see the "
        f"per-atom breakdown below, ranked by predicted contribution) and "
        f"file it, don't just log the number.\n\n"
        f"per-atom breakdown (largest predicted contribution first):\n"
        f"{ranked_lines}\n"
    )
    print(block)
    append_ledger(block)
    print(f"\n(appended to {LEDGER_PATH})", file=sys.stderr)
    return 0


def cmd_selftest(_args):
    """Generate a small synthetic /metrics fixture, run predict against it
    via --from-file, and assert the table computes end-to-end without
    touching the ledger. Exists so this script is verifiable with the mesh
    down (Q11 deliverable requirement)."""
    import tempfile

    fixture = "\n".join(
        [
            "# HELP elohim_atom_duration_ms Wall-clock cost of one convergence-path atom.",
            "# TYPE elohim_atom_duration_ms histogram",
            'elohim_atom_duration_ms_sum{atom="put_record"} 420.0',
            'elohim_atom_duration_ms_count{atom="put_record"} 10',
            'elohim_atom_duration_ms_sum{atom="inventory_page"} 900.0',
            'elohim_atom_duration_ms_count{atom="inventory_page"} 6',
            'elohim_atom_duration_ms_sum{atom="head_batch_per_id"} 150.0',
            'elohim_atom_duration_ms_count{atom="head_batch_per_id"} 30',
            'elohim_atom_duration_ms_sum{atom="head_record_verify"} 640.0',
            'elohim_atom_duration_ms_count{atom="head_record_verify"} 8',
            'elohim_atom_duration_ms_sum{atom="adopt_declare"} 200.0',
            'elohim_atom_duration_ms_count{atom="adopt_declare"} 10',
            # shard_fetch / manifest_persist / digest_fold deliberately absent
            # -> exercises the fallback-cost path.
        ]
    )
    tmp_dir = os.environ.get(
        "MESH_BUDGET_SELFTEST_DIR",
        "/tmp/claude-0/-projects-elohim/5a20ed95-5975-4026-a560-7bef7ffcb4f1/scratchpad",
    )
    os.makedirs(tmp_dir, exist_ok=True)
    fixture_path = os.path.join(tmp_dir, "hc-mesh-budget-selftest-metrics.txt")
    with open(fixture_path, "w") as f:
        f.write(fixture)

    collected = collect_from_files([fixture_path])
    assert collected["reachable"] == ["hc-mesh-budget-selftest-metrics"]
    table_md, total_s, per_atom = build_table(100, 3, collected, "selftest")
    assert total_s > 0, "predicted total must be positive"
    for atom in ATOMS:
        assert atom in per_atom, f"missing atom in computed table: {atom}"
    # observed atoms should use the scraped mean, not the fallback constant
    put_record_cost = per_atom["put_record"]["unit_ms"]
    assert abs(put_record_cost - 42.0) < 1e-6, (
        f"put_record unit cost should be observed mean 420/10=42.0, got {put_record_cost}"
    )
    assert per_atom["put_record"]["cost_source"] == "observed"
    # unobserved atoms fall back to the documented constant
    assert per_atom["digest_fold"]["cost_source"] == "fallback"
    assert per_atom["digest_fold"]["unit_ms"] == FALLBACK_UNIT_COST_MS["digest_fold"]

    print(f"self-test fixture: {fixture_path}")
    print(table_md)
    print(f"\nPredicted quiesce time: {total_s:.2f}s")
    print("\nSELF-TEST OK: table computed, observed vs fallback cost sourcing both verified.")
    return 0


def main():
    p = argparse.ArgumentParser(
        description="Q11 atomic-timing-budget probe for the hc-mesh quiesce measure."
    )
    sub = p.add_subparsers(dest="cmd", required=True)

    pred = sub.add_parser("predict", help="record a falsifiable prediction before a measured run")
    pred.add_argument(
        "--rows",
        type=int,
        default=3439,
        help="fixture row count for the rows-model fallback (default: 3439, the plan's baseline corpus)",
    )
    pred.add_argument(
        "--from-file",
        nargs="+",
        default=None,
        metavar="PATH",
        help="read one or more saved /metrics text dumps instead of live-scraping the mesh",
    )
    pred.set_defaults(func=cmd_predict)

    settle = sub.add_parser("settle", help="record the measured wall-clock + residual after a run")
    settle.add_argument(
        "--measured-secs",
        type=float,
        required=True,
        help="measured wall-clock quiesce time in seconds (e.g. from hc-mesh-quiesce.sh's wall=Ns line)",
    )
    settle.set_defaults(func=cmd_settle)

    st = sub.add_parser("selftest", help="synthetic-fixture self-test (no ledger writes, no network)")
    st.set_defaults(func=cmd_selftest)

    args = p.parse_args()
    sys.exit(args.func(args))


if __name__ == "__main__":
    main()
