"""Exact-session adapters. Runtime switching is a fresh conversation over durable evidence."""
import json
from pathlib import Path
import uuid

from shift_state import Refused


def events(path):
    for line in Path(path).read_text().splitlines():
        try:
            yield json.loads(line)
        except ValueError:
            continue


def completed(runtime, path, *, canary_root=None):
    """A zero process exit is insufficient: require the runtime's terminal event.

    A canary additionally proves a successful shell action in the declared tree;
    echoing the requested marker without obtaining tool permission cannot pass.
    """
    terminal = False
    marker = False
    tool_ok = False
    calls = set()
    for event in events(path):
        if runtime == 'codex':
            terminal |= event.get('type') == 'turn.completed'
            if event.get('type') in {'turn.failed', 'error'}:
                return False
            item = event.get('item', {})
            if event.get('type') == 'item.completed':
                marker |= item.get('type') == 'agent_message' and 'SHIFT_CANARY_OK' in item.get('text', '')
                tool_ok |= (item.get('type') == 'command_execution' and item.get('exit_code') == 0
                            and item.get('aggregated_output', '').strip() == str(canary_root))
        else:
            if event.get('type') == 'result':
                terminal = event.get('is_error') is False and event.get('subtype', 'success') == 'success'
                marker |= 'SHIFT_CANARY_OK' in event.get('result', '')
            for item in event.get('message', {}).get('content', []):
                if not isinstance(item, dict):
                    continue
                if item.get('type') == 'tool_use' and item.get('name') == 'Bash' and item.get('input', {}).get('command') == 'pwd':
                    calls.add(item['id'])
                if item.get('type') == 'tool_result' and item.get('tool_use_id') in calls:
                    tool_ok |= not item.get('is_error', False) and item.get('content', '').strip() == str(canary_root)
                marker |= item.get('type') == 'text' and 'SHIFT_CANARY_OK' in item.get('text', '')
    return terminal and (canary_root is None or (marker and tool_ok))


def session_from(runtime, path):
    sessions = set()
    try:
        lines = Path(path).read_text().splitlines()
    except FileNotFoundError:
        return None
    for line in lines:
        try:
            event = json.loads(line)
        except ValueError:
            continue  # interrupted final JSONL line is not a session id
        session = (event.get('thread_id') if event.get('type') == 'thread.started'
                   else None) if runtime == 'codex' else event.get('session_id')
        if session:
            try:
                uuid.UUID(session)
            except (ValueError, TypeError) as error:
                raise Refused('runtime emitted an invalid session id') from error
            sessions.add(session)
    if len(sessions) > 1:
        raise Refused('ambiguous runtime sessions in one attempt')
    return next(iter(sessions), None)


def command(runtime, *, session=None, fresh_id=None, tools=(), readonly=False):
    if session:
        uuid.UUID(session)  # never interpret a slug such as "latest" as a saved session
    if runtime == 'codex':
        args = ['codex', 'exec', '-c', 'approval_policy="never"', '-c',
                f'sandbox_mode="{"read-only" if readonly else "workspace-write"}"']
        if session:
            args += ['resume', session]
        return args + ['--json', '-']
    if runtime == 'claude':
        args = ['claude', '-p', '--permission-mode', 'default' if readonly else 'acceptEdits',
                '--permission-prompts', 'none', '--output-format', 'stream-json', '--verbose']
        if tools:
            args += ['--allowedTools', ','.join(tools)]
        args += ['--disallowedTools', ','.join([
            'Bash(git push*)', 'Bash(git merge*)', 'Bash(git reset*)',
            'Bash(just mesh*)', 'Bash(kubectl*)',
        ])]
        if session:
            args += ['--resume', session]
        elif fresh_id:
            args += ['--session-id', fresh_id]
        return args
    raise Refused(f'unknown runtime: {runtime}')


def prompt(contract, step, directory):
    return (
        f"Execute one bounded shift operation: {step['id']}.\n"
        f"Read {directory}/objective.json and {directory}/contract.json, then the existing "
        f"journal, operation receipts and native flow context {contract['scope_cid']}.\n"
        f"Intent: {contract['intent']}; commitment: {contract['commitment']}; "
        f"serves: {contract['habit']}. Preserve that scope CID.\n"
        f"Absolute deadline: {contract['deadline']}; recovery grants no extra time.\n"
        f"Allowed source paths: {json.dumps(contract['objective']['scope']['paths'])}.\n"
        "Inspect prior operations and side effects before retrying. Uncertain effects remain unresolved. "
        "Do not rewrite fulfillment reports, objectives, authorization, checkpoints or receipts. "
        "Do not land, push, or manage household lifecycle; the deterministic host launcher owns those acts. "
        "Do not use approval/sandbox bypass flags. Native appointed acceptance alone authorizes closure.\n\n"
        + step['prompt']
    )
