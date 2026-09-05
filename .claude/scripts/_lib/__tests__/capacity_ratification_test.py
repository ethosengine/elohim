#!/usr/bin/env python3
"""Both hosts consume the capacity fixture pair and identical JSON-pointer variants."""
import contextlib
import copy
import io
import importlib.util
import json
import os
from pathlib import Path
import sys
import subprocess
import tempfile
from unittest.mock import patch

import jsonschema

ROOT = Path(__file__).resolve().parents[4]
sys.path.insert(0, str(ROOT / '.claude/scripts'))
from _lib import epr_meta, epr_meta_git

FIXTURES = ROOT / 'elohim/eprfs/epr-cli/tests/fixtures/capacity'
spec = importlib.util.spec_from_file_location('capacity_git_gate', ROOT / '.claude/scripts/epr-meta-git-gate.py')
git_gate = importlib.util.module_from_spec(spec)
spec.loader.exec_module(git_gate)
MANIFEST = '''---
epr-meta-version: 1
root: true
rules:
  - id: test-bench-aggregate-capacity
    class: ask
    validator: epr:validator-test-bench-aggregate-capacity
---
'''
failures = []


def check(label, condition):
    print(f"{'GREEN' if condition else 'RED'} {label}")
    if not condition:
        failures.append(label)


def variant(base, patches):
    value = copy.deepcopy(base)
    for pointer, replacement in patches.items():
        keys = pointer.lstrip('/').split('/')
        obj = value
        for key in keys[:-1]:
            obj = obj[int(key)] if isinstance(obj, list) else obj[key]
        obj[int(keys[-1]) if isinstance(obj, list) else keys[-1]] = replacement
    return value


base = json.loads((FIXTURES / 'compute-capacity.json').read_text())
deployments = (FIXTURES / 'deployments.json').read_text()
schema = json.loads((ROOT / 'genesis/data/rakia/compute-capacity.schema.json').read_text())
validator = jsonschema.Draft7Validator(schema, format_checker=jsonschema.FormatChecker())
for case in json.loads((FIXTURES / 'cases.json').read_text()):
    ledger = variant(base, case['patches'])
    check(case['name'] + ' schema', validator.is_valid(ledger) == case['schemaValid'])
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        (root / '.git').mkdir()
        (root / '.epr-meta').write_text(MANIFEST)
        ledger_path = root / epr_meta._CAPACITY_LEDGER_PATH
        deploy_path = root / epr_meta._DEPLOYMENTS_PATH
        for path, content in [(ledger_path, json.dumps(ledger)), (deploy_path, deployments)]:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(content)
        with patch.object(epr_meta, '_CAPACITY_LEDGER_PATH', str(ledger_path)), \
             patch.object(epr_meta, '_DEPLOYMENTS_PATH', str(deploy_path)), \
             patch.dict(os.environ, {'EPR_META_NOW': case['now']}):
            for path in (ledger_path, deploy_path):
                stderr = io.StringIO()
                with contextlib.redirect_stderr(stderr):
                    result = epr_meta.resolve_write(path, {'path': str(path), 'content': path.read_text()}, root)
                verdict = (epr_meta.Verdict(result['cls'], result['reason'], result['rule_id'],
                                           (result.get('refer') or {}).get('reason'))
                           if result['cls'] else None)
                decision = result
                label = case['name'] + ' ' + path.name
                check(label + ' decision', decision['decision'] == case['decision'])
                check(label + ' class', decision['cls'] == case['cls'])
                message = verdict.reason if verdict else ''
                check(label + ' evidence', all(text in message and text in stderr.getvalue()
                                              for text in case['contains']))
                code, messages = epr_meta_git.decide([verdict], ack=False)
                check(label + ' reach', code == (1 if case['cls'] in ('deny', 'ask') else 0))
                native = {'decision': decision['decision'], 'winningClass': decision['cls'],
                          'referReason': (decision.get('refer') or {}).get('reason'),
                          'ruleId': decision['rule_id'], 'reason': message}
                # Exercise the native-authority adapter too: a refer must not re-harden
                # an inject advisory, while fresh asks and invalid records still block.
                with patch.object(epr_meta_git, 'changed_files', return_value=[(str(path), 'M')]), \
                     patch.object(epr_meta_git, 'content_of', return_value=path.read_text()), \
                     patch.object(epr_meta_git, 'head_has_parent', return_value=True), \
                     patch.object(epr_meta_git, 'verdict_for', return_value=verdict), \
                     patch.object(git_gate.epr_client, 'govern', return_value=native), \
                     patch.object(git_gate, '_witness_file'), \
                     patch.dict(os.environ, {'EPR_META_ACK': ''}), \
                     contextlib.redirect_stderr(io.StringIO()):
                    gate_code = git_gate.main(['--staged'])
                check(label + ' native-authority reach', gate_code == code)
                if case['name'] in ('stale-31-days', 'fresh-30-days', 'stale-invalid-still-refuses'):
                    payload = {'tool_name': 'Write', 'tool_input': {
                        'file_path': str(path), 'content': path.read_text()}}
                    hook = subprocess.run([sys.executable, str(ROOT / '.claude/hooks/epr-meta-resolver.py')],
                                          input=json.dumps(payload), text=True, capture_output=True,
                                          cwd=root, env={**os.environ, 'EPR_META_NOW': case['now']})
                    output = json.loads(hook.stdout)['hookSpecificOutput']
                    check(label + ' edit-time hook', hook.returncode == 0 and
                          output.get('permissionDecision') == (case['cls'] if case['cls'] != 'inject' else None))
                    if case['cls'] == 'inject':
                        check(label + ' hook evidence', '31 days' in output['additionalContext']
                              and 'limits.cpu_m=1200' in output['additionalContext'])
                if case['cls'] == 'inject':
                    check(label + ' advisory refer', decision['refer']['reason'] == 'stale-evidence'
                          and any('[refer]' in m for m in messages) and not any('[ask]' in m for m in messages))

check('live ledger schema', validator.is_valid(json.loads((ROOT / epr_meta._CAPACITY_LEDGER_PATH).read_text())))
print(f'{len(failures)} RED')
sys.exit(bool(failures))
