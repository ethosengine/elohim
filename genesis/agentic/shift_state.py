"""Private shift execution projection. Native flow records alone establish acceptance.

The objective and contract are immutable; checkpoints may be replaced only under
the state lock. A lost checkpoint cannot confer authority to start over.
"""
import contextlib
import datetime as dt
import fcntl
import hashlib
import json
import math
import os
from pathlib import Path
import re
import subprocess
import tempfile
import time


class Refused(RuntimeError):
    pass


def ci_policy(contract):
    policy = contract.get('ci_policy', 'success')
    if policy not in ('success', 'inspect'):
        raise Refused('ci_policy must be success or inspect')
    return policy


def digest(path):
    value = hashlib.sha256()
    with open(path, 'rb') as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b''):
            value.update(chunk)
    return value.hexdigest()


def read(path):
    try:
        return json.loads(Path(path).read_text())
    except (OSError, ValueError) as error:
        raise Refused(f"missing/corrupt state {path}: {error}") from error


def sync_dir(path):
    fd = os.open(path, os.O_DIRECTORY | os.O_RDONLY)
    try:
        os.fsync(fd)
    finally:
        os.close(fd)


def atomic(path, value, *, exclusive=False):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
    data = (json.dumps(value, indent=2, sort_keys=True) + "\n").encode()
    fd, temporary = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
    try:
        with os.fdopen(fd, "wb") as stream:
            stream.write(data)
            stream.flush()
            os.fsync(stream.fileno())
        if exclusive:
            os.link(temporary, path)  # cannot overwrite an existing authority or receipt
            os.unlink(temporary)
        else:
            os.replace(temporary, path)
        sync_dir(path.parent)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)


@contextlib.contextmanager
def lock(path, *, blocking=True):
    fd = os.open(path, os.O_CREAT | os.O_RDWR | os.O_NOFOLLOW, 0o600)
    try:
        try:
            fcntl.flock(fd, fcntl.LOCK_EX | (0 if blocking else fcntl.LOCK_NB))
        except BlockingIOError as error:
            raise Refused(f"execution already owned: {path}") from error
        yield fd
    finally:
        os.close(fd)


def finite_number(value):
    try:
        return type(value) in (int, float) and math.isfinite(value)
    except OverflowError:
        return False


def timestamp(value):
    try:
        parsed = dt.datetime.fromisoformat(value.replace("Z", "+00:00"))
        if parsed.tzinfo is None:
            raise ValueError("timezone required")
        result = parsed.timestamp()
        if not finite_number(result):
            raise ValueError('finite timestamp required')
        return result
    except (ValueError, TypeError, AttributeError, OverflowError, OSError) as error:
        raise Refused(f"invalid absolute time: {value}") from error


def boot_id():
    return Path('/proc/sys/kernel/random/boot_id').read_text().strip()


def incarnation(pid):
    """Only called in the host namespace. PID numbers alone are never identity."""
    try:
        fields = Path(f'/proc/{int(pid)}/stat').read_text().rsplit(')', 1)[1].split()
        return {'pid': int(pid), 'start': fields[19], 'boot': boot_id()}
    except (OSError, ValueError, IndexError):
        return None


def alive(handle):
    if not handle or incarnation(handle['pid']) != handle:
        return False
    try:
        phase = Path(f"/proc/{handle['pid']}/stat").read_text().rsplit(')', 1)[1].split()[0]
        return phase not in {'Z', 'X'}
    except (OSError, IndexError):
        return False


def signal_handle(handle, sig):
    if not alive(handle):
        return False
    # pidfd closes the check/kill PID-reuse race.
    import signal
    try:
        fd = os.pidfd_open(handle['pid'])
        try:
            if not alive(handle):
                return False
            signal.pidfd_send_signal(fd, sig)
        finally:
            os.close(fd)
        return True
    except ProcessLookupError:
        return False


def run(argv, cwd, *, timeout=30):
    result = subprocess.run(argv, cwd=cwd, text=True, capture_output=True, timeout=timeout)
    if result.returncode:
        raise Refused(f"{argv[0]} exited {result.returncode}: {result.stderr[-1500:]}")
    return result.stdout.strip()


def revision(root):
    return run(['git', 'rev-parse', 'HEAD'], root)


def under(root, relative):
    p = (Path(root) / relative).resolve()
    if not p.is_relative_to(Path(root).resolve()):
        raise Refused(f"path escapes owning worktree: {relative}")
    return p


def home(root, name):
    if not re.fullmatch(r'[a-z0-9][a-z0-9-]{2,80}', name):
        raise Refused('invalid shift id')
    return under(root, f'.claude/shifts/{name}')


def check_clock(contract, state, now=None):
    now = time.time() if now is None else now
    if not finite_number(state.get('last_wall')):
        raise Refused('invalid or missing durable clock floor')
    if not finite_number(now):
        raise Refused('invalid current wall clock')
    started, deadline = timestamp(contract.get('started_at')), timestamp(contract.get('deadline'))
    if deadline <= started:
        raise Refused('absolute deadline must follow original start')
    if now < state['last_wall'] or now < started:
        raise Refused('clock regression; new work refused')
    remaining = deadline - now
    if remaining <= 0:
        raise Refused('absolute deadline expired; downtime never extends it')
    return remaining


def authority(directory, contract, state, *, publication=False):
    if (directory / 'stop.json').exists():
        raise Refused('explicit stop marker; automatic recovery cannot clear it')
    if digest(directory / 'contract.json') != state['contract_sha256']:
        raise Refused('frozen execution contract changed')
    if digest(directory / 'objective.json') != contract['objective_sha256']:
        raise Refused('frozen objective changed')
    from shift_inputs import verify_inputs
    verify_inputs(contract)
    path = under(contract['worktree'], contract['authority']['path'])
    if digest(path) != contract['authority']['sha256']:
        raise Refused('original authorization missing or changed')
    grant = read(path)
    revocation = read(under(contract['worktree'], contract['authority']['revocation']))
    if revocation.get('revoked') is not False or grant.get('build') is not True:
        raise Refused('build authorization revoked or absent')
    if publication and grant.get('landing') is not True:
        raise Refused('separate landing/push authorization absent')
    if grant.get('deadline') != contract['deadline']:
        raise Refused('authority deadline differs from frozen contract')
    return check_clock(contract, state)


def load(directory):
    contract, state = read(directory / 'contract.json'), read(directory / 'checkpoint.json')
    if state.get('version') != 1 or contract.get('version') != 1:
        raise Refused('legacy journal has no complete execution contract; inspection only')
    if Path(contract['worktree']).resolve() != directory.parents[2].resolve():
        raise Refused('execution contract belongs to another worktree')
    authority(directory, contract, state)
    return contract, state


def checkpoint(directory, **updates):
    with lock(directory / 'state.lock'):
        contract, state = load(directory)
        now = time.time()
        check_clock(contract, state, now)
        state.update(updates)
        state['last_wall'] = now
        atomic(directory / 'checkpoint.json', state)
        return contract, state


def native_context(contract):
    raw = run(['epr', 'flow', 'context', contract['scope_cid'], '--json'], contract['worktree'])
    context = json.loads(raw)
    reconciliation = context['reconciliation']
    if reconciliation['scope_cid'] != contract['scope_cid']:
        raise Refused('native scope differs from original intent scope')
    assertions = [a for a in reconciliation['assertions'] if a['intent'] == contract['intent']]
    if len(assertions) != 1:
        raise Refused('exact native intent missing or ambiguous')
    promises = [c for c in assertions[0]['commitments'] if c['commitment'] == contract['commitment']]
    if len(promises) != 1:
        raise Refused('exact native commitment missing or ambiguous')
    return context, promises[0]


def require_acceptance(contract, delivered):
    """Ask the native evaluator first; inspect its current pinned report for revision binding."""
    context, promise = native_context(contract)
    if promise['acceptance'] != 'accepted' or promise['issues']:
        raise Refused(f"current native acceptance is {promise['acceptance']}: {promise['issues']}")
    ids = set(promise['acceptance_records'])
    decisions = {}
    for line in under(contract['worktree'], '.eprfs/status/flows.jsonl').read_text().splitlines():
        row = json.loads(line)
        if row['cid'] in ids:
            decisions[row['cid']] = row['record'].get('classifiedAs', [])
    superseded = {slot.removeprefix('supersedes:') for slots in decisions.values()
                  for slot in slots if slot.startswith('supersedes:')}
    leaves = [slots for cid, slots in decisions.items() if cid not in superseded]
    if len(leaves) != 1 or 'verdict:approved' not in leaves[0]:
        raise Refused('current native acceptance leaf is missing or ambiguous')
    paths = [slot.removeprefix('report-path:') for slot in leaves[0] if slot.startswith('report-path:')]
    if len(paths) != 1:
        raise Refused('current acceptance has no exact pinned report')
    report = read(under(contract['worktree'], paths[0]))
    if report['revision'] != delivered or report['intent'] != contract['intent']:
        raise Refused('accepted evidence does not bind the exact delivered revision and intent')
    return context
