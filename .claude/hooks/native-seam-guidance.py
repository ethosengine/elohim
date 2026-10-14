#!/usr/bin/env python3
"""Advisory native seam guidance; normalization and classification belong to epr."""
import json
import os
from pathlib import Path
import subprocess
import sys
import time


def main():
    started = time.monotonic()
    try:
        raw = sys.stdin.buffer.read(1572865)
        if len(raw) > 1572864:
            raise ValueError("event exceeds size limit")
        event = json.loads(raw)
        root = Path(__file__).resolve().parents[2]
        harness = sys.argv[1] if len(sys.argv) > 1 else "unknown"
        remaining = 1.8 - (time.monotonic() - started)
        if remaining <= 0:
            raise TimeoutError("normalization deadline")
        result = subprocess.run(
            [os.environ.get("EPR_SEAM_BIN", "epr"), "govern", "seam", "--repo", str(root)],
            input=json.dumps({"harness": harness, "event": event}),
            text=True, capture_output=True, timeout=remaining, check=True,
        )
        report = json.loads(result.stdout)
        if "packet" not in report:
            raise ValueError("native evaluator does not support seam guidance")
        context = report["packet"]
        if not isinstance(context, str):
            raise ValueError("native packet is not text")
        if len(context.encode("utf-8")) > 8192:
            raise ValueError("native packet exceeds 8 KiB")
    except (OSError, ValueError, subprocess.SubprocessError, TimeoutError) as error:
        context = "Seam guidance coverage gap: native evaluator unavailable, failed or timed out (" + type(error).__name__ + "). Existing governance gates remain in effect."
    if context:
        print(json.dumps({"hookSpecificOutput": {"hookEventName": "PreToolUse", "additionalContext": context}}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
