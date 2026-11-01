#!/usr/bin/env python3
"""Git adapter: native session owns attribution; Git owns trailer parsing and writes."""
import json
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / ".claude/scripts"))
from _lib import epr_client  # noqa: E402


def decorate(message: str) -> None:
    binary = epr_client.resolve_binary()
    if binary is None:
        raise ValueError("native session reader unavailable")
    result = subprocess.run(
        [binary, "session", "commit-trailers", "--json"],
        capture_output=True, text=True, timeout=5, check=True,
    )
    report = json.loads(result.stdout)
    if not isinstance(report, dict):
        raise ValueError("malformed native attribution report")
    if report.get("status") == "not-applicable":
        return
    pairs = report.get("trailers")
    if report.get("status") != "measured" or not isinstance(pairs, list) or not pairs:
        raise ValueError("native session attribution is unmeasured")
    args = ["git", "interpret-trailers", "--in-place", "--if-exists", "addIfDifferent",
            "--if-missing", "add"]
    for pair in pairs:
        if (not isinstance(pair, list) or len(pair) != 2
                or not all(isinstance(value, str) for value in pair)):
            raise ValueError("malformed native trailer pair")
        args.extend(["--trailer", f"{pair[0]}: {pair[1]}"])
    subprocess.run([*args, "--", message], check=True, timeout=5)


def main() -> int:
    try:
        if len(sys.argv) < 2:
            raise ValueError("Git message path missing")
        decorate(sys.argv[1])
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        # No new permission gate: missing measurement remains explicit and never invents identity.
        detail = str(error)
        if isinstance(error, subprocess.CalledProcessError):
            detail = (error.stderr or str(error)).strip()
        print(f"[commit-attribution] UNMEASURED: {detail}. Inspect epr session show --json; "
              "restore the mounted session/model before publishing.", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
