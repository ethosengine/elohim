"""Delegate namespace-local berth checks to the host guardian's existing lease.

The guardian authenticates the socket peer before calling handle_request. This
module never claims, renews, releases, or changes that lease: a nested command
can only use the resource the guardian currently holds for this operation.
"""
import datetime as dt
import fcntl
import json
import math
import os
from pathlib import Path
import socket
import subprocess
import sys
import time


PROXY_ENV = ('ELOHIM_SHIFT_BERTH_ADDRESS', 'ELOHIM_SHIFT_BERTH_TOKEN')
MAX_MESSAGE = 16384
OPTIONS = {
    'claim': {'--session', '--class', '--ttl', '--note'},
    'release': {'--session'},
    'owner-check': {'--session', '--action', '--note'},
    'who': {'--session'},
}


def refused(reason):
    return {'exit': 3, 'stdout': '', 'stderr': f'berth: shift lease REFUSED — {reason}\n'}


def host_environment():
    """Host inspection must not recursively call the contained-worker proxy."""
    return {key: value for key, value in os.environ.items() if key not in PROXY_ENV}


def proxy_request(argv, environ=None):
    """Fail closed if delegation is incomplete, unreachable, or malformed."""
    env = os.environ if environ is None else environ
    address, token = (env.get(name) for name in PROXY_ENV)
    if not address or not token:
        return refused('guardian address and token are both required')
    if not address.startswith('@') or len(address.encode()) > 107 or '\0' in address:
        return refused('invalid guardian socket address')
    arguments = list(argv)
    if '--session' not in arguments:
        session = env.get('BERTH_SESSION')
        if not session:
            return refused('the delegated shift session is missing')
        arguments += ['--session', session]
    request = json.dumps({'kind': 'berth', 'token': token, 'argv': arguments}).encode() + b'\n'
    if len(request) > MAX_MESSAGE:
        return refused('request exceeds the guardian message limit')
    try:
        with socket.socket(socket.AF_UNIX) as stream:
            stream.settimeout(5)
            stream.connect('\0' + address[1:])
            stream.sendall(request)
            data = bytearray()
            while b'\n' not in data:
                chunk = stream.recv(min(4096, MAX_MESSAGE + 1 - len(data)))
                if not chunk:
                    raise ValueError('guardian closed before its response')
                data.extend(chunk)
                if len(data) > MAX_MESSAGE:
                    raise ValueError('guardian response exceeds the message limit')
            result = json.loads(data)
            if (not isinstance(result, dict) or type(result.get('exit')) is not int
                    or result['exit'] not in (0, 3)
                    or not isinstance(result.get('stdout'), str)
                    or not isinstance(result.get('stderr'), str)):
                raise ValueError('invalid guardian response')
            return result
    except (OSError, ValueError, TypeError) as error:
        return refused(f'guardian unavailable: {error}')


def parse_request(argv, resource, session):
    if (not isinstance(argv, list) or not 2 <= len(argv) <= 64
            or not all(isinstance(arg, str) for arg in argv)):
        raise ValueError('invalid berth arguments')
    command, requested, *tail = argv
    if command not in OPTIONS or requested != resource:
        raise ValueError('only checks on the currently held resource are delegated')
    options = {}
    while tail:
        option = tail.pop(0)
        if option not in OPTIONS[command] or option in options or not tail:
            raise ValueError(f'unsupported or repeated option: {option}')
        options[option] = tail.pop(0)
    if options.get('--session') != session:
        raise ValueError('request does not name the guardian shift session')
    return command, options


def handle_request(contract, held_resource, lease_started, argv):
    """Return a CLI result after host peer authentication by shift_supervisor.

lease_started is the guardian's monotonic timestamp, never a namespace PID or a
timestamp supplied by the requester. A nested claim reports "renewed" so the
existing just wrappers retain the outer lease instead of releasing it.
"""
    try:
        if held_resource not in {'cargo', 'mesh', 'disk-heavy'}:
            raise ValueError('the guardian holds no delegated resource')
        session = f"shift-{contract['id']}"
        command, options = parse_request(argv, held_resource, session)
        elapsed = time.monotonic() - lease_started
        if not math.isfinite(elapsed) or elapsed < 0:
            raise ValueError('invalid guardian lease clock')
        deadline = dt.datetime.fromisoformat(contract['deadline'].replace('Z', '+00:00'))
        if deadline.tzinfo is None or deadline.timestamp() <= time.time():
            raise ValueError('original shift deadline expired or invalid')
        env = host_environment()
        store = Path(env.get('BERTH_DIR') or Path(env.get('CLAUDE_CONFIG_DIR', '/projects/.claude-config')) / 'berth')
        binary = Path(contract['worktree']) / 'genesis/agentic/bin/berth'
        # Read under berth's existing lock. Missing stores and concurrent writers
        # refuse; neither a new mooring nor an unleased fallback is permissible.
        with open(store / 'berth.lock', 'rb') as ownership:
            fcntl.flock(ownership, fcntl.LOCK_SH | fcntl.LOCK_NB)
            result = subprocess.run([sys.executable, str(binary), 'who', held_resource],
                                    cwd=contract['worktree'], env=env, text=True,
                                    capture_output=True, timeout=5)
            if result.returncode:
                raise ValueError('host berth inspection failed')
            current = json.loads(result.stdout)
            holders = [current] if isinstance(current, dict) else current or []
            mine = [row for row in holders if row.get('holder') == session]
            if len(mine) != 1:
                raise ValueError('the guardian no longer owns a live host lease')
            lease = mine[0]
            mooring = json.loads((store / 'moorings' / f'{session}.json').read_text())
            own_start = int(Path('/proc/self/stat').read_text().rsplit(')', 1)[1].split()[19])
            if (mooring.get('session') != session or mooring.get('pid') != os.getpid()
                    or mooring.get('pid_start') != own_start):
                raise ValueError('host mooring does not name this guardian incarnation')
            ttl = lease.get('ttl_s')
            if ttl is not None and (not math.isfinite(ttl) or ttl <= 0 or elapsed >= ttl):
                raise ValueError('the original host lease expired')
            if '--class' in options and options['--class'] != lease.get('class'):
                raise ValueError('nested claims cannot change the host lease class')
            if '--ttl' in options:
                requested = int(options['--ttl'])
                if ttl is None or requested <= 0 or requested > ttl:
                    raise ValueError('nested claims cannot extend the host lease TTL')
            if command == 'owner-check' and len(holders) != 1:
                raise ValueError('another host session also holds this resource')
            if command == 'who':
                output = json.dumps(lease) + '\n'
            else:
                verb = {'claim': 'renewed', 'release': 'retained', 'owner-check': 'owned'}[command]
                output = f'{held_resource}: {verb} by {session} (guardian lease unchanged)\n'
            return {'exit': 0, 'stdout': output, 'stderr': ''}
    except (OSError, ValueError, TypeError, KeyError, AttributeError, subprocess.SubprocessError) as error:
        return refused(str(error))
