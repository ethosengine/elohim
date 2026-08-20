#!/usr/bin/env python3
"""latency-scoreboard.py — the READER for the latency valueflow chain.

A reader, not a register. Raw numbers live in Prometheus; budgets live as
`Bound { sense: Ceiling }` on epr-rea Commitments; status lives in habits.yaml.
This script owns none of that — it joins them into one table you can drive down.
(This repo has a documented instrument-proliferation problem: registers with no
reader. If someone proposes `genesis/manifests/latency-budgets.yaml`, refuse it.)

Design: genesis/docs/superpowers/specs/2026-08-20-latency-valueflow-chain-design.md

    latency-scoreboard.py --local                    # scrape a local doorway
    latency-scoreboard.py --local --url http://h:8080/metrics
    latency-scoreboard.py --deployed --promtext -    # PromQL text on stdin
    latency-scoreboard.py --local --json out.json    # record a run
    latency-scoreboard.py --compare a.json b.json    # local vs deployed delta

WHAT IT REFUSES TO DO
---------------------
* It never prints a bare p50. Central tendency hides the failure mode we can
  actually catch early: BIMODALITY. Five stage0b runs on 2026-08-20 split
  cleanly 7-vs-9 poll ticks — a 1.0s step between identical runs — and a median
  would have reported "4.5s" and hidden it entirely.
* It never compares numbers taken over different windows, peer classes, or
  n-bases. Those rows print INCOMMENSURABLE, the vocabulary the repo already
  owns (elohim/epr/src/measure.rs).
* It never treats an ABSENT series as a zero. Absent means nobody wired it, or
  the peer's tier declined to record it; zero means measured-and-fast. Conflating
  them is the exact ambiguity this whole chain exists to remove.
"""
from __future__ import annotations

import argparse
import json
import math
import re
import sys
import urllib.request
from pathlib import Path

# ── Budgets. NOT a register: these mirror the Bound{Ceiling} on each hop's
#    Commitment. Ceiling in milliseconds. `None` = no budget declared yet, which
#    prints as "—" rather than inventing a target nobody committed to.
BUDGETS_MS = {
    "serve": 800.0,
    "proxy": 500.0,
    "proxy_blob": 800.0,
    "resolve": 200.0,
}

# ── The containment tree. The composition spike is (parent - sum(children)),
#    so the tree IS the arithmetic. A flat hop list cannot produce a residual.
CONTAINS = {
    "serve": ["resolve", "proxy", "proxy_blob"],
}

BEST_PATH = Path(__file__).resolve().parents[2] / ".claude" / "data" / "latency-best.json"

SAMPLE_RE = re.compile(
    r'^(?P<name>[a-zA-Z_:][a-zA-Z0-9_:]*)'
    r'(?:\{(?P<labels>[^}]*)\})?\s+(?P<value>[^\s]+)'
)


def parse_labels(raw: str | None) -> dict:
    if not raw:
        return {}
    out = {}
    for m in re.finditer(r'(\w+)="((?:[^"\\]|\\.)*)"', raw):
        out[m.group(1)] = m.group(2)
    return out


def parse_prom_text(text: str) -> dict:
    """Parse exposition text into {(metric, frozenset(labels)): value}."""
    out = {}
    for line in text.splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        m = SAMPLE_RE.match(line)
        if not m:
            continue
        try:
            val = float(m.group("value"))
        except ValueError:
            continue
        labels = parse_labels(m.group("labels"))
        out.setdefault(m.group("name"), []).append((labels, val))
    return out


class Histogram:
    """One label-set's bucket series, reconstructed from exposition text."""

    def __init__(self, key: dict):
        self.key = key
        self.buckets: list[tuple[float, float]] = []  # (le, cumulative_count)
        self.count = 0.0
        self.sum = 0.0

    def finalize(self):
        self.buckets.sort(key=lambda b: b[0])

    def quantile(self, q: float) -> float | None:
        """Linear-interpolated histogram quantile.

        Returns None when n == 0 — an unobserved hop has no p95, and inventing
        one (0.0) would read as 'measured and instant'.
        """
        if self.count <= 0 or not self.buckets:
            return None
        target = q * self.count
        prev_le, prev_c = 0.0, 0.0
        for le, c in self.buckets:
            if c >= target:
                if math.isinf(le):
                    return prev_le
                if c == prev_c:
                    return le
                frac = (target - prev_c) / (c - prev_c)
                return prev_le + frac * (le - prev_le)
            prev_le, prev_c = le, c
        return self.buckets[-1][0]

    def mean(self) -> float | None:
        return (self.sum / self.count) if self.count > 0 else None

    def modality(self) -> str:
        """Detect multi-modal structure — the earliest instability signal.

        Not a statistical test; a deliberately blunt one that catches the shape
        we actually saw. Per-bucket (non-cumulative) mass, then count 'peaks':
        buckets that are local maxima carrying >=15% of total mass. Two or more
        separated peaks => BIMODAL, which is a stability finding, not noise.
        """
        if self.count <= 0 or len(self.buckets) < 3:
            return "—"
        mass, prev = [], 0.0
        for le, c in self.buckets:
            mass.append((le, max(0.0, c - prev)))
            prev = c
        total = sum(m for _, m in mass) or 1.0
        peaks = []
        for i, (le, m) in enumerate(mass):
            if m / total < 0.15:
                continue
            left = mass[i - 1][1] if i > 0 else 0.0
            right = mass[i + 1][1] if i + 1 < len(mass) else 0.0
            if m >= left and m >= right:
                peaks.append(i)
        # Adjacent indices are one mode smeared across a bucket edge, not two.
        separated = [p for i, p in enumerate(peaks) if i == 0 or p - peaks[i - 1] > 1]
        if len(separated) >= 3:
            return "MULTI"
        if len(separated) == 2:
            return "BIMODAL"
        return "uni"


def collect_histograms(parsed: dict, metric: str) -> dict:
    """Group *_bucket/_count/_sum samples into Histogram objects by label-set."""
    hists: dict[tuple, Histogram] = {}

    def key_of(labels: dict) -> tuple:
        return tuple(sorted((k, v) for k, v in labels.items() if k != "le"))

    for suffix, apply in (
        ("_bucket", "bucket"),
        ("_count", "count"),
        ("_sum", "sum"),
    ):
        for labels, val in parsed.get(metric + suffix, []):
            k = key_of(labels)
            h = hists.setdefault(k, Histogram(dict(k)))
            if apply == "bucket":
                le = labels.get("le", "+Inf")
                h.buckets.append((float("inf") if le == "+Inf" else float(le), val))
            elif apply == "count":
                h.count = val
            else:
                h.sum = val
    for h in hists.values():
        h.finalize()
    return hists


def load_best() -> dict:
    try:
        return json.loads(BEST_PATH.read_text())
    except Exception:
        return {}


def save_best(best: dict):
    BEST_PATH.parent.mkdir(parents=True, exist_ok=True)
    BEST_PATH.write_text(json.dumps(best, indent=2, sort_keys=True) + "\n")


def fmt(v: float | None, width: int = 9) -> str:
    if v is None:
        return "—".rjust(width)
    if v >= 1000:
        return f"{v/1000:.2f}s".rjust(width)
    return f"{v:.1f}ms".rjust(width)


def build_rows(hists: dict, env: str) -> list[dict]:
    rows = []
    for key, h in sorted(hists.items(), key=lambda kv: str(kv[0])):
        labels = dict(key)
        hop = labels.get("hop", "?")
        rows.append(
            {
                "hop": hop,
                "route_class": labels.get("route_class", "—"),
                "outcome": labels.get("outcome", "—"),
                "env": env,
                "n": h.count,
                "p50": h.quantile(0.50),
                "p90": h.quantile(0.90),
                "p95": h.quantile(0.95),
                "p99": h.quantile(0.99),
                "mean": h.mean(),
                "modality": h.modality(),
                "budget": BUDGETS_MS.get(hop),
            }
        )
    return rows


def residuals(rows: list[dict]) -> list[str]:
    """Spike-B: E[parent] - sum(E[child]).

    MEANS ONLY. Percentiles do not sum; p95(parent) != sum(p95(children)), and
    a residual computed on percentiles is arithmetic on nonsense.
    """
    out = []
    by_hop: dict[str, float] = {}
    n_by_hop: dict[str, float] = {}
    for r in rows:
        if r["mean"] is None or r["n"] <= 0:
            continue
        by_hop[r["hop"]] = by_hop.get(r["hop"], 0.0) + r["mean"] * r["n"]
        n_by_hop[r["hop"]] = n_by_hop.get(r["hop"], 0.0) + r["n"]
    avg = {h: by_hop[h] / n_by_hop[h] for h in by_hop if n_by_hop[h] > 0}

    for parent, children in CONTAINS.items():
        if parent not in avg:
            out.append(f"  {parent:<12} residual: INCOMMENSURABLE — parent unobserved")
            continue
        present = [c for c in children if c in avg]
        missing = [c for c in children if c not in avg]
        if missing:
            # An absent child inflates the residual and would read as doorway
            # overhead that does not exist. Refuse rather than mislead.
            out.append(
                f"  {parent:<12} residual: INCOMMENSURABLE — child(ren) unobserved: "
                f"{', '.join(missing)} (raise the hop tier, or these hops took no traffic)"
            )
            continue
        child_sum = sum(avg[c] for c in present)
        resid = avg[parent] - child_sum

        # A NEGATIVE residual is arithmetically impossible when containment is
        # real — a child cannot cost more than the parent that awaits it. It
        # means the terms were drawn from DISJOINT request populations: e.g.
        # blob requests (proxy_blob, route_class=asset) summed against a parent
        # measured over EPR requests. Refuse rather than print a number that
        # looks quantitative and means nothing. This is the same failure the
        # `avg_resolution_ms` field embodied — a real measurement of the wrong
        # population — and it is why the design's admissibility rule requires
        # same window, same class, same n-basis.
        if resid < 0:
            out.append(
                f"  {parent:<12} residual: INCOMMENSURABLE — negative "
                f"({resid:.1f}ms: parent {avg[parent]:.1f}ms < children {child_sum:.1f}ms). "
                f"Children are drawn from a different request population than the parent "
                f"(compare per route_class, not globally)."
            )
            continue

        pct = (resid / avg[parent] * 100.0) if avg[parent] else 0.0
        verdict = "doorway-internal overhead dominates" if pct > 50 else "children dominate"
        out.append(
            f"  {parent:<12} residual: {resid:8.1f}ms  "
            f"({pct:5.1f}% of parent {avg[parent]:.1f}ms; children {child_sum:.1f}ms) — {verdict}"
        )
    return out


def print_table(rows: list[dict], best: dict, ratchet: bool, source_cmd: str):
    hdr = (
        f"{'hop':<12}{'class':<12}{'outcome':<9}{'env':<7}{'n':>8}"
        f"{'p50':>10}{'p90':>10}{'p95':>10}{'p99':>10}{'modality':>10}"
        f"{'budget':>10}{'best':>10}{'verdict':>12}"
    )
    print(hdr)
    print("-" * len(hdr))
    regressions = []
    for r in rows:
        bkey = f"{r['hop']}|{r['route_class']}|{r['outcome']}|{r['env']}"
        prev = best.get(bkey)
        p95 = r["p95"]
        verdict = "—"
        if r["n"] <= 0:
            verdict = "UNOBSERVED"
        elif p95 is not None:
            if r["budget"] is not None and p95 > r["budget"]:
                verdict = "OVER-BUDGET"
            elif prev is not None and p95 > prev * 1.10:
                verdict = "REGRESSED"
                regressions.append((bkey, prev, p95))
            else:
                verdict = "ok"
            if ratchet and (prev is None or p95 < prev):
                best[bkey] = p95
        print(
            f"{r['hop']:<12}{r['route_class']:<12}{r['outcome']:<9}{r['env']:<7}"
            f"{int(r['n']):>8}{fmt(r['p50'],10)}{fmt(r['p90'],10)}{fmt(r['p95'],10)}"
            f"{fmt(r['p99'],10)}{r['modality']:>10}"
            f"{fmt(r['budget'],10)}{fmt(prev,10)}{verdict:>12}"
        )

    bimodal = [r for r in rows if r["modality"] in ("BIMODAL", "MULTI")]
    if bimodal:
        print("\nSTABILITY — multi-modal hops (instability, not noise):")
        for r in bimodal:
            print(
                f"  {r['hop']}/{r['route_class']}/{r['outcome']}: {r['modality']} "
                f"— p50 {fmt(r['p50']).strip()} vs p95 {fmt(r['p95']).strip()}. "
                f"A single central-tendency number would hide this."
            )

    print("\nCOMPOSITION RESIDUAL (Spike-B, means only — percentiles do not sum):")
    for line in residuals(rows):
        print(line)

    if regressions:
        print("\nREGRESSIONS vs best_observed (>10% worse):")
        for k, prev, now in regressions:
            print(f"  {k}: best {prev:.1f}ms -> now {now:.1f}ms")

    print(f"\nre-run: {source_cmd}")
    print(
        "  (every row is re-derivable by that command; a number nobody can "
        "re-derive tomorrow is not admissible evidence)"
    )


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--local", action="store_true", help="scrape a local doorway /metrics")
    ap.add_argument("--deployed", action="store_true", help="read exposition text for a deployed peer")
    ap.add_argument("--url", default="http://localhost:8080/metrics")
    ap.add_argument("--promtext", help="read exposition text from a file, or - for stdin")
    ap.add_argument("--metric", default="doorway_hop_duration_ms")
    ap.add_argument("--json", help="write the run to a JSON file")
    ap.add_argument("--compare", nargs=2, metavar=("A.json", "B.json"),
                    help="print the local<->deployed delta between two recorded runs")
    ap.add_argument("--no-ratchet", action="store_true",
                    help="do not update best_observed (read-only run)")
    args = ap.parse_args()

    if args.compare:
        a = json.loads(Path(args.compare[0]).read_text())
        b = json.loads(Path(args.compare[1]).read_text())
        print(f"{'hop':<12}{'class':<12}{'outcome':<9}{'A p95':>11}{'B p95':>11}{'delta':>11}{'ratio':>9}")
        print("-" * 75)
        bi = {(r["hop"], r["route_class"], r["outcome"]): r for r in b["rows"]}
        for r in a["rows"]:
            k = (r["hop"], r["route_class"], r["outcome"])
            o = bi.get(k)
            if not o or r["p95"] is None or o["p95"] is None:
                print(f"{k[0]:<12}{k[1]:<12}{k[2]:<9}{'—':>11}{'—':>11}{'INCOMMENSURABLE':>22}")
                continue
            d = o["p95"] - r["p95"]
            ratio = (o["p95"] / r["p95"]) if r["p95"] else float("nan")
            print(f"{k[0]:<12}{k[1]:<12}{k[2]:<9}{fmt(r['p95'],11)}{fmt(o['p95'],11)}"
                  f"{fmt(d,11)}{ratio:>8.1f}x")
        print(f"\nA={a.get('env')} ({a.get('source')})   B={b.get('env')} ({b.get('source')})")
        print("  A large ratio is the SCALE EFFECT — the cheapest early warning we get,")
        print("  and for six hops (gossip fan-out, write-guard contention, WAN/NAT,")
        print("  restart churn, peer-class diversity, breaker cascade) it is the ONLY one.")
        return 0

    if args.promtext:
        text = sys.stdin.read() if args.promtext == "-" else Path(args.promtext).read_text()
        env, source = ("alpha" if args.deployed else "local"), f"--promtext {args.promtext}"
    else:
        try:
            with urllib.request.urlopen(args.url, timeout=15) as r:
                text = r.read().decode("utf-8", "replace")
        except Exception as e:
            print(f"scrape failed: {e}", file=sys.stderr)
            print("  (a failed scrape is an INSTRUMENT OUTAGE, not a zero — no row is printed)",
                  file=sys.stderr)
            return 2
        env, source = ("alpha" if args.deployed else "local"), f"--url {args.url}"

    parsed = parse_prom_text(text)
    hists = collect_histograms(parsed, args.metric)
    if not hists:
        print(f"metric {args.metric!r} ABSENT from this exposition body.", file=sys.stderr)
        print("  ABSENT is not zero: either this build predates the hop instrument, or", file=sys.stderr)
        print("  this peer's derived tier declined to record it. Check the boot log line", file=sys.stderr)
        print("  'chain-R hop metrics tier derived from this peer's own capacity'.", file=sys.stderr)
        return 3

    rows = build_rows(hists, env)
    best = load_best()
    cmd = f"latency-scoreboard.py {'--deployed' if args.deployed else '--local'} {source}"
    print_table(rows, best, ratchet=not args.no_ratchet, source_cmd=cmd)
    if not args.no_ratchet:
        save_best(best)
    if args.json:
        Path(args.json).write_text(
            json.dumps({"env": env, "source": source, "metric": args.metric, "rows": rows},
                       indent=2) + "\n"
        )
        print(f"recorded: {args.json}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
