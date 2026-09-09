#!/usr/bin/env bash
# A late joiner's storage must outlive the task process group that launched it.
# Exercise the actual helper with an owned executable, data root and session.
set -eu
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
python3 - "$here/../hc-mesh.sh" <<'PY'
import os
import pathlib
import signal
import subprocess
import sys
import tempfile
import time

def identity(pid):
    fields = pathlib.Path(f'/proc/{pid}/stat').read_text().rsplit(') ', 1)[1].split()
    return fields[19], fields[0]

def alive(pid, started):
    try:
        current, state = identity(pid)
        return current == started and state != 'Z'
    except FileNotFoundError:
        return False

with tempfile.TemporaryDirectory(prefix='mesh-storage-lifetime-') as directory:
    root = pathlib.Path(directory)
    fake = root / 'owned-storage'
    fake.write_text('''#!/usr/bin/env python3
import os, pathlib, time
pathlib.Path(os.environ['STORAGE_DIR']).joinpath('fixture.pid').write_text(str(os.getpid()))
while True: time.sleep(1)
''')
    fake.chmod(0o755)
    runner = root / 'caller.sh'
    runner.write_text('''#!/usr/bin/env bash
set -eu
export MESH_DIR="$1/mesh" MESH_PEERS=unit
source "$2" >/dev/null
STORAGE_BIN="$1/owned-storage"
LOCAL_DEV_DIR="$1/local-dev"
mkdir -p "$LOGDIR" "$LOCAL_DEV_DIR"
curl() { return 1; }
sandbox_agent_key() { echo fixture-agent; }
runtime_config_path_for() { echo "$MESH_DIR/runtime-config.toml"; }
start_storage_peer unit 0
touch "$MESH_DIR/launcher-ready"
while :; do sleep 1; done
''')
    child = None
    child_start = None
    caller = subprocess.Popen(['setsid', 'bash', str(runner), str(root), sys.argv[1]],
                              stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
    caller_start, _ = identity(caller.pid)
    try:
        deadline = time.monotonic() + 10
        while not (root / 'mesh/launcher-ready').exists():
            if caller.poll() is not None:
                raise AssertionError(caller.stderr.read().decode())
            assert time.monotonic() < deadline, 'owned storage launch timed out'
            time.sleep(0.05)
        child = int((root / 'mesh/unit/fixture.pid').read_text())
        child_start, _ = identity(child)
        args = pathlib.Path(f'/proc/{child}/cmdline').read_bytes().split(b'\0')
        assert str(fake).encode() in args, 'storage PID does not name our executable'
        assert pathlib.Path(f'/proc/{child}/exe').resolve() == pathlib.Path(sys.executable).resolve()
        assert alive(child, child_start), 'storage was not alive before caller teardown'
        # Never signal the test runner's group or any unverified caller PID.
        assert identity(caller.pid)[0] == caller_start
        assert os.getpgid(caller.pid) == caller.pid != os.getpgrp()
        assert os.getsid(caller.pid) == caller.pid
        assert str(runner).encode() in pathlib.Path(f'/proc/{caller.pid}/cmdline').read_bytes().split(b'\0')
        os.killpg(caller.pid, signal.SIGTERM)
        caller.wait(timeout=5)
        time.sleep(0.15)
        assert alive(child, child_start), 'storage died with its caller process group'
        assert os.getpgid(child) != caller.pid, 'storage retained the caller process group'
        print('ok late-join storage survives teardown of its exact owned caller group')
    finally:
        if caller.poll() is None and alive(caller.pid, caller_start):
            caller.terminate()
            caller.wait(timeout=5)
        if child is not None and alive(child, child_start):
            os.kill(child, signal.SIGTERM)
PY
