"""Bounded native transport and private delivery journal for the stasis adapters.

P2P: Private (B), keyed by session/incident and atomic delivery sequence. No DHT
entry, head, zome, HTTP route or sync projection; zero head-plane cost. Native
events.jsonl remains the source for classification, presence and budgets.
delivery.jsonl is an append-only outbox/receipt stream under the SAME journal.lock.
It is deliberately separate: session-event v1 has no live delivery vocabulary,
and a would-have record must never become an executable request.
Retire this transport when the native journal owns delivery and its receipts.
"""

from contextlib import contextmanager
import fcntl
import importlib.util
import json
import os
from pathlib import Path
import re
import signal
import subprocess
import sys
import time

REPO = Path(__file__).resolve().parents[2]
POLICY = '.epr-meta/elohim/policies/dialogue-stasis-policy.json'
PARK = ('parked as stasis `{}`. Do not retry this effect by any tool or spelling. '
        'Continue independent authorized work. Guidance will arrive.')


def mode():
    value = os.environ.get('STASIS_LADDER', 'shadow')
    return value if value in {'off', 'live'} else 'shadow'


def bounded(argv, *, cwd, timeout, payload=None, env=None):
    """The resolver AND its evaluator descend from this killable process group."""
    process = subprocess.Popen(argv, cwd=cwd, env=env, stdin=subprocess.PIPE,
                               stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                               text=True, start_new_session=True)
    try:
        out, _ = process.communicate(payload, timeout=max(.001, timeout))
        return process.returncode, out
    except subprocess.TimeoutExpired:
        return None, ''
    finally:
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        process.communicate()


def native(repo, args, payload=None, timeout=2):
    """Resolve via epr_client in the bounded child, never in the hook process."""
    try:
        code, out = bounded([sys.executable, str(Path(__file__).resolve()),
                             'native', *args], cwd=repo, timeout=timeout,
                            payload=json.dumps(payload) if payload is not None else None)
        value = json.loads(out) if code == 0 else None
        return value if isinstance(value, dict) else None
    except (OSError, ValueError, TypeError):
        return None


def pinned(status):
    return (isinstance(status, dict) and status.get('policyState') == 'pinned-match'
            and isinstance(status.get('policyCid'), str) and bool(status['policyCid']))


def snapshot(repo, root, session, timeout=2):
    """Read the exact policy around native verification; never calculate a second CID."""
    try:
        path = Path(repo) / POLICY
        before = path.read_bytes() if path.exists() else None
        value = native(repo, ['status', '--session', session, '--journal-root', str(root),
                              '--json'], timeout=timeout)
        after = path.read_bytes() if path.exists() else None
        if before != after:
            return None
        if pinned(value) and before is None:
            return None
        if value is not None:
            value = {**value, '_policy': json.loads(before) if before else {}}
        return value
    except (OSError, ValueError):
        return None


def session_status(status, session):
    return next((s for s in (status or {}).get('sessions', [])
                 if s.get('session') == session), {})


def presence(status, session):
    value = session_status(status, session).get('presence', {})
    # Native Presence serializes its basis flat; sensor envelopes nest it.
    return {'disposition': value.get('disposition', 'unknown'),
            'basis': value.get('basis', {
                'lastWitnessedAgeSeconds': value.get('lastWitnessedAgeSeconds'),
                'rulingRef': value.get('rulingRef')})}


@contextmanager
def delivery_lock(directory):
    with (directory / 'journal.lock').open('a') as lock:
        # Hook delivery must not wait behind a coordinator's bounded berth write.
        # The coordinator already owns the lock and uses Delivery directly.
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        yield Delivery(directory)


class Delivery:
    """Caller holds journal.lock. Receipts are claimed before external delivery.

    At most once is intentional: death after claim is unconfirmed delivery, never
    a second alert or Stop block. No acknowledgement creates execution authority.
    """
    def __init__(self, directory):
        self.path = directory / 'delivery.jsonl'
        self.rows = []
        if self.path.exists():
            for raw in self.path.read_text().splitlines():
                self.rows.append(json.loads(raw))  # corruption fails open, never replay

    def add(self, kind, key, **fields):
        if any(r['key'] == key for r in self.rows):
            return None
        row = {'version': 1, 'seq': len(self.rows) + 1, 'kind': kind,
               'key': key, 'at': time.time(), **fields}
        with self.path.open('a') as stream:
            stream.write(json.dumps(row, separators=(',', ':'), allow_nan=False) + '\n')
            stream.flush()
            os.fsync(stream.fileno())
        self.rows.append(row)
        return row

    def pending(self):
        parks = {}
        terminal = {r['incidentId'] for r in self.rows if r['kind'] == 'park-state'
                    and r['state'] in {'answered', 'withdrawn', 'expired'}}
        for row in self.rows:
            if row['kind'] == 'park' and row['incidentId'] not in terminal:
                parks[row['incidentId']] = row
            elif row['kind'] == 'park-detail' and row['incidentId'] in parks:
                parks[row['incidentId']] = {**parks[row['incidentId']], 'ask': row['ask']}
            elif row['kind'] == 'park-state' and row['state'] in {'answered', 'withdrawn', 'expired'}:
                parks.pop(row['incidentId'], None)
        return list(parks.values())

    def news(self, reader):
        consumed = {r['deliverySeq'] for r in self.rows if r['kind'] == 'receipt'
                    and r['reader'] == reader}
        return [r for r in self.rows if r['kind'] in {'guidance', 'PushNotification'}
                and r['seq'] not in consumed and r.get('agentId', 'main') == reader]

    def park(self, iid, cid, agent='main', summary='A step is awaiting steward guidance.',
             ask='Resolve this parked step? Recommendation: keep it parked pending fresh validation.'):
        return self.add('park', f'park:{iid}', incidentId=iid, policyCid=cid,
                        agentId=agent, summary=summary, ask=ask,
                        recommendation='Keep parked until fresh authority and permission checks pass.')


def context(event, message):
    if event == 'UserPromptSubmit':
        return message
    return json.dumps({'hookSpecificOutput': {'hookEventName': event,
                                              'additionalContext': message}})


def question_answered(payload):
    response = payload.get('tool_response')
    return (payload.get('hook_event_name') == 'PostToolUse'
            and payload.get('tool_name') == 'AskUserQuestion' and not payload.get('is_error')
            and isinstance(response, dict) and not response.get('is_error')
            and isinstance(response.get('answers'), dict) and bool(response['answers']))


def live_hook(repo, root, payload, normalized):
    """Deny or advise only. Nothing here can construct a permission grant."""
    if mode() != 'live':
        return None
    event, session = payload['hook_event_name'], payload['session_id']
    directory, reader = root / session, payload.get('agent_id') or 'main'
    started = time.monotonic()
    remaining = lambda: max(.001, 1.9 - (time.monotonic() - started))
    if event == 'PermissionRequest':
        # Native decide must still see missing/unpinned policies and record its inert reason.
        decision = native(repo, ['decide', '--live', '--session', session, '--event', '-',
                                 '--journal-root', str(root)], normalized, remaining())
        if (not decision or decision.get('effect') != 'deny' or
                decision.get('shadow') is not False or decision.get('inert') or
                not isinstance(decision.get('message'), str) or not decision['message'].strip()):
            return None
        status = snapshot(repo, root, session, remaining())
        iid = normalized.get('correlation', {}).get('incidentId')
        if (not pinned(status) or decision.get('policyCid') != status['policyCid'] or
                decision.get('incidentId') != iid or not iid or
                presence(status, session)['disposition'] != 'automated'):
            return None
        message = PARK.format(iid)
        with delivery_lock(directory) as journal:
            journal.park(iid, status['policyCid'], reader)
            journal.add('stall-closed', f'stall-closed:{iid}', incidentId=iid, closedBy='parked',
                        policyCid=status['policyCid'])
        return json.dumps({'hookSpecificOutput': {'hookEventName': 'PermissionRequest',
                           'decision': {'behavior': 'deny', 'message': message}}})
    if event == 'PreToolUse':
        # The normal hot path has no learned forms and needs no native process.
        try:
            if not json.loads((Path(repo) / POLICY).read_text()).get('learnedForms'):
                return None
        except (OSError, ValueError, AttributeError):
            return None
        status = snapshot(repo, root, session, remaining())
        forms = (status or {}).get('_policy', {}).get('learnedForms', [])
        if not pinned(status) or not forms:
            return None
        # The native classifier owns risk/ask precedence over style. Raw input
        # goes through stdin only and is never persisted by this transport.
        inputs = payload.get('tool_input', {})
        classified = native(repo, ['classify', '--corpus', '/dev/stdin'],
                            {'tool': payload.get('tool_name'),
                             'command': inputs.get('command')}, remaining())
        classification = (classified or {}).get('classification', {})
        if classification.get('concern') != 'shape' or classification.get('ruleMatch') != 'none':
            return None
        form = next((f for f in forms if f.get('tool') == payload.get('tool_name')
                     and f.get('shape') == normalized.get('shape')
                     and f.get('effectClass') == classification.get('effect')
                     and f.get('status') == 'active' and f.get('auditVerdict') == 'approved'
                     and f.get('auditRef') and f.get('rulingRef')
                     and isinstance(f.get('name'), str) and isinstance(f.get('legibleForm'), str)), None)
        if not form:
            return None
        return json.dumps({'hookSpecificOutput': {'hookEventName': 'PreToolUse',
                           'permissionDecision': 'deny', 'permissionDecisionReason':
                           f"Use learned legible form {form['name']}: {form['legibleForm']}. "
                           'Submit it through the unchanged palette and permission checks.'}})
    if event not in {'PostToolUse', 'UserPromptSubmit', 'Stop'}:
        return None
    with delivery_lock(directory) as journal:
        if not journal.news(reader) and not journal.pending():
            return None
    status = snapshot(repo, root, session, remaining())
    if not pinned(status):
        return None
    cid = status['policyCid']
    with delivery_lock(directory) as journal:
        if question_answered(payload):
            questions = json.dumps((payload.get('tool_input') or {}).get('questions', []))
            for park in journal.pending():
                if park['agentId'] == reader and re.search(
                        r'(?<![A-Za-z0-9:._-])' + re.escape(park['incidentId']) + r'(?![A-Za-z0-9:._-])', questions):
                    journal.add('park-state', f"answered:{park['incidentId']}",
                                incidentId=park['incidentId'], state='answered',
                                source='AskUserQuestion completion; fresh validation still required')
        if event == 'Stop':
            if payload.get('stop_hook_active') or normalized.get('question') == 'uncertain':
                return None
            # Stop is the harness evidence that this turn has no work left. Never
            # consume a new batch on the continuation caused by our own block.
            batch = [r for r in journal.pending() if r['agentId'] == reader and r['policyCid'] == cid]
            batched = {i for r in journal.rows if r['kind'] == 'batch' for i in r['incidents']}
            if not batch or all(r['incidentId'] in batched for r in batch):
                return None
            ids = [r['incidentId'] for r in batch]
            journal.add('batch', 'batch:' + ':'.join(ids), incidents=ids, reader=reader)
            message = ('Surface every parked item below as ONE AskUserQuestion now. '
                       'This is the single block for this batch. Never retry a parked effect; '
                       'include each incident id in its question. An answer still requires fresh '
                       'scoped authority and permission checks.\n' +
                       '\n'.join(f"{r['incidentId']}: {r['ask']}" for r in batch))
            return json.dumps({'decision': 'block', 'reason': message})
        rows = [r for r in journal.news(reader) if r['policyCid'] == cid]
        if not rows:
            return None
        lines = []
        for row in rows:
            journal.add('receipt', f"receipt:{reader}:{row['seq']}", reader=reader, deliverySeq=row['seq'])
            if row['kind'] == 'PushNotification':
                lines.append(f"Stasis {row['incidentId']}: request PushNotification once: {row['message']}")
            else:
                lines.append(f"Stasis {row['incidentId']} guidance (advice, never authority): {row['message']}")
        return context(event, '\n'.join(lines))


def capability(name):
    data = json.loads((REPO / 'genesis/agentic/agent-runtimes.json').read_text())
    runtime = next(r for r in data['runtimes'] if r['id'] == 'claude-code')
    cap = runtime.get('harnessCapabilities', {}).get(name, {})
    return cap.get('status') == 'yes' and bool(cap.get('evidence'))


def session_start(repo, root, session):
    """One line, with a total subprocess budget below one second."""
    from concurrent.futures import ThreadPoolExecutor
    command = [os.environ.get('STASIS_COORDINATOR_BIN', str(REPO / 'genesis/agentic/bin/stasis-coordinator')),
               'start', '--start-wait', '.35']
    if mode() == 'live':
        command.append('--live')
    with ThreadPoolExecutor(max_workers=2) as pool:
        started = pool.submit(bounded, command, cwd=repo, timeout=.65)
        state = pool.submit(snapshot, repo, root, session, .65)
        try:
            code, _ = started.result()
            alive = 'alive' if code == 0 else 'unknown'
        except Exception:
            alive = 'unknown'
        try:
            status = state.result()
        except Exception:
            status = None
    policy = ('pinned' if pinned(status) else 'absent' if
              (status or {}).get('policyState') == 'absent' or not (Path(repo) / POLICY).exists() else 'unpinned')
    effective = mode() if mode() != 'live' or pinned(status) else 'shadow'
    sessions = (status or {}).get('sessions')
    count = sum(len(s.get('openIncidents', [])) for s in sessions) if isinstance(sessions, list) else 'unknown'
    return context('SessionStart', f'dialogue-stasis: {effective} · policy {policy} · coordinator {alive} · '
                   f'{count} open incidents; when parked, continue independent authorized work '
                   'and never retry the parked effect.')


def rewake(repo, root, payload):
    """Async Stop watcher: exit 2 is the probed wake transport, never a Stop loop."""
    if mode() != 'live' or not capability('async-rewake-wakes-idle'):
        return 0
    directory = root / payload['session_id']
    if not directory.is_dir():
        return 0
    reader = payload.get('agent_id') or 'main'
    # The synchronous Stop hook writes this token, but registration order is not
    # promised. A watcher started before it changes waits for that one Stop.
    token = payload.get('last_assistant_message')
    deadline = time.monotonic() + min(1860, float(os.environ.get('STASIS_REWAKE_SECONDS', '1860')))
    wait_for_stop = time.monotonic() + .4
    seen_stop = False
    while time.monotonic() < deadline:
        try:
            with (directory / 'state.json').open() as stream:
                state = json.load(stream)
            stop = state.get('stasisStop', {})
            if stop.get('messageDigest') != __import__('hashlib').sha256(str(token).encode()).hexdigest():
                if not seen_stop and time.monotonic() < wait_for_stop:
                    time.sleep(.05)
                    continue
                return 0
            seen_stop = True
            with delivery_lock(directory) as journal:
                news = journal.news(reader)
                key = f"wake:{reader}:{stop['seq']}"
                if any(r['key'] == key for r in journal.rows):
                    return 0
            if not news:
                time.sleep(.1)
                continue
            status = snapshot(repo, root, payload['session_id'])
            if not pinned(status):
                return 0
            with delivery_lock(directory) as journal:
                current_stop = json.loads((directory / 'state.json').read_text()).get('stasisStop', {})
                if current_stop.get('seq') != stop['seq']:
                    return 0
                if not any(r.get('policyCid') == status['policyCid'] for r in journal.news(reader)):
                    return 0
                if not journal.add('wake', key, reader=reader):
                    return 0
            print('Stasis guidance is ready. Read it at your next hook; continue independent authorized '
                  'work and surface parked asks together when blocked. Never retry a parked effect.', file=sys.stderr)
            return 2
        except BlockingIOError:
            time.sleep(.1)
        except (OSError, ValueError, KeyError, TypeError):
            return 0
    return 0


def worker():
    spec = importlib.util.spec_from_file_location('stasis_epr_client', REPO / '.claude/scripts/_lib/epr_client.py')
    client = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(client)
    # Read-only resolution must not launch provisioning from a keystroke.
    client.bootstrap = lambda _: None
    binary, _ = client.resolve()
    if not binary:
        return 1
    result = subprocess.run([binary, 'session', 'stasis', *sys.argv[2:]], input=sys.stdin.read(),
                            text=True, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
    if result.returncode == 0:
        print(result.stdout, end='')
    return result.returncode


if __name__ == '__main__':
    try:
        sys.exit(worker() if sys.argv[1:2] == ['native'] else 1)
    except Exception:
        sys.exit(1)
