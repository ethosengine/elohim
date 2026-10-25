"""The native and both fallback shims consume exactly these process-table fixtures."""
import json
import os
from pathlib import Path
import sys
from unittest.mock import patch
import subprocess

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import session

CASES = Path(__file__).resolve().parents[4] / 'genesis/agentic/fixtures/session-cases.json'


def test_shared_session_cases(tmp_path):
    for i, case in enumerate(json.loads(CASES.read_text())):
        root = tmp_path / str(i)
        for pid, parent, comm, leader, tty, start, environ in case['processes']:
            path = root / str(pid)
            path.mkdir(parents=True)
            if environ is not None:
                (path / 'environ').write_bytes(b''.join(f'{k}={v}\0'.encode() for k, v in environ.items()))
            fields = ['0'] * 20
            fields[0] = 'S'
            for index, value in [(1, parent), (3, leader), (4, tty), (19, start)]:
                fields[index] = str(value)
            (path / 'stat').write_text(f'{pid} ({comm}) {" ".join(fields)}')
        assert session.resolve_fallback(environ=case['env'], proc_root=root,
            start_pid=case['startPid'], registry=case['registry']) == case['expected'], case['name']


def test_native_answer_and_failure_fallback():
    expected = session._identity('declared', 'native', source='declared')
    with patch.object(subprocess, 'run', return_value=subprocess.CompletedProcess([], 0, json.dumps(expected))) as run:
        assert session.resolve(environ={'EPR_BIN': '/candidate'}) == expected
        assert run.call_args.args[0] == ['/candidate', 'session', 'id', '--json']
        assert run.call_args.kwargs['timeout'] == 2
        assert run.call_args.kwargs['env']['EPR_SESSION_RESOLVING'] == '1'
    for error in [FileNotFoundError(), subprocess.TimeoutExpired('epr', 2), subprocess.CalledProcessError(2, 'epr')]:
        with patch.object(subprocess, 'run', side_effect=error):
            assert session.resolve(environ={'ELOHIM_SESSION_ID': 'local', 'ELOHIM_SESSION_OWNER_PID': str(os.getpid())})['session'] == 'local'


def test_recursion_guard_and_bad_registry(tmp_path):
    with patch.object(subprocess, 'run', side_effect=AssertionError('recursive spawn')):
        assert session.resolve(environ={'EPR_SESSION_RESOLVING': '1', 'ELOHIM_SESSION_ID': 'guard', 'ELOHIM_SESSION_OWNER_PID': str(os.getpid())})['session'] == 'guard'
    for body in ['invalid', '{}', '{"runtimes":null}', '{"runtimes":[{}]}']:
        path = tmp_path / 'registry.json'
        path.write_text(body)
        assert session.load_registry({'EPR_AGENT_RUNTIMES': str(path)}) == {'runtimes': []}
