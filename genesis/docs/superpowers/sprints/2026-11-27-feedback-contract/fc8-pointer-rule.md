---
id: feedback-contract-fc8-pointer-rule
title: "Feedback contract, FC-8 — where a pointer to the contract can be shown before a write, and where it cannot"
date: 2026-11-28
status: OPEN
steward: human:matthew
author: shem session 3d494616 (Claude Fable 5.2), orchestrating; mechanism read from the code by Codex (astra); citations opened by a Claude Haiku subagent; prose by a Claude scribe agent
serves: [dev-system-equilibrium]
cites:
  - "feedback-loop-contract-plan | the plan whose station FC-8 this note answers, and whose condition it finds cannot be met with the rule vocabulary as it stands | sha256:8b44cca1e3eb8aee | path: genesis/docs/superpowers/plans/2026-11-27-feedback-loop-contract-plan.md"
  - "feedback-as-a-lifecycle-item-design | the design the added pointer sends a session to, and whose declared term list is where a list of failure names is found | sha256:02c6c9be76a9434f | path: genesis/docs/superpowers/specs/2026-11-26-feedback-as-a-lifecycle-item-design.md"
  - "feedback-contract-fc7-producer-checklist | the checklist the added point in the ledger rule names as what to read before a new ledger's first row | sha256:acee0872ee71078a | path: genesis/docs/superpowers/sprints/2026-11-27-feedback-contract/fc7-producer-checklist.md"
  - "epr-meta-compose-gate | the design of the edit-time resolver whose matcher, classes and cascade this note reads to say what a rule can and cannot see | sha256:6052ce071bfec509 | path: genesis/docs/superpowers/specs/2026-06-25-epr-meta-compose-gate-design.md"
---

# FC-8: where a pointer to the contract can be shown before a write, and where it cannot

## The station

Station FC-8 of the feedback loop contract plan: a session that is about to create a log, an error list, a findings ledger or a store is pointed at the contract before it writes. The plan names the means: a co-located `.epr-meta` rule that adds a pointer on the edit. A rule that refuses is not proposed. The verify condition: an edit that adds a new append-only ledger under `.claude/data/`, or a new closed list of failure names in a first-party crate, shows the pointer once per session, and an edit that does neither does not.

## The finding

The condition is not met and cannot be met with the rule vocabulary as it stands. Of its two cases, one is met in part by one sentence added to a rule that already existed, and the other cannot be written as a rule today. Three things the condition asks for are absent from the mechanism: showing a pointer once per session, telling text an edit adds from text already in the file, and seeing a file that a script creates. A fourth is a matter of placement: no single manifest reaches every first-party crate. The station stays open.

## How it was made, and by whom

Codex (astra) read the resolver, its engine, the authoring guide and the tests, read-only, on this work's branch before it was rebased onto dev; the files it read are unchanged on dev at `bf76a01d91`. A Claude Haiku agent that had not written the inventory opened all 54 of its citations: 50 supported their sentence, 4 supported it in part (a line number off by a few lines, or a line that showed less than was claimed), none failed and none was missing. The designing session then confirmed facts 3 to 6 against the code itself, made the one change described under "The ledger case", and drove the hook by hand for the four cases tabled there. No gate was run.

## The mechanism, in six facts

| # | Fact | Where it is in the code |
|---|---|---|
| 1 | The resolver runs before `Edit` and `Write` tool calls only. | `.claude/settings.json:250` |
| 2 | It is given the tool name, the file path and, for a Write, the content; for an Edit it reads the file and builds the whole text as it would be after the edit. The rule matcher receives that text, the path and whether the file is new. It is not given the text before the edit beside the text after. | `.claude/hooks/epr-meta-resolver.py:348`, `:375`, `:437` |
| 3 | A rule's `write` pattern is matched against the file's name alone, ignoring case, so a pattern with a directory in it matches nothing. | `.claude/scripts/_lib/epr_meta.py:389` |
| 4 | "New" means a Write to a path where no file exists. An Edit is not new. | `.claude/hooks/epr-meta-resolver.py:370`, `.claude/scripts/_lib/epr_meta.py:395` |
| 5 | Content is matched by literal substring (`contains`, `contains-any`). There is no regular expression and no notion of a list or an enum. | `.claude/scripts/_lib/epr_meta.py:397` |
| 6 | The resolver's own debounce (four hours, state in `.claude/data/epr-meta-advice.json`) is applied to advice from a line-count measure. A plain advisory rule (class `inject`) is shown on every matching write. | `.claude/hooks/epr-meta-resolver.py:47`, `:92`, `:448` |

Rules accumulate up the directory tree until a manifest marked `root: true`, and the nearest rule with a given id wins (`.claude/scripts/_lib/epr_meta.py:158`, `:179`). The advisory class adds context and refuses nothing; the classes in order of weight are inject, dispatch, ask, deny (`.claude/skills/elohim-epr-metafile/SKILL.md:83`).

## The ledger case

`.claude/data/.epr-meta` already carried an advisory rule, `new-ledger-declares-mode-reader-and-drain`, matching a new `*.jsonl` file in that directory (`.claude/data/.epr-meta:34`). Before this station it asked four things of a new ledger: reach the shared row primitive `.claude/scripts/_lib/findings_ledger.py`; declare whether the ledger is a live set or append-only; name its reader by path; declare a forgetting rule and a pin.

This station added a fifth point to that rule's `why:` text and no new rule: ask whether the feedback loop already carries it. The added text says that if a row is a tool saying what went wrong for whoever ran it, the device has a store for that, `epr feedback` (the device's private record of what a tool observed about its own runs, which the lifecycle design describes), and it names two documents to read before the first row: the producer checklist (`fc7-producer-checklist.md` in this sprint directory) and the lifecycle design. One rule was extended, not a second one added with the same match, so that a new ledger gets one piece of advice.

Evidence from driving the hook by hand:

| Case | What the hook returned |
|---|---|
| A Write of a new `.jsonl` under `.claude/data/` | The rule's advice, naming both documents once each |
| An Edit of an existing `.jsonl` there | Nothing |
| A Write of a new `.json` there | Nothing |
| A second new `.jsonl` in the same session | The advice again |

The target files named in these calls did not exist and were not created, since the hook reads the call and writes no file. The manifest's own health check returned no finding, and the five resolver harnesses under `.claude/scripts/_lib/__tests__/` exited 0.

Where this falls short of the condition:

- It is shown for each new ledger file, not once per session (the last case in the table).
- It matches any new `.jsonl`, append-only or not.
- It does not match a store in another form (a `.json` file, a database, a directory of files).
- It is shown only when the file is born by a Write tool call. A ledger in that directory is machine-written, so its usual birth is a script's first append, which no hook sees. The edit that decides such a ledger is the edit to the script that will write it, and the matcher cannot tell an edit that adds a new ledger name to a script from any other edit to a file that already names one (fact 2).

## The failure-list case

No rule can meet it. A closed list of failure names is content, and the matcher has substrings only (fact 5). It cannot tell a list from a comment that uses the same word, and it cannot tell a list the edit adds from one the file already had (fact 2).

The nearest rule that can be written today: an advisory on a new `*.rs` file whose content contains one of a few literal strings such as `Error {` or `Refusal`. It would be shown for most new Rust modules that define an error type, whether or not they are a list of names a tool reports. It would be shown for no list added to an existing file, which is the commoner case. It would repeat for each such file. It was not written. The authoring guide's own test for a rule is that it names a drift seen in that directory and is the lightest signal that converges (`.claude/skills/elohim-epr-metafile/SKILL.md`), and a rule that is shown at that rate for that little is advice a session learns to skip.

Placement is a separate obstacle. First-party Rust lives under at least six roots: `elohim/`, `bridges/`, `crates/`, `doorway/`, `steward/` and `genesis/orchestrator/rust-style/`. One manifest at the repository root does not reach all of them, because a manifest marked `root: true` ends the walk upward: `bridges/did/.epr-meta:4` is one. `elohim/.epr-meta:8` also says that submodules are outside its cascade. The guide's answer for a rule wanted in several directories is a policy in the registry `.claude/epr-meta/policies.yaml`, bound from each manifest with a version pin.

## What is missing, as nodes

| # | Missing node | Between | What it would let the station check | State |
|---|---|---|---|---|
| 1 | An advisory rule can ask to be shown once per session | The resolver's debounce, which exists for line-count advice, and a plain `inject` rule | "shows the pointer once per session" | Not built; the debounce is keyed by rule and path and lasts four hours, and nothing in it is keyed by session |
| 2 | A matcher for what an edit adds | The Edit call's `old_string` and `new_string`, which the hook receives, and the rule matcher, which receives only the text after the edit | A new list in an existing file; a new ledger name in an existing script | Not built |
| 3 | A recognised shape for "a closed list of failure names" | A substring and the thing it is meant to find | The failure-list case at all | Not built, and not a matcher's work alone: the lifecycle design's answer is that a producer declares its list, so the list is found by its declaration and not by reading source |
| 4 | A birth that no tool call makes | A script's first append and any hook | A ledger created by the script that owns it | Outside the hook's reach; the existing rule's own `retire-when` names the place it could be seen, which is the row primitive (`findings_ledger.py`, the module the rule asks every ledger to append through), once no ledger can be opened except through it |
| 5 | One home that reaches every first-party crate | The registry and the root manifest of each tree | The failure-list case in every crate | Not built; needs a policy and a binding in each root manifest |

None of the five is proposed here as work; whether the station is worth new resolver code is a question for the plan's steward, and the lighter reading is that node 3 moves the failure-list case out of this station and into the declaration (station FC-2 of the same plan, where a producer ships its list of the names it can report).

## What checks a rule

Five harnesses, plain scripts with no test runner: `epr_meta_eval_test.py`, `epr_meta_resolver_test.py`, `epr_meta_cascade_test.py`, `epr_meta_schema_test.py`, `epr_meta_examples_test.py`, all under `.claude/scripts/_lib/__tests__/`. Pre-push runs them when a path under `.claude/scripts/` changes; a change to a manifest alone does not select that leg (`.husky/pre-push.bash:2172`). A change to any `.epr-meta` does select and re-key the gate project `eprfs-live-corpus` (`elohim/eprfs/build-manifest.json:77`; the test `live_corpus_manifests_are_integrity_clean` declares the read at `elohim/eprfs/epr-cli/tests/live_manifest_integrity.rs:74`). No test names the ledger rule, so the evidence tabled under "The ledger case" is this session's run and not a standing check.

## Not read, and not settled

The native matcher in `elohim/eprfs/eprfs-meta/src/evaluation.rs` was read only as far as its `contains` handling at line 1145 to 1159, so "no regular expression" is established for the Python engine and not for the native one. The debounce helper has three call sites in the resolver, at lines 212, 451 and 504: a finding about a file over its hard line count, the soft line-count advice, and a notice that the native evaluator is degraded. None is reached by a plain advisory rule. The count of first-party Rust roots is from the manifests opened and is not a census.
