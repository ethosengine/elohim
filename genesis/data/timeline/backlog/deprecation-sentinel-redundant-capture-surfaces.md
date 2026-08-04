---
id: "backlog-deprecation-sentinel-redundant-capture-surfaces"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "deprecation-sentinel fingerprint instability — Class 3 (grep -n prefix) and Class 4 (aggregate-banner drift) remain after Guards J/K and the Class-5 pid fix landed"
slug: "deprecation-sentinel-redundant-capture-surfaces"
written: "2026-07-30"
author: "deprecation-triage"
status: "backlog"
priority: "high"
deprecation_status: blocked
severity: medium
fingerprints: []
evidence_fingerprints: ["fb31d99a0ba8"]
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

## Usage inventory

Single file — `.claude/hooks/deprecation-sentinel.py`:

- `DEPRECATION_PATTERNS` (~line 58) — `\bDEPRECATED\b` under `re.IGNORECASE`
  matches the bare word `deprecated`, which is what admits the Class-1 summary
  line. Patterns at lines 62–66 are consequently redundant.
- `_is_echo_line()` — where the structural guards live. Guards A–N are now
  present (J and K from this entry; L and M from the derived-artifact /
  JSDoc-annotation class landed in the same commit; N from the Class-6
  four-homes gap, folded into Guard E's own regex). No remaining work here.
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
fixed in the landing commit. **Classes 3 and 4 remain BLOCKED**, and they are
now the entire remaining concern.

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
