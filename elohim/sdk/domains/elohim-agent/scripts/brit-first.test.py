import importlib.util
from pathlib import Path
import unittest
from unittest.mock import patch
ROOT=Path(__file__).resolve().parents[5]
spec=importlib.util.spec_from_file_location('brit_hook', ROOT/'.codex/hooks/brit-first.py')
hook=importlib.util.module_from_spec(spec)
spec.loader.exec_module(hook)
class BritHook(unittest.TestCase):
 def test_refusals(self):
  for command in ['git status','/usr/bin/git log','command git diff','env FOO=bar git status',"bash -lc 'git status'",'cd /projects/elohim&&git status','echo hello | git hash-object --stdin']:
   with self.subTest(command=command):
    self.assertEqual(hook.evaluate({'tool_name':'Bash','tool_input':{'command':command}})['permissionDecision'],'deny')
 def test_allowed(self):
  for command in ['brit status --no-write',"echo 'git status'",'bash scripts/check.sh','python3 test.py']:
   self.assertIsNone(hook.evaluate({'tool_name':'exec_command','tool_input':{'cmd':command}}))
 def test_fallback(self):
  event={'tool_name':'Bash','tool_input':{'command':'# Brit fallback: installed Brit lacks this operation\ngit status'}}
  self.assertNotIn('permissionDecision',hook.evaluate(event))
  event['tool_input']['command']='# Brit fallback: habit\ngit status'
  self.assertEqual(hook.evaluate(event)['permissionDecision'],'deny')
 def test_disabled(self):
  with patch.object(Path, "read_text", return_value='{"enabled":false}'):
   self.assertIsNone(hook.evaluate({"tool_name":"Bash","tool_input":{"command":"git status"}}))
 def test_edit(self):
  self.assertIsNone(hook.evaluate({'tool_name':'Edit','tool_input':{'command':'git status'}}))
unittest.main()
