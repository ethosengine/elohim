#!/usr/bin/env node
/**
 * habit-delta — S3c CLI wrapper over lib/habit-delta.ts.
 *
 * Usage:
 *   habit-delta.ts --repo-root <path> --concern <tag> --date YYYY-MM-DD --once-key <text>
 *     --text "<delta sentence>" [--label "<parenthetical>"]
 *
 * Resolves exactly one `.habit.md` whose `checks:` names `@concern:<tag>` and appends one
 * dated DELTA line to it, newest-first, directly after the closing frontmatter `---`. Prints a
 * JSON result and exits 0 on success (including the idempotent no-op case), 1 otherwise.
 */
import { resolve } from 'node:path';

import { appendDelta, findHabitByConcern } from './lib/habit-delta.js';

function parseArgs(argv: string[]): Record<string, string> {
  const out: Record<string, string> = {};
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    if (arg.startsWith('--')) {
      out[arg.slice(2)] = argv[i + 1];
      i++;
    }
  }
  return out;
}

function main(argv = process.argv.slice(2)): void {
  const args = parseArgs(argv);
  const repoRoot = resolve(args['repo-root'] ?? process.cwd());
  const concern = args.concern;
  const date = args.date;
  const text = args.text;
  const onceKey = args['once-key'];
  if (!concern || !date || !text || !onceKey) {
    console.error(
      'usage: habit-delta.ts --repo-root <path> --concern <tag> --date YYYY-MM-DD ' +
        '--once-key <text> --text "<delta sentence>" [--label "<parenthetical>"]'
    );
    process.exitCode = 1;
    return;
  }

  const found = findHabitByConcern(repoRoot, concern);
  if (!found.ok) {
    console.error(`habit-delta: ${found.reason}`);
    process.exitCode = 1;
    return;
  }

  const result = appendDelta(found.path, { date, label: args.label, text, onceKey });
  console.log(JSON.stringify({ path: found.path, ...result }, null, 2));
  if (!result.ok) process.exitCode = 1;
}

main();
