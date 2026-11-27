from contextlib import redirect_stdout
import importlib.util
import io
import json
import os
from pathlib import Path
from tempfile import TemporaryDirectory
from types import SimpleNamespace
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


class ScratchExemption(unittest.TestCase):
 """Ruling 2026-11-27: the outside-repository exemption is a narrow allow-list that fails closed.

 A git call passes only when every git stage names a scratch repository by one literal absolute -C
 and runs a local verb; a quoted heredoc body is data only for cat or tee writing to a file; any
 command the hook cannot read with certainty is refused when a git token is in it."""
 def setUp(self):
  p=patch.object(hook,'installed_brit_version',return_value=hook.BRIT_TABLE_VERSION)
  p.start(); self.addCleanup(p.stop)
  env=patch.dict(os.environ,{}); env.start(); self.addCleanup(env.stop)
  for name in hook.LOCATING_ENV: os.environ.pop(name,None)
  self.tmpdir=TemporaryDirectory(dir='/tmp'); self.addCleanup(self.tmpdir.cleanup)
  self.tmp=Path(self.tmpdir.name).resolve()
  self.ws=self.tmp/'workspace'; (self.ws/'.git').mkdir(parents=True)
  (self.ws/'.epr-meta').mkdir(); (self.ws/'.epr-meta/brit-first.json').write_text('{"enabled":true}')
  (self.ws/'elohim/brit/.git').mkdir(parents=True)
  wt=self.ws/'.worktrees'/'one'; wt.mkdir(parents=True); (wt/'.git').write_text('gitdir: ../../.git/worktrees/one\n')
  self.scratch=self.tmp/'scratch-repo'; (self.scratch/'.git').mkdir(parents=True); (self.scratch/'.git/config').write_text('[core]\n\tbare = false\n')
  root=patch.object(hook,'ROOT',self.ws); root.start(); self.addCleanup(root.stop)
 def verdict(self,command):
  r=hook.evaluate({'tool_name':'Bash','tool_input':{'command':command}})
  return None if r is None else r.get('permissionDecision','context')
 def assertPasses(self,commands):
  for c in commands:
   with self.subTest(command=c): self.assertIsNone(self.verdict(c))
 def assertRefused(self,commands):
  for c in commands:
   with self.subTest(command=c): self.assertEqual(self.verdict(c),'deny')

 def test_a_scratch_repository_named_by_absolute_C_passes(self):
  s=self.scratch
  link=self.tmp/'link-to-scratch'; link.symlink_to(s)
  self.assertPasses([f'git -C {s} status',f'git -C {s} add a.txt && git -C {s} commit -m fixture',
   f'git -C {s} reset --hard HEAD~1',f'git -C {s} stash list',f'git -C {s} log -n 1 stash@{{0}}',
   f'git -C {link} status',f'git -C {self.tmp} init fixture'])
 def test_data_heredocs_pass(self):
  t=self.tmp
  self.assertPasses([
   f"cat > {t}/brief.md <<'EOF'\nquote `brit show X | git apply` and git push origin HEAD:dev\nEOF",
   f"cat <<'EOF' > {t}/brief2.md\ngit reset --hard\nEOF",
   f"tee {t}/notes.md <<'EOF'\n- git stash is absent; git reset too\nEOF",
   f"cat > {t}/a.md <<'EOF'\ngit status\nEOF\nbrit status --no-write",
   # $"…" quotes as "…" does, so an apostrophe inside it does not open a quote.
   f"echo $\"it's\" ; cat > {t}/b.md <<'EOF'\nprose about git push\nEOF",
   'brit show a:b | head','brit show HEAD:x | wc -l'])

 def test_dev_refused_stays_refused(self):
  ws,s,t=self.ws,self.scratch,self.tmp
  table={
   'submodules are this workspace':[
    'git -C elohim/brit push','cd elohim/rakia && git reset --hard',f'git -C {ws}/elohim/brit push',
    f'git -C {ws}/.worktrees/one reset --hard'],
   'a heredoc fed to a shell is scanned':[
    "sh <<'EOF'\ngit reset --hard\nEOF","bash -s <<'EOF'\ngit reset --hard\nEOF",
    "cat <<'EOF' | sh\ngit reset --hard\nEOF","python3 - <<'EOF'\ngit reset --hard\nEOF",
    f"cat > {t}/x.sh <<'EOF'\ngit reset --hard\nEOF\nbash {t}/x.sh",
    f"cat > {t}/y.sh <<'EOF' && bash {t}/y.sh\ngit reset --hard\nEOF",
    f"cat > {t}/z.sh <<'EOF'\ngit reset --hard\nEOF\nchmod +x {t}/z.sh; {t}/z.sh",
    "tee >(sh) <<'EOF'\ngit reset --hard\nEOF",f"cat > {t}/p.md <<'EOF' | sh\ngit reset --hard\nEOF",
    f"cat <<EOF > {t}/u.md\ngit reset --hard\nEOF",f"(cat > {t}/q.md <<'EOF')\ngit reset --hard\nEOF"],
   'a comment ends the line before any heredoc':[
    "# note <<'EOF'\ngit push origin HEAD:dev","echo hi # <<'EOF'\ngit push origin HEAD:dev\nEOF"],
   'quotes span lines':['echo "\ncat > f <<\'EOF\'\n" ; git push origin HEAD:dev\nEOF'],
   # The integrator's reader's construction: an escaped quote inside ANSI-C quoting must not end it,
   # or the heredoc it seems to open swallows a git call bash runs.
   'ANSI-C quoting cannot hide a call':[
    "echo $'\\'' '; cat > f <<\\EOF ; $'\\'' '\ngit push origin HEAD:dev\nEOF"],
   'cd and pushd are never tracked':[
    f'cd {s} || git push',f'cd {s} & git push',f'(cd {s}) && git push',f'pushd {s} && git reset --hard',
    f'cd {s} && cd - && git reset --hard',f'cd {s} && cd && git status',f'cd {s} && git status'],
   'an unexpanded path is not literal':[
    'cd "$VAR" && git status','git -C "$SCRATCH" status','git -C ~/x status',f'git -C {t}/scr* status',
    f'git -C $(echo {s}) status'],
   'an outside repository cannot reach this one or a remote':[
    f'git -C {s} push {ws} HEAD:dev',f'git -C {s} push origin HEAD:dev',f'git -C {s} fetch origin',
    f'git -C {s} pull',f'git -C {s} remote add w {ws}',f'git -C {s} submodule update',f'git -C {s} clone x',
    f'git -C {s} ls-remote origin',f'git -C {s} config core.worktree {ws}',f'git -C {s} worktree add {ws}/x',
    f'git -C {s} apply --directory=../../workspace p.diff',f'git -C {s} diff --output={ws}/x',
    f'git -C {s} add {ws}/../workspace/file',f'git -C {s} log --format=x https://example.com/r',
    f'git -C {s} rebase --exec "git -C {ws} reset --hard" main',f'git -C {s} rebase -x true main',
    f'git -C {s} -c core.worktree=/y status',f'git -C {s} -C ../workspace status',
    f'GIT_DIR={ws}/.git git -C {s} status',f'export GIT_WORK_TREE={ws}; git -C {s} status',
    f'git -C {s} --git-dir={ws}/.git status',f'git -C {s} status && git status'],
  }
  for cls,commands in table.items():
   with self.subTest(cls=cls): self.assertRefused(commands)
 def test_a_scratch_directory_tied_to_the_workspace_is_policed(self):
  ws,t=self.ws,self.tmp
  worktree=t/'wt'; worktree.mkdir(); (worktree/'.git').write_text(f'gitdir: {ws}/.git/worktrees/wt\n')
  linked=t/'linked'; linked.mkdir(); (linked/'.git').symlink_to(ws/'.git')
  configured=t/'configured'; (configured/'.git').mkdir(parents=True)
  (configured/'.git/config').write_text(f'[core]\n\tworktree = {ws}\n')
  included=t/'included'; (included/'.git').mkdir(parents=True); (included/'.git/config').write_text('[include]\n\tpath = /x\n')
  bare=t/'no-repository'; bare.mkdir()
  alias=t/'alias'; alias.symlink_to(ws)
  # The integrator's reader's construction: a commondir makes the git directory a worktree of the
  # workspace repository, directly or behind a gitdir: file to a scratch directory.
  (ws/'.git/refs/heads').mkdir(parents=True)
  common=t/'common'; (common/'.git').mkdir(parents=True); (common/'.git/HEAD').write_text('ref: refs/heads/dev\n')
  (common/'.git/commondir').write_text(f'{ws}/.git\n')
  gd=t/'gd'; gd.mkdir(); (gd/'HEAD').write_text('ref: refs/heads/dev\n'); (gd/'commondir').write_text(f'{ws}/.git\n')
  via=t/'via'; via.mkdir(); (via/'.git').write_text(f'gitdir: {gd}\n')
  borrowed=t/'borrowed'; (borrowed/'.git/objects/info').mkdir(parents=True)
  (borrowed/'.git/objects/info/alternates').write_text(f'{ws}/.git/objects\n')
  nested=t/'nested'; (nested/'.git/refs').mkdir(parents=True); (nested/'.git/refs/heads').symlink_to(ws/'.git/refs/heads')
  dirs=(worktree,linked,configured,included,bare,alias,common,via,borrowed,nested)
  self.assertRefused([f'git -C {d} status' for d in dirs]+[f'git -C {common} commit --allow-empty -m x',
   f'git -C {via} update-ref refs/heads/dev HEAD'])
 def test_a_scratch_root_inside_the_workspace_is_not_scratch(self):
  inner=self.ws/'tmp'; (inner/'repo/.git').mkdir(parents=True)
  with patch.dict(os.environ,{'TMPDIR':str(inner)}):
   self.assertRefused([f'git -C {inner}/repo status'])
 def test_a_locating_variable_in_the_environment_removes_the_exemption(self):
  with patch.dict(os.environ,{'GIT_DIR':str(self.ws/'.git')}):
   self.assertRefused([f'git -C {self.scratch} status'])
  with patch.dict(os.environ,{'GIT_PAGER':'cat'}):
   self.assertPasses([f'git -C {self.scratch} status'])

 def test_unreadable_with_a_git_token_is_refused(self):
  s=self.scratch
  self.assertRefused(['echo "unbalanced && git status',"git status <<'EOF'\nno end",
   f'echo $((1<<2)); git -C {s} status',"bash -c 'echo \"x'; git status",
   # Dev passes this one (its tokenizer cannot read ANSI-C quoting and so sees no stage); now the body
   # is never data when $' appears, and the git call in it is refused.
   f"echo $'it\\'s' ; cat > {self.tmp}/c.md <<'EOF'\ngit push origin HEAD:dev\nEOF"])
  self.assertPasses(['echo "unbalanced and no token','brit status --no-write'])
 def test_an_internal_error_with_a_git_token_is_refused(self):
  with patch.object(hook,'_exempt',side_effect=OSError('boom')):
   self.assertRefused([f'git -C {self.scratch} status'])
 def test_an_undecodable_event_with_a_git_token_is_refused(self):
  for raw,expected in [(b'{not json git status',"deny"),(b'{not json at all',None)]:
   with self.subTest(raw=raw):
    out=io.StringIO()
    with patch.object(hook.sys,'stdin',SimpleNamespace(buffer=io.BytesIO(raw))), redirect_stdout(out):
     hook.main()
    payload=json.loads(out.getvalue())['hookSpecificOutput']
    self.assertEqual(payload.get('permissionDecision'),expected)
 def test_a_fallback_line_still_passes_a_refused_class(self):
  self.assertEqual(self.verdict("# Brit fallback: fixture needs a reset brit lacks\nsh <<'EOF'\ngit reset --hard\nEOF"),'context')

if __name__=='__main__':
 unittest.main()
