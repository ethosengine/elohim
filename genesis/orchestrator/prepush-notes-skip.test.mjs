// Drive the ACTUAL pushed-ref loop of .husky/pre-push.bash, against a throwaway scratch repository,
// with one notes ref and one branch ref: the notes ref is skipped (it is not a commit tree), the
// branch ref is still style-gated and change-detected.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync, mkdtempSync, mkdirSync, writeFileSync, rmSync, chmodSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const ROOT = fileURLToPath(new URL("../../", import.meta.url));
const HOOK = readFileSync(join(ROOT, ".husky/pre-push.bash"), "utf8");

function slice(from, to) {
  const a = HOOK.indexOf(from);
  const b = HOOK.indexOf(to, a);
  assert.ok(a >= 0 && b > a, `hook has the region ${JSON.stringify(from)}`);
  return HOOK.slice(a, b);
}
const NOTES_FN = slice("is_notes_ref() {\n", "\n}\n") + "\n}\n";
const LOOP = slice('CHANGED=""\nUNFILTERED_CHANGED=""', "# Keep the unfiltered union");

const cleanEnv = Object.fromEntries(
  Object.entries(process.env).filter(([k]) => !k.startsWith("GIT_") || k.startsWith("GIT_CONFIG_")),
);

test("notes_ref_is_skipped_and_branch_ref_still_gated_by_the_pushed_ref_loop", () => {
  const dir = mkdtempSync(join(tmpdir(), "prepush-notes-"));
  try {
    const git = (input, ...a) => {
      const r = spawnSync("git", a, { cwd: dir, env: cleanEnv, encoding: "utf8", input });
      assert.equal(r.status, 0, r.stderr);
      return r.stdout.trim();
    };
    git(undefined, "init", "-q", "-b", "main");
    git(undefined, "config", "user.email", "t@example.invalid");
    git(undefined, "config", "user.name", "t");
    writeFileSync(join(dir, "a.txt"), "a\n");
    git(undefined, "add", "a.txt");
    git(undefined, "commit", "-q", "-m", "base");
    const base = git(undefined, "rev-parse", "HEAD");
    git(undefined, "update-ref", "refs/remotes/origin/dev", base);
    writeFileSync(join(dir, "b.txt"), "b\n");
    git(undefined, "add", "b.txt");
    git(undefined, "commit", "-q", "-m", "feature");
    const feature = git(undefined, "rev-parse", "HEAD");
    // A notes-style commit with unrelated history: change detection against origin/dev would read it
    // as unreadable, which is exactly why the loop must not touch it.
    const emptyTree = git("", "mktree");
    const notes = git(undefined, "commit-tree", emptyTree, "-m", "notes");
    git(undefined, "update-ref", "refs/notes/brit/validation", notes);

    mkdirSync(join(dir, "genesis/orchestrator"), { recursive: true });
    const stub = join(dir, "genesis/orchestrator/rust-style.sh");
    writeFileSync(stub, '#!/usr/bin/env bash\necho "STYLE $*"\n');
    chmodSync(stub, 0o755);

    const zero = "0".repeat(40);
    const script = [
      `ZERO_SHA=${zero}`,
      NOTES_FN,
      'leg_red() { echo "LEG_RED $*"; }',
      "PUSH_LINES=(",
      `  "refs/notes/brit/validation ${notes} refs/notes/brit/validation ${zero}"`,
      `  "refs/heads/feature ${feature} refs/heads/feature ${zero}"`,
      ")",
      LOOP,
      'echo "TARGETS=[$PUSH_TARGETS] CHANGED=[$(echo $CHANGED)] UNREADABLE=[$CHANGE_UNREADABLE]"',
    ].join("\n");
    const r = spawnSync("bash", ["-c", script], { cwd: dir, env: cleanEnv, encoding: "utf8", timeout: 20000 });
    assert.equal(r.status, 0, r.stdout + r.stderr);
    const out = r.stdout;
    assert.match(out, /^notes: refs\/notes\/brit\/validation skipped \(not a commit tree\)$/m);
    assert.equal((out.match(/^notes: /gm) || []).length, 1, "one skip line for the one notes ref");
    assert.ok(!out.includes(`--head ${notes}`), "notes commit never reaches the style gate");
    assert.ok(out.includes(`STYLE --base origin/dev --head ${feature}`), "branch ref is still style-gated");
    assert.ok(!out.includes("LEG_RED"), out);
    assert.match(out, /TARGETS=\[ refs\/heads\/feature\] CHANGED=\[b\.txt\] UNREADABLE=\[\]/);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
