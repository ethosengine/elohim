import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { CHANGE_ID_RE, parseChangeId } from './commit-tag-parser.mjs';

export { CHANGE_ID_RE, parseChangeId } from './commit-tag-parser.mjs';

export function changeIdFor({ tree, parent, author, committer, message }) {
  const material = `tree ${tree}\nparent ${parent}\nauthor ${author}\ncommitter ${committer}\n\n${message}`;
  return `I${createHash('sha1').update(material).digest('hex')}`;
}

export function stampDecision(message, parsed = parseChangeId(message)) {
  if (/^(fixup|squash|amend)! /.test(message.split('\n', 1)[0])) {
    return { stamp: false, why: 'fixup/squash: the target keeps its trailer' };
  }
  if (!message) return { stamp: false, why: 'empty message' };
  if (parsed.id) return { stamp: false, why: 'Change-Id present' };
  if (
    /^malformed/.test(parsed.why || '') ||
    /^[2-9]\d* Change-Id trailers$/.test(parsed.why || '')
  ) {
    return { stamp: false, why: parsed.why };
  }
  return { stamp: true, why: null };
}

export function stamp(
  msgFile,
  { cwd = process.cwd(), env = process.env, spawn = spawnSync, read = readFileSync } = {},
) {
  const git = (args, extra = {}) => {
    let result;
    try {
      result = spawn('git', args, { cwd, env, encoding: 'utf8', ...extra });
    } catch (error) {
      return { error: `git ${args[0]} failed: ${error.message}` };
    }
    if (result.error || result.status !== 0) {
      const detail = (result.stderr || result.error?.message || `exit ${result.status}`)
        .trim()
        .split('\n')[0];
      return { error: `git ${args[0]} failed: ${detail}` };
    }
    return { stdout: (result.stdout || '').trimEnd() };
  };

  const strip = bytes => git(['stripspace', '--strip-comments'], { input: bytes });
  const first = strip(read(msgFile, 'utf8'));
  if (first.error) return { stamped: null, why: first.error };
  const message = first.stdout;
  const decision = stampDecision(message, parseChangeId(message));
  if (!decision.stamp) return { stamped: null, why: decision.why };

  const tree = git(['write-tree']);
  if (tree.error) return { stamped: null, why: tree.error };
  let parent = git(['rev-parse', '--verify', '-q', 'HEAD^{commit}']);
  if (parent.error) {
    parent = git(['hash-object', '-t', 'tree', '/dev/null']);
    if (parent.error) return { stamped: null, why: parent.error };
  }
  const author = git(['var', 'GIT_AUTHOR_IDENT']);
  if (author.error) return { stamped: null, why: author.error };
  const committer = git(['var', 'GIT_COMMITTER_IDENT']);
  if (committer.error) return { stamped: null, why: committer.error };
  const id = changeIdFor({
    tree: tree.stdout,
    parent: parent.stdout,
    author: author.stdout,
    committer: committer.stdout,
    message,
  });
  const write = git([
    'interpret-trailers',
    '--in-place',
    '--where',
    'end',
    '--if-exists',
    'doNothing',
    '--trailer',
    `Change-Id: ${id}`,
    msgFile,
  ]);
  if (write.error) return { stamped: null, why: write.error };
  const confirmed = strip(read(msgFile, 'utf8'));
  if (confirmed.error) return { stamped: null, why: confirmed.error };
  const parsed = parseChangeId(confirmed.stdout);
  if (parsed.id !== id) {
    return { stamped: null, why: `trailer write did not take: ${parsed.why}` };
  }
  return { stamped: id, why: null };
}

if (process.argv[1]?.endsWith('gate-change-id.mjs')) {
  const [, , command, msgFile] = process.argv;
  if (command === 'stamp' && msgFile) {
    try {
      const result = stamp(msgFile);
      const silent = [
        'fixup/squash: the target keeps its trailer',
        'empty message',
        'Change-Id present',
      ].includes(result.why);
      if (!result.stamped && !silent) {
        process.stderr.write(`[change-id] not stamped: ${result.why} (cure: git commit --amend)\n`);
      }
    } catch (error) {
      process.stderr.write(
        `[change-id] not stamped: ${error.message} (cure: git commit --amend)\n`,
      );
    }
  }
}
