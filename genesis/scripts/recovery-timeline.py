#!/usr/bin/env python3
"""Read the local mesh recovery JSONL as a series, summary, or comparison.

This is a reader, not a register. ``hc-mesh-recovery.sh`` owns the raw record
shape and appends it to ``$MESH_DIR/recovery-timeline.jsonl``; this tool turns
those observations into tables without minting a second source of truth.

Examples:

    recovery-timeline.py --series
    recovery-timeline.py --table /tmp/elohim-local-mesh/recovery-timeline.jsonl
    recovery-timeline.py --before baseline.jsonl --after selection-on.jsonl

Timing statistics include recovered runs only. Missing measurements stay
absent: in particular, a null conductor receipt is never rendered as 0.0.
"""
from __future__ import annotations

import argparse
import json
import os
import statistics
import sys
from collections import defaultdict
from pathlib import Path
from typing import Any


DEFAULT_SERIES = (
    Path(os.environ.get("MESH_DIR", "/tmp/elohim-local-mesh"))
    / "recovery-timeline.jsonl"
)
UNLABELED = "<unlabeled>"
UNKNOWN = "<unknown>"


def numeric(value: Any) -> float | int | None:
    """Return a real JSON number, excluding bool and numeric-looking strings."""
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        return None
    return value


def label(record: dict[str, Any], name: str, fallback: str) -> str:
    labels = record.get("labels")
    value = labels.get(name) if isinstance(labels, dict) else None
    return str(value) if value not in (None, "") else fallback


def scenario_of(record: dict[str, Any]) -> str:
    return label(record, "scenario", UNLABELED)


def shape_of(record: dict[str, Any]) -> str:
    value = record.get("shape")
    if value not in (None, ""):
        return str(value)
    return label(record, "shape", UNKNOWN)


def failing_legs(record: dict[str, Any]) -> set[str]:
    legs = record.get("failing_legs")
    if not isinstance(legs, list):
        return set()
    return {str(leg) for leg in legs if leg not in (None, "")}


def survivor_receipt(record: dict[str, Any]) -> float | int | None:
    receipts = record.get("conductor_receipt_max_s")
    if not isinstance(receipts, dict):
        return None
    return numeric(receipts.get("survivor"))


def load_records(path: Path) -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    with path.open(encoding="utf-8") as stream:
        for line_number, line in enumerate(stream, 1):
            if not line.strip():
                continue
            try:
                record = json.loads(line)
            except json.JSONDecodeError as exc:
                raise ValueError(f"{path}:{line_number}: invalid JSON: {exc.msg}") from exc
            if not isinstance(record, dict):
                raise ValueError(f"{path}:{line_number}: recovery record must be an object")
            records.append(record)
    return records


def recovered_times(records: list[dict[str, Any]]) -> list[float | int]:
    return [
        value
        for record in records
        if record.get("recovered") is True
        and (value := numeric(record.get("time_to_recover_s"))) is not None
    ]


def summarize(records: list[dict[str, Any]]) -> dict[str, Any]:
    times = recovered_times(records)
    receipts = [
        value
        for record in records
        if (value := survivor_receipt(record)) is not None
    ]
    legs = set().union(*(failing_legs(record) for record in records)) if records else set()
    zome_paths = {
        str(record["zome_path"])
        for record in records
        if record.get("zome_path") not in (None, "")
    }
    recovered = sum(record.get("recovered") is True for record in records)
    return {
        "runs": len(records),
        "recovered": recovered,
        "recovery_rate": recovered / len(records) if records else None,
        "median": statistics.median(times) if times else None,
        "min": min(times) if times else None,
        "max": max(times) if times else None,
        "spread": max(times) - min(times) if times else None,
        "receipt_max": max(receipts) if receipts else None,
        "failing_legs": legs,
        "zome_paths": zome_paths,
    }


def grouped(records: list[dict[str, Any]]) -> dict[tuple[str, str], list[dict[str, Any]]]:
    groups: dict[tuple[str, str], list[dict[str, Any]]] = defaultdict(list)
    for record in records:
        groups[(scenario_of(record), shape_of(record))].append(record)
    return groups


def fmt_number(value: float | int | None, *, signed: bool = False) -> str:
    if value is None:
        return "—"
    prefix = "+" if signed and value > 0 else ""
    if float(value).is_integer():
        return f"{prefix}{int(value)}"
    return f"{prefix}{value:.1f}"


def fmt_set(values: set[str]) -> str:
    return ",".join(sorted(values)) or "—"


def cell(value: Any) -> str:
    return str(value).replace("|", "\\|").replace("\n", " ")


def render_series(records: list[dict[str, Any]]) -> str:
    lines = [
        "| scenario | shape | run | peer | recovered | t_recover s | survivor receipt max s | zome path | failing legs |",
        "|---|---|---:|---|---|---:|---:|---|---|",
    ]
    for record in records:
        run = label(record, "run", "—")
        recovered = "yes" if record.get("recovered") is True else "no"
        lines.append(
            "| "
            + " | ".join(
                cell(value)
                for value in (
                    scenario_of(record),
                    shape_of(record),
                    run,
                    record.get("peer", "—"),
                    recovered,
                    fmt_number(numeric(record.get("time_to_recover_s"))),
                    fmt_number(survivor_receipt(record)),
                    record.get("zome_path", "—") or "—",
                    fmt_set(failing_legs(record)),
                )
            )
            + " |"
        )
    return "\n".join(lines)


def render_table(records: list[dict[str, Any]]) -> str:
    lines = [
        "| scenario | shape | runs | recovered | t_recover median s | min | max | survivor receipt max s | zome paths | failing legs seen |",
        "|---|---|---:|---:|---:|---:|---:|---:|---|---|",
    ]
    for (scenario, shape), rows in sorted(grouped(records).items()):
        summary = summarize(rows)
        lines.append(
            "| "
            + " | ".join(
                cell(value)
                for value in (
                    scenario,
                    shape,
                    summary["runs"],
                    summary["recovered"],
                    fmt_number(summary["median"]),
                    fmt_number(summary["min"]),
                    fmt_number(summary["max"]),
                    fmt_number(summary["receipt_max"]),
                    fmt_set(summary["zome_paths"]),
                    fmt_set(summary["failing_legs"]),
                )
            )
            + " |"
        )
    return "\n".join(lines)


def delta(after: float | int | None, before: float | int | None) -> float | int | None:
    if after is None or before is None:
        return None
    return after - before


def fmt_recovery(summary: dict[str, Any]) -> str:
    if not summary["runs"]:
        return "—"
    return f'{summary["recovered"]}/{summary["runs"]}'


def render_comparison(before: list[dict[str, Any]], after: list[dict[str, Any]]) -> str:
    before_groups = grouped(before)
    after_groups = grouped(after)
    lines = [
        "| scenario | shape | recovered before | after | reliability Δ pp | median before s | after s | median Δ s | spread before s | after s | spread Δ s | failing legs removed | added |",
        "|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|---|",
    ]
    for scenario, shape in sorted(set(before_groups) | set(after_groups)):
        old = summarize(before_groups.get((scenario, shape), []))
        new = summarize(after_groups.get((scenario, shape), []))
        rate_delta = delta(new["recovery_rate"], old["recovery_rate"])
        rate_points = rate_delta * 100 if rate_delta is not None else None
        removed = old["failing_legs"] - new["failing_legs"]
        added = new["failing_legs"] - old["failing_legs"]
        lines.append(
            "| "
            + " | ".join(
                cell(value)
                for value in (
                    scenario,
                    shape,
                    fmt_recovery(old),
                    fmt_recovery(new),
                    fmt_number(rate_points, signed=True),
                    fmt_number(old["median"]),
                    fmt_number(new["median"]),
                    fmt_number(delta(new["median"], old["median"]), signed=True),
                    fmt_number(old["spread"]),
                    fmt_number(new["spread"]),
                    fmt_number(delta(new["spread"], old["spread"]), signed=True),
                    fmt_set(removed),
                    fmt_set(added),
                )
            )
            + " |"
        )
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument(
        "--series",
        nargs="?",
        const=str(DEFAULT_SERIES),
        metavar="JSONL",
        help="render every record (default: $MESH_DIR/recovery-timeline.jsonl)",
    )
    parser.add_argument(
        "--table",
        nargs="?",
        const=str(DEFAULT_SERIES),
        metavar="JSONL",
        help="group records by scenario and shape",
    )
    parser.add_argument("--before", metavar="JSONL", help="baseline series for comparison")
    parser.add_argument("--after", metavar="JSONL", help="new series for comparison")
    args = parser.parse_args(argv)

    selected = sum(value is not None for value in (args.series, args.table))
    comparing = args.before is not None or args.after is not None
    if selected + int(comparing) != 1:
        parser.error("choose one of --series, --table, or --before/--after")
    if comparing and (args.before is None or args.after is None):
        parser.error("--before and --after must be provided together")

    try:
        if args.series is not None:
            print(render_series(load_records(Path(args.series))))
        elif args.table is not None:
            print(render_table(load_records(Path(args.table))))
        else:
            print(
                render_comparison(
                    load_records(Path(args.before)), load_records(Path(args.after))
                )
            )
    except (OSError, ValueError) as exc:
        print(f"recovery-timeline: {exc}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
