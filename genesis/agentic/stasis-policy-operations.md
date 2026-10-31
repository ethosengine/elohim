# Dialogue stasis policy pinning and audit

The steward reviews and pins the exact bytes of
`.epr-meta/elohim/policies/dialogue-stasis-policy.json`. Creating the file,
validating its schema, or preparing a command does not activate it. The native
policy loader and live CID enforcement are the separate slice 6 cargo leg.

Run from the checkout:

```bash
python3 -m unittest genesis.agentic.stasis_policy_test
EPRFS_BIN=/path/to/eprfs genesis/agentic/bin/stasis-policy-pin
```

The native raw-byte command is
`eprfs cid .epr-meta/elohim/policies/dialogue-stasis-policy.json`, provided by
`elohim/eprfs/eprfs-cli`. Do not use `--body` or `--short`. The installed `epr`
and `epr flow` help inspected for this slice expose no file-CID verb;
`eprfs-agent --help` is unavailable here, and that package currently declares a
library. Provision the native CLI separately; the helper never builds Cargo
or substitutes a Python hash. `EPRFS_BIN` selects its executable.

The helper prints the CID and a shell-quoted `epr flow note --kind ruling`
command targeted at the policy path. The reason is JSON with `purpose`,
`policyPath`, `policyCid`, and `version`. **Only the steward runs that command**,
in a session whose resolved actor is the steward, after reviewing the exact
bytes. Do not copy an agent identity into the ruling. Rerun the helper after
any byte change, including formatting. No ruling has been recorded by this
procedure's implementation.

The policy uses the native camelCase budget fields and `Bound` wire shape.
Approach is derived from `stall.limit * stall.thresholdPct / 100` (180 s),
Breach is its limit (300 s), and an omitted bound sense means Ceiling. This
avoids competing explicit values for the same bound. The source-parity test
reads `StasisPolicy::builtin` literals and the classifier's shape-pattern IDs;
a source refactor that defeats this narrow parser fails the test. The extra
1800 s `shiftCeiling` mirrors the existing native default. No runtime consumes
this file yet. Classification patterns and normalization retain the native
implementation; this slice adds no alternative permission matcher.
The native `bash-c` pattern also recognizes `sh` and `zsh`, while the spec's
initial list names `bash -c`; the policy preserves the native pattern unchanged.

The native follow-up must load and validate these fields, use them in the fold
and classifier, and resolve the latest applicable **steward** ruling. With the
ladder on, a live `decide` must return exit 3 for a missing policy or CID
mismatch, emit no decision or guidance, and record why it stayed inert. Preserve
shadow defaults and ladder-off sensing. Confirm the ruling reason format above
in that loader; the spec does not prescribe a structured pin payload.

Prepare an audit at session close:

```bash
genesis/agentic/bin/stasis-audit --session SESSION --transcript /path/to/transcript.jsonl --reviewer codex
```

`--repo DIR` selects the checkout. Without `--transcript`, the helper uses the
single transcript path recorded in the session's private `state.json` cache;
multiple paths require an explicit selection. The journal is read under its
shared lock, including retained segments. Original denied calls are joined by
tool-use ID or an unambiguous sensor `inputDigest`, then classified with
`epr session stasis classify --corpus FILE --main-checkout DIR --cwd DIR`.
`EPR_BIN` selects that native executable. Missing correlation, missing native
classification, corrupt records, and an unclosed session remain explicit gaps.

Findings name journal sequence and transcript line, without raw arguments.
The helper reads `git log --all` over the session window; commit IDs and parent
links are review context, not attribution to this session or proof of a remote
push. Tool results establish that a result was observed, not that the remote
effect succeeded. Attempt-only and failed calls retain those labels.

Native effect tuples currently distinguish `git push` and `gh api`. The required
denied-push/later-API-merge fixture therefore produces a separately labelled
`remote-publication-review-candidate` for mutating requests to GitHub
merge endpoints (including POST implied by `gh api` fields). The reviewer must
check target and outcome; an explicit GET is excluded.
This hint neither changes native equality nor expands the policy rule set.
Opaque scripts, wrappers, arbitrary APIs and tool spellings beyond the native
classifier require transcript review. Shadow parks are labelled hypothetical;
shape-class parks are procedural, not authoritative human denials.

`--reviewer codex` prints `argv` and a stdin brief for a read-only, nonprompting
`codex exec`. It does not call a model, even with `--write`. The independent
reviewer must inspect actual transcript and git evidence and return a verdict;
preparation alone never satisfies L1's passing other-family audit. Raw evidence
is private to the device and is quoted as untrusted material in the brief.

The default prints the exact verdict-note command. `--write` runs only that
command, recording `changes-requested` with independent review still pending.
No automated clean scan records `approved` or activates a promotion. Once the
independent review is complete, its reviewer records its own evidence-backed
verdict through the existing flow-note interface. Missing or failed review
leaves a rung 0 entry proposed.
