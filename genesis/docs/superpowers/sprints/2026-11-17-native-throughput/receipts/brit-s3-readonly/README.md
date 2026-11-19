# brit sprint 3: more read-only commands under the closed-reader rule (built on shem)

The base is `run-ci/sprint3-integrate` at `8e5072ffd35cd31d7cfaf8d31fa6b28ae53dbda1`. This lane adds five commits on
top of it, on the local branch `sprint3/read-only-ext-shem`, with head `5245423ae`. Apply them in the brit repository
with `git am 000*.patch`.

| Patch | What |
|---|---|
| 0001 | `brit-cli/src/output.rs`: build JSON and DOT output go through `closed_reader::Watch` instead of `println!` |
| 0002 | Adds the first 32 paths to `READ_ONLY_COMMANDS`. `is_closed_reader_write()` now recognises JSON wrapped in a `gix::Error`. Adds the journey `read_only_closed_reader.rs` and two unit tests |
| 0003 | `gitoxide-core/src/repository/tree.rs`: `tree info` resolves its revision before it writes its format note to stderr |
| 0004 | Narrows the list to 26 paths, each left-out path with its reason beside the list. For a read-only command, stdout is flushed through the watched writer before the exit status is chosen. Adds journey rows for the review's cases |
| 0005 | Readmits `remote refs` and `remote ref-map` alongside `ls-remote`, with the reasoning and the remaining gap in a comment beside the list. Restores their journey rows (local `file://` remote) and lists them as admitted in the unit test |

Of the 32 paths in 0002, 0004 removes six and 0005 puts two of them back. **28 paths are admitted by this lane**: 22
plumbing paths and 6 build paths.

`brit-cli/src/main.rs` changes in three places:
- the list and its tests;
- one branch in `is_closed_reader_write()`, because `tree info` writes JSON through `.or_error()`;
- the flush in `main()` between `run()` and `exit_status()`.

The admission rule for each command was (a) it writes nothing, though a network read is allowed, and (b) everything it
prints goes through the watched writer. I applied it by reading each command's code path, and checked it with the
probe in `before-after.txt`.

| Candidate | Decision | Why |
|---|---|---|
| `show` | ADMIT | `show.rs` writes no index, ref or file and prints through `prepare_and_run`. Its rendering uses temporary scratch. (The `objects.refresh` setting the brit session mentioned is not in `show.rs`; I did not check further) |
| `tag list` | ADMIT | It reads refs only |
| bare `tag` | LEFT OUT | `tag <name>` on the same path creates a tag |
| `remote url` | ADMIT | It reads config and opens no connection |
| `remote refs`, `remote ref-map` | ADMIT (0005, decided with the brit session) | These and `ls-remote` are one function under three spellings and are treated alike. Their only effect is the credential helper's: approve after a successful authentication, erase after a failed one. After an erase the command has failed, and its own failure wins. After an approve the command otherwise succeeded, and Git dies of SIGPIPE in the same pipeline having done the same thing. Gap: output written by a helper or a prompt during authentication does not go through the watched writer, so on that path a closed reader may go unseen |
| `commit describe` | LEFT OUT (by review) | Its `--dirty-suffix` on the same path runs the status engine, which can run a clean filter |
| `commit verify` | LEFT OUT | It runs the configured verifier and writes the verifier's output to an unwatched stderr |
| `revision previous-branches` | ADMIT | It reads the reflog |
| `revision list` | LEFT OUT | Its `--svg` on the same path writes and opens a file |
| `verify`, `fsck` | ADMIT | Reads only |
| `tree entries` | ADMIT | Read |
| `tree info` | FIXED (0003) | It now resolves the revision before writing its note |
| `commit-graph list`, `commit-graph verify` | ADMIT | Reads |
| `mailmap entries`, `mailmap check` | ADMIT | Reads |
| `attributes query`, `exclude query` | ADMIT | Reads |
| `attributes validate-baseline` | LEFT OUT | Its children's output is piped. But it reports a failed helper, or a mismatch, with a fallible write before it returns its own failure, so a closed reader would replace that failure |
| `submodule` | ADMIT | The bare form passes no `--dirty-suffix` |
| `submodule list` | LEFT OUT (by review) | Its `--dirty-suffix` runs the status engine |
| `is-clean`, `is-changed` | LEFT OUT (by review) | They are predicates: their answer is their exit status, so a 141 would replace the answer. Their status engine can also run a clean filter with inherited stderr |
| `config`, `config show`, `config list` | ADMIT | Bare `config` runs `show` |
| `config fmt` | LEFT OUT | It writes |
| `index entries`, `dirwalk` | ADMIT | Reads |
| `build graph discover`, `build graph show`, `build affected`, `build plan`, `build fingerprint`, `build baseline read` | FIXED (0001), then admitted | `println!` panicked with 101 on a closed reader. They read only (`fingerprint` uses temporary scratch) |
| `build run`, `build baseline write`, `build baseline migrate` | LEFT OUT | They run tasks or write refs |

Known limits, recorded beside the list and not fixed:
- `--trace` and the verbose progress renderer write to raw stderr, outside the watched writer. This is shared plumbing where three lanes meet.
- A streaming query stops at its first failed write, even if a later input would have failed. Git, killed by SIGPIPE, does the same.

`before-after.txt` (regenerated at `5245423ae`, 29 rows) gives every printing row of the journey, run on the base binary (`8e5072ff`, "before") and the head
("after") with stdout closed before start. It also says whether live stdout and status are identical; they are,
except `build plan`, whose `generatedAt` timestamp differs between any two runs.

The new rows, measured on the binary built at `1cceb1264` and then at `1e321e7a1`:

| Row | 1cceb1264 | 1e321e7a1 |
|---|---|---|
| `--no-verbose tree info no-such-revision`, stderr closed | 141 | 1, its own status (live: 1) |
| `--no-verbose show HEAD:nonl` (short blob, no newline), stdout closed | 0 | 141, stderr empty |
| `--no-verbose -f json revision previous-branches` (whole output `[]`), stdout closed | 0 | 141, stderr empty |
| `--no-verbose tree info` (human note on stderr), stderr closed | 141 | 141 |
| `--no-verbose mailmap entries` (human note on stderr), stderr closed | 141 | 141 |

The journey fails on the `1cceb1264` binary at the blob row and the `[]` row.

Proof at `5245423ae` (default `max` features, each exit 0): `cargo fmt -p brit-cli -p cli-journey --check`,
`cargo clippy -p brit-cli -p cli-journey --all-targets -- -D warnings`, `cargo test -p brit-cli`, and the
`read_only_closed_reader` and `ls_remote` journeys.

Proof at `1e321e7a1`, default (`max`) features, each exit 0:
- `cargo fmt -p brit-cli -p cli-journey --check`, plus `rustfmt --check` on `tree.rs`
- `cargo clippy -p gitoxide-core -p brit-cli -p cli-journey --all-targets -- -D warnings`
- `cargo test -p brit-cli`
- the journey suite: `--lib`, `cli_surface`, `brit`, `daily_driver`, `rev_parse`, `for_each_ref`, `ls_remote`,
  `check_ref_format`, `change_dir`, `broken_pipe`, `merge_base_is_ancestor`, `rev_list`, `read_only_closed_reader`
  and `rakia`. `memory_build` was not rerun this round; it passed at `1cceb1264`

`max-pure` was compiled at `1cceb1264` (`cargo check`, exit 0) but not this round.

The journey runs on Linux only, because it closes the pipe with `libc`, as `broken_pipe.rs` does.
