#!/usr/bin/env python3
"""rust-style hook, shell branch: speak when the command names cargo as a command word, or names a
.rs token and is not provably read-only. Only an allowlisted reader with no redirection stays silent."""
import importlib.util
from pathlib import Path
import time
import unittest

HOOK = Path(__file__).resolve().parents[1] / "rust-style.py"
spec = importlib.util.spec_from_file_location("rust_style", HOOK)
mod = importlib.util.module_from_spec(spec)
spec.loader.exec_module(mod)

EMIT = [
    # cargo as a command word, in any position or wrapper
    "cargo test",
    "cargo --locked test",
    "cargo -q test",
    "cargo -C crate build",
    "cargo t -p x",
    "/usr/bin/cargo test",
    "{ cargo test; }",
    "if true; then cargo test; fi",
    "for d in a b; do cargo test; done",
    "! cargo test",
    "xargs cargo fmt",
    "echo `cargo test`",
    "echo $(cargo +nightly fmt)",
    "bash <<'EOF'\ncargo test\nEOF",
    "bash -c 'cargo test' argv0",
    "CARGO_TARGET_DIR=/x cargo test",
    "timeout 60 cargo clippy",
    "cd crate && nice -n 5 cargo build --release",
    "$CARGO build",
    "${CARGO} test",
    # redirections, numbered or not
    "echo x > src/a.rs",
    "echo x &> src/a.rs",
    "echo x >| src/a.rs",
    "echo x 1>src/a.rs",
    "echo x 2>src/a.rs",
    "exec 3>src/a.rs",
    "ls src/*.rs > /tmp/list.txt",
    "cat >> src/b.rs <<'EOF'\nfn main() {}\nEOF\n",
    # interpreters and heredocs
    "python3 -c \"open('src/a.rs','w').write('x')\"",
    "python3 - <<'EOF'\nfrom pathlib import Path\nPath('src/a.rs').write_text('x')\nEOF",
    "ruby -i -pe 1 src/a.rs",
    "perl -pi -e s/a/b/ src/a.rs",
    "perl -i.bak -pe s/a/b/ src/a.rs",
    # writing subcommands of an allowlisted program
    "git checkout other -- src/a.rs",
    "git restore --source=other src/a.rs",
    "git branch new-branch src/a.rs",
    "brit restore src/a.rs",
    "gawk -i inplace 1 src/a.rs",
    "sed -i 's/a/b/' src/main.rs",
    "pnpm install; sed -i 's/a/b/' src/lib.rs",
    "cp a b; sed -i s/x/y/ src/a.rs",
    "sed -i.bak s/a/b/ src/a.rs",
    "sed -i'' s/a/b/ src/a.rs",
    "sed -i 's/foo/& bar/' src/lib.rs",
    "sed --in-place s/a/b/ src/main.rs",
    'find . -name "*.rs" -exec sed -i s/a/b/ {} +',
    'find src -name "*.rs" -delete',
    'fd -g "*.rs" -x rustfmt',
    "fd . -e rs -X rustfmt src/a.rs",
    # programs outside the allowlist
    "patch src/a.rs < fix.patch",
    "rsync /tmp/x.rs src/a.rs",
    "curl -o src/a.rs URL",
    "wget -O src/a.rs URL",
    "rustfmt src/a.rs",
    "touch src/new.rs",
    "truncate -s0 src/a.rs",
    "cp x src/a.rs",
    "mv a b; cp x src/a.rs",
    "printf hi | tee -a src/lib.rs",
    "cat a.rs | tee b.rs",
    "dd if=x of=src/a.rs",
    "xargs sed -i s/a/b/ < list.rs",
    "cp /tmp/new.txt src/c.rs",
    "mv a.txt b.rs",
    "ln -s a b.rs",
    "install -m 644 a b.rs",
    # Allowlisted readers used in a form that writes a file or runs a program.
    "sed -n 'w src/a.rs' x",
    "sed 's/a/b/w src/a.rs' x",
    "sed -f fix.sed src/a.rs",
    "sed -ni s/a/b/ src/a.rs",
    "sed --in-place=.bak s/a/b/ src/a.rs",
    "awk 'BEGIN{system(\"rustfmt src/a.rs\")}'",
    "awk -f fix.awk src/a.rs",
    "sort -o a.rs a.rs",
    "uniq in.txt src/a.rs",
    "xxd -r dump a.rs",
    "yq -i . a.rs",
    "tree -o a.rs",
    "find . -fprint a.rs",
    "find . -fls a.rs",
    "find . -name '*.rs' -okdir rm {} ;",
    "fd -e rs --exec rm a.rs",
    "rg --pre rustfmt foo src/a.rs",
    "rg --pre=rustfmt foo src/a.rs",
    "bat --pager=rustfmt src/a.rs",
    "git diff --output=src/a.rs",
    "git log -p --output=src/a.rs",
    "git grep -Orustfmt fn -- src/a.rs",
    "GIT_EXTERNAL_DIFF=./w.sh git diff -- src/a.rs",
    "cat a.rs <(rustfmt a.rs)",
    "diff <(rustfmt src/a.rs) b",
    "echo x->src/a.rs",
    "echo x=>src/a.rs",
    "\"cargo\" test",
    "'cargo' test",
    "\"$CARGO\" test",
    "${CARGO:-cargo} test",
    # Found by the adversarial probe: separators or options hidden by quoting, expansion or grouping.
    "git log --grep '; echo ' --output=out.rs -- a.rs",
    "find . -name ';echo ' -o -name '*.rs' -delete",
    "find . -name '*.rs' '-delete'",
    "find . -name '*.rs' -{delete,true}",
    "find . -name '*.rs' -rm",
    "sed -n 1p a.rs '-i'",
    "sed -n 1p a.rs \\\necho -i",
    "git log '--output=out.rs' -- a.rs",
    "git grep --open=./ev fn -- a.rs",
    "git grep -iO./ev fn -- a.rs",
    "git --config-env=diff.external=SHELL diff -- a.rs",
    "rg --hostname-bin=./ev --hyperlink-format=default fn a.rs",
    "brit revision list --svg out.rs HEAD",
    "echo fn main ->x.rs",
    "echo fn main -> x.rs",
    "echo fn main >&2x.rs",
    "cd () ( touch PWNED ); cd a.rs",
    "printf -v PATH %s ./binz:$PATH; cat a.rs",
    "file -C -m src/a.rs",
    "for f in gen/*.rs.in; do cp x y; done",
]
SILENT = [
    "grep -rn foo --include=*.rs .",
    "ls src/main.rs",
    "rg -t rust foo",
    "rg -t rust foo src/a.rs",
    "wc -l src/*.rs",
    "sed -n 1,5p a.rs",
    "head a.rs",
    "git diff -- a.rs",
    "git -C sub log --oneline -- src/a.rs",
    "git branch -vv && ls src/a.rs",
    'find . -name "*.rs"',
    "cat a.rs",
    "cat a.rs | grep foo | head -5",
    "head -5 a.rs && wc -l b.rs",
    "grep -c 'a -> b' src/a.rs",
    "grep foo src/a.rs 2>/dev/null",
    "ls src/*.rs 2>&1 | wc -l",
    "brit diff -- src/a.rs",
    "cd src && ls lib.rs",
    "cargo-pool key",
    "CARGO_TARGET_DIR=/x just gate",
    "pnpm install",
    "ls Cargo.toml",
]
# The rule defaults to emitting and silences only provably read-only commands, so a harmless
# command outside that proof speaks. These are known false positives, pinned so the choice is
# deliberate: cargo named as a word, any redirection, an interpreter, xargs, a wrapper, a writer
# whose target is not a .rs file.
ACCEPTED_FALSE_POSITIVES = [
    "grep cargo Cargo.toml",
    "grep foo a.rs > out.txt",
    "python3 x.py 2>&1 | grep a.rs",
    "cat a.rs | xargs echo",
    "timeout 5 grep foo a.rs",
    "cat a.rs | tee b.txt",
    "mv a.rs b.txt",
    "awk '{print $1}' src/a.rs",
    "sort src/a.rs",
    "RIPGREP_CONFIG_PATH=/dev/null rg foo src/a.rs",
]
BROAD = [
    ("apply_patch", "*** Begin Patch\n*** Update File: src/lib.rs\n@@\n-a\n+b\n*** End Patch"),
    ("functions.exec", 'await tools.exec_command({cmd: "cargo test"})'),
    ("functions.exec", "run('cat src/lib.rs')"),
]
APPLY_PATCH_CMD = "apply_patch <<'EOF'\n*** Begin Patch\n*** Update File: src/lib.rs\n@@\n-a\n+b\n*** End Patch\nEOF"


class ShellBranch(unittest.TestCase):
    def test_emits(self):
        for command in EMIT:
            with self.subTest(command=command):
                self.assertTrue(mod.shell_runs_rust(command))
                self.assertIsNotNone(mod.inspect(Path("/nonexistent"), {"tool_name": "Bash", "tool_input": {"command": command}}))

    def test_silent(self):
        for command in SILENT:
            with self.subTest(command=command):
                self.assertFalse(mod.shell_runs_rust(command))
                self.assertIsNone(mod.inspect(Path("/nonexistent"), {"tool_name": "Bash", "tool_input": {"command": command}}))

    def test_accepted_false_positives_emit(self):
        for command in ACCEPTED_FALSE_POSITIVES:
            with self.subTest(command=command):
                self.assertTrue(mod.shell_runs_rust(command))

    def test_apply_patch_through_shell_tool_emits(self):
        root = Path("/nonexistent")
        self.assertIsNotNone(mod.inspect(root, {"tool_name": "exec_command", "tool_input": {"cmd": APPLY_PATCH_CMD}}))

    def test_codex_shell_dict_is_narrowed_and_other_shapes_stay_broad(self):
        root = Path("/nonexistent")
        self.assertIsNotNone(mod.inspect(root, {"tool_name": "exec_command", "tool_input": {"cmd": "cargo test"}}))
        self.assertIsNone(mod.inspect(root, {"tool_name": "exec_command", "tool_input": {"cmd": "head a.rs"}}))
        for tool, text in BROAD:
            with self.subTest(text=text):
                self.assertIsNotNone(mod.inspect(root, {"tool_name": tool, "tool_input": text}))
        self.assertIsNotNone(mod.inspect(root, {"tool_name": "exec", "tool_input": {"code": "read('a.rs')"}}))

    def test_adversarial_input_is_fast(self):
        units = ("<<EOF\n", "a << 3\n", "cargo ", "tee ", "sed -i ", "> a", "cp ", "a.rs ", "-i", "install ",
                 "&> ", ";", "| ", "git ", "find ", ">", "-C ", "a=b ", "( ", "-aaaa", "inplace ")
        for unit in units:
            body = unit * (200_000 // len(unit))
            # Each body alone, and behind a .rs reader so the read-only proof runs over all of it.
            for text in (body, "cat a.rs; " + body, "git diff " + body + " a.rs", "sed " + body + " a.rs"):
                start = time.monotonic()
                mod.shell_runs_rust(text)
                self.assertLess(time.monotonic() - start, 0.3, unit)

    def test_edit_on_rs_path_unchanged(self):
        event = {"tool_name": "Read", "tool_input": {"file_path": "/nonexistent/x.rs"}}
        self.assertIsNone(mod.inspect(Path("/elsewhere"), event))  # outside root -> None, as before

    def test_other_tool_on_rs_path_still_speaks(self):
        root = Path(__file__).resolve().parents[3]
        event = {"tool_name": "Read", "tool_input": {"file_path": str(root / "x.rs")}}
        self.assertIn("applies to this crate", mod.inspect(root, event)["context"])


if __name__ == "__main__":
    unittest.main()
