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
import datetime as dt
import fcntl
import hashlib
import importlib.util
from itertools import islice
import json
import os
from pathlib import Path
import re
import signal
import stat
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
def delivery_lock(directory, read_limit=None):
    with (directory / 'journal.lock').open('a') as lock:
        # Hook delivery must not wait behind a coordinator's bounded berth write.
        # The coordinator already owns the lock and uses Delivery directly.
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        yield Delivery(directory, read_limit=read_limit)


class Delivery:
    """Caller holds journal.lock. Receipts are claimed before external delivery.

    At most once is intentional: death after claim is unconfirmed delivery, never
    a second alert or Stop block. No acknowledgement creates execution authority.
    """
    def __init__(self, directory, read_limit=None):
        self.path = directory / 'delivery.jsonl'
        self.rows = []
        if self.path.exists():
            raw_text = self.path.read_text() if read_limit is None else read_evidence(self.path, read_limit)
            for raw in raw_text.splitlines():
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


def read_evidence(path, limit=2 * 1024 * 1024):
    """Only bounded regular files; a broken log is missing proof, never green."""
    fd = os.open(path, os.O_RDONLY | os.O_NONBLOCK)
    with os.fdopen(fd, 'rb') as stream:
        if not stat.S_ISREG(os.fstat(stream.fileno()).st_mode):
            raise ValueError('non-regular evidence')
        raw = stream.read(limit + 1)
    if len(raw) > limit:
        raise ValueError('evidence exceeds read budget')
    return raw.decode('utf-8')


def stop_history(journal):
    remaining, events = 2 * 1024 * 1024, []
    for path in journal.segments() + ([journal.path] if journal.path.exists() else []):
        raw = read_evidence(path, remaining)
        remaining -= len(raw.encode())
        events.extend(json.loads(line) for line in raw.splitlines() if line.strip())
    return events


def gate_reason(common, head):
    """Read gate-live's shared worktree root and final exit verdicts, not RED hints.

    gate-live.mjs: liveRoot/openRun/closeRun; gate-runner.mjs: gate-verdict.
    A newer incomplete/deferred run vetoes an older green for the same HEAD.
    """
    directory = Path(common).parent / '.eprfs/status/gate-live/runs'
    candidates, remaining = [], 2 * 1024 * 1024
    deadline = time.monotonic() + .15
    paths = list(islice(directory.iterdir(), 129))
    if len(paths) > 128:
        raise ValueError('gate evidence exceeds read budget')
    for path in paths:
        if time.monotonic() >= deadline:
            raise ValueError('gate evidence exceeds time budget')
        if path.suffix != '.jsonl':
            continue
        # Read every retained log: unreadable evidence cannot hide a newer red.
        raw = read_evidence(path, remaining)
        remaining -= len(raw.encode())
        rows = [json.loads(line) for line in raw.splitlines() if line.strip()]
        opened = rows[0]
        if opened['kind'] != 'run-open':
            raise ValueError('missing run-open')
        if opened['candidate']['sha'] == head:
            at = dt.datetime.fromisoformat(opened['at'].replace('Z', '+00:00')).timestamp()
            candidates.append((at, path.stat().st_mtime_ns, path.name, rows))
    if not candidates:
        return 'gate: no run for ' + head[:12]
    rows = max(candidates, key=lambda item: item[:3])[3]
    verdicts = [r for r in rows if r['kind'] == 'gate-verdict']
    red = sorted({r['project'] for r in verdicts if r['verdict']['exit'] != 0})
    if red:
        return 'gate: red on ' + ', '.join(red)
    if any(r['verdict'].get('deferred') or r.get('status') == 'Unanswered' for r in verdicts):
        return 'gate: deferred or unanswered'
    closed = rows[-1]
    if closed['kind'] != 'run-close':
        return 'gate: unfinished on ' + head[:12]
    if closed['outcome'] not in {'passed', 'gates-passed'}:
        return 'gate: ' + closed['outcome'] + ' on ' + head[:12]
    if closed.get('deferred', 0):
        return 'gate: deferred projects'
    if not verdicts:
        return 'gate: no project verdicts'
    return None


def checkout_evidence(cwd, events, now):
    """All git children share one <1 s budget and never refresh the index."""
    reasons, low = [], False
    deadline = time.monotonic() + .85
    env = {k: v for k, v in os.environ.items() if not k.startswith('GIT_')}
    env['GIT_OPTIONAL_LOCKS'] = '0'

    def git(*args):
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            raise TimeoutError()
        code, out = bounded(['git', *args], cwd=cwd, timeout=remaining, env=env)
        if code != 0:
            raise ValueError('git evidence unavailable')
        return out

    try:
        if not isinstance(cwd, str) or not Path(cwd).is_absolute():
            raise ValueError('payload cwd absent')
        top, common, head = git('rev-parse', '--path-format=absolute', '--show-toplevel',
                                '--git-common-dir', 'HEAD').splitlines()
        history = git('log', '-256', '--format=%H %at', 'HEAD').splitlines()
        first = events[0]['clock']['wall'] if events else now['wall']
        since = dt.datetime.fromisoformat(first.replace('Z', '+00:00')).timestamp()
        # Structured journal evidence only; never mine assistant prose for a SHA.
        recorded = {e.get('wouldHave', {}).get('commit') for e in events}
        if not any(int(line.split()[1]) >= since or line.split()[0] in recorded for line in history):
            reasons.append('commit: no session commit in HEAD history')
            low = len(history) == 256
        # -z avoids quoting ambiguity; both sides of a rename must be tooling paths.
        entries = git('-C', top, 'status', '--porcelain=v1', '-z', '--untracked-files=no').split('\0')
        dirty, index = [], 0
        while index < len(entries) and entries[index]:
            entry = entries[index]
            paths = [entry[3:]]
            index += 1
            if 'R' in entry[:2] or 'C' in entry[:2]:
                paths.append(entries[index])
                index += 1
            dirty.extend(p for p in paths if not p.startswith(('.claude/data/', '.eprfs/status/')))
        if dirty:
            names = sorted(set(dirty))
            reasons.append(f'uncommitted: {len(names)} files: ' + ', '.join(names))
        try:
            reason = gate_reason(common, head)
            if reason:
                reasons.append(reason)
        except (OSError, ValueError, KeyError, TypeError, IndexError, AttributeError):
            reasons.append('gate: unreadable evidence')
            low = True
    except (OSError, ValueError, KeyError, TypeError, IndexError, AttributeError, TimeoutError):
        reasons.append('git: unavailable evidence within 0.85 s budget (payload cwd)')
        low = True
    return reasons, low


def stop_event(journal, question):
    """Observe before acting. The native fold alone opens/closes incidents."""
    events, reasons, low = [], [], False
    outcome = None
    try:
        events = stop_history(journal)
        delivery = Delivery(journal.directory, read_limit=2 * 1024 * 1024)
        pending = [r for r in delivery.pending() if r.get('agentId', 'main') == journal.agent]
        human = {r['incidentId'] for r in delivery.rows if r['kind'] == 'human'}
        batched = {i for r in delivery.rows if r['kind'] == 'batch' for i in r['incidents']}
        # A fresh turn, answer or closure retires a previously escalated question.
        questions = set()
        witness = journal.state.get('presence', {}).get('lastWitnessWall')
        for e in events:
            if e.get('agentId') == journal.agent:
                if e['kind'] == 'turn-ended' and e.get('outcome') != 'exception':
                    questions.clear()
                    answered = (witness and dt.datetime.fromisoformat(witness.replace('Z', '+00:00'))
                                > dt.datetime.fromisoformat(e['clock']['wall'].replace('Z', '+00:00')))
                    if not answered and (e.get('outcome') == 'question' or e.get('question') == 'yes'):
                        questions.add(f"{journal.payload['session_id']}:{e['seq']}")
                elif e['kind'] in {'tool-completed', 'steward-ping', 'session-ended'}:
                    questions.clear()
        rung5 = any(e.get('wouldHave', {}).get('effect') == 'incident-advancement'
                    and e['wouldHave'].get('rung') == 5
                    and e['wouldHave'].get('incidentId') in questions for e in events)
        if any(r['incidentId'] in human | batched for r in pending) or rung5:
            outcome = 'exception'
        elif question == 'yes':
            outcome = 'question'
        else:
            reasons, low = checkout_evidence(journal.payload.get('cwd'), events, journal.now)
            reasons.extend('open incident ' + r['incidentId'] for r in pending)
            outcome = 'stalled' if reasons else 'accomplished'
    except (OSError, ValueError, KeyError, TypeError, IndexError, AttributeError):
        outcome, reasons, low = 'stalled', ['journal: unreadable stop evidence'], True
    fields = {'question': question, 'outcome': outcome,
              'correlation': {'incidentId': None, 'confidence': 'low' if low else 'high', 'method': 'none'}}
    if outcome == 'stalled':
        fields['stallReason'] = '; '.join(reasons)
    recorded = journal.append('turn-ended', **fields)
    previous = next((e for e in reversed(events) if isinstance(e, dict) and e.get('kind') == 'turn-ended'
                     and e.get('agentId') == journal.agent), {})
    if outcome == 'stalled' and previous.get('stallReason') == recorded['stallReason']:
        key = stall_key(journal.agent, recorded)
        journal.append('would-have', wouldHave={
            'effect': 'Refer', 'incidentId': f"{journal.payload['session_id']}:{recorded['seq']}",
            'rung': 1, 'key': key, 'reason': recorded['stallReason']})
        return {**recorded, '_repeatStall': True}
    return recorded


def stall_key(reader, event):
    return 'Stop-stall:' + reader + ':' + hashlib.sha256(event['stallReason'].encode()).hexdigest()


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
        # Native decide owns presence for this event. A later status can change
        # after the act closes its stall; it only revalidates the policy pin here.
        if (not pinned(status) or decision.get('policyCid') != status['policyCid'] or
                decision.get('incidentId') != iid or not iid):
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
    if event == 'Stop' and normalized.get('outcome') in {'accomplished', 'exception', 'question'}:
        return None
    if event == 'Stop' and normalized.get('_repeatStall'):
        return None
    stop_limit = 2 * 1024 * 1024 if event == 'Stop' else None
    with delivery_lock(directory, read_limit=stop_limit) as journal:
        if (normalized.get('outcome') != 'stalled'
                and not journal.news(reader) and not journal.pending()):
            return None
    status = snapshot(repo, root, session, min(.65, remaining()) if event == 'Stop' else remaining())
    if not pinned(status):
        return None
    cid = status['policyCid']
    with delivery_lock(directory, read_limit=stop_limit) as journal:
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
            if normalized.get('outcome') == 'stalled':
                if presence(status, session)['disposition'] != 'automated':
                    return None
            # Prefer the existing parked-ask batch when unfinished work is parked.
            # Never consume a new batch on our own block's continuation.
            batch = [r for r in journal.pending() if r['agentId'] == reader and r['policyCid'] == cid]
            batched = {i for r in journal.rows if r['kind'] == 'batch' for i in r['incidents']}
            if not batch or all(r['incidentId'] in batched for r in batch):
                if normalized.get('outcome') != 'stalled':
                    return None
                key = stall_key(reader, normalized)
                if not journal.add('Stop-block', key, reader=reader, policyCid=cid,
                                   incidentId=f"{session}:{normalized['seq']}",
                                   stallReason=normalized['stallReason']):
                    return None
                message = ('Unfinished work: ' + normalized['stallReason'] + '. '
                           'Next authorized step: inspect the named files or gate evidence, '
                           'finish the already authorized work and run its owning gates; '
                           'commit only if already authorized. If a step requires the steward, '
                           'prepare that ask. Never retry a denied effect or alter another session\'s work. '
                           'This is the single Stop block for these unmet items.')
                return json.dumps({'decision': 'block', 'reason': message})
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
