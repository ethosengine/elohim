"""Habit declaration atoms inside the `.epr-meta` governance package.

A habit is what this system RELIABLY DOES, bound to a runnable check that proves it. Until
2026-08-27 the whole register lived in one hand-written file (`genesis/manifests/habits.yaml`,
171KB of habit rows) read through one hardcoded path — the last global singleton in a layer whose
every other scope authority is `.epr-meta`. This module makes a habit an atom of that same
authority:

    <dir>/.epr-meta/<id>.habit.md      frontmatter = the covenant declaration
                                       body        = the evidence ledger, newest-first

`<dir>` is the directory whose BEHAVIOUR the habit describes, so a habit inherits the resolver,
the cascade, `covers: subtree` termination, `cites:` and `retire-when:` for free — no second
resolver, no second manifest kind, no second birth rule. Two walks follow from that placement,
and they are different questions:

  * `in_scope(target)` walks UP (the cascade's direction) — "which habits govern the file I am
    editing?" The answer is derived from where you are, which is what dissolves the meaningless
    cross-domain comparison a flat register forces.
  * `census(root)` walks DOWN — "what has this repo committed to?" That is the register, and
    `genesis/manifests/habits.yaml` is its GENERATED projection (see `habits-project.py`), kept
    so every existing reader — the a2o declared-concern denominator, saga-status, ci-harvest,
    latency-scoreboard, seam_forecast — keeps its path and its shape.

WHY THE EVIDENCE IS THE BODY. 97% of the old register was evidence prose and 3% was declaration.
Folded into a YAML scalar it could only ever be a wall; as a markdown body it is the same bytes,
diffable and readable, and the declaration above it fits on a screen. The projector re-indents
the body verbatim under `evidence: >`, so the round-trip is exact rather than approximate.

THE COVENANT, as amended 2026-08-27 (`.epr-meta/habits-covenant.md` carries the prose):
  * `max 2 active` stays GLOBAL — it bounds ATTENTION (one operator, one day job), and survives
    composition as a roll-up assertion over the resolved tree. Enforced here for the first time;
    it was prose-only in every prior form of the register.
  * The headcount cap is GONE. It bounded DECLARATION, and 12-in-one-directory is a smell where
    60-across-8-lanes is not. `retire-when:` — required on every habit — is the instrument that
    actually prevents accumulation, because an exit condition retires a habit and a headcount
    only ever refuses the next one. Its removal is why `unwired` can be spent again: with no free
    slots you never spend one on a commitment you cannot yet observe, which welded shut the
    register's own valve for honest uncertainty.
"""
from __future__ import annotations

import subprocess
from pathlib import Path

from _lib import frontmatter as fm

try:
    import yaml
except Exception:  # pragma: no cover — callers degrade to an empty census
    yaml = None

MANIFEST_DIR_NAME = ".epr-meta"
HABIT_SUFFIX = ".habit.md"
HABIT_VERSION = 1
STATUSES = ("green", "red", "unwired")
MAX_ACTIVE = 2  # the WIP fence — attention, not headcount (covenant rule 3)

# Keys the projection never emits: they are the atom's own metadata, not register content.
_META_KEYS = {"epr-habit-version", "cites"}
# Canonical field order in the projection. Anything else follows in declared order.
_FIELD_ORDER = ("id", "invariant", "status", "active", "checks", "evidence")

_SKIP = ("/node_modules/", "/.claude/worktrees/", "/fixtures/", "/__tests__/")


def governed_dir(habit_path: Path) -> Path:
    """The directory a habit governs: `<dir>/.epr-meta/<id>.habit.md` -> `<dir>`."""
    return Path(habit_path).parent.parent


def discover(root: Path) -> list[Path]:
    """Every habit atom in the tree. Git's index first (44ms against 1.9s for a full rglob — this
    runs on the session-start path), including untracked-but-not-ignored files so a habit you just
    wrote is discoverable before it is committed. Falls back to a walk when git is unavailable."""
    root = Path(root)
    found: list[Path] = []
    try:
        out = subprocess.run(
            ["git", "-C", str(root), "ls-files", "--cached", "--others", "--exclude-standard",
             "--", f"*{HABIT_SUFFIX}"],
            capture_output=True, text=True, timeout=10, check=False)
        if out.returncode == 0:
            found = [root / line for line in out.stdout.splitlines() if line.strip()]
        else:
            raise RuntimeError(out.stderr)
    except Exception:  # noqa: BLE001 — no git, no index, or a hostile environment
        found = list(root.rglob(f"*{HABIT_SUFFIX}"))
    keep = []
    for p in found:
        rel = f"/{p.relative_to(root).as_posix()}" if p.is_absolute() else f"/{p}"
        if any(s in rel for s in _SKIP):
            continue
        if p.parent.name != MANIFEST_DIR_NAME:
            continue  # a `*.habit.md` outside the governance package declares nothing
        if p.is_file():
            keep.append(p)
    return sorted(keep)


def load(path: Path) -> dict:
    """Parse one habit atom. Returns {} when it cannot be read as one — the caller's census
    reports the file as an error rather than silently dropping a declared commitment."""
    path = Path(path)
    if yaml is None:
        return {}
    try:
        parsed = fm.parse_file(path)
        data = yaml.safe_load(parsed.raw_block) or {}
    except Exception:  # noqa: BLE001
        return {}
    if not isinstance(data, dict):
        return {}
    habit = dict(data)
    # Evidence is the BODY when it is a ledger of prose — the common case, and the reason the
    # declaration fits on a screen. A habit whose evidence is a LIST keeps it in the frontmatter:
    # that is data, not a ledger, and moving it would silently change the field's type.
    if "evidence" not in habit:
        habit["evidence"] = parsed.body.strip("\n")
    habit["_source"] = path
    habit["_raw"] = parsed.raw_block
    habit["_dir"] = governed_dir(path)
    return habit


def validate(habit: dict, label: str) -> list[str]:
    """Schema contract for one habit. [] = valid.

    `retire-when` is required and `never` is a legitimate answer — but a BARE `never` is
    indistinguishable from not having thought about it, so it carries its reason. Same discipline
    `.epr-meta` rules already apply, and the same reason: make the uncomfortable state declarable
    and counted instead of papered over.
    """
    errs: list[str] = []
    if not isinstance(habit, dict) or not habit:
        return [f"{label}: not a habit declaration (unreadable frontmatter?)"]
    if habit.get("epr-habit-version") != HABIT_VERSION:
        errs.append(f"{label}: missing/invalid `epr-habit-version` (must be {HABIT_VERSION})")
    hid = habit.get("id")
    if not isinstance(hid, str) or not hid:
        errs.append(f"{label}: missing `id`")
    elif habit.get("_source") is not None:
        expect = f"{hid}{HABIT_SUFFIX}"
        actual = Path(habit["_source"]).name
        if actual != expect:
            errs.append(f"{label}: filename `{actual}` must match `id` (`{expect}`) — the "
                        f"filename IS the address the cascade resolves")
    if not isinstance(habit.get("invariant"), str) or not habit.get("invariant", "").strip():
        errs.append(f"{label}: missing `invariant` — a habit with no stated invariant is a mood")
    status = habit.get("status")
    if status not in STATUSES:
        errs.append(f"{label}: `status` must be one of {list(STATUSES)} — got {status!r}")
    if not isinstance(habit.get("active", False), bool):
        errs.append(f"{label}: `active` must be a boolean")
    checks = habit.get("checks")
    if status == "unwired":
        if checks:
            errs.append(f"{label}: `unwired` means NO runnable check exists — this one declares "
                        f"checks, so its status is green or red, measured")
    elif not isinstance(checks, list) or not checks or not all(
            isinstance(c, str) and c.strip() for c in checks):
        errs.append(f"{label}: `checks` must be a non-empty list of runnable checks (a habit is "
                    f"bound to evidence, never to intention) — or `status: unwired`, which is the "
                    f"honest declaration that no check exists yet")
    rw = habit.get("retire-when")
    if not isinstance(rw, str) or not rw.strip():
        errs.append(f"{label}: missing `retire-when` — an exit condition, not a date. A habit "
                    f"with no stated retirement can only accrete; `never: <why this is a floor>` "
                    f"is a legitimate answer and the point of admitting it is that it is counted")
    elif rw.strip().lower() == "never":
        errs.append(f"{label}: a bare `retire-when: never` is indistinguishable from not having "
                    f"thought about it — write `never: <why this is a floor>`. A constitutional "
                    f"floor genuinely never retires; saying so is the point")
    return errs


COVENANT_REL = Path(MANIFEST_DIR_NAME) / "habits-covenant.md"


def load_covenant(root: Path) -> tuple[dict, str, str]:
    """The register's cross-cutting leg: `version`/`updated`/`vision`, the priority `order:`, and
    the covenant prose. Declaration is local; PRIORITY is not — "the top red" is an operator
    judgment ACROSS domains and no single lane can hold it. That is the same reason `max 2 active`
    stays global: both bound attention, and attention does not compose."""
    path = Path(root) / COVENANT_REL
    if yaml is None or not path.is_file():
        return {}, "", ""
    try:
        parsed = fm.parse_file(path)
        data = yaml.safe_load(parsed.raw_block) or {}
    except Exception:  # noqa: BLE001
        return {}, "", ""
    return (data if isinstance(data, dict) else {}), parsed.body, parsed.raw_block


def census(root: Path) -> tuple[list[dict], list[str]]:
    """The register: every habit declared anywhere in the tree, plus the roll-up assertions no
    single manifest can make. Ordered by the covenant's `order:`, then by governed directory —
    so a habit the operator has not yet ranked is visible but never silently jumps the queue."""
    root = Path(root)
    habits: list[dict] = []
    errs: list[str] = []
    for path in discover(root):
        rel = path.relative_to(root).as_posix() if path.is_absolute() else str(path)
        habit = load(path)
        if not habit:
            errs.append(f"{rel}: unreadable habit declaration")
            continue
        errs.extend(validate(habit, rel))
        habits.append(habit)

    seen: dict[str, str] = {}
    for h in habits:
        hid = h.get("id")
        rel = _rel(h, root)
        if not isinstance(hid, str):
            continue
        if hid in seen:
            errs.append(f"duplicate habit id `{hid}` — declared at {seen[hid]} and {rel}. One "
                        f"habit, one home: the id is the join across register, CI and Gherkin")
        else:
            seen[hid] = rel

    # Concern ids stay GLOBALLY unique even though declaration is local — a habit's `checks:`
    # string carries the `@concern:` tag that is the `check_id` in a sprint report, the single
    # join across register / CI / Gherkin. Composing declaration must not fragment that namespace.
    owner: dict[str, str] = {}
    for h in habits:
        for concern in concerns_of(h):
            prior = owner.get(concern)
            if prior is not None and prior != h.get("id"):
                errs.append(f"@concern:{concern} is claimed by two habits (`{prior}` and "
                            f"`{h.get('id')}`) — a concern joins ONE habit to its evidence; "
                            f"split the check or merge the habits")
            else:
                owner[concern] = h.get("id")

    active = [h for h in habits if h.get("active") is True]
    if len(active) > MAX_ACTIVE:
        names = ", ".join(sorted(str(h.get("id")) for h in active))
        errs.append(f"WIP fence breached: {len(active)} habits are `active` (max {MAX_ACTIVE}) — "
                    f"{names}. Finishing beats starting; this bound is attention, and attention "
                    f"does not compose")

    covenant, _, _ = load_covenant(root)
    order = [i for i in (covenant.get("order") or []) if isinstance(i, str)]
    for stale in [i for i in order if i not in seen]:
        errs.append(f"covenant `order:` ranks `{stale}`, which no habit declares — a rank with no "
                    f"habit is a queue position nobody can ever take")
    rank = {hid: i for i, hid in enumerate(order)}
    habits.sort(key=lambda h: (rank.get(str(h.get("id")), len(rank)),
                               str(h.get("_dir", "")), str(h.get("id", ""))))
    return habits, errs


def in_scope(target: Path, root: Path) -> list[dict]:
    """Habits governing `target`, NEAREST FIRST — the upward walk, so "the top red" becomes "the
    top red in scope". A habit governs a path when its declaring directory is an ancestor of it
    (the repo root governing everything, which is what a cross-cutting habit means)."""
    target = Path(target).resolve()
    root = Path(root).resolve()
    out = []
    for h in census(root)[0]:
        hdir = Path(h["_dir"]).resolve()
        if hdir == target or hdir in target.parents:
            out.append((len(hdir.parts), h))
    return [h for _, h in sorted(out, key=lambda t: -t[0])]


def concerns_of(habit: dict) -> list[str]:
    """The `@concern:` tags a habit's checks name — its join to CI and Gherkin."""
    import re
    tags: list[str] = []
    for c in habit.get("checks") or []:
        if isinstance(c, str):
            tags.extend(re.findall(r"@concern:([a-z0-9][a-z0-9-]*)", c))
    return tags


def _rel(habit: dict, root: Path) -> str:
    src = Path(habit.get("_source", "?"))
    try:
        return src.relative_to(Path(root)).as_posix()
    except ValueError:
        return str(src)


# ---------------------------------------------------------------------------
# Projection — the census rendered as the register file every existing reader knows.
# ---------------------------------------------------------------------------

def _raw_field_lines(raw: str) -> dict[str, list[str]]:
    """Split a habit's frontmatter SOURCE into per-key line runs. Source, not re-serialized
    values: re-emitting through PyYAML would re-quote every check string and re-fold every block
    scalar, so a projection built that way could never be proven byte-faithful to its source."""
    fields: dict[str, list[str]] = {}
    key = None
    for line in raw.split("\n"):
        if line and not line[0].isspace() and ":" in line:
            key = line.split(":", 1)[0].strip()
            fields[key] = [line]
        elif key is not None:
            fields[key].append(line)
    return fields


def project_habit(habit: dict) -> list[str]:
    """One habit as the register's list-item source lines (4-space body indent under `  - `)."""
    fields = _raw_field_lines(habit.get("_raw", ""))
    ordered = [k for k in _FIELD_ORDER if k in fields or k == "evidence"]
    ordered += [k for k in fields if k not in ordered and k not in _META_KEYS]

    out: list[str] = []
    for key in ordered:
        if key in _META_KEYS:
            continue
        if key == "evidence" and key not in fields:
            body = habit.get("evidence", "")
            if not isinstance(body, str) or not body.strip():
                continue
            out.append("    evidence: >")
            out.extend(f"      {ln}" if ln.strip() else "" for ln in body.split("\n"))
            continue
        if key not in fields:
            continue
        out.extend(f"    {ln}" if ln.strip() else "" for ln in fields[key])
    if out:
        out[0] = "  - " + out[0][4:]
    return out


def project(habits: list[dict], header: str, covenant_raw: str) -> str:
    """The full register file: covenant header, top-level declarations, then every habit.

    The top-level fields are emitted from the covenant's SOURCE lines for the same reason a
    habit's are — re-serializing a folded scalar through PyYAML silently changes the value it
    parses back to (a trailing newline is enough), and a projection that cannot be proven equal
    to its source is the drift it exists to prevent."""
    lines = [header.rstrip("\n"), ""]
    fields = _raw_field_lines(covenant_raw)
    for key in ("version", "updated", "vision"):
        if key in fields:
            lines.extend(fields[key])
    lines.append("")
    lines.append("habits:")
    for habit in habits:
        lines.extend(project_habit(habit))
        lines.append("")
    while lines and not lines[-1]:
        lines.pop()
    return "\n".join(lines) + "\n"
