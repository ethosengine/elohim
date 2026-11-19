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
