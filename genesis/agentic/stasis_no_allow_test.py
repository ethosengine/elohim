"""Verification row 6: actuator sources cannot construct a permission allow."""

import ast
from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[2]


def literal(node):
    if isinstance(node, ast.Constant):
        return node.value
    if isinstance(node, ast.BinOp) and isinstance(node.op, ast.Add):
        left, right = literal(node.left), literal(node.right)
        if isinstance(left, str) and isinstance(right, str):
            return left + right
    return None


def forbidden(source):
    tree = ast.parse(source)
    # 'allow' as the KEY of a keeper's static read-only settings is not a
    # permission decision. As a VALUE, including concatenated strings, it is
    # prohibited in these two actuator files. Fail closed on dynamic decisions.
    keys = {id(k) for node in ast.walk(tree) if isinstance(node, ast.Dict) for k in node.keys}
    findings = []
    for node in ast.walk(tree):
        value = literal(node)
        if isinstance(value, str) and id(node) not in keys:
            if value.strip().lower() == 'allow' or re.search(
                    r'''["']?(?:behavior|permissionDecision)["']?\s*:\s*["']?allow\b''', value):
                findings.append(node.lineno)
        pairs = []
        if isinstance(node, ast.Dict):
            pairs = [(literal(k), v) for k, v in zip(node.keys, node.values) if k]
        elif isinstance(node, ast.keyword):
            pairs = [(node.arg, node.value)]
        elif isinstance(node, ast.Assign):
            pairs = [(literal(t.slice), node.value) for t in node.targets if isinstance(t, ast.Subscript)]
        for key, val in pairs:
            if key in {'behavior', 'permissionDecision'} and literal(val) not in {'deny', 'ask'}:
                findings.append(node.lineno)
    return findings


class NoAllowTests(unittest.TestCase):
    def test_actuators_have_no_permission_allow_construction(self):
        for path in ['.claude/hooks/dialogue-stasis-sense.py', 'genesis/agentic/bin/stasis-coordinator']:
            with self.subTest(path=path):
                self.assertEqual(forbidden((ROOT / path).read_text()), [])

    def test_negative_controls_cover_equivalent_spellings(self):
        for source in ["x = {'behavior': 'allow'}", "x = {'permissionDecision': 'al' + 'low'}",
                       "x['behavior'] = decision", "x.update(permissionDecision=choice)",
                       '''print('{"behavior":"allow"}')''', "decision = 'allow'",
                       "x = {'behavior': f'{choice}'}"]:
            with self.subTest(source=source):
                self.assertTrue(forbidden(source))
        self.assertEqual(forbidden("x = {'permissions': {'allow': ['Read']}}"), [])
        self.assertEqual(forbidden("x = {'behavior': 'deny'}"), [])


if __name__ == '__main__':
    unittest.main()
