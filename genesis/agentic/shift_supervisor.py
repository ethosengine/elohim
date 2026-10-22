"""External deadline guardian; ark restarts the contained controller.

The guardian owns operations so a controller crash cannot lose their receipts.
Every agent has a PID namespace, including daemonized descendants. setpriv and
unshare propagate guardian death to namespace init; namespace death reaps all.
"""
import json
import ctypes
import os
from pathlib import Path
import signal
import socket
import struct
import subprocess
import sys
import time
import uuid

from shift_ops import (execute, next_operation, op_dir, prepare, receipts, require_delivery_evidence)
from shift_evidence import binary_observation
from shift_state import (Refused, alive, atomic, authority, checkpoint, digest,
                         incarnation, load, lock, native_context, read, revision,
                         run, signal_handle)


HEARTBEAT = 15
STOPPED_RECOVERY = 60


def read_message(connection):
    incoming = bytearray()
    while b'\n' not in incoming:
        chunk = connection.recv(4096)
        if not chunk or len(incoming) + len(chunk) > 16384:
            raise ValueError('guardian message truncated or oversized')
        incoming.extend(chunk)
    message = json.loads(incoming)
    if not isinstance(message, dict):
        raise ValueError('guardian message must be an object')
    return message


def contained(argv):
    return ['setpriv', '--pdeathsig', 'KILL', 'unshare', '--user', '--map-root-user',
            '--pid', '--fork', '--kill-child=KILL', '--mount-proc',
            sys.executable, str(Path(__file__).with_name('shift.py')), '_init', *argv]


def spawn_owned(argv, **kwargs):
    parent = os.getpid()
    def bind_parent():
        # Close fork->exec->setpriv's orphan race before the child can execute any work.
        if ctypes.CDLL(None).prctl(1, signal.SIGKILL, 0, 0, 0) != 0 or os.getppid() != parent:
            os._exit(125)
    return subprocess.Popen(argv, preexec_fn=bind_parent, **kwargs)


def namespace_init(argv):
    # PID 1 must explicitly handle TERM; otherwise Linux ignores its default disposition.
    child = subprocess.Popen(argv, start_new_session=True)
    def terminate(_sig, _frame):
        try:
            os.killpg(child.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        raise SystemExit(128 + signal.SIGTERM)
    signal.signal(signal.SIGTERM, terminate)
    signal.signal(signal.SIGINT, terminate)
    return child.wait()  # exiting PID 1 also kills/reaps double-forked descendants


def descendant(pid, parent):
    for _ in range(128):
        if pid == parent:
            return True
        if pid <= 1:
            return False
        try:
            fields = Path(f'/proc/{pid}/stat').read_text().rsplit(')', 1)[1].split()
            pid = int(fields[1])
        except (OSError, ValueError, IndexError):
            return False
    return False


def controller(directory, address, token):
    while True:
        with socket.socket(socket.AF_UNIX) as stream:
            stream.settimeout(10)
            stream.connect('\0' + address[1:] if address.startswith('@') else address)
            stream.sendall(json.dumps({'token': token}).encode() + b'\n')
            reply = stream.recv(256)
            if reply != b'ack\n':
                raise Refused('guardian/controller handshake refused')
        print('SHIFT_CONTROLLER_READY', flush=True)
        time.sleep(HEARTBEAT)


def ark_declarations(directory, contract, address, token):
    interpreter = Path(sys.executable).resolve()
    manifest = {
        'schema': 1, 'kind': 'runtime-manifest', 'reach': 'trusted',
        'processes': [{
            'name': 'controller', 'kind': 'native', 'env_scrub': False,
            'artifact': {'pinned': {'sha256': digest(interpreter)}},
            'argv': ['{artifact}', str(Path(__file__).with_name('shift.py')), '_controller',
                     str(directory), address, token],
            'readiness': [{'stdout_line': {'contains': 'SHIFT_CONTROLLER_READY', 'patience_ms': 15000}}],
            'policy': {'restart': 'permanent', 'shutdown': {'signal': 15, 'grace_ms': 1000},
                       'intensity': {'max_deaths': 5, 'window_s': 300},
                       'same_cause_limit': 3, 'backoff': {'min_s': 1, 'max_s': 15, 'steps': 4}},
        }],
    }
    atomic(directory / 'ark-manifest.json', manifest)
    cid = run([contract['ark'], 'manifest', 'cid', '--manifest', str(directory / 'ark-manifest.json')],
              contract['worktree'])
    atomic(directory / 'ark-berth.json', {
        'manifest': cid, 'data_root': str(directory / 'ark'),
        'artifacts': {'controller': str(interpreter)},
    })


def lease(contract, resource, release=False):
    if resource is None:
        return
    if resource not in {'cargo', 'mesh', 'disk-heavy'}:
        raise Refused('unknown resource lease')
    binary = str(Path(contract['worktree']) / 'genesis/agentic/bin/berth')
    session = f"shift-{contract['id']}"
    if release:
        run([binary, 'release', resource, '--session', session], contract['worktree'])
    else:
        # This function runs ONLY in the host guardian: --pid is a host process handle.
        run([binary, 'moor', '--session', session, '--runtime', 'shift', '--pid', str(os.getpid()),
             '--task', contract['id']], contract['worktree'])
        argv = [binary, 'claim', resource, '--session', session]
        if resource == 'mesh':
            argv += ['--class', 'verify', '--ttl', '1800']
        run(argv, contract['worktree'])


def guardian(directory):
    with lock(directory / 'execution.lock', blocking=False):
        return _guard(directory)


def _guard(directory):
    contract, state = load(directory)
    if alive(state.get('worker')):
        raise Refused('previous worker incarnation is still live; execution ownership unresolved')
    # Abstract sockets avoid AF_UNIX's 108-byte pathname limit in retained worktrees.
    token = str(uuid.uuid4())
    address = '\0elohim-shift-' + token
    # NUL cannot travel in argv. The controller adds it from the @ notation.
    advertised = '@' + address[1:]
    server = socket.socket(socket.AF_UNIX)
    server.bind(address)
    server.listen(4)
    server.settimeout(0.2)
    ark_declarations(directory, contract, advertised, token)
    wrapper = None
    worker = None
    active_step = None
    held_resource = None
    lease_started = None
    lease_touched = None
    controller_handle = None
    stopped_since = None
    last_heartbeat = None
    launched = 0
    handshake_started = time.monotonic()
    guardian_handle = incarnation(os.getpid())
    checkpoint(directory, guardian=guardian_handle, phase='handshake')
    def terminate(_sig, _frame):
        raise Refused('guardian termination requested')
    signal.signal(signal.SIGTERM, terminate)
    signal.signal(signal.SIGINT, terminate)
    signal.signal(signal.SIGALRM, terminate)
    signal.setitimer(signal.ITIMER_REAL, authority(directory, contract, state))
    try:
        while True:
            contract, state = load(directory)
            remaining = authority(directory, contract, state)
            if wrapper is None or wrapper.poll() is not None:
                launched += 1
                if launched > 3:
                    raise Refused('contained supervisor restart budget exhausted')
                with open(directory / 'ark.log', 'ab', buffering=0) as output:
                    wrapper = spawn_owned(contained([
                        contract['ark'], 'run', '--manifest', str(directory / 'ark-manifest.json'),
                        '--berth', str(directory / 'ark-berth.json'),
                    ]), stdout=output, stderr=subprocess.STDOUT, stdin=subprocess.DEVNULL)
                controller_handle, last_heartbeat = None, None
                handshake_started = time.monotonic()
                checkpoint(directory, wrapper=incarnation(wrapper.pid), phase='handshake')
            tick = False
            try:
                connection, _ = server.accept()
                with connection:
                    connection.settimeout(1)
                    pid, uid, _gid = struct.unpack('3i', connection.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))
                    try:
                        message = read_message(connection)
                    except (OSError, ValueError):
                        # A malformed local client cannot tear down authorized execution.
                        try:
                            connection.sendall(b'refused\n')
                        except OSError:
                            pass
                        continue
                    if message.get('kind') == 'berth':
                        if (message.get('token') != token or uid != os.getuid() or worker is None
                                or worker.poll() is not None or not descendant(pid, worker.pid)):
                            reply = {'exit': 3, 'stdout': '', 'stderr': 'berth: shift delegation refused\n'}
                        else:
                            from shift_lease import handle_request
                            reply = handle_request(contract, held_resource, lease_started, message.get('argv'))
                        connection.sendall(json.dumps(reply).encode() + b'\n')
                        continue
                    if message != {'token': token} or uid != os.getuid() or not descendant(pid, wrapper.pid):
                        connection.sendall(b'refused\n')
                        continue
                    controller_handle = incarnation(pid)
                    last_heartbeat = time.monotonic()
                    checkpoint(directory, controller=controller_handle, heartbeat=time.time(), phase='running')
                    connection.sendall(b'ack\n')
                    tick = True
                    atomic(directory / 'guardian-ready.json', {'guardian': guardian_handle, 'controller': controller_handle})
            except socket.timeout:
                pass
            if controller_handle is None and time.monotonic() - handshake_started > 25:
                raise Refused('contained controller never completed launch handshake')
            if controller_handle and alive(controller_handle):
                status = Path(f"/proc/{controller_handle['pid']}/stat").read_text().rsplit(')', 1)[1].split()[0]
                if status in {'T', 't'}:
                    stopped_since = stopped_since or time.monotonic()
                    if time.monotonic() - stopped_since >= STOPPED_RECOVERY:
                        signal_handle(controller_handle, signal.SIGKILL)
                        stopped_since = None
                else:
                    stopped_since = None
            # Persist wall-clock floor while quiet builds run; transcript silence is not a stall.
            if time.time() - state['last_wall'] >= 1:
                checkpoint(directory)
            if worker and worker.poll() is not None:
                if active_step['kind'] == 'household':
                    binary_observation(directory, contract, active_step, 'after')
                if held_resource:
                    lease(contract, held_resource, release=True)
                    held_resource = None
                worker = None
                tick = bool(last_heartbeat and time.monotonic() - last_heartbeat < 2 * HEARTBEAT)
            if worker is not None and held_resource:
                if held_resource == 'mesh' and time.monotonic() - lease_started >= 1800:
                    raise Refused('household verify lease expired; scope the lane down or use just measure')
                if time.monotonic() - lease_touched >= HEARTBEAT:
                    # Touch the host guardian's mooring; never reset the lease's original TTL.
                    run([str(Path(contract['worktree']) / 'genesis/agentic/bin/berth'), 'touch',
                         '--session', f"shift-{contract['id']}", '--pid', str(os.getpid())], contract['worktree'])
                    lease_touched = time.monotonic()
            if worker is not None or not tick:
                continue
            native_context(contract)  # original intent/commitment must still resolve before every dispatch
            if not alive(controller_handle):
                continue
            results = receipts(directory, contract)
            by_id = {r['operation']: r for r in results}
            for observed in contract['steps']:
                if (observed['kind'] == 'household' and by_id.get(observed['id'], {}).get('exit') == 0
                        and not (op_dir(directory, observed) / 'binary-after.json').exists()):
                    # Reconcile a receipt persisted before guardian death. A new
                    # observation must still name the exact original live process.
                    binary_observation(directory, contract, observed, 'after')
            step = next_operation(directory, contract, state)
            if step is None:
                if contract['mode'] == 'delivery':
                    require_delivery_evidence(directory, contract, revision(contract['worktree']), published=True)
                checkpoint(directory, phase='complete' if contract['mode'] == 'delivery' else 'rehearsal-complete', pending=None)
                return 0
            if remaining < 1:
                raise Refused('no remaining execution time')
            prepare(directory, contract, state, step)
            checkpoint(directory, pending=step['id'])
            held_resource = step.get('resource')
            lease(contract, held_resource)
            lease_started = lease_touched = time.monotonic()
            active_step = step
            if step['kind'] == 'household':
                binary_observation(directory, contract, step, 'before')
            argv = [sys.executable, str(Path(__file__).with_name('shift.py')), '_execute', str(directory), step['id']]
            # Household lifecycle is explicitly host-owned; its intended services outlive this command.
            argv = ['setpriv', '--pdeathsig', 'KILL', *argv] if step['kind'] == 'lifecycle' else contained(argv)
            with open(directory / 'worker-errors.log', 'ab') as errors:
                worker = spawn_owned(argv, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                                     stderr=errors, env={**os.environ, **contract.get('workload_env', {}),
                                                        'BERTH_SESSION': f"shift-{contract['id']}",
                                                        'ELOHIM_SHIFT_BERTH_ADDRESS': advertised,
                                                        'ELOHIM_SHIFT_BERTH_TOKEN': token})
            checkpoint(directory, worker=incarnation(worker.pid))
    except (Refused, OSError, ValueError, KeyError) as error:
        # Refusal evidence is append-only and does not repair or recreate corrupt authority/state.
        atomic(directory / f'refusal-{time.time_ns()}.json', {'reason': str(error), 'at': time.time()})
        return 3
    finally:
        for process in (worker, wrapper):
            if process is not None and process.poll() is None:
                # These are Popen handles owned by this guardian, never namespace-local PIDs.
                process.kill()
                process.wait()
        if held_resource:
            lease(contract, held_resource, release=True)
        server.close()
        signal.setitimer(signal.ITIMER_REAL, 0)
