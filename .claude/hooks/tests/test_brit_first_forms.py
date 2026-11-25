import importlib.util
from pathlib import Path
import unittest
from unittest.mock import patch
ROOT=Path(__file__).resolve().parents[3]
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
 def reason(self,command):
  r=hook.evaluate({'tool_name':'Bash','tool_input':{'command':command}})
  self.assertEqual(r['permissionDecision'],'deny')
  return r['permissionDecisionReason']
 def test_has_names_form(self):
  r=self.reason('git show origin/dev:CLAUDE.md')
  self.assertIn('brit has this: brit show',r); self.assertIn(hook.BRIT_TABLE_VERSION,r)
  self.assertIn('brit merge-base [--is-ancestor]',self.reason('git merge-base --is-ancestor a b'))
  self.assertIn('brit status --no-write',self.reason('git -C /x status'))
 def test_partial_names_both(self):
  r=self.reason('git switch -c x')
  self.assertIn('brit switch <existing branch>',r); self.assertIn('fallback only for switch -c',r)
  r=self.reason('git push origin HEAD:dev')
  self.assertIn('brit has this',r); self.assertIn('GATE_EVIDENCE=reuse brit -c core.hooksPath=<worktree>/.husky/_ push origin HEAD:dev',r)
  self.assertNotIn('expected fallback',r)
 def test_absent_expects_fallback(self):
  r=self.reason('git stash')
  self.assertIn('no brit form in '+hook.BRIT_TABLE_VERSION,r); self.assertIn('a fallback is expected',r)
 def test_unparseable_is_generic(self):
  for c in ['git','git -C /x']:
   self.assertEqual(self.reason(c),hook.GENERIC)
 def test_brit_not_refused(self):
  for c in ['brit merge-base --is-ancestor a b','brit -r /x log','brit -C /x status --no-write','env -u EPR_BIN brit show a:b']:
   self.assertIsNone(hook.evaluate({'tool_name':'Bash','tool_input':{'command':c}}))
 def test_nested_bash_steered(self):
  self.assertIn('brit show',self.reason("bash -c 'git show HEAD:x'"))
 def test_length_bound(self):
  self.assertLessEqual(len(self.reason('git status;git log;git diff;git add;git commit;git push;git stash')),600)
  self.assertLessEqual(len(self.reason('git push origin HEAD:dev')),600)
 def test_internal_error_degrades(self):
  with patch.object(hook,'steer',side_effect=RuntimeError('boom')):
   self.assertEqual(self.reason('git status'),hook.GENERIC)
  with patch.object(hook,'BRIT_TABLE',None):
   self.assertEqual(self.reason('git status'),hook.GENERIC)
 def test_fallback_still_passes(self):
  event={'tool_name':'Bash','tool_input':{'command':'# Brit fallback: installed Brit lacks stash\ngit stash'}}
  self.assertNotIn('permissionDecision',hook.evaluate(event))
if __name__=='__main__':
 unittest.main()
