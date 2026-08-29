#!/usr/bin/env python3
"""Tests for the RAM guard — daemon (genesis/agentic/bin/ram-guard) and hook (.claude/hooks/ram-guard.py).

Evidence basis (2026-08-29 17:12Z OOM): the pod cgroup carried memory.oom.group=1, so one OOM
event killed 35 processes + PID 1; the trigger was a rustc/rust-lld tree on top of three
conductors. These tests pin the sensor math, the classification (critical never shed,
unknown never shed), the shed order (tier, then newest tree first) and the hook decisions.
"""
import importlib.machinery
import importlib.util
import json
import os
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
DAEMON = ROOT / "genesis" / "agentic" / "bin" / "ram-guard"
HOOK = ROOT / ".claude" / "hooks" / "ram-guard.py"


def load(path, name):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_file_location(name, path, loader=loader)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


rg = load(DAEMON, "ram_guard_daemon")
hook = load(HOOK, "ram_guard_hook")

FAILS = []


def check(name, cond, detail=""):
    print(("ok   " if cond else "FAIL ") + name + (f"  — {detail}" if detail and not cond else ""))
    if not cond:
        FAILS.append(name)


# ---------------------------------------------------------------- policy
p = rg.load_policy("/nonexistent/pool-policy.json")
check("policy defaults soft<high<hard<memory_high", p["soft_pct"] < p["high_pct"] < p["hard_pct"] < p["memory_high_pct"] < 100)
with tempfile.TemporaryDirectory() as d:
    pf = Path(d) / "pool-policy.json"
    pf.write_text(json.dumps({"volume_hard_pct": 92, "ram": {"high_pct": 77}}))
    p2 = rg.load_policy(str(pf))
    check("policy ram block overrides one key, keeps defaults", p2["high_pct"] == 77 and p2["soft_pct"] == p["soft_pct"])

# ---------------------------------------------------------------- levels
check("level ok below soft", rg.level_for(10, p) == "ok")
check("level soft", rg.level_for(p["soft_pct"], p) == "soft")
check("level high", rg.level_for(p["high_pct"], p) == "high")
check("level hard", rg.level_for(p["hard_pct"] + 5, p) == "hard")

# ---------------------------------------------------------------- sensor
with tempfile.TemporaryDirectory() as d:
    cg = Path(d)
    (cg / "memory.max").write_text("33285996544\n")
    (cg / "memory.current").write_text("19232079872\n")
    (cg / "memory.high").write_text("max\n")
    (cg / "memory.oom.group").write_text("1\n")
    (cg / "memory.stat").write_text("anon 9517334528\nfile 11155361792\nkernel 1111687168\nshmem 0\nunevictable 43892736\n")
    (cg / "memory.pressure").write_text("some avg10=1.50 avg60=0.00 avg300=0.00 total=1\nfull avg10=0.00 avg60=0.00 avg300=0.00 total=1\n")
    r = rg.CgroupSensor(str(cg)).read()
    committed = 9517334528 + 1111687168 + 0 + 43892736
    check("sensor committed = anon+kernel+shmem+unevictable", r.committed == committed)
    check("sensor pct is committed/max (not current/max)", r.pct == round(committed * 100 / 33285996544, 1))
    check("sensor psi some10", r.psi_some10 == 1.5)
    check("sensor oom_group read", r.oom_group == 1)
    # steering (dry) computes the memory.high target from policy
    raw = int(33285996544 * p["memory_high_pct"] / 100)
    tgt = rg.memory_high_target(r.max_bytes, p)
    check("memory.high target = memory_high_pct of max, page-aligned", raw - 4096 < tgt <= raw and tgt % 4096 == 0)
    # memory.max = max → sensor falls back to devfile-declared bytes, never crashes
    (cg / "memory.max").write_text("max\n")
    r2 = rg.CgroupSensor(str(cg), fallback_max=30 * 2**30).read()
    check("sensor memory.max=max falls back", r2.max_bytes == 30 * 2**30)

# ---------------------------------------------------------------- classification
P = rg.Proc
cases = [
    (P(1, 0, "entrypoint-volu", "/bin/sh /checode/entrypoint-volume.sh", 3, 1000), "critical"),
    (P(838, 54, "MainThread", "/checode/checode-linux-libc/ubi9/node out/server-main.js --host 127.0.0.1", 140, 1000), "critical"),
    (P(3878, 3816, "claude", "claude", 500, 900), "critical"),
    (P(25012, 24975, "claude.exe", "/usr/local/lib/node_modules/@anthropic-ai/claude-code/bin/claude.exe --session-id x --fork", 400, 900), "critical"),
    (P(41074, 41054, "holochain", "holochain --piped --structured=Log --config-path /x/conductor-config.yaml", 1300, 30), "critical"),
    (P(41054, 41047, "hc", "hc sandbox --piped -f 4444 run", 8, 30), "critical"),
    (P(41509, 38159, "elohim-storage", "/pool/debug/elohim-storage --http-port 8091", 90, 30), "critical"),
    (P(38315, 38159, "doorway", "/pool/debug/doorway --dev-mode", 58, 100), "critical"),
    (P(38238, 1, "mongod", "/usr/local/bin/mongod --dbpath /tmp/x", 140, 100), "critical"),
    (P(38159, 38157, "hc-mesh.sh", "/bin/bash /projects/elohim/app/elohim-app/scripts/hc-mesh.sh start", 4, 100), "critical"),
    (P(4386, 3700, "bash", "/usr/bin/bash --init-file x", 4, 400), "critical"),
    (P(33117, 25012, "mempalace-mcp", "/usr/bin/python3 /usr/local/bin/mempalace-mcp --palace x", 75, 100), "critical"),
    (P(900, 4386, "python3", "python3 /projects/elohim/genesis/agentic/bin/ram-guard run", 10, 10), "critical"),
    (P(901, 4386, "ram-guard", "/projects/elohim/genesis/agentic/bin/ram-guard run", 10, 10), "critical"),
    (P(500, 4386, "cargo", "cargo clippy --all-targets", 50, 20), "tier1"),
    (P(501, 500, "rustc", "/opt/rust/bin/rustc --crate-name elohim_storage", 4300, 15), "tier1"),
    (P(502, 501, "rust-lld", "rust-lld -flavor gnu", 1500, 5), "tier1"),
    (P(503, 501, "cc", "cc -m64 x.o", 100, 5), "tier1"),
    (P(504, 4386, "cargo", "cargo metadata --format-version 1", 50, 2), "unknown"),
    (P(600, 4386, "node", "node /x/node_modules/.bin/ng build --configuration production", 900, 60), "tier2"),
    (P(601, 4386, "node", "node /x/node_modules/vitest/vitest.mjs run --config vite.config.ts", 900, 60), "tier2"),
    (P(602, 4386, "node", "node /x/tsc -p tsconfig.json", 900, 60), "tier2"),
    (P(603, 600, "esbuild", "/x/@esbuild/linux-x64/bin/esbuild --service=0.28.1 --ping", 50, 60), "tier2"),
    (P(604, 4386, "node", "node /x/cucumber-js -p local", 300, 60), "tier2"),
    (P(38844, 38829, "ng serve --port", "ng serve --port 8081 --serve-path /threshold --host 127.0.0.1", 670, 85), "tier3"),
    (P(3943, 3878, "java", "java -jar /opt/mcp/sonarqube-mcp.jar", 400, 400), "tier3"),
    (P(700, 838, "java", "java -jar SonarLint Local x", 300, 400), "tier3"),
    (P(701, 4386, "chrome", "/x/chrome-headless-shell --headless", 300, 40), "tier3"),
    (P(702, 4386, "node", "node /x/storybook dev -p 6006", 600, 40), "tier3"),
    (P(800, 4386, "some-unknown-thing", "some-unknown-thing --flag", 5000, 1), "unknown"),
]
for proc, want in cases:
    got = rg.classify(proc)
    check(f"classify {proc.comm} ({proc.args[:40]}) -> {want}", got == want, f"got {got}")

# ---------------------------------------------------------------- tree + plan
procs = [
    P(1, 0, "entrypoint-volu", "/bin/sh /checode/entrypoint-volume.sh", 3, 1000),
    P(4386, 1, "bash", "/usr/bin/bash --init-file x", 4, 400),
    P(4400, 4386, "bash", "/bin/bash -c CARGO_TARGET_DIR=x cargo clippy", 3, 30),
    P(500, 4400, "cargo", "cargo clippy --all-targets", 50, 30),          # old build
    P(501, 500, "rustc", "rustc --crate-name a", 4300, 25),
    P(502, 501, "rust-lld", "rust-lld", 1500, 5),
    P(4401, 4386, "bash", "/bin/bash -c cargo test", 3, 12),
    P(510, 4401, "cargo", "cargo test -p x", 50, 12),                       # newer build
    P(511, 510, "rustc", "rustc --crate-name b", 2000, 10),
    P(600, 4386, "node", "node /x/ng build", 900, 60),
    P(603, 600, "esbuild", "/x/esbuild --service", 50, 60),
    P(3943, 4386, "java", "java -jar /opt/mcp/sonarqube-mcp.jar", 400, 400),
    P(41074, 4386, "holochain", "holochain --piped", 1300, 30),
]
tbl = rg.ProcTable(procs)
check("tree_root of rust-lld is the cargo driver", tbl.tree_root(502) == 500)
check("tree_root of cargo is itself (parent is a shell)", tbl.tree_root(500) == 500)
check("tree_root of esbuild is the ng build node", tbl.tree_root(603) == 600)
check("descendants of cargo 500", sorted(tbl.descendants(500)) == [501, 502])

plan = rg.shed_plan(tbl, tiers=["tier1"], self_pid=999999)
check("tier1 plan: two cargo trees, newest first", [t.root for t in plan] == [510, 500], str([t.root for t in plan]))
check("tier1 plan tree pids include descendants, root first", plan[1].pids[0] == 500 and set(plan[1].pids) == {500, 501, 502})
plan_all = rg.shed_plan(tbl, tiers=["tier1", "tier2", "tier3"], self_pid=999999)
check("tier ladder order tier1 → tier2 → tier3", [t.root for t in plan_all] == [510, 500, 600, 3943], str([t.root for t in plan_all]))
check("critical never appears in any plan", all(t.root != 41074 for t in plan_all))
check("plan carries tree rss (MB) for the ledger", plan[1].rss_kb == 4300 + 1500 + 50)

# a tree rooted at a critical/unknown ancestor is never widened past its tier root
procs2 = procs + [P(41090, 41074, "rustc", "rustc (spawned by a conductor?)", 10, 1)]
tbl2 = rg.ProcTable(procs2)
check("tree_root stops below a critical ancestor", tbl2.tree_root(41090) == 41090)

# ---------------------------------------------------------------- tiers for a level
check("tiers at ok/soft are empty", rg.tiers_for_level("ok") == [] and rg.tiers_for_level("soft") == [])
check("tiers at high = tier1 only", rg.tiers_for_level("high") == ["tier1"])
check("tiers at hard = full ladder", rg.tiers_for_level("hard") == ["tier1", "tier2", "tier3"])

# ---------------------------------------------------------------- oom_score_adj steering
check("adj for tier1/2/3 = 1000", all(rg.oom_adj_for(t) == 1000 for t in ("tier1", "tier2", "tier3")))
check("adj for critical = 0 (below k8s burstable 869)", rg.oom_adj_for("critical") == 0)
check("adj for unknown = None (left alone)", rg.oom_adj_for("unknown") is None)

# ---------------------------------------------------------------- dry-run shed writes ledger, kills nothing
with tempfile.TemporaryDirectory() as d:
    st = rg.StateStore(d)
    killed = []
    reading = rg.Reading(current=1, max_bytes=100, anon=90, kernel=0, shmem=0, unevictable=0, committed=90, pct=90.0, psi_some10=0.0, oom_group=0, high="max")
    n = rg.shed(tbl, reading, p, st, tiers=["tier1"], self_pid=999999, killer=lambda pids, sig: killed.append((tuple(pids), sig)), remeasure=lambda: reading, max_kills=1)
    check("shed kills the newest tree first via the injected killer", killed and killed[0][0][0] == 510, str(killed))
    check("shed respects max_kills", n == 1)
    ev = [json.loads(l) for l in (Path(d) / "events.jsonl").read_text().splitlines()]
    check("shed appends one ledger event with tier/root/comm/level", len(ev) == 1 and ev[0]["root"] == 510 and ev[0]["tier"] == "tier1" and ev[0]["level"] == "hard" and "cargo test" in ev[0]["cmd"])
    # remeasure returning a recovered reading stops the ladder early
    killed.clear()
    seq = iter([rg.Reading(1, 100, 10, 0, 0, 0, 10, 10.0, 0.0, 0, "max")])
    n = rg.shed(tbl, reading, p, st, tiers=["tier1", "tier2"], self_pid=999999, killer=lambda pids, sig: killed.append(pids), remeasure=lambda: next(seq), max_kills=10)
    check("shed stops once remeasure drops below high", n == 1, str(n))
    st.write_state(reading, "hard", p)
    s = json.loads((Path(d) / "state.json").read_text())
    check("state.json carries level/pct/committed/max/ts", s["level"] == "hard" and s["pct"] == 90.0 and s["max_bytes"] == 100 and "ts" in s)

# ---------------------------------------------------------------- hook: heavy detection
heavy = [
    "CARGO_TARGET_DIR=/x cargo clippy --all-targets",
    "cd elohim/elohim-storage && cargo test -p elohim-storage",
    "just gate elohim-storage",
    "just test app",
    "cd app/elohim-app && pnpm exec vitest run --config vite.config.ts",
    "pnpm --filter elohim-app build",
    "npx ng build --configuration production",
    "pnpm test",
    "cd sophia && pnpm build",
    "node genesis/a2o/node_modules/.bin/cucumber-js -p local",
]
light = [
    "git commit -m 'cargo build fixed'",
    "cargo-pool status",
    "cargo metadata --format-version 1",
    "ls -la && echo vitest",
    "pnpm install",
    "just status ram",
    "pnpm look https://x",
]
for c in heavy:
    check(f"heavy: {c}", hook.is_heavy(c))
for c in light:
    check(f"light: {c}", not hook.is_heavy(c))

# ---------------------------------------------------------------- hook: decisions from state
def state(level, pct=50.0, ts_age=0):
    import time
    return {"level": level, "pct": pct, "committed_bytes": 1, "max_bytes": 100, "ts": time.time() - ts_age, "pid": os.getpid()}

d = hook.decide_pretool("cargo build", state("high", 82.0), p)
check("pretool deny at high for heavy", d and d["permissionDecision"] == "deny" and "RAM" in d["permissionDecisionReason"])
d = hook.decide_pretool("cargo build", state("hard", 90.0), p)
check("pretool deny at hard", d and d["permissionDecision"] == "deny")
d = hook.decide_pretool("cargo build", state("soft", 72.0), p)
check("pretool advise (not deny) at soft", d and "permissionDecision" not in d and "additionalContext" in d)
d = hook.decide_pretool("cargo build", state("ok", 40.0), p)
check("pretool silent at ok", d is None)
d = hook.decide_pretool("git status", state("hard", 90.0), p)
check("pretool never gates a light command", d is None)
d = hook.decide_pretool("cargo build", state("hard", 90.0, ts_age=600), p)
check("stale state (daemon dead) never denies — fail-open with a note", d is None or d.get("permissionDecision") != "deny")
d = hook.decide_pretool("cargo build", None, p)
check("no state at all never denies", d is None or d.get("permissionDecision") != "deny")

# ---------------------------------------------------------------- hook: prompt banner + per-session cursor
with tempfile.TemporaryDirectory() as d:
    store_dir = Path(d)
    (store_dir / "events.jsonl").write_text(
        json.dumps({"ts": 1000, "level": "high", "pct": 81.2, "tier": "tier1", "root": 510, "comm": "cargo", "cmd": "cargo test -p x", "rss_kb": 2050, "pids": [510, 511]}) + "\n"
    )
    b1 = hook.prompt_banner(state("ok", 40.0), str(store_dir), "sess-A")
    check("banner surfaces an unseen shed event even at level ok", b1 and "cargo test -p x" in b1 and "RAM-GUARD" in b1, str(b1))
    b2 = hook.prompt_banner(state("ok", 40.0), str(store_dir), "sess-A")
    check("banner is silent once that session has seen the event and level is ok", b2 is None, str(b2))
    b3 = hook.prompt_banner(state("ok", 40.0), str(store_dir), "sess-B")
    check("another session still sees it (cursor is per-session)", b3 and "cargo test" in b3)
    b4 = hook.prompt_banner(state("soft", 72.0), str(store_dir), "sess-A")
    check("banner at soft prints the pressure line", b4 and "72" in b4)
    b5 = hook.prompt_banner(state("ok", 40.0, ts_age=600), str(store_dir), "sess-A")
    check("stale state → banner says guard is not running", b5 and "not running" in b5.lower())

print()
print(f"{len(FAILS)} failing" if FAILS else "all passing")
sys.exit(1 if FAILS else 0)
