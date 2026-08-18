#!/usr/bin/env python3
"""hc-admission-probe.py — the LIVE leg of the `conductor-capacity-represented`
habit, measured against a real conductor.

WHAT THIS IS FOR. The habit's claim is not that a semaphore bounds a semaphore —
a unit test proves that. It is that the conductor's DB read pool is a REPRESENTED
resource: that occupancy (L) and hold-time (W) are exported so Little's Law is
computable, and that a call which cannot get capacity is shed BEFORE dispatch.
Only a running conductor can settle that, so this script drives real zome-call
demand through elohim-storage and reads the gate's own series off GET /metrics.

WHAT IT MEASURES. `L = lambda * W` is computed from three independently emitted
quantities and checked against a fourth:
  lambda  = d(elohim_conductor_admission_acquired_total) / elapsed
  W       = d(hold_ms_sum) / d(hold_ms_count)
  L_calc  = lambda * W
  L_obs   = time-average of elohim_conductor_admission_in_flight, sampled at 5Hz
A small |residual| between L_calc and L_obs means the gate's occupancy and its
hold-time describe the same population — i.e. the pool is honestly represented.
A large one means at least one of the three series is lying.

READING THE RESULT. Two numbers decide the sizing question, and BOTH are needed:
  - `L_obs` pinned at `capacity` with `acquired_arrival_saturated` rising says
    the gate is the binding constraint. This is NECESSARY evidence for raising
    the conductor's db_max_readers — and, on its own, NOT SUFFICIENT.
  - Whether lambda RISES when you re-run with a larger ELOHIM_CONDUCTOR_PERMITS
    is the sufficient test. If lambda stays flat (or falls) while W grows in
    proportion to the extra permits, the pool was already oversubscribed and
    admitting more concurrency buys queueing, not throughput.
Measured locally 2026-08-18 (see the habit's evidence block): doubling permits
17 -> 34 left lambda flat and doubled W. The pool was oversubscribed, not
undersized.

USAGE (with `just dev start` or an equivalent conductor + storage already up):
    python3 app/elohim-app/scripts/hc-admission-probe.py --concurrency 64 --seconds 20
    ELOHIM_CONDUCTOR_PERMITS=34 <restart storage>; re-run; compare lambda.

The default target route is GET /api/v1/source-chain/{agent}/entries — one
imagodei `query_my_source_chain` per request and nothing else, so hold-time is
conductor service time rather than storage bookkeeping. The coordinator derives
the agent from `agent_info()` on the calling cell, so `--agent` is routing
context only and its default (`self`) is fine.

CAVEATS, because this measure is easy to over-read:
  - This bounds ELOHIM-STORAGE's share of the pool. The doorway is a second
    process holding its own websockets to the same conductor with no gate at
    all, so occupancy here is never node-wide.
  - One zome call is not one read permit (an extern may take several), and the
    conductor spends permits on its own gossip/validation work this gate cannot
    see. Prefer an ISLAND conductor (bootstrap/signal pointed at a dead port)
    when comparing capacities, or that invisible load becomes your variance.
  - Throughput on a shared dev box tracks machine load harder than it tracks
    capacity. Compare ADJACENT runs and prefer ratios; the printed loadavg is
    there so a comparison across a load change can be thrown out.
"""

import argparse, json, re, statistics, sys, threading, time, urllib.request, urllib.error
from collections import defaultdict

SERIES = "elohim_conductor_admission_"

def scrape(base, timeout=5.0):
    with urllib.request.urlopen(base + "/metrics", timeout=timeout) as r:
        return r.read().decode("utf-8", "replace")

def parse(text):
    """{metric: {labelkey: value}} for the admission family only."""
    out = defaultdict(dict)
    for line in text.splitlines():
        if not line.startswith(SERIES):
            continue
        m = re.match(r'^([a-z_]+)(\{[^}]*\})?\s+([0-9.eE+-]+)$', line)
        if not m:
            continue
        name, labels, val = m.group(1), m.group(2) or "", m.group(3)
        try:
            out[name][labels] = float(val)
        except ValueError:
            pass
    return out

def total(snap, name):
    return sum(snap.get(name, {}).values())

def hist_quantiles(buckets, count, qs=(0.5, 0.9, 0.99)):
    """buckets: {le_float: cumulative_count} -> approximate quantiles."""
    if count <= 0:
        return {}
    ordered = sorted(buckets.items())
    res = {}
    for q in qs:
        target = q * count
        for le, c in ordered:
            if c >= target:
                res[q] = le
                break
        else:
            res[q] = float("inf")
    return res

def bucket_map(snap, name, prefix_filter=None):
    """cumulative bucket counts summed over labels (excluding le)."""
    agg = defaultdict(float)
    for labels, v in snap.get(name + "_bucket", {}).items():
        m = re.search(r'le="([^"]+)"', labels)
        if not m:
            continue
        if prefix_filter and prefix_filter not in labels:
            continue
        le = float("inf") if m.group(1) == "+Inf" else float(m.group(1))
        agg[le] += v
    return agg

def diff_buckets(a, b):
    keys = set(a) | set(b)
    return {k: b.get(k, 0.0) - a.get(k, 0.0) for k in keys}

class Sampler(threading.Thread):
    def __init__(self, base, interval=0.2):
        super().__init__(daemon=True)
        self.base, self.interval = base, interval
        self.samples = []          # (t, in_flight)
        self.stop_evt = threading.Event()
        self.err = 0
    def run(self):
        while not self.stop_evt.is_set():
            t0 = time.time()
            try:
                snap = parse(scrape(self.base, timeout=3.0))
                self.samples.append((t0, total(snap, SERIES + "in_flight")))
            except Exception:
                self.err += 1
            time.sleep(max(0.0, self.interval - (time.time() - t0)))

class Worker(threading.Thread):
    def __init__(self, url, deadline, stats, lock):
        super().__init__(daemon=True)
        self.url, self.deadline, self.stats, self.lock = url, deadline, stats, lock
    def run(self):
        while time.time() < self.deadline:
            t0 = time.time()
            code, shed = 0, False
            try:
                with urllib.request.urlopen(self.url, timeout=30) as r:
                    r.read()
                    code = r.status
            except urllib.error.HTTPError as e:
                body = ""
                try:
                    body = e.read().decode("utf-8", "replace")
                except Exception:
                    pass
                code = e.code
                shed = "conductor admission: shed" in body
            except Exception:
                code = -1
            dt = (time.time() - t0) * 1000.0
            with self.lock:
                self.stats["lat"].append(dt)
                self.stats["codes"][code] = self.stats["codes"].get(code, 0) + 1
                if shed:
                    self.stats["shed_bodies"] += 1

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--base", default="http://localhost:8090")
    ap.add_argument("--agent", default="self",
                    help="routing context only; the coordinator derives the agent from agent_info()")
    ap.add_argument("--concurrency", type=int, default=64)
    ap.add_argument("--seconds", type=float, default=30.0)
    ap.add_argument("--label", default="run")
    ap.add_argument("--out", default=None)
    a = ap.parse_args()

    url = f"{a.base}/api/v1/source-chain/{a.agent}/entries"
    def loadavg():
        try:
            return [float(x) for x in open("/proc/loadavg").read().split()[:3]]
        except Exception:
            return []
    load_before = loadavg()
    before = parse(scrape(a.base))
    cap = total(before, SERIES + "capacity")

    sampler = Sampler(a.base); sampler.start()
    stats = {"lat": [], "codes": {}, "shed_bodies": 0}
    lock = threading.Lock()
    t_start = time.time()
    deadline = t_start + a.seconds
    ws = [Worker(url, deadline, stats, lock) for _ in range(a.concurrency)]
    for w in ws: w.start()
    for w in ws: w.join()
    elapsed = time.time() - t_start
    time.sleep(0.5)
    sampler.stop_evt.set(); sampler.join(timeout=3)
    after = parse(scrape(a.base))

    acq = {k: after.get(SERIES+"acquired_total", {}).get(k, 0) - before.get(SERIES+"acquired_total", {}).get(k, 0)
           for k in set(after.get(SERIES+"acquired_total", {})) | set(before.get(SERIES+"acquired_total", {}))}
    shed = {k: after.get(SERIES+"shed_total", {}).get(k, 0) - before.get(SERIES+"shed_total", {}).get(k, 0)
            for k in set(after.get(SERIES+"shed_total", {})) | set(before.get(SERIES+"shed_total", {}))}
    acq_n = sum(acq.values()); shed_n = sum(shed.values())
    sat_n = sum(v for k, v in acq.items() if 'arrival="saturated"' in k)

    hold_sum = total(after, SERIES+"hold_ms_sum") - total(before, SERIES+"hold_ms_sum")
    hold_cnt = total(after, SERIES+"hold_ms_count") - total(before, SERIES+"hold_ms_count")
    wait_sum = total(after, SERIES+"wait_ms_sum") - total(before, SERIES+"wait_ms_sum")
    wait_cnt = total(after, SERIES+"wait_ms_count") - total(before, SERIES+"wait_ms_count")

    W = (hold_sum / hold_cnt) if hold_cnt else 0.0            # mean hold, ms
    lam = (acq_n / elapsed) if elapsed else 0.0                # admissions / s
    L_computed = lam * (W / 1000.0)
    infl = [v for _, v in sampler.samples]
    # time-average occupancy over the load window only
    win = [v for t, v in sampler.samples if t_start <= t <= t_start + elapsed]
    L_observed = statistics.fmean(win) if win else 0.0

    hq = hist_quantiles(diff_buckets(bucket_map(before, SERIES+"hold_ms"), bucket_map(after, SERIES+"hold_ms")), hold_cnt)
    wq = hist_quantiles(diff_buckets(bucket_map(before, SERIES+"wait_ms"), bucket_map(after, SERIES+"wait_ms")), wait_cnt)

    lat = sorted(stats["lat"])
    def pct(p):
        return lat[min(len(lat)-1, int(p*len(lat)))] if lat else 0.0

    report = {
        "label": a.label, "url": url,
        "loadavg_before": load_before, "loadavg_after": loadavg(),
        "concurrency": a.concurrency, "elapsed_s": round(elapsed, 2),
        "capacity": cap,
        "requests": len(lat), "codes": stats["codes"], "shed_bodies": stats["shed_bodies"],
        "http_ms": {"p50": round(pct(.5),1), "p90": round(pct(.9),1), "p99": round(pct(.99),1),
                    "max": round(lat[-1],1) if lat else 0},
        "admission": {
            "acquired_total": acq_n,
            "acquired_arrival_saturated": sat_n,
            "shed_total": shed_n,
            "shed_by_label": {k: v for k, v in shed.items() if v},
            "acquired_by_label": {k: v for k, v in acq.items() if v},
        },
        "littles_law": {
            "lambda_per_s": round(lam, 2),
            "W_mean_hold_ms": round(W, 2),
            "L_computed_lambda_W": round(L_computed, 3),
            "L_observed_time_avg_in_flight": round(L_observed, 3),
            "L_peak_in_flight": max(infl) if infl else 0,
            "residual_pct": (round(100*(L_computed-L_observed)/L_observed, 1) if L_observed else None),
            "samples": len(win), "sampler_errors": sampler.err,
        },
        "hold_ms_quantiles": {str(k): v for k, v in hq.items()},
        "wait_ms_quantiles": {str(k): v for k, v in wq.items()},
        "occupancy_series": [round(v, 2) for _, v in sampler.samples],
    }
    txt = json.dumps(report, indent=2)
    print(txt)
    if a.out:
        open(a.out, "w").write(txt + "\n")

if __name__ == "__main__":
    main()
