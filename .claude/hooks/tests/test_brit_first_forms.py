import importlib.util
from pathlib import Path
import unittest
from unittest.mock import patch
ROOT=Path(__file__).resolve().parents[3]
spec=importlib.util.spec_from_file_location('brit_hook', ROOT/'.codex/hooks/brit-first.py')
hook=importlib.util.module_from_spec(spec)
spec.loader.exec_module(hook)
class BritHook(unittest.TestCase):
 def setUp(self):
  # The table under test is the current one; the brit on PATH is not asked.
  p=patch.object(hook,'installed_brit_version',return_value=hook.BRIT_TABLE_VERSION)
  p.start(); self.addCleanup(p.stop)
 def on(self,version):
  return patch.object(hook,'installed_brit_version',return_value=version)
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
 def test_table_version_is_035(self):
  self.assertEqual(hook.BRIT_TABLE_VERSION,'0.3.5'); self.assertEqual(hook.BRIT_TABLE_PRIOR_VERSION,'0.3.4')
  self.assertTrue(self.reason('git status').startswith('Brit 0.3.5 table. '))
 def test_035_cherry_pick(self):
  r=self.reason('git cherry-pick abc')
  self.assertIn('brit cherry-pick <commit> (one commit per call)',r); self.assertIn('fallback only for a range',r)
 def test_035_merge_ff_only(self):
  r=self.reason('git merge --ff-only origin/dev')
  self.assertIn('brit merge --ff-only <ref>',r); self.assertIn('the ref is required',r)
  self.assertIn('fallback only for a merge that is not a fast-forward',r)
 def test_035_pull(self):
  r=self.reason('git pull')
  self.assertIn('brit pull --ff-only <remote> <branch> | --status | --continue | --abort',r)
  self.assertIn('fallback only for a merging or rebasing pull',r)
 def test_035_notes(self):
  r=self.reason('git notes show HEAD')
  self.assertIn('brit notes [--ref <notes ref>] show [object]',r)
  self.assertIn('brit fetch|push <remote> refs/notes/<x>:refs/notes/<x>',r)
  self.assertIn('fallback only for add, append, remove, copy, list',r)
 def test_035_grep(self):
  r=self.reason('git grep needle')
  self.assertIn('brit grep [-n] [-i] [-l] [-F|-E] <rev> -- <pattern>',r)
  self.assertIn('fallback only for searching the working tree or index without a revision',r)
 def test_035_log_pickaxe(self):
  self.assertIn('[-S <string>]',self.reason('git log -S needle'))
 def test_035_gitlink(self):
  form='brit add --gitlink <full commit> <submodule path>'
  self.assertIn('| --gitlink <full commit> <submodule path>',self.reason('git add elohim/brit'))
  r=self.reason('git update-index --cacheinfo 160000,abc,elohim/brit')
  self.assertIn(form,r); self.assertIn('fallback only for every other update-index form',r)
 def test_035_commit_amend_kept(self):
  self.assertIn('[--amend --no-edit]',self.reason('git commit --amend'))
 def test_035_still_absent(self):
  for verb in ['stash','reset','apply','format-patch','am','bisect','reflog','gc','repack']:
   with self.subTest(verb=verb):
    r=self.reason('git '+verb)
    self.assertIn('git %s: no brit form in 0.3.5'%verb,r); self.assertIn('a fallback is expected',r)
 def test_035_still_partial(self):
  self.assertIn('fallback only for --source other than HEAD',self.reason('git restore x'))
  self.assertIn('fallback only for add, status, sync, foreach',self.reason('git submodule status'))
  self.assertIn("git's exact output format",self.reason('git ls-tree HEAD'))
  self.assertIn("git's exact output format",self.reason('git ls-files -s'))
  self.assertIn('brit config show [filter]',self.reason('git config --get user.name'))
 def test_034_session_gets_034_rows(self):
  with self.on('0.3.4'):
   r=self.reason('git cherry-pick abc')
   self.assertTrue(r.startswith('Brit 0.3.4 table (this session'),r); self.assertIn('0.3.5 is landed',r)
   self.assertIn('git cherry-pick: no brit form in 0.3.4',r); self.assertNotIn('brit cherry-pick',r)
   for verb in ['notes','grep','update-index']:
    with self.subTest(verb=verb):
     self.assertIn('git %s: no brit form in 0.3.4'%verb,self.reason('git '+verb))
   r=self.reason('git merge --ff-only x')
   self.assertNotIn('brit merge --ff-only',r); self.assertIn('brit pull --ff-only',r)
   self.assertNotIn('-S',self.reason('git log')); self.assertNotIn('--gitlink',self.reason('git add x'))
   self.assertIn('brit status --no-write',self.reason('git status'))
 def test_prior_rows_only_override(self):
  self.assertEqual(sorted(hook.BRIT_TABLE_PRIOR),['add','cherry-pick','grep','log','merge','notes','update-index'])
  self.assertLessEqual(set(hook.BRIT_TABLE_PRIOR),set(hook.BRIT_TABLE))
 def test_unread_version_says_which_form(self):
  with self.on(None):
   r=self.reason('git cherry-pick abc')
   self.assertIn('(brit 0.3.5 form; a session still on 0.3.4 falls back with a stated reason)',r)
   self.assertIn('brit cherry-pick <commit>',r)
 def test_other_version_marks_table_stale(self):
  with self.on('0.4.0'):
   r=self.reason('git stash')
   self.assertIn('installed brit is 0.4.0: the table is stale',r); self.assertIn('no brit form in 0.3.5',r)
 def test_every_row_fits(self):
  for version in ['0.3.5','0.3.4',None,'0.4.0']:
   with self.on(version):
    for verb in hook.BRIT_TABLE:
     with self.subTest(version=version,verb=verb):
      r=self.reason('git '+verb)
      self.assertNotEqual(r,hook.GENERIC); self.assertLessEqual(len(r),600)
 def test_pass_never_probes(self):
  with patch.object(hook,'installed_brit_version',side_effect=AssertionError('probed on a pass')):
   self.assertIsNone(hook.evaluate({'tool_name':'Bash','tool_input':{'command':'brit status --no-write'}}))
   self.assertNotIn('permissionDecision',hook.evaluate({'tool_name':'Bash','tool_input':{'command':'# Brit fallback: installed Brit lacks stash\ngit stash'}}))
class VersionProbe(unittest.TestCase):
 def probe(self,**kw):
  import subprocess
  with patch.object(subprocess,'run',**kw) as run:
   return hook.installed_brit_version(),run
 def test_reads_first_line(self):
  class Out:
   stdout='brit 0.3.4\nsource HEAD: e92dd41e\n'
  v,run=self.probe(return_value=Out)
  self.assertEqual(v,'0.3.4'); self.assertEqual(run.call_args[0][0],['brit','--version'])
  self.assertLessEqual(run.call_args[1]['timeout'],0.5)
 def test_failure_is_none(self):
  import subprocess
  class Odd:
   stdout='something else'
  self.assertIsNone(self.probe(return_value=Odd)[0])
  self.assertIsNone(self.probe(side_effect=subprocess.TimeoutExpired('brit',0.5))[0])
  self.assertIsNone(self.probe(side_effect=FileNotFoundError())[0])
if __name__=='__main__':
 unittest.main()
