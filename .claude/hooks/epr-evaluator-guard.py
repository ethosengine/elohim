#!/usr/bin/env python3
"""SessionStart guard for a verified epr head and persistent device launcher.

The guard is fail-open. It never changes the PATH entry until ``current`` resolves to bytes whose
hash matches its CID. A missing or headless store is resumable through ``bootstrap``.
"""

from __future__ import annotations

import os
import subprocess
import sys
import tempfile
import time
from pathlib import Path


def _repo() -> Path | None:
    here = Path(__file__).resolve()
    for parent in here.parents:
        if (parent / "genesis/agentic/bin/epr-store").is_file():
            return parent
    return None


def _say(line: str) -> None:
    print(f"EPR EVALUATOR: {line}")


def _run(cli: Path, repo: Path, *args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [str(cli), *args], cwd=repo, capture_output=True, text=True,
        timeout=900 if args[:1] == ("bootstrap",) else 20, check=False,
    )


def _detail(result: subprocess.CompletedProcess[str]) -> str:
    lines = (result.stderr or result.stdout).strip().splitlines()
    return lines[-1] if lines else f"exit {result.returncode}"


def _write_if_changed(content: bytes, destination: Path, mode: int) -> None:
    try:
        if destination.read_bytes() == content and destination.stat().st_mode & 0o777 == mode:
            return
    except OSError:
        pass
    destination.parent.mkdir(parents=True, exist_ok=True)
    fd, name = tempfile.mkstemp(prefix=f".{destination.name}.", dir=destination.parent)
    temporary = Path(name)
    try:
        with os.fdopen(fd, "wb") as target:
            target.write(content)
            target.flush()
            os.fsync(target.fileno())
        temporary.chmod(mode)
        os.replace(temporary, destination)
    finally:
        temporary.unlink(missing_ok=True)


def _install_link(launcher: Path) -> tuple[Path, str | None, bool]:
    cargo_home = Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo")).expanduser().resolve()
    destination = cargo_home / "bin/epr"
    destination.parent.mkdir(parents=True, exist_ok=True)
    try:
        if destination.is_symlink() and destination.resolve() == launcher.resolve():
            return destination, None, False
    except OSError:
        pass
    try:
        previous_target = os.readlink(destination)
    except OSError:
        previous_target = None
    temporary = destination.with_name(f".epr-launch.{os.getpid()}.{time.time_ns()}")
    temporary.symlink_to(launcher)
    try:
        os.replace(temporary, destination)
    finally:
        temporary.unlink(missing_ok=True)
    return destination, previous_target, True


def _restore_link(destination: Path, previous_target: str | None, changed: bool) -> None:
    if not changed:
        return
    if previous_target is None:
        destination.unlink(missing_ok=True)
        return
    temporary = destination.with_name(f".epr-restore.{os.getpid()}.{time.time_ns()}")
    temporary.symlink_to(previous_target)
    try:
        os.replace(temporary, destination)
    finally:
        temporary.unlink(missing_ok=True)


def _installed_healthy(destination: Path, store: Path) -> tuple[bool, str]:
    result = subprocess.run(
        [str(destination), "--version"],
        env={"PATH": "/usr/bin:/bin", "EPR_STORE": str(store)},
        capture_output=True,
        text=True,
        timeout=20,
        check=False,
    )
    return result.returncode == 0, _detail(result)


def main() -> None:
    repo = _repo()
    if repo is None:
        return
    store = Path(
        os.environ.get("EPR_STORE", "/projects/.claude-config/tools/bin-store/epr")
    ).expanduser().resolve()
    os.environ["EPR_STORE"] = str(store)
    cli = repo / "genesis/agentic/bin/epr-store"
    head = _run(cli, repo, "resolve", "--current")
    if head.returncode != 0:
        bootstrap = _run(cli, repo, "bootstrap", "--repo", str(repo))
        if bootstrap.returncode != 0:
            _say(f"⚠ no verified head; launcher left untouched: {_detail(bootstrap)}")
            return
        head = _run(cli, repo, "resolve", "--current")
        if head.returncode != 0:
            _say(f"⚠ bootstrap returned without a verified head; launcher left untouched: {_detail(head)}")
            return

    persistent_bin = store.parent / "bin"
    launcher = persistent_bin / "epr-launch"
    store_module = (persistent_bin / "epr_store.py").resolve()
    _write_if_changed((repo / ".claude/scripts/_lib/epr_store.py").read_bytes(), store_module, 0o644)
    template = (repo / "genesis/agentic/bin/epr-launch").read_text(encoding="utf-8")
    rendered = (
        template.replace("@EPR_PYTHON@", str(Path(sys.executable).expanduser().resolve()))
        .replace("@EPR_STORE_PY@", str(store_module))
        .encode()
    )
    _write_if_changed(rendered, launcher.resolve(), 0o755)
    destination, previous_target, changed = _install_link(launcher.resolve())
    healthy, detail = _installed_healthy(destination, store)
    if not healthy:
        _restore_link(destination, previous_target, changed)
        _say(f"⚠ installed launcher verification failed; previous link restored: {detail}")
        return

    pinned = _run(cli, repo, "pin")
    if pinned.returncode == 0:
        _say(f"verified head · persistent launcher {destination} · process-ancestry pin active")
    elif "session-absent" in (pinned.stderr or ""):
        _say(f"verified head · persistent launcher {destination} · no session ancestry to pin")
    else:
        _say(f"verified head · persistent launcher {destination} · pin refused: {_detail(pinned)}")


if __name__ == "__main__":
    try:
        main()
    except Exception as error:  # noqa: BLE001 — SessionStart repair must never block the session
        _say(f"guard error (ignored): {error}")
