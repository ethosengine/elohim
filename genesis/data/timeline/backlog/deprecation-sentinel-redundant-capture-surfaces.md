---
id: "backlog-deprecation-sentinel-redundant-capture-surfaces"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "deprecation-sentinel fingerprint instability — Class 3 (grep -n prefix) and Class 4 (aggregate-banner drift) remain after Guards J/K/N/O/P/Q/R/A2 and the Class-5 pid fix landed"
slug: "deprecation-sentinel-redundant-capture-surfaces"
written: "2026-07-30"
author: "deprecation-triage"
status: "backlog"
priority: "high"
deprecation_status: blocked
severity: medium
fingerprints: []
evidence_fingerprints: ["fb31d99a0ba8", "dc758c9c3d3f", "6ffe65fc044a", "c8babb72ae2c"]
relatedNodeIds: []
tags: [deprecation, tooling, deprecation-sentinel, pnpm, fingerprint-stability, automation-cost, epr-meta]
cites:
  - .claude/hooks/deprecation-sentinel.py
  - .claude/agents/deprecation-triage.md
  - .claude/data/deprecations.jsonl
  - .claude/skills/deprecation-stasis
  - .epr-meta/elohim/packages/skills/libp2p-transport.json
  - genesis/data/timeline/backlog/deprecation-sophia-eslint-8-eol-flat-config-migration.md
  - genesis/data/timeline/backlog/security-jquery-2-1-1-shipped-in-sophia-umd-bundle.md
---

## What is deprecated

Not a third-party deprecation — this is a **capture-surface defect in the
sentinel itself**, canonicalized here because it costs one background Opus
triage dispatch per redundant fingerprint, in perpetuity, on every pnpm
workspace in the monorepo.

One real warning is being fingerprinted more than once. Two distinct
redundant-surface classes, both observed on 2026-07-30 during the sophia
`pnpm install`:

**Class 1 — pnpm's package-changes summary twin.** pnpm reports a deprecated
dependency TWICE in one install, from two stages. The resolution stage emits
the real warning, carrying the upstream message and URL:

```
 WARN  deprecated eslint@8.57.1: This version is no longer supported. Please see https://eslint.org/version-support for other options.
```

The end-of-install lockfile-diff listing emits a bare marker with no message,
no URL, no workspace prefix:

```
+ eslint 8.57.1 deprecated
```

These differ textually, so they hash differently and fp-dedupe cannot collapse
them. Fingerprint `819aa7c6f6bd` (resolution WARN) and `bba59aabdf63` (summary
twin) were dispatched as two separate triage runs for ONE concern.

**Class 2 — escape-rendered re-read of a captured log.** The sentinel already
normalizes the U+2009 THIN SPACE that pnpm pads its `WARN` prefix with (raw
` WARN  deprecated` stores as `WARN  deprecated`). But when an agent
re-reads that same captured log through an escape-rendering tool (`cat -A`,
`od -c`), U+2009 renders as the literal ASCII `M-bM-^@M-^I`. That is a
different byte string, so it hashes differently and re-mints. Three
fingerprints (`b1561f3d429d`, `9d31ba938515`, `010ff5a7bfb5`) were minted this
way during *this* triage run, for the eslint / jquery / intersection-observer
warnings already held under `819aa7c6f6bd` / `011f5406331d` / `50aa3734f6b0`.

**Class 3 — `grep -n` line-number prefix makes fingerprints file-edit-volatile.**
Observed 2026-07-30 at 14:27, and it is the class that *dispatched the run which
found it*. `fingerprint()` (hook line 484) normalizes only whitespace and case:

```python
def fingerprint(line: str, cls: str) -> str:
    norm = re.sub(r"\s+", " ", line).strip().lower()
    if cls == "security" and SECURITY_SUMMARY.search(line):
        norm = re.sub(r"\d+", "#", norm)      # digit-collapse: security summaries ONLY
    return hashlib.sha256(norm.encode()).hexdigest()[:12]
```

The leading `<lineno>:` that `grep -n` prepends is therefore **part of the
hashed string** for every non-`security`-summary class. When any edit shifts a
lockfile, the identical warning re-mints under a new fingerprint. Exactly that
happened to sophia's jQuery notice: the `feat/node24` security commit
`f0157adbb` moved `pnpm-lock.yaml`'s deprecation line from **6618 → 6620**, and
one unchanged warning became two live ledger entries and two Opus dispatches.

Reproduced exactly against the real `fingerprint()` normalization — the message
text is byte-identical in both, only the prefix differs:

| Captured line | Fingerprint |
|---|---|
| `6618:    deprecated: This version is deprecated. …herodevs.com/support/jquery-nes.` | `313c6eac27c1` |
| `6620:    deprecated: This version is deprecated. …herodevs.com/support/jquery-nes.` | `fb31d99a0ba8` |
| *(same text, no `grep -n` prefix)* | `fe896c58f14e` |

Note the third row: the same warning captured **without** `-n` is a *third*
distinct fingerprint. So the prefix does not merely churn — it also splits
`grep -n` captures from `grep` captures of the same line.

This class is strictly worse than Classes 1 and 2, because those are bounded
(one twin per install; only on `cat -A` re-reads) whereas Class 3 re-mints
**every time an unrelated dependency edit shifts the lockfile** — unboundedly,
on a file whose line numbers move constantly, for a concern already documented
as `blocked`. Note the digit-collapse guard on line 3 of `fingerprint()` already
solves precisely this churn shape for security summaries; it is simply not
reached by any other class.

**Class 4 (observed 2026-07-30, not yet worked) — the pnpm aggregate
subdependency notice is count-bearing and unnormalized.** A root `pnpm install`
emits a single summary line naming every deprecated transitive at once:

```
 WARN  25 deprecated subdependencies found: @humanwhocodes/config-array@0.13.0, @humanwhocodes/object-schema@2.0.3, @npmcli/move-file@2.0.1, abab@2.0.6, … glob@7.2.3, glob@8.1.0, inflight@1.0.6, mathjax-full@3.2.2, …
```

Two properties make it a poor fingerprint, and they compound:

- **The leading count drifts.** `fingerprint()`'s digit-collapse normalization
  is gated on `cls == "security"`, so a deprecation summary keeps its literal
  `25`. Any dependency change that moves the count re-mints the whole line —
  the same churn shape Fix N addresses for `grep -n`, arriving through a
  different door. The *package list* also reorders and grows, so even a stable
  count is not a stable string.
- **It is an aggregate across many independent concerns.** This one line spans
  the eslint-8 tree (`@humanwhocodes/*`), the glob support window
  (`glob@7.2.3`, `glob@8.1.0`, `inflight`), mathjax, and ~20 more — several
  already canonicalized under their own entries. Fingerprinting it as one
  finding maps N concerns onto 1 unstable fp, which is the exact inverse of the
  1-concern-per-entry discipline the backlog runs on.

Recorded here rather than acted on: it is neither an echo (the line is a real
warning) nor a pure hash-stability bug, so it wants its own decision — most
likely *digit-collapse for deprecation summaries too* plus a routing rule that
treats the aggregate as a pointer to the per-package entries rather than a
concern. Live ledger entry: `ce0de21b8053`, status **`triaged`** as of 2026-07-30
— the aggregate was decomposed by carrier into 8 concerns (5 new sophia entries
plus 3 folded into existing ones), which is exactly the "pointer to the
per-package entries" routing this class argues for, done by hand. The fingerprint
is deliberately **retained**, not deleted: it is the shared banner all 8 entries
hang off, and it clears only when the banner does. That triage also landed one of
them — three dead devDependency declarations removed in sophia commit
`a4d931cca1` — dropping the count from 25 to 23, which will re-mint this
fingerprint and demonstrate the count-drift defect live.

> **It fired, exactly as predicted, on 2026-07-30 at 15:47.** The next root
> `pnpm install` emitted the banner at its new count and minted
> **`6bbd169077f5`**, costing a triage dispatch for a concern already decomposed
> into eight entries. `ce0de21b8053` was then deleted (its 25-count text is
> unreachable and can never re-fire) and all eight sibling entries were
> retargeted to `6bbd169077f5`.
>
> The firing also **settles the design question this class was holding open.**
> The proposed remedy was digit-collapse, as for security summaries. That would
> not have helped: the count moved 25 → 23 *because packages were removed*, so
> the package **list** changed too and the line re-mints under digit-collapse
> anyway. Count-collapse is not merely partial here — it is inert for the
> dominant shape. So Class 4 needs the *routing* fix this entry already
> gestured at, not a normalization: the aggregate banner is not a concern, it
> is a **pointer to per-package concerns**, and the sentinel should either
> decline to fingerprint it as a finding, or fingerprint it on the invariant
> stem (`deprecated subdependencies found`) with the count and list excluded
> from the hash. That is a behavioral change to what counts as a finding, so it
> keeps its own decision — but it is now a decision with the evidence in hand
> rather than an open question.

**Class 5 (observed live 2026-07-30 15:01–15:02, the sharpest instance) — the
Node `(node:PID)` prefix makes every process a new fingerprint.** Node prefixes
its deprecation warnings with the emitting process id:

```
(node:810849) [DEP0040] DeprecationWarning: The `punycode` module is deprecated. Please use a userland alternative instead.
(node:811654) [DEP0040] DeprecationWarning: The `punycode` module is deprecated. Please use a userland alternative instead.
(node:812228) [DEP0040] DeprecationWarning: The `punycode` module is deprecated. Please use a userland alternative instead.
```

Three ledger entries — `534a561884b4`, `e0db70ae40c7`, `9f2941b72728` — all
`status: open`, all the **same** warning, minted inside ninety seconds by one
concurrent session running three `jest` invocations. Normalizing `(node:\d+)`
collapses all three to a single concern (verified against the live ledger).

This is Class 3's disease at its worst. Class 3 re-mints when a lockfile line
*moves*; Class 5 re-mints **once per process**, so a single test suite can
mint dispatches faster than triage can retire them, forever, for one long-known
upstream warning (`punycode` is deprecated in Node ≥21 and reaches this repo
through transitive deps). Any run that shells out to node repeatedly pays it.

The fix is one normalization beside Fix N in `fingerprint()`:

```python
norm = re.sub(r"\(node:\d+\)", "(node:#)", norm)
```

**Class 5 LANDED 2026-07-30** — and the "bundle it with Fix N" sequencing
advice recorded here was wrong, which is worth keeping because it is the
generalizable part. The two fixes were assumed to share one ledger migration.
They do not: measured against the live ledger, the pid normalization moves
**exactly 8 rows and Fix N moves 105**, and the 8 are a strict subset of the
punycode rows. Bundling would have held a zero-cost fix hostage to a 105-row
one. The lesson: *measure the migration surface per-fix before bundling on the
assumption of a shared one.*

The pid normalization went in beside `SECURITY_SUMMARY` in
`.claude/hooks/deprecation-sentinel.py`, applied unconditionally in
`fingerprint()` before the security digit-collapse:

```python
NODE_PID_PREFIX = re.compile(r"\(node:\d+\)")
...
norm = NODE_PID_PREFIX.sub("(node:#)", norm)
```

It landed at **zero migration cost**: all 8 affected rows were the punycode
captures, closed and deleted in the same commit when the underlying warning was
actually fixed (`tr46@3 → ^4.1.1`, sophia `576dd73f88`). See Verification.

**Class 6 (observed + CLOSED 2026-08-04) — the same managed surface has FOUR
homes, and the guard knew only one.** Guard E dismisses agent-surface prose
keyed on the path `.claude/{hooks,scripts,skills,agents,data,memory}/`. But
every one of those surfaces is *also* stored content-addressed in the EPR
package store and re-projected per agent runtime:

```
.claude/skills/libp2p-transport/SKILL.md                              ← Guard E dismissed
.epr-meta/elohim/packages/skills/libp2p-transport.json                ← dispatched
.epr-meta/elohim/projections/{claude,codex,codexProject}/skills/…     ← dispatched
.codex/skills/libp2p-transport/SKILL.md                               ← dispatched
```

So a repo-wide scope grep hits all four and the sentinel dismissed exactly one
of them. Live proof, and it is the capture that dispatched the run which found
it: **`09f2f5632c00`** (2026-08-04 15:18) — a scope grep for
`enable_mdns|enable_relaying`, run during the holochain-iroh convergence work,
hit `packages/skills/libp2p-transport.json` whose *inlined JSON body* carries
the prose "uses `StreamExt::next()` (not the deprecated `select_next_event()`)".

That is the **same sentence, from the same skill**, that Guard B already
dismisses when it is read from the root CLAUDE.md — fingerprint `97d4865837a9`,
2026-07-21, dispositioned then as "a doc narrating a *past* upstream API
deprecation, not a live toolchain warning." The libp2p migration it describes is
long since done: the codebase uses `StreamExt::next()`, and `select_next_event()`
appears nowhere outside prose. One dismissal, already reasoned and recorded, was
re-derived by a full background Opus dispatch because the prose arrived via a
different path.

The class is worse than a single duplicate: it is a **×4 multiplier on the
entire `.claude/` echo surface**, and fp-dedupe cannot collapse it, because each
package re-seal rewrites the JSON and each projection carries its own distinct
text. Only a structural path guard closes it.

**Guard N landed 2026-08-04** as two additive clauses folded into
`ECHO_TOOLING_SOURCE_RE` (the same regex Guard E uses, so the class is dismissed
by the guard that always should have covered it):

```python
r"|(?:^|/|\s)\.epr-meta/"                          # EPR package store + projections
r"|(?:^|/|\s)\.codex/(?:skills|agents|commands)/"  # codex mirror
```

Narrow in two deliberate ways. The `.epr-meta/` clause requires a **trailing
slash**, so it matches only the root package store and never the
directory-local `.epr-meta` compose-gate manifest **files** (`elohim/.epr-meta`,
`doorway/doorway-service/.epr-meta` — nothing follows those). The `.codex/`
clause enumerates the agent-surface directories, so `.codex/config.toml` stays
capturable. Zero true-positive risk on the same argument Guard E already rests
on: a live toolchain warning is never sourced from a checked-in agent-doc
projection — if the finding is real it belongs to the code the doc describes.

**Class 7 (observed + CLOSED 2026-08-07) — the guard comment bills for the
concern it guards.** The largest single class yet, and structurally the most
perverse: *documenting* a deprecation is what turns it into a permanent dispatch
generator.

When a deprecation is triaged to `blocked`, the right engineering response is
often to leave a comment where the trap lives. `elohim/elohim-storage/Cargo.toml`
carries a 13-line `TOMBSTONE GUARD` block explaining why `holochain_sqlite` is
lock-pinned. `devfile.yaml` carries a three-line note pointing at `hc-start.sh`.
Both are good comments doing their job. Both are also **permanent, high-traffic
capture surfaces**: they sit in files that get diffed and grepped constantly, and
every read re-emits them under a fresh `git diff` `+` marker or `grep -n` prefix
that fp-dedupe cannot collapse.

Guard G was supposed to cover this and leaks three ways at once:

| Guard G anchor | What escapes |
|---|---|
| `//`-openers only | `#` comments — Cargo.toml, shell, YAML, Dockerfile, devfile — and markdown prose |
| ALL-CAPS `DEPRECATED` only | ordinary lowercase prose (`# … is deprecated and no longer`) |
| diff-EXEMPT (`+`/`-`) | the comment **ADD** — which is the dominant shape, because writing the guard comment is exactly what fix work *does* |

The holochain_sqlite guard comment hit all three. It minted `4e4f2598ff5c` /
`93a86c9e5657` / `1f681f440327` from a `git diff Cargo.toml` (three `+#` lines,
lowercase, `#`-opener — one dispatch), and then **the triage run dispatched to
handle those minted three more** (`6dd68a9c0f58` / `07364812bedc` /
`022dae188a5f`) from its own verification `grep -n` of the same comment,
requesting a *second* dispatch mid-run. Nine rows total for that one comment
block, against a concern documented as `blocked` since the morning of the same
day. The devfile note cost two dispatches the same way (`a2464d792194`, then
`8cb5f41fe4ea` from a later scope grep).

**Measured live before landing: 33 of 286 ledger rows (11.5%) are comment-line
captures in the deprecation class.** The zero-true-positive argument is
empirical rather than regex reasoning, and it is the strongest one available:

| Disposition | Rows | Reading |
|---|---|---|
| `false-positive` (hand-marked) | 5 | already judged echo by a human/agent |
| `blocked`, backlog-attached | 12 | prose about concerns already canonicalized |
| `open` | 16 | untriaged — each still owed a dispatch |
| **`triaged`** | **0** | **no comment-line capture in the ledger's history ever became an actionable fix** |

The 16 open rows, inspected: Rust doc-comments (`/// deprecated in favour of
[`Standing::evaluate`]`), `#![allow(deprecated)]`, a `//` inline note about a
legacy WebSocket route, a shell comment, a markdown task heading, and the two
guard blocks above. First-party prose, every one.

**Guard O landed 2026-08-07** — the deprecation-class analog of H2, and the
general form of G:

```python
if cls == "deprecation" and ECHO_COMMENT_OPENER_RE.match(line):
    return True
```

Deliberately **not** diff-exempt, on H2's and M's argument: a comment is an
annotation whether it is being added or merely re-read, and the `+#` add is the
dominant shape. `#[deprecated…]` is an attribute, not a comment, and stays
capturable via the opener's existing `#(?!\[)` clause.

### The live channel that wears a comment costume

Landing Guard O required narrowing `ECHO_COMMENT_OPENER_RE` first, and this is
the part worth carrying forward. **Docker BuildKit prints `#<step> <elapsed>
<text>`** — so a real warning from a container build arrives as:

```
#12 3.456 npm warn deprecated glob@7.2.3: Glob versions prior to v9 are no longer supported
```

Verified 2026-08-07: that line matched the pre-narrowing opener regex **and**
classified as `deprecation`. A naive Guard O would have silently eaten every
npm/cargo/node deprecation warning emitted inside a Docker build — and **every CI
build in this repo runs inside Docker**, so that is not an edge case, it is the
primary live channel for the very warnings the sentinel exists to catch. The
same hole was latent in Guard H2: a `#12 3.456 … RUSTSEC-2024-0437` line was
dismissible as a remediation annotation.

The fix is one lookahead — `#` followed by a digit is never a comment opener:

```python
r"(?://[/!]?|\#(?!\[)(?!\d))"
```

Narrowing an echo guard can only ever *lose a dismissal*, never gain one, so it
errs toward capture by construction. Closing H2's hole came free with it.

The generalizable lesson, and it is a new one — the earlier classes are all
about *where* a surface lives (Class 6: how many homes does the path have?).
Class 7 is about *what the text is*: **a note about a warning is not a warning**,
and the sentinel had no way to tell them apart in the one language (`#` comments)
that manifests, shell, YAML and CI config all share. The corollary for anyone
triaging: when you write a guard comment to document a blocked concern, you have
just created a capture surface — check that a guard covers it, or the concern
you just closed will keep billing.

**Class 8 (observed + CLOSED 2026-08-07) — the guard knew the *syntax*, not the
*invariant*.** Class 7's sibling, found the same day, and the fastest-minting
class yet measured: **seven fingerprints for one already-`blocked` concern
inside ten minutes.**

Guard F dismisses a crates.io `+deprecated` build-metadata token in exactly two
shapes — the quoted toml literal (`version = "0.9.34+deprecated"`, F1) and the
hyphen-joined registry filename (`serde_yaml-0.9.34+deprecated.crate`, F2).
Those are the token as the *toolchain stores* it. The moment an agent
**extracts** that version and reprints it, the shape changes and the guard goes
blind — while the semantics are unchanged. It is still registry metadata; it is
still not a live warning.

The emitting diagnostic is not incidental: comparing lock versions across
workspaces is *exactly what triaging a blocked dependency concern requires*.
The main session, mid-fix on the `holochain_sqlite` tombstone, ran a shell loop
over eight holochain crates and printed

```
holochain_sqlite: storage=0.7.0-dev.19 steward=0.7.0+deprecated
```

→ `f7f949929c67`, one dispatch. Ninety-eight seconds later it re-did the same
comparison in Python, whose list-repr changed the bytes —

```
holochain_sqlite: storage=['0.7.0-dev.19'] steward=['0.7.0+deprecated']
```

→ `e898eca36448`, a second row. Then the triage run dispatched to handle those
minted **five more** from its own verification harness (`f4d8b5b3c714`,
`c40d6cb35607`, `8db0f74af09c`, `1249d4932448`, `9ebdacdc42e2`) and requested a
*second* dispatch mid-run — the identical self-amplification Class 7 exhibited,
through a different door. Every join is a fresh fingerprint (`=`, `:`, tab,
markdown table cell, JSON, python list-repr), and every version pair is a fresh
fingerprint, so fp-dedupe can never collapse the class.

**Guard P landed 2026-08-07.** The fix is not a third syntax bolted onto F — it
is F restated as the **invariant F already rests on**, in three steps:

```python
if (
    cls == "deprecation"
    and BUILD_META_DEPRECATED_RE.search(line)                       # 1. carries the token
    and not CARGO_VERSION_JOIN_RE.search(line)                      # 2. not cargo's live shape
    and not DEPRECATION_PATTERNS.search(BUILD_META_DEPRECATED_RE.sub("", line))  # 3. residual
):
    return True
```

Step 2 is the carve-out that keeps the real channel open, and it is quoted
verbatim from Guard F's own comment: *"a genuine build/compile deprecation is
emitted UNQUOTED and prose-shaped with a SPACE-`v` version join."* So
`Compiling serde_yaml v0.9.34+deprecated` and `Updating holochain_sqlite
v0.7.0-dev.17 -> v0.7.0+deprecated` — the original true positive of the
tombstone concern — survive untouched.

Step 3 is the property worth carrying forward, because **no earlier guard has
it**: strip every build-metadata token and ask whether any deprecation signal
*remains*. If one does, the line carries a real warning alongside the metadata
and must capture. That makes the guard structurally incapable of eating a
co-located finding — a safety net regex shape-matching cannot provide, and the
reason Guard P can be stated as a broad invariant rather than a narrow shape.
(Guard F, which lacks it and is not class-gated, does eat two adversarial
co-located constructions; recorded in Verification, not fixed — no toolchain
emits a registry filename and a live warning on one line.)

The generalizable lesson, and it is the complement of Class 6's: Class 6 asked
*how many homes does the surface have?*; Class 7 asked *is this text a warning
or a note about one?*; **Class 8 asks whether a guard is keyed on the syntax it
first met or on the property that makes the class dismissible.** A guard keyed
on syntax is defeated by anyone who reformats the data — and an agent doing
scope work reformats data constantly. When adding an echo guard, write down the
invariant first, then the carve-out for the live channel, then a residual check;
the regex is the last step, not the first.

**Class 9 (observed + CLOSED 2026-08-11) — the guard could see the tree, but not
the file.** Guard E dismisses first-party tooling source keyed on a
`.claude/{hooks,scripts,…}/` path **appearing in the line**. But that path is
only present when grep is given *multiple* files or `-H`. Point grep at ONE
file and it emits `73:<code>` — no filename at all. So the identical content
Guard E dismisses from a repo-wide scope grep is **captured** when an agent
greps the single file directly, which is the more common act.

Live proof, and it is the capture that dispatched the run which found it:
**`3054d0cb4bd7`** (2026-08-11) —

```
grep -n "ACTIVE_WORDS\s*=\|…\|DEAD_WORDS\s*=" .claude/scripts/memory-kit/placement-audit.py
→ 73:DEAD_WORDS = {"superseded", "abandoned", "cancelled", "canceled", "deprecated", "retired"}
```

That is a plain Python **status-vocabulary set literal** — the placement audit's
own word list for classifying doc status. It has never been a warning about
anything. The same constant was already captured at line **72** as
`802862c393b2` (hand-marked `false-positive` on 2026-06-06) and with no prefix
at all as `5723985e3232`. **Three fingerprints, one string literal, one Opus
dispatch each** — and Class 3 guarantees a fourth the next time the file is
edited, because the `grep -n` prefix moves with it.

This is precisely the gap `_CMD_HISTORY_TREE_RE` was introduced to close for the
*prose* trees. Quoting its own comment: *"cat/tail/grep of a single doc carries
no path prefix per line, so the command string is the only signal of source."*
The tooling tree had no such command-level counterpart.

**Guard Q landed 2026-08-11**, read-gated with a live-channel residual:

```python
if (cmd_is_tooling_read
        and not _LIVE_TOOLCHAIN_CHANNEL_RE.search(line)
        and not _LIVE_VITEST_BANNER_RE.search(line)):
    return True
```

Two deliberate narrowings, both of which the Guard-O BuildKit lesson demanded be
settled *before* the regex:

- **READ-gated, never exec-gated.** The command must use a read utility
  (`grep`/`cat`/`head`/`sed`/`git diff`/…) on the tooling path. *Executing* one
  (`python3 .claude/scripts/memory-kit/delivery-status-distribution.py`) stays
  fully capturable, because our own Python tools genuinely can emit a real
  `DeprecationWarning` (`datetime.utcnow()`, Python 3.12+) and that IS a
  first-party finding worth having. Verified: the exec command does **not**
  match the gate.
- **A live-channel residual (Guard P step 3).** A line carrying a real tool's
  own token still captures, so a compound `cargo build && grep …
  .claude/scripts/x.py` cannot swallow cargo's genuine warning.

**The separator constraint is where the first cut was wrong, and it is the part
worth carrying forward.** Guard Q originally required the utility and the path
to sit in one segment via `[^;&|\n]*?` — excluding pipes. That regex **passed a
harness of twenty hand-written cases and still missed its own dispatching
capture**, because a `grep` alternation carries literal `\|` characters between
the utility and its file operand. The harness had used a *simplified*
stand-in command (`grep -n 'DEAD_WORDS' <path>`) rather than the real one.
The lesson is sharp and general: **a guard harness must replay the verbatim
command from the ledger row, not a cleaned-up paraphrase of it** — the
paraphrase is written by the same understanding that wrote the regex, so it
inherits the same blind spot. Pipes are now allowed (`[^;&\n]*?`); `;`, `&&` and
newline still block, and the residual covers what the widening admits.

**Class 10 (observed + CLOSED 2026-08-11) — Guard D knew the marker, not the
invariant; and the class it misses is aimed at this hook.** Guard D dismisses an
agent's own ephemeral script output keyed on Python's `<string>:N:` / `<stdin>:N:`
source markers — i.e. `python3 -c "…"` and stdin. Write that same code to a
scratch **file** and run it, and D goes blind, while the semantics are unchanged:
still the agent's own throwaway code, still not a finding about this repo.

The dominant shape is the sharpest self-amplification yet recorded, because it
targets the triage loop itself: **a deprecation-triage run verifying an echo
guard must print an adversarial fixture corpus, and that corpus is by
construction full of warning-shaped lines.** Every harness run mints
fingerprints for warnings that do not exist. Measured live 2026-08-11: **7 of
275 rows**, every one a triage harness printing its own test vectors —

```
[PASS] diff-add of #[deprecated] attr still classifies: got='deprecation'
text: let deprecated: Vec<u32> = vec![];
text: 10  warning  `LocalSourceChainService` is deprecated. M-AGGR-2: …
```

Dispositions: 5 `false-positive`, 2 `blocked`, **0 `triaged`**. Classes 7 and 8
each noted in passing that "the triage run's own verification minted more"; this
is that observation promoted to its own class and closed.

**Guard R landed 2026-08-11** — Guard D restated on the property that makes the
class dismissible (the code is **ephemeral and agent-authored**) rather than on
the marker syntax D first met, which is Class 8's lesson applied at a new seam:

```python
_CMD_EPHEMERAL_SCRIPT_RE = re.compile(
    r"\b(?:python3?|node|bash|sh|uv\s+run)\s+[^\s;&|]*"
    r"(?:/tmp/|/var/tmp/|/private/tmp/|/scratchpad/)"
    r"[^\s;&|]*\.(?:py|mjs|js|sh)\b"
)
```

Unconditional, exactly as Guard D is, and deliberately **residual-free** — a
verification harness's whole job is to print live-channel-shaped fixtures, so a
residual would re-admit precisely the rows the guard exists to stop. This makes
the guard **self-protecting**: the harness that verified Guards Q/R/A2 ran from
the scratchpad and minted nothing, where its predecessor minted five rows.

**Class 11 (observed + CLOSED 2026-08-11) — reading the ledger re-mints the
ledger.** Guard A keys on the literal `deprecations.jsonl` appearing **in the
line**. But a ledger ROW read back out is a JSON object that does not contain
its own filename — it contains the *original captured warning text*. So
inspecting the ledger re-mints its own contents as fresh findings.

Live proof: **`18ddd0fb6e64`** — an agent listing `.claude/data/*.jsonl` captured
row `c4bc9714e080`'s stored text (the GitHub *191 vulnerabilities* banner) as a
**new security finding**. The agent was, at the time, dispatched to triage that
ledger.

**Guard A2 landed 2026-08-11** as a second clause on Guard Q's flag:

```python
_CMD_FINDINGS_LEDGER_RE = re.compile(r"\.claude/data/[^\s;&|]*\.jsonl")
```

Deliberately **not** segment-ordered like Guard Q, because the dominant shape is
a shell loop that puts the **path before the reader**
(`for f in .claude/data/*.jsonl; do head -1 $f; done`) — an ordered regex misses
it. Order-insensitivity is safe here in a way it would not be for Guard Q:
reading a stored JSONL file emits no toolchain output of its own, so there is no
live channel in the command to protect. It still carries Guard Q's residual, so
a compound `pnpm install && cat .claude/data/x.jsonl` keeps its real warnings.

### Recorded, not fixed — Guard E eats first-party tool *runtime* warnings

Surfaced by Guard Q's harness and **pre-existing** (Guard E, unchanged by this
work). Guard E dismisses any line containing a `.claude/…` path — including a
genuine runtime warning emitted **by** one of our own Python tools, whose
traceback prefix *is* such a path:

```
/projects/elohim/.claude/scripts/memory-kit/placement-audit.py:73: DeprecationWarning: datetime.utcnow() is deprecated
```

Guard E dismisses that today. It is a real blind spot — the memory-kit scripts
are load-bearing Python and `datetime.utcnow()` is removed in newer Pythons — but
closing it means **widening** an echo guard, which is a behavioural change of the
opposite sign to everything else in this entry (narrowing can only lose a
dismissal; widening can admit noise). It therefore wants its own decision and its
own measurement. Guard Q was deliberately built read-gated so that it does not
make this worse, and the harness pins the current disposition so it cannot drift
silently.

## Usage inventory

Single file — `.claude/hooks/deprecation-sentinel.py`:

- `DEPRECATION_PATTERNS` (~line 58) — `\bDEPRECATED\b` under `re.IGNORECASE`
  matches the bare word `deprecated`, which is what admits the Class-1 summary
  line. Patterns at lines 62–66 are consequently redundant.
- `_is_echo_line()` — where the structural guards live. Guards A–R are now
  present (J and K from this entry; L and M from the derived-artifact /
  JSDoc-annotation class landed in the same commit; N from the Class-6
  four-homes gap, folded into Guard E's own regex; O and P from Classes 7/8;
  Q, R and A2 from Classes 9/10/11 — the first three keyed on the COMMAND
  rather than the line, joining the pre-existing `cmd_is_git_history` /
  `cmd_is_history_tree` flags). No remaining work here.
- `_LIVE_TOOLCHAIN_CHANNEL_RE` / `_LIVE_VITEST_BANNER_RE` — the residual Guard Q
  and A2 consult. This is the first *positive* statement in the hook of what a
  live warning looks like; every other regex describes what an echo looks like.
  Anything added to the deprecation channel vocabulary belongs here too, or
  Guard Q will quietly start eating it.
- The diff-hunk exemptions in Guards C and G (`not line.startswith(("+","-"))`)
  do **not** cover Class 1: pnpm's summary marker *is* `+`/`-`, so the shape
  must be matched precisely rather than exempted.
- `fingerprint()` (line 484) — Class 3 lives here, **not** in `_is_echo_line()`.
  A `grep -n` capture is a perfectly legitimate warning surface that must still
  be *captured*; the defect is that it is *hashed unstably*. Suppression would
  be the wrong instrument.

Ledger evidence (`.claude/data/deprecations.jsonl`): originally 4 redundant
live entries against 3 genuine concerns; all 4 deleted when Guards J/K landed.
One Class-3 entry (`fb31d99a0ba8`) remains.

## Migration path

Two additive guards in `_is_echo_line()` — **both landed 2026-07-30**, kept
here because they are the worked record of the two classes. Regex-verified
against real log bytes plus adversarial negatives.

**Guard J — pnpm package-changes-summary redundant surface.** Safe because the
summary surface is a strict *subset* of the resolution surface: the resolution
WARN fires on every install (it re-reads the lockfile), while the summary block
only prints when the package set changes. Verified on the 2026-07-30 sophia
install — `pnpm-install.log` (no set change) carries the WARN alone;
`pnpm-install2.log` carries BOTH the WARN at line 3 and the summary twin at
line 135. Collapsing the summary can never lose a finding.

```python
ECHO_PNPM_CHANGE_SUMMARY_RE = re.compile(
    r"^\s*(?:[^\s:]+:\d+[:-])?\s*(?:\d+[:-])?\s*"  # optional grep path:line: / -n prefix
    r"[+-]\s+"                                     # pnpm add/remove marker
    r"(?:@[A-Za-z0-9._-]+/)?[A-Za-z0-9._-]+\s+"    # package name (optionally scoped)
    r"\d[A-Za-z0-9.+-]*\s+"                        # version (starts with a digit)
    r"deprecated\s*$",                             # bare marker, nothing after it
    re.IGNORECASE,
)
```

The `deprecated\s*$` anchor is what keeps it narrow: a real diff hunk adding a
`#[deprecated(note = "…")]` attribute, a `version = "0.9.34+deprecated"`
literal, or any prose sentence (`+ serde_yaml 0.9.34 deprecated and
unmaintained`) carries punctuation, quotes, or trailing words and cannot match.
The optional grep prefix tolerance means a later re-read of a captured log
cannot defeat fp-dedupe either.

**Guard K — escape-rendered mangling.** No real toolchain emits `cat -A`'s
escape vocabulary; it appears only when an agent re-reads a captured log
through an escape-rendering tool. A mangled rendering is never a live warning.

```python
ECHO_ESCAPE_RENDERED_RE = re.compile(r"M-[A-Za-z]M-\^?[@-_A-Za-z]|M-\^[@-_]")
```

Verified to catch both observed mangled shapes while leaving the real U+2009
line, `use of deprecated`, `npm warn deprecated`, and `DeprecationWarning`
untouched.

Both belong early in `_is_echo_line()` alongside Guards D and F — deterministic,
no command-flag dependency. Both are the same species as Guard F's
registry-metadata collapse: a structural collapse of a surface that carries no
information the primary surface lacks.

**Fix N (Class 3) — strip the `grep -n` prefix before hashing.** A different
instrument in a different function: normalization in `fingerprint()`, not
suppression in `_is_echo_line()`. The line stays captured; it just hashes
stably.

```python
GREP_LINENO_PREFIX_RE = re.compile(r"^\s*(?:[^\s:]+:)?\d+[:-]\s*")

def fingerprint(line: str, cls: str) -> str:
    norm = re.sub(r"\s+", " ", line).strip().lower()
    norm = GREP_LINENO_PREFIX_RE.sub("", norm)   # Fix N: line numbers churn; content does not
    if cls == "security" and SECURITY_SUMMARY.search(line):
        norm = re.sub(r"\d+", "#", norm)
    return hashlib.sha256(norm.encode()).hexdigest()[:12]
```

Two properties worth stating before anyone lands it:

- **It is a one-way merge, and it is deliberately narrow.** The regex requires a
  digit run followed by `:` or `-`, at line start, optionally after a
  colon-terminated path — `grep -n` / `grep -Hn` shape. A real warning rarely
  opens that way; `2.1.1: deprecated` cannot match (the digits are broken by
  dots before the colon).
- **It changes existing fingerprints.** Any live ledger entry whose captured
  `line` carries a `grep -n` prefix re-hashes and reads as NEW once. Landing Fix
  N therefore needs a one-time ledger migration (recompute `fp` for affected
  live lines) or it will fire a dispatch per affected entry — the exact cost it
  removes. This makes Fix N *cheaper to land early*, before more prefixed
  captures accumulate.

Worth folding in while there: `\bDEPRECATED\b` + `re.IGNORECASE` makes the
bare word `deprecated` a catch-all and renders lines 62–66 dead. Guard G's own
comment states the intent — "real deprecations are lowercase prose … Vitest
`DEPRECATED: <prose>`" — i.e. `\bDEPRECATED\b` was meant to be the
case-*sensitive* Vitest banner. Tightening that is a broader behavioral change
than the two guards and should be measured separately, not bundled.

## Current decision

**Classes 1 and 2 are CLOSED — Guards J and K landed 2026-07-30.** The
permission blocker recorded below did **not** reproduce: a later triage run
(dispatched on the `.angular/cache` capture `c3259594f97a`) edited
`.claude/hooks/deprecation-sentinel.py` through the normal `Edit` path and was
permitted. The paste-ready patch above was landed verbatim as Guards J and K,
verified, and the four evidence fingerprints were deleted from the ledger per
this entry's own stated deletion condition.

> *Superseded blocker, retained for the record:* "BLOCKED — needs an
> operator-approved edit to a protected configuration surface. The `Edit` was
> denied by the permission classifier." That denial was **not** a standing
> policy on the file — treating one denial as a permanent property of the
> surface is what kept this entry blocked longer than it needed to be. The
> lesson is narrow and worth keeping: **re-probe a permission blocker before
> inheriting it**; a denial is an event, not an attribute.

**Class 5 is CLOSED — the pid normalization landed 2026-07-30** with an
adversarial harness and zero migration cost (see Verification). Classes 1 and 2
closed earlier the same day. **Class 6 is CLOSED — Guard N landed 2026-08-04**,
also at zero migration cost: exactly one live ledger row matched the new clauses
(`09f2f5632c00`, the capture that dispatched the run), and it was deleted as
fixed in the landing commit. **Class 7 is CLOSED — Guard O landed 2026-08-07**,
retiring 17 live rows (286 → 269) at zero true-positive cost and closing a
latent Guard-H2 hole on the way. **Class 8 is CLOSED — Guard P landed
2026-08-07**, retiring 8 live rows (276 → 268) at zero true-positive cost and
generalizing Guard F from two hard-coded syntaxes to the invariant F rests on.
**Classes 9, 10 and 11 are CLOSED — Guards Q, R and A2 landed 2026-08-11**,
retiring 13 live rows (275 → 262) at zero true-positive cost: 0 rows newly
captured, and — as in every prior class — **0 `triaged` rows affected**.
**Classes 3 and 4 remain BLOCKED**, and they are now the entire remaining
concern.

Classes 9–11 landed from a single dispatch whose entire finding was that a
Python `DEAD_WORDS = {…, "deprecated", …}` set literal is not a deprecation.
Taken together they say something the earlier classes had only half-stated:
**every one of the sentinel's own working surfaces is also one of its capture
surfaces.** Class 9 is the tooling *source*, Class 10 is the triage harness's
*fixture corpus*, Class 11 is the *ledger itself*. Class 7 established that
triaging a blocked dependency generates captures; Classes 9–11 establish that
**triaging the sentinel generates captures** — reading its code, verifying its
guards, and inspecting its ledger were all billable acts. The three guards make
the tool's own maintenance loop free, which is the precondition for Classes 3
and 4 ever being worked without paying a dispatch tax to do it.

Classes 7 and 8 landed the same day, from the same underlying concern
(`holochain_sqlite`), and together they say something the earlier classes only
hinted at: **triaging a blocked dependency is itself a capture-generating
activity.** Class 7 came from *writing the guard comment*; Class 8 came from
*printing the version comparison*. Both are correct engineering practice. Both
minted dispatches for a decision already written down. The sentinel's echo
surface is therefore not a fixed set of shapes to enumerate — it grows with the
repertoire of the agents working the ledger, which is the argument for keying
guards on invariants (Class 8) rather than on the syntax first encountered.

Class 6 restates the lesson the earlier classes keep teaching, at a new seam:
*a guard keyed on a path is only as complete as the path list*. The `.claude/`
tree is not the only home of the surfaces Guard E dismisses — the EPR package
store and the codex/codexProject projections carry byte-copies, and any surface
with N homes is an N× echo multiplier the moment a guard names one of them. The
generalizable check when adding a path-keyed guard: **ask how many homes the
named surface has, and enumerate them, before assuming the path is the
surface.**

**Class 3 (Fix N — strip the `grep -n` prefix in `fingerprint()`) stays blocked
on migration scale, and that scale is now measured rather than estimated.** The
earlier text said Fix N "changes existing fingerprints" without saying how many.
Measured against the live 254-row ledger by recomputing every row under both
normalizations:

| | rows re-hashed | merge groups | notes |
|---|---|---|---|
| Class 5 (pid) | **8** | 1 | all punycode; deleted as fixed → zero cost. **Landed.** |
| Fix N (`grep -n`) | **105** | **22** | ~41% of the ledger |

The 22 merge groups are the real blocker, and they are not a mechanical rewrite.
Fix N does not just re-hash rows, it **merges rows that currently carry
different `status` and different `backlog` values**, so each group needs a
decision about which status and which owner entry survives. Two worked examples:

- `f47f0600b001` ← `3d8af5658223`(triaged), `cdb5ca58ee6c`(false-positive),
  `d7af212f42f2`(false-positive), `ea88a05cc69f`(open), `fef5d7190486`(open) —
  five captures of one `X-Schema-Version` warning at three different statuses.
- `efdbd32ba5ca` ← **11** rows: the `datetime.utcfromtimestamp()` warning
  captured from `<string>:N:`, `<stdin>:N:` and `/tmp/parse_jobs.py:N:`.

That merge table is also the clearest existing measure of the defect's cost:
**105 of 254 live rows are redundant captures of warnings the ledger already
holds.** The dispatch bill for those was paid one Opus run at a time.

Two things are now discharged that were owed: the migration surface is
enumerated (exact old→new mapping computed, reproducible from the script in
Verification), and the *method* for verifying such a fix is proven by the
Class-5 harness. What remains owed is Fix N's own adversarial-negative harness
plus the 22 per-group status/backlog decisions — an operator-initiated pass, not
a background run, because 22 judgment calls on live suppression state is exactly
the shape that should not be automated silently.

**Class 4 (aggregate banner) stays blocked, now with a decided direction** —
see the Class-4 section above: it fired live on 2026-07-30 (`ce0de21b8053` →
`6bbd169077f5`), and that firing ruled out digit-collapse as the remedy. It
needs the routing change (banner = pointer, not finding), which is a change to
what the sentinel counts as a finding and so wants an operator decision.

**This entry canonicalizes ZERO ledger fingerprints** (`fingerprints: []`). One
evidence fingerprint remains:

| Evidence fp | Redundant surface of | Ledger cites (owner entry) |
|---|---|---|
| `fb31d99a0ba8` | `313c6eac27c1` jquery@2.1.1 lockfile notice (Class 3, line 6618→6620) | `security-jquery-2-1-1-shipped-in-sophia-umd-bundle.md` |

That routing is deliberate: a redundant surface carries no independent concern,
so on re-encounter the sentinel should cite what the line is *about* (jQuery
2.1.1 is a shipped XSS family, blocked) rather than a hook defect. The owner
entry already lists this fp, so the mapping is N:1 onto concerns with no
duplicate claim. It stays in the ledger until Fix N lands.

The Class-5 evidence fingerprints are deliberately **not** listed: all eight
were deleted when the underlying `punycode` warning was fixed at its root, so
there is nothing left to suppress. If `DEP0040` ever returns from another
carrier it will mint a single fingerprint (`58fe49f33e62`) regardless of how
many processes emit it — which is the fix working.

## Verification

**Classes 1–2 (Guards J, K) — verified and landed 2026-07-30.** Both guards ran
against a combined harness driving the hook's own `classify()` +
`_is_echo_line()`: **14/14 positives suppressed, 20/20 live-channel negatives
preserved, zero leaks**. The negatives deliberately include the surfaces these
guards sit closest to and must never silence — the pnpm *resolution* `WARN`
carrying its real U+2009-padded prefix (the surface Guard J's summary twin is a
subset of), `npm warn deprecated` for both plain and `@scope/`-prefixed
packages, a diff-added `#[deprecated(note = "…")]` attribute, and the sophia
jQuery lockfile notice. End-to-end through the hook's real entrypoint, a
synthetic `pnpm install` emitting two genuine warnings still captured both and
fired the dispatch directive (`+2 new → deprecation-triage dispatch`), which is
the regression that matters: the guards narrow the echo surface without
narrowing the warning surface.

**Class 5 (pid normalization) — verified and landed 2026-07-30.** Harness at
`.claude/hooks/` sibling scratch (reproducible; drives the hook's own
`fingerprint()` by loading `deprecation-sentinel.py` directly, so it tests the
shipped code path rather than a copy):

- **Positives — 15 pid-variants collapsed into 4 concerns, 4/4 groups PASS.**
  The eight real punycode captures (pids 810849…913131) → one fp
  `58fe49f33e62`; `DEP0044` across pids 1 / 99999 / 4194303 → one fp;
  an `ExperimentalWarning` pair → one fp; and a mid-line pid
  (`stderr | (node:500) …`) pair → one fp, confirming the sub is not
  line-anchored.
- **Negatives — 10/10 distinct lines stayed 10 distinct fingerprints, zero
  collisions.** These are the adversarial cases that matter: three *different*
  DEP codes sharing **one** pid (proves the message, not the pid, carries
  identity); two real Node **stack frames** (`at Module._compile
  (node:internal/modules/cjs/loader:1234:14)`) which contain `(node:` but no
  digit run and must stay untouched; a `(node:worker)` non-numeric variant;
  and `glob@7.2.3` vs `glob@8.1.0` vs `eslint@8.57.1`, proving package
  versions remain concern-bearing (the sub is scoped to the literal
  `(node:<digits>)` wrapper, never a general digit-collapse).
- **Whole-ledger stability — PASS.** Recomputing all 254 live rows under the
  previous and new normalization: exactly **8 rows re-hash, and every one is a
  node-pid line; 0 non-node rows moved.** This is the property that made the
  fix free to land — the 8 were the punycode rows, deleted in the same commit
  as fixed.

- **End-to-end through the hook's real entrypoint — PASS.** Three synthetic
  `PostToolUse` payloads carrying the same `DEP0040` warning from pids
  `111111` / `222222` / `333333`, against a throwaway project dir, produced
  **one** ledger row (`58fe49f33e62`, matching the harness prediction). The
  first emitted `deprecation-sentinel: +1 new → deprecation-triage dispatch`;
  the second and third emitted `known re-encounter cited` and minted nothing.
  Before the fix this sequence was three rows and three dispatches. The
  regression that matters is also covered: the warning is still *captured* and
  still *fires once* — the fix narrows re-minting, not the warning surface.

Harness exit 0, `RESULT: ALL PASS`.

**Class 6 (Guard N) — verified and landed 2026-08-04.** Harness at the Class-5
standard (loads `deprecation-sentinel.py` directly, so it drives the shipped
`classify()` + `_is_echo_line()` rather than a copy):

- **Positives — 9/9 suppressed** (a 10th never classifies, so it could not reach
  the ledger anyway). Covers all four homes: the package JSON with its inlined
  body, the `claude` / `codex` / `codexProject` projections, an agentdocs
  package, the `.codex/{skills,agents,commands}` mirror, a capture with **no**
  `grep -n` prefix, and a security-class advisory quoted inside a projected
  skill.
- **Negatives — 14/14 preserved, zero leaks.** The adversarial set is the point:
  the directory-local compose-gate manifest **file**
  (`doorway/doorway-service/.epr-meta:12:`, no trailing slash — the shape the
  guard must not eat), the Rust path token `epr_meta::describe`, a crate named
  `epr-meta/core` without the leading dot, `.codex/config.toml` (not an
  agent-surface dir), and prose that merely *mentions* the store while carrying a
  real `glob@7.2.3` warning. Plus the live channels: `npm warn deprecated`, the
  pnpm resolution `WARN`, `warning: use of deprecated function`, the ESLint
  `no-deprecated` prose, a `(node:PID)` DeprecationWarning, a real `src/` finding,
  a diff-added `#[deprecated(…)]` attribute, and both security shapes.
- **Whole-ledger stability — PASS, zero collateral.** Every live row's captured
  `line` was re-tested against the new clauses: **exactly 1 of 257 matches**, and
  it is `09f2f5632c00` itself. No other live entry — open, triaged, or blocked —
  changes suppression state, so the guard needed no ledger migration.
- **End-to-end through the hook's real entrypoint — PASS, measured both ways.**
  One synthetic `PostToolUse` payload carrying three agent-surface projection
  lines plus one genuine `npm warn deprecated glob@7.2.3`, against a throwaway
  project dir:

  | | ledger rows minted | dispatch |
  |---|---|---|
  | before Guard N (`git show HEAD:` copy of the hook) | **4** (3 junk) | 1 |
  | after Guard N | **1** (the glob warning) | 1 |

  The regression that matters is covered by the second row: the genuine warning
  is still captured, still fingerprinted, and still emits
  `deprecation-sentinel: +1 new → deprecation-triage dispatch`. Guard N narrows
  the echo surface, not the warning surface.

**Class 7 (Guard O + opener narrowing) — verified and landed 2026-08-07.**
Harness at the Class-5/Guard-N standard (loads `deprecation-sentinel.py`
directly, driving the shipped `classify()` + `_is_echo_line()`):

- **Positives — 17/17 suppressed.** All six fingerprints from the dispatching
  run (three `+#` diff-hunk lines, three `grep -n` re-captures of the same
  comment), plus every leaky shape Guard G could not see: `///` lowercase
  doc-comments, `//!` module docs, `#![allow(deprecated)]`, a `#` shell comment
  behind a `grep -n` prefix, a markdown `###` task heading, a `##` changelog
  line, a `//` inline note, a `+    #` indented diff-add, and both the bare and
  `path:line:`-prefixed forms of a manifest pin comment.
- **Negatives — 14/14 preserved, zero leaks.** The adversarial set carries the
  finding: **five Docker BuildKit shapes** (`#12 3.456 npm warn deprecated …`,
  `#8 12.34 warning: use of deprecated function`, `#5 0.221 (node:N) [DEP0040]`,
  `#33 15.7  WARN  deprecated eslint@8.57.1`, `#1 0.001 DEPRECATED: …`) — each
  confirmed to classify as `deprecation` and to survive the guard. Plus the
  unprefixed live channels, the ESLint `no-deprecated` prose, a
  `Compiling serde_yaml v0.9.34+deprecated` build line, three `#[deprecated…]`
  attribute forms (bare, diff-added, and with `since`/`note`), and a
  `path.py:12: DeprecationWarning` whose prefix must not read as a comment.
- **Security class — 5/5.** Guard H2's two canonical dismissals still dismiss
  (`+# … fixing RUSTSEC-2024-0437.`, `// origin isolation (GHSA-…)`), and the
  three live advisory channels survive — including
  `#12 3.456 error: 1 vulnerability found: RUSTSEC-2024-0437 …`, which the
  narrowing **gave back**: it was dismissible before this change.
- **Whole-ledger stability — PASS.** Every live row re-tested under both the
  `HEAD` hook and the new one: **17 of 286 change suppression state, all in the
  newly-suppressed direction, 0 newly captured.** Dispositions of the 17: 12
  `open`, 4 `blocked`, 1 `false-positive` — and **0 `triaged`**, i.e. the guard
  cannot have cost a fix in flight. The 33-row measurement in Class 7 above is
  the whole comment-line class; 16 of those were already suppressed by Guards
  B/E/G at line level, so 17 is the honest marginal delta.
- **End-to-end through the hook's real entrypoint — PASS, measured both ways.**
  One synthetic `PostToolUse` payload carrying three guard-comment lines, one
  genuine `npm warn deprecated glob@7.2.3`, and one BuildKit-prefixed
  `npm warn deprecated inflight@1.0.6`, against a throwaway project dir:

  | | ledger rows minted | dispatch |
  |---|---|---|
  | before Guard O (`git show HEAD:` copy of the hook) | **4** (2 junk) | 1 |
  | after Guard O | **2** (both genuine warnings) | 1 |

  Two regressions that matter are covered by the second row: the genuine
  warnings are still captured and still emit the dispatch directive, and the
  **BuildKit-prefixed warning survives in both runs** — the narrowing is what
  makes Guard O safe to key on `#`. The `before` run also reproduced
  `4e4f2598ff5c` byte-identically, confirming the harness drives the same path
  that minted the real capture.

Harness exit 0, `RESULT: ALL PASS`.

**Class 8 (Guard P) — verified and landed 2026-08-07.** Harness at the
Class-5/N/O standard (loads `deprecation-sentinel.py` directly, driving the
shipped `classify()` + `_is_echo_line()`):

- **Positives — 14/14 suppressed.** All seven fingerprints from the dispatching
  run, plus the join-shapes the class will keep inventing: `:`-join, bare
  space-join, tab/column, markdown table cell, JSON, python list-repr, a
  `grep -n`-prefixed readout, and a diff-ADDED readout. F1 and F2's own four
  shapes are included and pass, confirming Guard P **subsumes** Guard F rather
  than sitting beside it.
- **Negatives — 15/15 preserved, zero leaks.** The carve-out carries the
  verification: **eight cargo ` v<ver>+deprecated` shapes** — `Compiling`,
  `Checking`, `Downloaded … (registry …)`, the parenthesised join, the bare
  `cargo tree` form, a BuildKit-prefixed `#12 3.456 Compiling …`, and both
  live rows of the tombstone concern itself (`Updating holochain_sqlite
  v0.7.0-dev.17 -> v0.7.0+deprecated`, `Downgrading … -> v0.7.0-dev.24`). Plus
  two **co-located** cases proving step 3's residual net (a readout that also
  carries `is deprecated` prose; a readout beside a real npm warning), the
  tombstone's own `compile_error!` text, npm/pnpm/rustc/node channels, and a
  diff-added `#[deprecated(note = …)]` attribute.
- **Security class — 2/2 preserved.** Guard P is deprecation-gated, so an
  advisory line is never reached.
- **Whole-ledger stability — PASS.** Every live row re-tested under the `HEAD`
  hook and the new one: **8 of 276 change suppression state, all in the
  newly-suppressed direction, 0 newly captured.** Dispositions of the 8: 7
  `open`, 1 `false-positive`, and — as in every prior class — **0 `triaged`**.
  No readout capture in the ledger's history ever became an actionable fix. All
  8 were deleted in the landing commit as structurally unreachable (276 → 268).
- **End-to-end through the hook's real entrypoint — PASS, measured both ways.**
  One synthetic `PostToolUse` payload carrying three version readouts, the real
  cargo `Updating … -> v0.7.0+deprecated` line, and a BuildKit-prefixed
  `npm warn deprecated glob@7.2.3`, against a throwaway project dir:

  | | ledger rows minted | dispatch |
  |---|---|---|
  | before Guard P (`git show HEAD:` copy of the hook) | **5** (3 junk) | 1 |
  | after Guard P | **2** (both genuine) | 1 |

  The `before` run reproduced `f7f949929c67` **and** `ff2716a33179`
  byte-identically, confirming the harness drives the same path that minted the
  real captures. The two regressions that matter are covered by the second row:
  the genuine cargo tombstone line and the BuildKit npm warning are both still
  captured and still emit the dispatch directive. Guard P narrows the echo
  surface, not the warning surface.

Two **pre-existing Guard F leaks** were surfaced by this harness and are
deliberately **recorded, not fixed** — Guard F runs before P and is unaffected
by it. F matches on shape alone with no residual check and is not class-gated,
so it eats `serde_yaml-0.9.34+deprecated.crate: npm warn deprecated glob@7.2.3…`
(deprecation) and `RUSTSEC-2024-0437 in serde_yaml-0.9.34+deprecated.crate`
(security). Both are adversarial-only constructions: no toolchain emits a
registry artifact filename and a live warning on the same line. Folding F into
P's residual form would close them, but that is a behavioral change to a guard
with a clean live record and belongs to its own decision.

Harness exit 0, `RESULT: ALL PASS`.

**Classes 9–11 (Guards Q, R, A2) — verified and landed 2026-08-11.** Harness at
the Class-5/N/O/P standard (loads `deprecation-sentinel.py` directly, driving the
shipped `classify()` + `_is_echo_line()` with the shipped command-flag
computation):

- **Positives — 20/20 suppressed.** All three DEAD_WORDS fingerprints
  (`3054d0cb4bd7` / `802862c393b2` / `5723985e3232`), the memory-prose and
  `git diff .claude/memory/` captures, the `.epr-meta` and `.codex` homes read
  via `sed`/`tail`, the ledger-content self-read (`18ddd0fb6e64`), and all five
  triage-harness fixture rows. Separator coverage carries the Class-9 lesson:
  the **verbatim** dispatching command with its `\|` alternation, a `grep -E`
  bare `|`, a read piped into another reader, and multiple tooling operands.
- **Negatives — 20/20 preserved, zero leaks.** The carve-outs are the point:
  **compound commands** where a real warning sits beside a tooling read
  (`cargo build && grep … .claude/scripts/x.py`; `pnpm install && grep -rn …`;
  `docker build . && head .claude/hooks/…` with a BuildKit-prefixed warning),
  the `;`/`&&` separators the gate must refuse to cross, the Vitest
  `DEPRECATED:` banner (case-distinguished, hence its own residual clause),
  Guard N's directory-local `.epr-meta` manifest **file** negative, a **repo**
  script and a **repo** python tool that must not read as ephemeral, `/tmp` in
  argv without an interpreter running a scratch script, and every plain live
  channel (npm/pnpm/rustc/node/Vitest/tsc/git-push-banner/cargo-audit).
- **Whole-ledger stability — PASS.** Every live row re-tested under the pre-Q/R
  flags and the new ones: **13 of 275 change suppression state, all in the
  newly-suppressed direction, 0 newly captured.** Dispositions of the 13: 7
  `false-positive`, 4 `open`, 2 `blocked`, and **0 `triaged`**. The 2 `blocked`
  rows are `text: `-prefixed harness echoes whose concerns
  (`deprecation-doorway-warm-projection-cache-retire.md`,
  `deprecation-local-source-chain-service-retire.md`) retain 10 and 5 primary
  fingerprints respectively, so no citation was orphaned. Per-guard split,
  measured separately per the Class-5 bundling lesson: **Q 5 rows, R 7 rows,
  A2 1 row, overlap 0** — three independent fixes, none holding another hostage.
  All 13 deleted in the landing commit as structurally unreachable (275 → 262).
- **End-to-end through the hook's real entrypoint — PASS, measured both ways**,
  against throwaway project dirs:

  | Payload | | rows minted | dispatch |
  |---|---|---|---|
  | Q: tooling read + 2 genuine warnings | before (`git show HEAD:`) | **4** (2 junk) | 1 |
  | | after | **2** (both genuine) | 1 |
  | R: harness printing its fixture corpus | before (`git show HEAD:`) | **4** (all junk) | 1 |
  | | after | **0** | none |

  The `before` runs reproduced `3054d0cb4bd7` **and** `078248917097`
  byte-identically, confirming the harness drives the same path that minted the
  real captures. The regression that matters is covered by the Q `after` row:
  both genuine warnings are still captured and still emit the dispatch
  directive — Guard Q narrows the echo surface, not the warning surface.
- **Self-protection — PASS, and it is the cleanest demonstration available.**
  This harness ran from the scratchpad and minted **zero** ledger rows. Its
  direct predecessor — the Fix-N measurement script of 2026-08-11 — minted
  **five** (`2c22da9d20b3`, `f420501739eb`, `902442508a27`, `0a652fe0dc60`,
  `078248917097`). Guard R paid for itself inside the run that landed it.

Harness exit 0, `RESULT: ALL PASS`.

The Fix N (Class 3) and Class 4 measurements quoted in Current decision come
from the same script run in comparison mode over the live ledger, recomputing
each row's fingerprint under the candidate normalization and grouping by the
resulting hash.

Class 3 is verified by exact reproduction rather than by regex reasoning: both
`313c6eac27c1` and `fb31d99a0ba8` were **recomputed from the same warning text**
through the hook's own `fingerprint()` normalization, differing only in the
`grep -n` prefix (`6618:` vs `6620:`), and both matched the ledger byte-for-byte.
The bare-text variant hashes to `fe896c58f14e`. Fix N's regex is *not* yet
verified against adversarial negatives — that is owed before it lands.

Re-check trigger for the stasis sweep — **narrowed to Fix N (Class 3) and the
Class-4 routing change**, both of which need an operator-initiated pass:

1. **Fix N** — land the `grep -n` prefix strip with (a) its own
   adversarial-negative harness at the Class-5 standard, (b) the 105-row ledger
   migration, and (c) the 22 per-group status/backlog decisions written down.
   Confirm afterwards that re-grepping a *shifted* lockfile does **not** mint a
   new fingerprint, and that the migration left no live entry re-reading as NEW.
   Then delete the `fb31d99a0ba8` evidence line (its owner entry keeps the
   primary fp).
2. **Class 4** — decide the banner-routing change (pointer, not finding).
   Confirm afterwards that a root `pnpm install` whose package set changed does
   not mint a new aggregate fingerprint.

Delete this entry when both are discharged.

Two thirds of the original trigger are already discharged. Guards J/K: a fresh
`pnpm install` in a changed workspace now mints exactly ONE fingerprint per
deprecated package, because the summary twin is collapsed at source. Class 5:
repeated node processes emitting the same warning now mint exactly ONE
fingerprint, verified across 15 pid-variants.

Self-demonstrating footnote: `b1561f3d429d`, `9d31ba938515`, and
`010ff5a7bfb5` were minted *by this triage run*, when a `cat -A` inspection of
the already-captured install log re-emitted three warnings the ledger was
already holding. The defect cost three dispatch directives inside the very run
that diagnosed it.

Third footnote, 2026-07-30 — this entry has now been *self-demonstrating three
times over*. The triage run that landed the Class-5 fix was itself dispatched by
a Class-5 re-mint (`18bf6594df8d`, the eighth pid-variant of one punycode
warning), and while that run was working, Class 4 fired and requested a *ninth*
dispatch (`6bbd169077f5`). Two wasted dispatches inside one run, from two
different classes of the same defect. That is the argument for treating
fingerprint stability as infrastructure rather than as cleanup.

Second footnote, 2026-07-30 — the cost is now measured, not projected. Class 3
**spent a full background Opus dispatch**: `fb31d99a0ba8` was triaged as a new
finding, and the entire finding was that a lockfile line had moved two rows
under a jQuery concern already canonicalized and already `blocked`. That is the
steady-state price of leaving this unlanded — not a one-time diagnosis cost but
one dispatch per lockfile shift, per workspace, indefinitely.

Fourth footnote, 2026-09-02 — **Class 3 confirmed on a non-lockfile channel, and
a new sub-class named: the cwd-relative diagnostic path prefix.** One
TypeScript `TS5107` warning from `@elohim/storage-client`'s `tsc`
(`deprecation-storage-client-ts-moduleresolution-node10.md`) minted **four**
fingerprints:

| fp | Variant | Minted by |
|---|---|---|
| `9ab4dfa901ea` | `grep -n` prefix `1799:` | the sentinel's own capture command |
| `dc758c9c3d3f` | `grep -n` prefix `526:` — same file, second hit | the same command, same invocation |
| `6ffe65fc044a` | **bare text**, no prefix (`sed -n` read of the same log) | **the triage agent, reading its own evidence** |
| `c8babb72ae2c` | text prefixed `elohim/sdk/storage-client-ts/` | **the triage agent, reproducing the failure** |

The first two are textbook Class 3, and they close the open question the Class-3
analysis left: the defect is **not lockfile-specific**. Any `grep -n` over any
multi-hit log re-mints, and here it minted twice inside a *single* command
because one warning appeared at two offsets in one file. Fix N's prefix strip
would have collapsed all three of `9ab4dfa901ea` / `dc758c9c3d3f` /
`6ffe65fc044a` to one row — the bare-text variant `6ffe65fc044a` is precisely
the `fe896c58f14e`-shaped normalization target Fix N already predicts.

`c8babb72ae2c` is the **new sub-class**, and Fix N does *not* cover it. A
compiler diagnostic carries a path that is relative to the compiler's invoking
directory. The identical `tsc` error hashes one way when run from inside
`elohim/sdk/storage-client-ts` (`tsconfig.json(25,25): error TS5107: …`) and
another when run from the repo root with `-p` (`elohim/sdk/storage-client-ts/
tsconfig.json(25,25): error TS5107: …`). Nothing about the finding changed —
only the cwd of the process that observed it. Every compiler, linter, and test
runner in the monorepo emits cwd-relative paths, so this class is live on
`tsc`, `cargo`, `eslint`, and `vitest` channels alike. A candidate normalization
sits alongside Fix N: strip a leading repo-relative directory prefix from a
`path(line,col): error CODE:` shaped diagnostic before hashing. It needs the
same adversarial-negative harness at the Class-5 standard before it lands —
over-stripping would collapse the *same* diagnostic code from two genuinely
different files into one row, which is worse than the redundancy it cures.

**The costliest observation is the mechanism, not the count.** Both self-minted
rows came from the triage agent doing its job correctly: `6ffe65fc044a` from
*reading the captured evidence*, `c8babb72ae2c` from *reproducing the failure to
verify a candidate fix*. Those are the two irreducible steps of triage. So the
defect is not merely "an agent occasionally re-emits a warning" — it is that
**verifying a deprecation fix mints new deprecation fingerprints by
construction**, each requesting a fresh background Opus dispatch for the concern
already under triage. Left unlanded, the automation's cost scales with its own
diligence. Both directives were declined by hand this run; an unwitting agent
would have dispatched two redundant Opus runs, and each of those would in turn
have reproduced and re-minted.

That closes the argument the second footnote opened with a measured price: the
steady-state cost is not one dispatch per lockfile shift, it is one-to-two
dispatches per *triage run*, on every channel, indefinitely.
