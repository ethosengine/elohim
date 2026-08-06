#!/usr/bin/env python3
"""config-provenance.py — find config keys the code READS that nothing SETS.

The inverse of `scripts/ci/dead-config-lint.sh`, and the same class of defect. That script
asks "does this declared knob have a reader?" — a knob with no reader is a claim, not a
feature. This one asks the other direction: "does this read key have a setter?" A key the
binary reads but no manifest, Dockerfile, env file, or CI script ever sets is a feature that
ships configured OFF, with nothing red anywhere.

WHY (2026-08-06): the branch enabled `ELOHIM_TRANSPORT_BACKEND=dual` fleet-wide while the
iroh transport reads `ELOHIM_IROH_RELAY_URL` to decide whether it gets a relay — and no
manifest, script, or env file in the repo sets it. The iroh leg therefore boots with
`RelayMode::Disabled` on every peer while the boot sentinel operators are told to watch
prints green. Three separate reviewer agents each hand-grepped their way to that finding.
This makes it computable.

Usage:
  config-provenance.py                    unset keys (read by code, set by nothing)
  config-provenance.py --prefix ELOHIM_   scope to a key prefix
  config-provenance.py --key K [--key K]  full provenance for specific keys
  config-provenance.py --all              every key with its verdict
  config-provenance.py --json             machine-readable

Verdicts: unset (read, never set) · defaulted (read WITH a default, never set — degrades
quietly instead of failing, which is what made the iroh case invisible) · set · orphan-set.

Read-only, no network. Exit 0 always — this reports, it does not gate. (Gating on today's
count would redden CI on 28 pre-existing elohim-storage findings; ratchet from a baseline
if you want teeth.)
"""
import argparse
import json
import sys
from pathlib import Path

_here = Path(__file__).resolve()
_ROOT = _here.parent.parent
for _ in range(8):
    if (_here / ".claude" / "scripts" / "_lib").is_dir():
        sys.path.insert(0, str(_here / ".claude" / "scripts"))
        _ROOT = _here
        break
    _here = _here.parent

from _lib import config_provenance as cp  # noqa: E402


def main() -> int:
    ap = argparse.ArgumentParser(
        add_help=True, description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    ap.add_argument("--prefix", default="", help="only keys starting with this prefix")
    ap.add_argument("--key", action="append", default=[], help="full provenance for one key (repeatable)")
    ap.add_argument("--all", action="store_true", help="every key, not just unset ones")
    ap.add_argument("--json", action="store_true")
    args = ap.parse_args()

    prov = cp.provenance(_ROOT, keys=args.key or None)
    if args.prefix:
        prov = {k: v for k, v in prov.items() if k.startswith(args.prefix)}

    rows = list(prov.values()) if (args.all or args.key) else cp.unset_keys(prov)
    # The scanner is textual, so it reads the env-var literals inside its OWN test fixtures.
    # Those are not repo config; excluding them here keeps the module general and the report honest.
    _SELF = "_lib/__tests__/config_provenance_test.py"
    rows = [r for r in rows if not (r.reads and all(_SELF in x.path for x in r.reads))]
    rows = sorted(rows, key=lambda r: (r.verdict, r.key))

    if args.json:
        print(json.dumps([{
            "key": r.key, "verdict": r.verdict,
            "reads": [f"{x.path}:{x.line}" for x in r.reads],
            "writes": [f"{x.path}:{x.line}" for x in r.writes],
        } for r in rows], indent=2))
        return 0

    if not rows:
        print("config-provenance: no keys matched")
        return 0

    for r in rows:
        where = r.reads[0] if r.reads else None
        loc = f"{where.path}:{where.line}" if where else "—"
        print(f"  {r.verdict:<11} {r.key:<38} read at {loc}"
              + (f"  (+{len(r.reads) - 1} more)" if len(r.reads) > 1 else ""))
    counts: dict[str, int] = {}
    for r in rows:
        counts[r.verdict] = counts.get(r.verdict, 0) + 1
    print("\n  " + " · ".join(f"{v} {k}" for k, v in sorted(counts.items())))
    return 0


if __name__ == "__main__":
    sys.exit(main())
