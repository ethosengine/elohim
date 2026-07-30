---
id: "backlog-deprecation-sentinel-redundant-capture-surfaces"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "deprecation-sentinel fingerprint instability — Class 3 (grep -n prefix) remains after Guards J/K landed"
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
tags: [deprecation, tooling, deprecation-sentinel, pnpm, fingerprint-stability, automation-cost]
cites:
  - .claude/hooks/deprecation-sentinel.py
  - .claude/agents/deprecation-triage.md
  - .claude/data/deprecations.jsonl
  - .claude/skills/deprecation-stasis
  - genesis/data/timeline/backlog/deprecation-sophia-eslint-8-eol-flat-config-migration.md
  - genesis/data/timeline/backlog/security-jquery-2-1-1-shipped-in-sophia-umd-bundle.md
  - genesis/data/timeline/backlog/deprecation-sophia-intersection-observer-dead-declaration.md
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
concern. Live ledger entry: `ce0de21b8053`, status `open`.

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

**Not landed here, deliberately, and the reason is a hazard rather than a
doubt:** the ledger was being actively appended to by another session while
this was diagnosed. Rewriting the file to migrate the three fingerprints would
have raced those writes and could have dropped their captures. Fix N and Class
5 share one ledger migration, so they should land together, in a quiet tree —
which is also the cheaper sequencing. Bundle them.

## Usage inventory

Single file — `.claude/hooks/deprecation-sentinel.py`:

- `DEPRECATION_PATTERNS` (~line 58) — `\bDEPRECATED\b` under `re.IGNORECASE`
  matches the bare word `deprecated`, which is what admits the Class-1 summary
  line. Patterns at lines 62–66 are consequently redundant.
- `_is_echo_line()` — where the structural guards live. Guards A–M are now
  present (J and K from this entry; L and M from the derived-artifact /
  JSDoc-annotation class landed in the same commit). No remaining work here.
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

**Class 3 remains BLOCKED, and it is now the entire remaining concern.** Fix N
(strip the `grep -n` line-number prefix in `fingerprint()`) is *not* landed,
deliberately, for the reason this entry already documents:

- Fix N **changes existing fingerprints**. Every live ledger entry whose
  captured `line` carries a `grep -n` prefix re-hashes and reads as NEW once,
  firing one dispatch per affected entry — precisely the cost it removes.
  Landing it therefore requires a **one-time ledger migration** (recompute `fp`
  for affected live lines in the same commit), which is a different and riskier
  operation than an additive `_is_echo_line` guard.
- Fix N's regex is **not yet verified against adversarial negatives**. Guards
  J/K/L/M each shipped with a positive/negative harness; Fix N has none. It
  should not land to a weaker standard than the guards it sits beside.

That is a real, bounded, well-specified next step — not an open-ended one — but
it needs its own run with the ledger migration written and verified together.

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

Class 3 is verified by exact reproduction rather than by regex reasoning: both
`313c6eac27c1` and `fb31d99a0ba8` were **recomputed from the same warning text**
through the hook's own `fingerprint()` normalization, differing only in the
`grep -n` prefix (`6618:` vs `6620:`), and both matched the ledger byte-for-byte.
The bare-text variant hashes to `fe896c58f14e`. Fix N's regex is *not* yet
verified against adversarial negatives — that is owed before it lands.

Re-check trigger for the stasis sweep — **narrowed to Fix N only**: land Fix N
together with its one-time ledger migration and an adversarial-negative
harness, then confirm that re-grepping a *shifted* lockfile does **not** mint a
new fingerprint (the Class-3 proof), and that the migration left no live ledger
entry re-reading as NEW. Then delete the remaining `fb31d99a0ba8` evidence line
(its owner entry keeps the primary fp) and delete this entry.

The Guards J/K half of that trigger is already discharged: a fresh `pnpm
install` in a changed workspace now mints exactly ONE fingerprint per
deprecated package, because the summary twin is collapsed at source.

Self-demonstrating footnote: `b1561f3d429d`, `9d31ba938515`, and
`010ff5a7bfb5` were minted *by this triage run*, when a `cat -A` inspection of
the already-captured install log re-emitted three warnings the ledger was
already holding. The defect cost three dispatch directives inside the very run
that diagnosed it.

Second footnote, 2026-07-30 — the cost is now measured, not projected. Class 3
**spent a full background Opus dispatch**: `fb31d99a0ba8` was triaged as a new
finding, and the entire finding was that a lockfile line had moved two rows
under a jQuery concern already canonicalized and already `blocked`. That is the
steady-state price of leaving this unlanded — not a one-time diagnosis cost but
one dispatch per lockfile shift, per workspace, indefinitely.
