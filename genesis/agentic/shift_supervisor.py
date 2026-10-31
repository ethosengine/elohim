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
import traceback
import uuid

from shift_ops import (execute, next_operation, op_dir, prepare, receipts, require_delivery_evidence)
from shift_evidence import binary_observation
from shift_lease import host_berth, resources
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


def reply(connection, payload):
    """A departed local peer loses its reply, not the guardian-owned operation."""
    try:
        connection.sendall(payload)
        return True
    except (ConnectionError, socket.timeout):
        return False


def controller_status(handle):
    """Observe only the saved incarnation; exit/reaping can race the liveness check."""
    if not alive(handle):
        return None
    try:
        record = Path(f"/proc/{handle['pid']}/stat").read_text()
    except (FileNotFoundError, ProcessLookupError):
        return None
    fields = record.rsplit(')', 1)[-1].split()
    if len(fields) < 20:
        raise Refused('malformed controller process status')
    if fields[19] != handle['start'] or fields[0] in {'Z', 'X'}:
        return None
    return fields[0]


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
    selected = resources(resource)
    if not selected:
        return
    session = f"shift-{contract['id']}"
    def release_all(owned):
        failures = []
        for name in reversed(owned):
            try:
                host_berth(contract, 'release', name, '--session', session)
            except (Refused, OSError, subprocess.SubprocessError) as error:
                failures.append(f'{name}: {error}')
        if failures:
            raise Refused('resource release unresolved: ' + '; '.join(failures))
    if release:
        release_all(selected)
    else:
        # Host guardian/readiness owner: one mooring, never namespace-local PIDs.
        host_berth(contract, 'moor', '--session', session, '--runtime', 'shift', '--pid', str(os.getpid()),
                   '--task', contract['id'])
        attempted = []
        try:
            for name in selected:
                attempted.append(name)  # A lost claim response may already have acquired it.
                argv = ['claim', name, '--session', session]
                if name == 'mesh':
                    argv += ['--class', 'verify', '--ttl', '1800']
                host_berth(contract, *argv)
        except BaseException as error:
            try:
                release_all(attempted)  # Releases only our session; another holder is untouched.
            except Refused as cleanup:
                raise Refused(f'{error}; rollback failed: {cleanup}') from error
            raise


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
    held_resources = []
    lease_started = None
    lease_touched = None
    controller_handle = None
    stopped_since = None
    last_heartbeat = None
    launched = 0
    handshake_started = time.monotonic()
    guardian_handle = incarnation(os.getpid())
    termination_requested = False
    def kill_children():
        for process in (worker, wrapper):
            try:
                if process is not None and process.poll() is None:
                    process.kill()
            except OSError:
                # An owned child may exit between poll and kill. Reaping belongs
                # to finally, never a signal handler interrupting that reap.
                pass
    def terminate(_sig, _frame):
        nonlocal termination_requested
        termination_requested = True
        # Stop contained execution immediately even while a host call is blocked.
        # Never raise asynchronously through refusal persistence or cleanup.
        kill_children()
    def check_termination():
        if termination_requested:
            kill_children()  # also covers a signal during spawn, before assignment
            raise Refused('guardian termination requested')
    signal.signal(signal.SIGTERM, terminate)
    signal.signal(signal.SIGINT, terminate)
    signal.signal(signal.SIGALRM, terminate)
    try:
        checkpoint(directory, guardian=guardian_handle, phase='handshake')
        signal.setitimer(signal.ITIMER_REAL, authority(directory, contract, state))
        while True:
            check_termination()
            contract, state = load(directory)
            remaining = authority(directory, contract, state)
            if wrapper is None or wrapper.poll() is not None:
                launched += 1
                if launched > 3:
                    raise Refused('contained supervisor restart budget exhausted')
                check_termination()
                with open(directory / 'ark.log', 'ab', buffering=0) as output:
                    wrapper = spawn_owned(contained([
                        contract['ark'], 'run', '--manifest', str(directory / 'ark-manifest.json'),
                        '--berth', str(directory / 'ark-berth.json'),
                    ]), stdout=output, stderr=subprocess.STDOUT, stdin=subprocess.DEVNULL)
                check_termination()
                controller_handle, last_heartbeat = None, None
                stopped_since = None
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
                        reply(connection, b'refused\n')
                        continue
                    if message.get('kind') == 'berth':
                        if (message.get('token') != token or uid != os.getuid() or worker is None
                                or worker.poll() is not None or not descendant(pid, worker.pid)):
                            response = {'exit': 3, 'stdout': '', 'stderr': 'berth: shift delegation refused\n'}
                        else:
                            from shift_lease import handle_request
                            response = handle_request(contract, held_resources, lease_started, message.get('argv'))
                        reply(connection, json.dumps(response).encode() + b'\n')
                        continue
                    if message != {'token': token} or uid != os.getuid() or not descendant(pid, wrapper.pid):
                        reply(connection, b'refused\n')
                        continue
                    candidate = incarnation(pid)
                    checkpoint(directory, controller=candidate, heartbeat=time.time(), phase='running')
                    if reply(connection, b'ack\n'):
                        if candidate != controller_handle:
                            stopped_since = None
                        controller_handle = candidate
                        last_heartbeat = time.monotonic()
                        tick = True
                        atomic(directory / 'guardian-ready.json', {'guardian': guardian_handle, 'controller': controller_handle})
            except socket.timeout:
                pass
            check_termination()
            if controller_handle is None and time.monotonic() - handshake_started > 25:
                raise Refused('contained controller never completed launch handshake')
            status = controller_status(controller_handle)
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
                if held_resources:
                    lease(contract, held_resources, release=True)
                    held_resources = []
                worker = None
                tick = bool(last_heartbeat and time.monotonic() - last_heartbeat < 2 * HEARTBEAT)
            if worker is not None and held_resources:
                if 'mesh' in held_resources and time.monotonic() - lease_started >= 1800:
                    raise Refused('household verify lease expired; scope the lane down or use just measure')
                if time.monotonic() - lease_touched >= HEARTBEAT:
                    # Touch the host guardian's mooring; never reset the lease's original TTL.
                    host_berth(contract, 'touch', '--session', f"shift-{contract['id']}", '--pid', str(os.getpid()))
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
            check_termination()
            if step is None:
                if contract['mode'] == 'delivery':
                    require_delivery_evidence(directory, contract, revision(contract['worktree']), published=True)
                checkpoint(directory, phase='complete' if contract['mode'] == 'delivery' else 'rehearsal-complete', pending=None)
                return 0
            if remaining < 1:
                raise Refused('no remaining execution time')
            prepare(directory, contract, state, step)
            checkpoint(directory, pending=step['id'])
            selected = resources(step.get('resource'))
            lease_started = time.monotonic()  # Acquiring another resource never extends an earlier TTL.
            lease(contract, selected)
            held_resources = selected  # Expose only the fully acquired set to authenticated workers.
            lease_touched = time.monotonic()
            check_termination()
            active_step = step
            if step['kind'] == 'household':
                binary_observation(directory, contract, step, 'before')
            argv = [sys.executable, str(Path(__file__).with_name('shift.py')), '_execute', str(directory), step['id']]
            # Household lifecycle is explicitly host-owned; its intended services outlive this command.
            argv = ['setpriv', '--pdeathsig', 'KILL', *argv] if step['kind'] == 'lifecycle' else contained(argv)
            with open(directory / 'worker-errors.log', 'ab') as errors:
                check_termination()
                worker = spawn_owned(argv, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                                     stderr=errors, env={**os.environ, **contract.get('workload_env', {}),
                                                        'BERTH_SESSION': f"shift-{contract['id']}",
                                                        'ELOHIM_SHIFT_BERTH_ADDRESS': advertised,
                                                        'ELOHIM_SHIFT_BERTH_TOKEN': token})
            check_termination()
            checkpoint(directory, worker=incarnation(worker.pid))
    except (Refused, OSError, ValueError, KeyError) as error:
        # Refusal evidence is append-only and does not repair or recreate corrupt authority/state.
        atomic(directory / f'refusal-{time.time_ns()}.json', {
            'reason': str(error), 'at': time.time(), 'exception_type': type(error).__name__,
            'traceback': traceback.format_exc(),
        })
        return 3
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
        kill_children()
        for process in (worker, wrapper):
            if process is not None:
                # These are Popen handles owned by this guardian, never namespace-local PIDs.
                process.wait()
        if held_resources:
            lease(contract, held_resources, release=True)
        server.close()
