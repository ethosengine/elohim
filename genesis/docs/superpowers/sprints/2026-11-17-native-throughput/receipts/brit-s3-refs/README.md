# brit sprint 3, ref spellings lane (built on shem)

Six commits on brit `1db7a955197b10bc7672c9aa544768a329bb9008`, head `7786c8185dd0747fbdb174ddca8e2bfd9838ba98`
(branch `sprint3/refs-shem`, on shem only). Apply in the brit repository with `git am 000*.patch`.

| Patch | What |
|---|---|
| 0001 | `brit rev-parse`: Git's spelling of `brit revision resolve` |
| 0002 | `brit for-each-ref` with Git's format atoms and sort keys |
| 0003 | `brit ls-remote`: Git's spelling of `brit remote refs` |
| 0004 | `brit check-ref-format` |
| 0005 | global `-C <dir>` with Git's meaning; all `-C` apply first, then `-r` resolves from there |
| 0006 | a closed reader ends brit quietly with exit 141 (isolated, droppable) |

Shared files touched, kept local: `src/plumbing/main.rs` (three variants appended to the read-only list in
`requires_mutation_guard`, one cfg'd `LsRemote` arm, `change_dirs` and its call, four dispatch arms before
`Subcommands::Cat`), `src/plumbing/options/mod.rs` (a `pub mod`, the `-C` field, four variants before
`Completions`), `gitoxide-core/src/repository/mod.rs` (one `pub mod`), `tests/cli-journey/src/support/mod.rs`
(one line; new `support/parity.rs`), `brit-cli/src/main.rs` (0006 only). All four spellings are real clap
subcommands. No `--wait`/`-w`.

`census-transcripts.txt`: git invocation forms counted from this device's agent transcripts since 2026-11-06
(3,767 invocations, 1,340 declared fallbacks). The brit-first hook's own ledger could not rank anything: it
keeps no command text, and its `epr feedback capture` call fails with "unknown command feedback".

Built with default `max` features only; `small`, `max-pure` and `lean-async` were not compiled.

## Round 2 (2026-11-19, head `e534083f0`)

Six more commits on top of `7786c8185`; apply after 0001-0006 with `git am 0007*.patch … 0012*.patch`.

| Patch | What |
|---|---|
| 0007 | the oracle helper also compares whether stderr is empty (`assert_same_streams`) |
| 0008 | `rev-parse` reads its arguments in one ordered pass like Git's argv loop (`--verify HEAD -- x` prints one oid); failures return a typed exit so `-v` no longer loses output; `-C <missing>` fails as Git does (`fatal: cannot change to …`, exit 128) and is applied before clap's help and usage exits |
| 0009 | the closed-reader fix, narrowed, in its own commit: error KIND on a stdout/stderr write, read-only commands only; the exit code is decided in one place, `exit_status()` in `brit-cli/src/main.rs`; the text match is gone. `push.rs` untouched |
| 0010 | `ls-remote -h` means heads, `From <url>` on stderr, `--exit-code`; `for-each-ref --sort -refname` as two arguments, `%+1` literal; `check-ref-format` usage exits 129 |
| 0011 | `brit merge-base --is-ancestor` (a flag on the existing verb), exit 0 / 1 / 128 |
| 0012 | `brit rev-list --count`, `--left-right --count A...B`, over `brit revision list` |

Two things to look at when merging. (1) 0008 adds `discovery_start()` in `src/plumbing/main.rs` and changes one line to
`let repository = discovery_start(args.repository);`: before it, every brit command run from inside a `.git` directory
failed looking for `./.git/objects`. That is a behaviour change for all commands, wider than this lane. (2) The tree at
0011 alone was never built: it was split out of the 0012 tree by removing hunks; every other commit's tree was built
and tested. Only the default `max` feature set was compiled.

`review-of-0001-0006.md` is the independent review that round 2 answers.
