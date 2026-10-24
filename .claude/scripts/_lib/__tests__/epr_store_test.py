"""The epr store keeps heads atomic, sessions immutable, and device state isolated."""

from __future__ import annotations

import datetime as dt
import hashlib
import importlib.util
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path
from unittest import mock

import pytest

LIB = Path(__file__).resolve().parents[1]
REPO = LIB.parents[2]
sys.path.insert(0, str(LIB.parent))

from _lib import epr_client, epr_store  # noqa: E402

LAUNCHER_SOURCE = REPO / "genesis/agentic/bin/epr-launch"
GUARD = REPO / ".claude/hooks/epr-evaluator-guard.py"


@pytest.fixture(autouse=True)
def no_claude_ancestor(tmp_path_factory: pytest.TempPathFactory, monkeypatch: pytest.MonkeyPatch) -> None:
    """Run as if no ``claude`` process were an ancestor, even when the suite runs under one."""
    monkeypatch.setenv("EPR_PROC_ROOT", str(tmp_path_factory.mktemp("no-proc")))


def git(repo: Path, *args: str) -> str:
    result = subprocess.run(
        ["git", *args], cwd=repo, capture_output=True, text=True, check=True
    )
    return result.stdout.strip()


@pytest.fixture
def scratch(tmp_path: Path) -> tuple[Path, Path, str]:
    store = tmp_path / "store"
    repo = tmp_path / "repo"
    repo.mkdir()
    git(repo, "init", "-q")
    git(repo, "config", "user.name", "Store Test")
    git(repo, "config", "user.email", "store@example.test")
    (repo / "tracked").write_text("fixture\n", encoding="utf-8")
    git(repo, "add", "tracked")
    git(repo, "commit", "-qm", "fixture")
    return store, repo, git(repo, "rev-parse", "HEAD")


def commit(repo: Path, text: str) -> str:
    (repo / "tracked").write_text(text, encoding="utf-8")
    git(repo, "add", "tracked")
    git(repo, "commit", "-qm", text.strip())
    return git(repo, "rev-parse", "HEAD")


def fake_epr(path: Path, label: str) -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        f"#!{sys.executable}\n"
        "import hashlib, json, pathlib, sys\n"
        f"LABEL = {label!r}\n"
        "here = pathlib.Path(__file__)\n"
        "cid = 'sha256:' + hashlib.sha256(here.read_bytes()).hexdigest()\n"
        "if sys.argv[1:] == ['--version']:\n"
        "    print(json.dumps({'id': 'elohim-epr-cli', 'version': LABEL, 'cid': cid}))\n"
        "    raise SystemExit(0)\n"
        "if sys.argv[1:] == ['flow', 'report', 'scope']:\n"
        "    raise SystemExit(1 if LABEL.startswith('fail') else 0)\n"
        "raise SystemExit(2)\n",
        encoding="utf-8",
    )
    path.chmod(0o755)
    return path


def ingest(
    store: Path, repo: Path, source_commit: str, binary: Path, *, clean: bool = True
) -> dict:
    return epr_store.ingest(
        binary, root=store, repo=repo, source_commit=source_commit,
        gate_run="test", tree_clean=clean,
    )


def clean_env(**extra: str) -> dict[str, str]:
    env = {
        name: value
        for name, value in os.environ.items()
        if name not in {*epr_store.SESSION_ENV, "EPR_BIN", "EPR_PIN_CID", "EPR_STORE"}
    }
    env.update(extra)
    return env


def proc_entry(root: Path, pid: int, ppid: int, comm: str, starttime: str) -> None:
    process = root / str(pid)
    process.mkdir(parents=True)
    process.joinpath("comm").write_text(f"{comm}\n", encoding="utf-8")
    fields = ["S", str(ppid), *(["0"] * 17), starttime]
    process.joinpath("stat").write_text(
        f"{pid} ({comm}) {' '.join(fields)}\n", encoding="utf-8"
    )


def install_launcher(tmp_path: Path) -> tuple[Path, Path]:
    root = tmp_path / "device/bin"
    root.mkdir(parents=True)
    launcher = root / "epr-launch"
    rendered = (
        LAUNCHER_SOURCE.read_text(encoding="utf-8")
        .replace("@EPR_PYTHON@", str(Path(sys.executable).resolve()))
        .replace("@EPR_STORE_PY@", str((root / "epr_store.py").resolve()))
    )
    launcher.write_text(rendered, encoding="utf-8")
    launcher.chmod(0o755)
    shutil.copyfile(LIB / "epr_store.py", root / "epr_store.py")
    return launcher, root / "epr_store.py"


def age_record(store: Path, cid: str, when: dt.datetime) -> None:
    path = store / cid / "record.json"
    record = json.loads(path.read_text(encoding="utf-8"))
    record["ingested_at"] = when.isoformat().replace("+00:00", "Z")
    path.write_text(json.dumps(record), encoding="utf-8")


def load_guard():
    spec = importlib.util.spec_from_file_location("epr_evaluator_guard_under_test", GUARD)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


def test_session_key_uses_nearest_claude_ancestor_and_starttime(tmp_path: Path) -> None:
    proc = tmp_path / "proc"
    proc_entry(proc, 400, 300, "python3", "4000")
    proc_entry(proc, 300, 200, "bash", "3000")
    proc_entry(proc, 200, 100, "claude", "2000")
    proc_entry(proc, 100, 1, "claude", "1000")

    identity = epr_store.session_identity(
        proc_root=proc, start_pid=400, environ={"ELOHIM_SESSION_ID": "fallback"}
    )

    assert identity == {
        "session": "claude-200-2000", "owner_pid": 200, "owner_starttime": "2000"
    }


def test_session_key_falls_back_to_declared_environment(tmp_path: Path) -> None:
    assert epr_store.session_identity(
        proc_root=tmp_path / "missing", start_pid=9,
        environ={"ELOHIM_SESSION_ID": "declared"},
    ) == {"session": "declared"}


def test_resolution_order_is_candidate_explicit_session_then_current(
    scratch: tuple[Path, Path, str], tmp_path: Path
) -> None:
    store, repo, head = scratch
    first = ingest(store, repo, head, fake_epr(tmp_path / "bin/first", "first"))
    second = ingest(store, repo, head, fake_epr(tmp_path / "bin/second", "second"))
    epr_store._atomic_link(store, "current", first["cid"])
    session_env = clean_env(CLAUDE_CODE_SESSION_ID="session-a")
    epr_store.pin(root=store, environ=session_env, proc_root=tmp_path / "no-proc", start_pid=9)
    epr_store._atomic_link(store, "current", second["cid"])

    assert epr_store.resolve(
        root=store, environ=clean_env(EPR_BIN=str(tmp_path / "bin/first"))
    )[1:] == ("candidate", epr_store.hash_file(tmp_path / "bin/first"))
    assert epr_store.resolve(
        root=store, environ=clean_env(EPR_PIN_CID=first["cid"])
    )[1:] == ("explicit-pin", first["cid"])
    assert epr_store.resolve(
        root=store, environ=session_env, proc_root=tmp_path / "no-proc", start_pid=9
    )[1:] == ("session-pin", first["cid"])
    assert epr_store.resolve(root=store, environ=clean_env())[1:] == ("current", second["cid"])


def test_exec_resolution_checks_executable_without_hashing(
    scratch: tuple[Path, Path, str], tmp_path: Path
) -> None:
    store, repo, head = scratch
    record = ingest(store, repo, head, fake_epr(tmp_path / "candidate", "no-exec-hash"))
    epr_store._atomic_link(store, "current", record["cid"])

    with mock.patch.object(epr_store, "hash_file", side_effect=AssertionError("exec hashed")):
        path, source, cid = epr_store.resolve_exec(root=store, environ=clean_env())

    assert (path, source, cid) == (
        store / record["cid"] / "epr", "current", record["cid"]
    )


def test_client_reports_the_resolved_executable_cid_without_spawning(
    scratch: tuple[Path, Path, str], tmp_path: Path
) -> None:
    store, repo, head = scratch
    record = ingest(store, repo, head, fake_epr(tmp_path / "candidate", "client-cid"))
    epr_store._atomic_link(store, "current", record["cid"])
    env = clean_env(EPR_STORE=str(store))

    with mock.patch.object(epr_store, "resolve", wraps=epr_store.resolve) as resolve:
        assert epr_client.resolve(env) == (str(store / record["cid"] / "epr"), record["cid"])

    resolve.assert_called_once()


def test_launcher_alias_is_refused(tmp_path: Path) -> None:
    store = tmp_path / "device/store"
    launcher, _ = install_launcher(tmp_path)
    cargo_link = tmp_path / "cargo/bin/epr"
    cargo_link.parent.mkdir(parents=True)
    cargo_link.symlink_to(launcher)

    with pytest.raises(epr_store.StoreRefusal) as failure:
        epr_store.resolve(root=store, environ=clean_env(EPR_BIN=str(cargo_link)))

    assert failure.value.kind == "launcher-alias"


def test_launcher_executes_the_python_resolver(
    scratch: tuple[Path, Path, str], tmp_path: Path
) -> None:
    store, repo, head = scratch
    record = ingest(store, repo, head, fake_epr(tmp_path / "candidate", "exec"))
    epr_store._atomic_link(store, "current", record["cid"])
    launcher, _ = install_launcher(tmp_path)

    result = subprocess.run(
        [str(launcher), "--version"], env=clean_env(EPR_STORE=str(store)),
        capture_output=True, text=True, check=False,
    )

    assert result.returncode == 0, result.stderr
    assert json.loads(result.stdout)["cid"] == record["cid"]
    assert (store / record["cid"] / ".used").is_file()


@pytest.mark.parametrize(
    ("extra", "source"),
    [
        ({}, "current"),
        ({"EPR_PIN_CID": "{cid}"}, "explicit-pin"),
        ({"CLAUDE_CODE_SESSION_ID": "lease-session"}, "session-pin"),
    ],
)
def test_every_stored_exec_refreshes_the_usage_lease(
    scratch: tuple[Path, Path, str], tmp_path: Path, extra: dict[str, str], source: str
) -> None:
    store, repo, head = scratch
    record = ingest(store, repo, head, fake_epr(tmp_path / "candidate", source))
    epr_store._atomic_link(store, "current", record["cid"])
    launcher, _ = install_launcher(tmp_path)
    marker = store / record["cid"] / ".used"
    env_extra = {name: value.replace("{cid}", record["cid"]) for name, value in extra.items()}

    result = subprocess.run(
        [str(launcher), "--version"],
        env=clean_env(EPR_STORE=str(store), **env_extra),
        capture_output=True,
        text=True,
        check=False,
    )

    assert result.returncode == 0, result.stderr
    assert marker.is_file()


def test_atomic_pin_creation_recovers_an_empty_pin(
    scratch: tuple[Path, Path, str], tmp_path: Path
) -> None:
    store, repo, head = scratch
    record = ingest(store, repo, head, fake_epr(tmp_path / "candidate", "pin"))
    epr_store._atomic_link(store, "current", record["cid"])
    path = store / "sessions/session-a"
    path.parent.mkdir(parents=True)
    path.touch()
    env = clean_env(CLAUDE_CODE_SESSION_ID="session-a")

    with mock.patch.object(epr_store.os, "link", wraps=os.link) as link:
        pinned = epr_store.pin(
            root=store, environ=env, proc_root=tmp_path / "no-proc", start_pid=7
        )

    assert pinned["cid"] == record["cid"]
    assert json.loads(path.read_text(encoding="utf-8"))["cid"] == record["cid"]
    assert link.call_count == 1


def test_two_launcher_processes_publish_one_complete_pin(
    scratch: tuple[Path, Path, str], tmp_path: Path
) -> None:
    store, repo, head = scratch
    record = ingest(store, repo, head, fake_epr(tmp_path / "candidate", "race"))
    epr_store._atomic_link(store, "current", record["cid"])
    launcher, _ = install_launcher(tmp_path)
    env = clean_env(EPR_STORE=str(store), CLAUDE_CODE_SESSION_ID="same-session")
    processes = [
        subprocess.Popen([str(launcher), "--version"], env=env, text=True,
                         stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        for _ in range(2)
    ]
    results = [process.communicate(timeout=10) for process in processes]

    assert [process.returncode for process in processes] == [0, 0], results
    assert {json.loads(stdout)["cid"] for stdout, _ in results} == {record["cid"]}
    assert json.loads((store / "sessions/same-session").read_text())["cid"] == record["cid"]


def test_ingest_hashes_staged_bytes_and_publishes_read_only(
    scratch: tuple[Path, Path, str], tmp_path: Path
) -> None:
    store, repo, head = scratch
    source = fake_epr(tmp_path / "candidate", "staged")
    original_copy = shutil.copyfile

    def copy_then_mutate(src, destination):
        result = original_copy(src, destination)
        Path(src).write_text("changed after staging\n", encoding="utf-8")
        return result

    with mock.patch.object(epr_store.shutil, "copyfile", side_effect=copy_then_mutate):
        record = ingest(store, repo, head, source)

    stored = store / record["cid"] / "epr"
    assert epr_store.hash_file(stored) == record["cid"]
    assert epr_store.hash_file(source) != record["cid"]
    assert stored.stat().st_mode & 0o222 == 0
    assert not any((store / ".staging").iterdir())


def test_land_updates_heads_under_lock(
    scratch: tuple[Path, Path, str], tmp_path: Path
) -> None:
    store, repo, head = scratch
    first = ingest(store, repo, head, fake_epr(tmp_path / "a", "a"))
    second = ingest(store, repo, head, fake_epr(tmp_path / "b", "b"))
    epr_store.land(first["cid"], head, root=store, repo=repo)
    second_landed = epr_store.land(second["cid"], head, root=store, repo=repo)

    assert epr_store._read_link(store, "current") == second["cid"]
    assert epr_store._read_link(store, "previous") == first["cid"]
    assert json.loads((store / "current.json").read_text(encoding="utf-8")) == {
        "cid": second["cid"],
        "commit": head,
        "landed_at": second_landed["landed_at"],
        "previous": first["cid"],
    }


def test_reingest_preserves_primary_provenance_and_appends_sources(
    scratch: tuple[Path, Path, str], tmp_path: Path
) -> None:
    store, repo, first_commit = scratch
    binary = fake_epr(tmp_path / "candidate", "same-bytes")
    first = ingest(store, repo, first_commit, binary)
    epr_store.land(first["cid"], first_commit, root=store, repo=repo)
    second_commit = commit(repo, "second\n")

    second = ingest(store, repo, second_commit, binary)

    assert second["source_commit"] == first_commit
    assert second["tree_clean"] is True
    assert [source["source_commit"] for source in second["sources"]] == [
        first_commit,
        second_commit,
    ]
    landed = epr_store.land(second["cid"], second_commit, root=store, repo=repo)
    assert landed["cid"] == first["cid"]
    assert json.loads((store / "current.json").read_text())["commit"] == second_commit


def test_land_ancestry_uses_current_json_not_mutable_ingest_provenance(
    scratch: tuple[Path, Path, str], tmp_path: Path
) -> None:
    store, repo, first_commit = scratch
    first = ingest(store, repo, first_commit, fake_epr(tmp_path / "first", "first-current"))
    epr_store.land(first["cid"], first_commit, root=store, repo=repo)
    git(repo, "checkout", "--orphan", "divergent-provenance")
    (repo / "tracked").unlink()
    git(repo, "add", "-u")
    (repo / "other").write_text("other\n", encoding="utf-8")
    git(repo, "add", "other")
    git(repo, "commit", "-qm", "divergent")
    divergent_commit = git(repo, "rev-parse", "HEAD")
    divergent = ingest(
        store, repo, divergent_commit, fake_epr(tmp_path / "divergent", "divergent-current")
    )
    current_record_path = store / first["cid"] / "record.json"
    current_record = json.loads(current_record_path.read_text())
    current_record["source_commit"] = divergent_commit
    current_record["sources"] = [{"source_commit": divergent_commit, "tree_clean": True}]
    current_record_path.write_text(json.dumps(current_record), encoding="utf-8")

    with pytest.raises(epr_store.StoreRefusal) as failure:
        epr_store.land(divergent["cid"], divergent_commit, root=store, repo=repo)

    assert failure.value.kind == "non-descendant"


def test_health_runs_outside_lock_and_checkout_is_revalidated_afterward(
    scratch: tuple[Path, Path, str], tmp_path: Path
) -> None:
    store, repo, head = scratch
    candidate = ingest(store, repo, head, fake_epr(tmp_path / "candidate", "health-window"))

    def health(_binary: Path, _cid: str, _repo: Path) -> None:
        with epr_store.store_lock(store, timeout=0.1):
            pass
        (repo / "tracked").write_text("changed during health\n", encoding="utf-8")

    with mock.patch.object(epr_store, "_health_check", side_effect=health):
        with pytest.raises(epr_store.StoreRefusal) as failure:
            epr_store.land(candidate["cid"], head, root=store, repo=repo)

    assert failure.value.kind == "checkout-dirty"


def test_land_refuses_when_current_moves_during_health(
    scratch: tuple[Path, Path, str], tmp_path: Path
) -> None:
    store, repo, head = scratch
    first = ingest(store, repo, head, fake_epr(tmp_path / "first", "move-first"))
    candidate = ingest(store, repo, head, fake_epr(tmp_path / "candidate", "move-candidate"))
    raced = ingest(store, repo, head, fake_epr(tmp_path / "raced", "move-raced"))
    epr_store.land(first["cid"], head, root=store, repo=repo)

    def health(_binary: Path, _cid: str, _repo: Path) -> None:
        with epr_store.store_lock(store):
            epr_store._atomic_link(store, "current", raced["cid"])
            epr_store._atomic_json(store / "current.json", {
                "cid": raced["cid"], "commit": head, "landed_at": epr_store.now(),
                "previous": first["cid"],
            })

    with mock.patch.object(epr_store, "_health_check", side_effect=health):
        with pytest.raises(epr_store.StoreRefusal) as failure:
            epr_store.land(candidate["cid"], head, root=store, repo=repo)

    assert failure.value.kind == "current-moved"


def test_land_refuses_failed_health_without_moving_current(
    scratch: tuple[Path, Path, str], tmp_path: Path
) -> None:
    store, repo, head = scratch
    good = ingest(store, repo, head, fake_epr(tmp_path / "good", "good"))
    bad = ingest(store, repo, head, fake_epr(tmp_path / "bad", "fail-health"))
    epr_store.land(good["cid"], head, root=store, repo=repo)

    with pytest.raises(epr_store.StoreRefusal) as failure:
        epr_store.land(bad["cid"], head, root=store, repo=repo)

    assert failure.value.kind == "health-flow-failed"
    assert epr_store._read_link(store, "current") == good["cid"]


def test_land_checks_record_head_and_checkout_cleanliness(
    scratch: tuple[Path, Path, str], tmp_path: Path
) -> None:
    store, repo, head = scratch
    dirty_build = ingest(store, repo, head, fake_epr(tmp_path / "dirty", "dirty"), clean=False)
    clean_build = ingest(store, repo, head, fake_epr(tmp_path / "clean", "clean"))
    with pytest.raises(epr_store.StoreRefusal) as dirty:
        epr_store.land(dirty_build["cid"], head, root=store, repo=repo)
    assert dirty.value.kind == "dirty-build"
    with pytest.raises(epr_store.StoreRefusal) as record_mismatch:
        epr_store.land(clean_build["cid"], "0" * 40, root=store, repo=repo)
    assert record_mismatch.value.kind == "commit-mismatch"

    later = commit(repo, "later\n")
    with pytest.raises(epr_store.StoreRefusal) as mismatch:
        epr_store.land(clean_build["cid"], head, root=store, repo=repo)
    assert mismatch.value.kind == "head-mismatch"

    later_build = ingest(store, repo, later, fake_epr(tmp_path / "later", "later"))
    (repo / "tracked").write_text("uncommitted\n", encoding="utf-8")
    with pytest.raises(epr_store.StoreRefusal) as dirty_checkout:
        epr_store.land(later_build["cid"], later, root=store, repo=repo)
    assert dirty_checkout.value.kind == "checkout-dirty"


def test_land_requires_descent_unless_revert(
    scratch: tuple[Path, Path, str], tmp_path: Path
) -> None:
    store, repo, first_commit = scratch
    first = ingest(store, repo, first_commit, fake_epr(tmp_path / "first", "first"))
    epr_store.land(first["cid"], first_commit, root=store, repo=repo)
    main_branch = git(repo, "branch", "--show-current")
    second_commit = commit(repo, "second\n")
    second = ingest(store, repo, second_commit, fake_epr(tmp_path / "second", "second"))
    epr_store.land(second["cid"], second_commit, root=store, repo=repo)

    git(repo, "checkout", "--orphan", "divergent")
    (repo / "tracked").unlink()
    git(repo, "add", "-u")
    (repo / "other").write_text("other\n", encoding="utf-8")
    git(repo, "add", "other")
    git(repo, "commit", "-qm", "divergent")
    divergent_commit = git(repo, "rev-parse", "HEAD")
    divergent = ingest(store, repo, divergent_commit, fake_epr(tmp_path / "divergent", "divergent"))
    with pytest.raises(epr_store.StoreRefusal) as ancestry:
        epr_store.land(divergent["cid"], divergent_commit, root=store, repo=repo)
    assert ancestry.value.kind == "non-descendant"

    git(repo, "checkout", "-q", "--detach", first_commit)
    reverted = epr_store.land(first["cid"], first_commit, root=store, repo=repo, revert=True)
    assert reverted["ring"] == "reverted"
    assert epr_store._read_link(store, "current") == first["cid"]
    assert epr_store._read_link(store, "previous") == second["cid"]
    assert main_branch


def test_gc_holds_lock_and_keeps_heads_live_pins_and_grace(
    scratch: tuple[Path, Path, str], tmp_path: Path
) -> None:
    store, repo, head = scratch
    labels = ("current", "previous", "live", "ownerless", "recent", "garbage")
    records = {
        label: ingest(store, repo, head, fake_epr(tmp_path / label, label))
        for label in labels
    }
    current_time = dt.datetime(2026, 10, 24, tzinfo=dt.timezone.utc)
    old = current_time - dt.timedelta(days=20)
    for record in records.values():
        age_record(store, record["cid"], old)
    age_record(store, records["recent"]["cid"], current_time - dt.timedelta(days=2))
    epr_store._atomic_link(store, "current", records["current"]["cid"])
    epr_store._atomic_link(store, "previous", records["previous"]["cid"])

    proc = tmp_path / "proc"
    proc_entry(proc, 200, 1, "claude", "live-start")
    proc_entry(proc, 201, 1, "claude", "new-owner-start")
    sessions = store / "sessions"
    sessions.mkdir()
    sessions.joinpath("live").write_text(json.dumps({
        "cid": records["live"]["cid"], "session": "claude-200-live-start",
        "owner_pid": 200, "owner_starttime": "live-start",
        "pinned_at": old.isoformat(),
    }))
    sessions.joinpath("ownerless").write_text(json.dumps({
        "cid": records["ownerless"]["cid"], "session": "fallback",
        "pinned_at": (current_time - dt.timedelta(days=2)).isoformat(),
    }))
    sessions.joinpath("dead").write_text(json.dumps({
        "cid": records["garbage"]["cid"], "session": "claude-201-old",
        "owner_pid": 201, "owner_starttime": "old", "pinned_at": old.isoformat(),
    }))

    calls: list[int] = []
    real_flock = epr_store.fcntl.flock
    with mock.patch.object(
        epr_store.fcntl, "flock", side_effect=lambda fd, operation: (calls.append(operation), real_flock(fd, operation))[1]
    ):
        result = epr_store.gc(root=store, proc_root=proc, at=current_time)

    expected = {records[name]["cid"] for name in ("current", "previous", "live", "ownerless", "recent")}
    assert set(result["kept"]) == expected
    assert result["removed"] == [records["garbage"]["cid"]]
    assert "dead" in result["removed_sessions"]
    assert any(operation & epr_store.fcntl.LOCK_EX for operation in calls)


def test_gc_keeps_an_old_artifact_used_within_fourteen_days(
    scratch: tuple[Path, Path, str], tmp_path: Path
) -> None:
    store, repo, head = scratch
    record = ingest(store, repo, head, fake_epr(tmp_path / "used", "used"))
    current_time = dt.datetime(2026, 10, 24, tzinfo=dt.timezone.utc)
    age_record(store, record["cid"], current_time - dt.timedelta(days=30))
    marker = store / record["cid"] / ".used"
    marker.touch()
    used_at = (current_time - dt.timedelta(days=13)).timestamp()
    os.utime(marker, (used_at, used_at))

    result = epr_store.gc(root=store, at=current_time)

    assert record["cid"] in result["kept"]
    assert result["removed"] == []


def test_exec_pin_lock_timeout_falls_back_to_current_with_one_warning(
    scratch: tuple[Path, Path, str], tmp_path: Path
) -> None:
    store, repo, head = scratch
    record = ingest(store, repo, head, fake_epr(tmp_path / "current", "lock-fallback"))
    epr_store._atomic_link(store, "current", record["cid"])
    warnings: list[str] = []
    refusal = epr_store.StoreRefusal("lock-timeout", "busy for 5s")

    with mock.patch.object(epr_store, "pin", side_effect=refusal):
        path, source, cid = epr_store.resolve_exec(
            root=store,
            environ=clean_env(CLAUDE_CODE_SESSION_ID="waiting"),
            proc_root=tmp_path / "no-proc",
            start_pid=99,
            warn=warnings.append,
        )

    assert (path, source, cid) == (
        store / record["cid"] / "epr", "current-lock-timeout", record["cid"]
    )
    assert len(warnings) == 1
    assert not (store / "sessions/waiting").exists()


def test_other_lock_waits_refuse_with_a_clear_bound(tmp_path: Path) -> None:
    with mock.patch.object(epr_store.fcntl, "flock", side_effect=BlockingIOError):
        with pytest.raises(epr_store.StoreRefusal) as failure:
            with epr_store.store_lock(tmp_path / "store", timeout=0.01):
                pass

    assert failure.value.kind == "lock-timeout"
    assert "0.01s" in failure.value.detail


def test_bootstrap_environment_uses_gate_toolchain_and_resource_caps(tmp_path: Path) -> None:
    environment = epr_store._bootstrap_environment(REPO, tmp_path / "target")
    assert environment["RUSTUP_TOOLCHAIN"] == "1.96.1"
    assert environment["CARGO_BUILD_JOBS"] == "1"
    assert environment["RUST_TEST_THREADS"] == "1"
    assert environment["RUSTFLAGS"] == ""
    assert environment["CARGO_TARGET_DIR"] == str(tmp_path / "target")


def test_headless_bootstrap_resumes_an_existing_candidate(
    scratch: tuple[Path, Path, str], tmp_path: Path
) -> None:
    store, repo, head = scratch
    candidate = ingest(store, repo, head, fake_epr(tmp_path / "candidate", "resume"))

    result = epr_store.bootstrap(root=store, repo=repo)

    assert result["cid"] == candidate["cid"]
    assert epr_store._read_link(store, "current") == candidate["cid"]


def test_guard_installs_only_after_a_verified_head_into_persistent_device_bin(
    scratch: tuple[Path, Path, str], tmp_path: Path
) -> None:
    store, repo, head = scratch
    record = ingest(store, repo, head, fake_epr(tmp_path / "candidate", "guard"))
    epr_store._atomic_link(store, "current", record["cid"])
    cargo_home = tmp_path / "cargo-home"
    env = clean_env(
        EPR_STORE=str(store), CARGO_HOME=str(cargo_home),
        CLAUDE_CODE_SESSION_ID="guard-session",
    )

    result = subprocess.run(
        [sys.executable, str(GUARD)], input="{}", env=env,
        capture_output=True, text=True, check=False,
    )

    assert result.returncode == 0, result.stderr
    installed = store.parent / "bin/epr-launch"
    installed_text = installed.read_text(encoding="utf-8")
    assert installed_text.count("\n") == 2
    assert "readlink" not in installed_text and "dirname" not in installed_text
    assert str(Path(sys.executable).resolve()) in installed_text
    assert str((store.parent / "bin/epr_store.py").resolve()) in installed_text
    assert (store.parent / "bin/epr_store.py").read_bytes() == (LIB / "epr_store.py").read_bytes()
    assert (cargo_home / "bin/epr").resolve() == installed.resolve()
    launched = subprocess.run(
        [str(cargo_home / "bin/epr"), "--version"],
        env={"PATH": "/usr/bin:/bin", "EPR_STORE": str(store)},
        capture_output=True, text=True, check=False,
    )
    assert launched.returncode == 0, launched.stderr
    assert json.loads(launched.stdout)["cid"] == record["cid"]


def test_guard_expands_relative_install_paths_to_absolute(
    scratch: tuple[Path, Path, str], tmp_path: Path
) -> None:
    store, repo, head = scratch
    record = ingest(store, repo, head, fake_epr(tmp_path / "candidate", "relative-guard"))
    epr_store._atomic_link(store, "current", record["cid"])

    result = subprocess.run(
        [sys.executable, str(GUARD)],
        input="{}",
        cwd=tmp_path,
        env=clean_env(EPR_STORE="store", CARGO_HOME="cargo-home", PATH="/usr/bin:/bin"),
        capture_output=True,
        text=True,
        check=False,
    )

    assert result.returncode == 0, result.stderr
    installed = tmp_path / "store/../bin/epr-launch"
    assert installed.resolve().is_file()
    text = installed.read_text(encoding="utf-8")
    assert str(Path(sys.executable).resolve()) in text
    assert str((tmp_path / "bin/epr_store.py").resolve()) in text
    assert (tmp_path / "cargo-home/bin/epr").resolve() == installed.resolve()


def test_guard_restores_previous_link_when_installed_verification_fails(
    scratch: tuple[Path, Path, str], tmp_path: Path
) -> None:
    store, repo, head = scratch
    record = ingest(store, repo, head, fake_epr(tmp_path / "candidate", "rollback"))
    epr_store._atomic_link(store, "current", record["cid"])
    cargo_home = tmp_path / "cargo-home"
    destination = cargo_home / "bin/epr"
    destination.parent.mkdir(parents=True)
    previous = tmp_path / "previous-epr"
    previous.write_text("previous\n", encoding="utf-8")
    destination.symlink_to(previous)
    guard = load_guard()

    with mock.patch.dict(
        os.environ,
        clean_env(EPR_STORE=str(store), CARGO_HOME=str(cargo_home)),
        clear=True,
    ), mock.patch.object(guard, "_installed_healthy", return_value=(False, "fixture failure")):
        guard.main()

    assert os.readlink(destination) == str(previous)


def test_guard_leaves_existing_path_entry_untouched_when_bootstrap_refuses(tmp_path: Path) -> None:
    store = tmp_path / "empty-store"
    cargo_home = tmp_path / "cargo-home"
    existing = cargo_home / "bin/epr"
    existing.parent.mkdir(parents=True)
    existing.write_text("working sentinel\n", encoding="utf-8")

    result = subprocess.run(
        [sys.executable, str(GUARD)], input="{}",
        env=clean_env(EPR_STORE=str(store), CARGO_HOME=str(cargo_home)),
        capture_output=True, text=True, check=False,
    )

    assert result.returncode == 0
    assert existing.read_text(encoding="utf-8") == "working sentinel\n"
    assert "launcher left untouched" in result.stdout


def test_cleanliness_is_judged_over_epr_sources_not_hook_ledgers(
    scratch: tuple[Path, Path, str], monkeypatch: pytest.MonkeyPatch
) -> None:
    _, repo, head = scratch
    (repo / "src").mkdir()
    (repo / "src" / "lib.rs").write_text("fn main() {}\n", encoding="utf-8")
    git(repo, "add", "src")
    git(repo, "commit", "-qm", "source")
    head = git(repo, "rev-parse", "HEAD")
    monkeypatch.setattr(epr_store, "source_paths", lambda _repo: ["src"])

    (repo / "ledger.jsonl").write_text("{}\n", encoding="utf-8")
    assert epr_store.git_state(repo) == (head, True)

    (repo / "src" / "lib.rs").write_text("fn main() { todo!() }\n", encoding="utf-8")
    assert epr_store.git_state(repo) == (head, False)


def test_source_paths_fall_back_to_the_whole_tree_without_a_cargo_answer(tmp_path: Path) -> None:
    assert epr_store.source_paths(tmp_path) == ["."]


def test_shadows_lists_every_epr_reached_before_the_launcher(
    scratch: tuple[Path, Path, str], tmp_path: Path
) -> None:
    store, _, _ = scratch
    store.mkdir(parents=True, exist_ok=True)
    launcher = store.parent / "bin/epr-launch"
    launcher.parent.mkdir(parents=True, exist_ok=True)
    launcher.write_text("#!/bin/sh\n", encoding="utf-8")
    stale, foreign, ours = tmp_path / "stale", tmp_path / "foreign", tmp_path / "ours"
    for directory in (stale, foreign, ours):
        directory.mkdir()
    (stale / "epr").symlink_to(tmp_path / "gone/debug/epr")
    fake_epr(foreign / "epr", "foreign")
    (ours / "epr").symlink_to(launcher)
    after = tmp_path / "after"
    after.mkdir()
    fake_epr(after / "epr", "after-the-launcher")

    found = epr_store.shadows(root=store, path_env=f"{stale}:{foreign}:{ours}:{after}")

    assert [(Path(entry["path"]).parent.name, entry["superseded"]) for entry in found] == [
        ("stale", True), ("foreign", False),
    ]


def test_guard_keeps_a_hand_installed_binary_and_repoints_stale_shadows(
    scratch: tuple[Path, Path, str], tmp_path: Path
) -> None:
    store, repo, head = scratch
    record = ingest(store, repo, head, fake_epr(tmp_path / "candidate", "head"))
    epr_store._atomic_link(store, "current", record["cid"])
    cargo_bin = tmp_path / "cargo-home/bin"
    cargo_bin.mkdir(parents=True)
    hand_built = fake_epr(cargo_bin / "epr", "hand-built-by-cargo-install")
    hand_cid = "sha256:" + hashlib.sha256(hand_built.read_bytes()).hexdigest()
    stale = tmp_path / "local-bin"
    stale.mkdir()
    (stale / "epr").symlink_to(tmp_path / "old-pool/debug/epr")

    result = subprocess.run(
        [sys.executable, str(GUARD)], input="{}",
        env=clean_env(EPR_STORE=str(store), CARGO_HOME=str(tmp_path / "cargo-home"),
                      PATH=f"{stale}:{cargo_bin}:/usr/bin:/bin"),
        capture_output=True, text=True, check=False,
    )

    assert result.returncode == 0, result.stderr
    launcher = (store.parent / "bin/epr-launch").resolve()
    assert (cargo_bin / "epr").resolve() == launcher
    assert (stale / "epr").resolve() == launcher
    kept = json.loads((store / hand_cid / "record.json").read_text(encoding="utf-8"))
    assert kept["gate_run"] == "adopted-from-path" and kept["tree_clean"] is False
    assert f"EPR_PIN_CID={hand_cid}" in result.stdout
    assert "still shadows" not in result.stdout


def _landed_at(store: Path, repo: Path, commit_sha: str, label: str, tmp_path: Path) -> dict:
    record = ingest(store, repo, commit_sha, fake_epr(tmp_path / label, label))
    epr_store.land(record["cid"], commit_sha, root=store, repo=repo)
    return record


def test_follow_adopts_a_newer_published_dev_commit_the_checkout_holds(
    scratch: tuple[Path, Path, str], tmp_path: Path
) -> None:
    store, repo, head = scratch
    _landed_at(store, repo, head, "landed", tmp_path)
    newer = commit(repo, "a newer epr source\n")
    git(repo, "update-ref", epr_store.PUBLISHED_REF, newer)
    candidate = ingest(store, repo, newer, fake_epr(tmp_path / "newer", "newer"))

    result = epr_store.follow(root=store, repo=repo)

    assert result["followed"] is True
    assert epr_store._read_link(store, "current") == candidate["cid"]
    assert json.loads((store / "current.json").read_text())["commit"] == newer


def test_follow_leaves_the_head_for_an_unpublished_or_unchanged_checkout(
    scratch: tuple[Path, Path, str], tmp_path: Path
) -> None:
    store, repo, head = scratch
    landed = _landed_at(store, repo, head, "landed", tmp_path)
    local_only = commit(repo, "a local commit nobody landed\n")
    git(repo, "update-ref", epr_store.PUBLISHED_REF, head)

    assert "not published on dev" in epr_store.follow(root=store, repo=repo)["why"]

    git(repo, "update-ref", epr_store.PUBLISHED_REF, local_only)
    (repo / "tracked").write_text("dirty\n", encoding="utf-8")
    assert "dirty" in epr_store.follow(root=store, repo=repo)["why"]
    assert epr_store._read_link(store, "current") == landed["cid"]


def test_follow_does_nothing_when_epr_sources_did_not_change(
    scratch: tuple[Path, Path, str], tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    store, repo, head = scratch
    (repo / "src").mkdir()
    (repo / "src" / "lib.rs").write_text("fn main() {}\n", encoding="utf-8")
    git(repo, "add", "src")
    git(repo, "commit", "-qm", "source")
    base = git(repo, "rev-parse", "HEAD")
    landed = _landed_at(store, repo, base, "landed", tmp_path)
    monkeypatch.setattr(epr_store, "source_paths", lambda _repo: ["src"])
    docs_only = commit(repo, "a docs-only change\n")
    git(repo, "update-ref", epr_store.PUBLISHED_REF, docs_only)

    assert epr_store.follow(root=store, repo=repo)["why"] == "no epr source change since the landed head"
    assert epr_store._read_link(store, "current") == landed["cid"]
