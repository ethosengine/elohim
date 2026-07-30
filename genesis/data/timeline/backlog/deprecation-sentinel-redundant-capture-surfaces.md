---
id: "backlog-deprecation-sentinel-redundant-capture-surfaces"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "deprecation-sentinel mints N fingerprints per real warning — collapse the redundant capture surfaces"
slug: "deprecation-sentinel-redundant-capture-surfaces"
written: "2026-07-30"
author: "deprecation-triage"
status: "backlog"
priority: "high"
deprecation_status: blocked
severity: medium
fingerprints: []
evidence_fingerprints: ["bba59aabdf63", "b1561f3d429d", "9d31ba938515", "010ff5a7bfb5"]
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

## Usage inventory

Single file — `.claude/hooks/deprecation-sentinel.py`:

- `DEPRECATION_PATTERNS` (~line 58) — `\bDEPRECATED\b` under `re.IGNORECASE`
  matches the bare word `deprecated`, which is what admits the Class-1 summary
  line. Patterns at lines 62–66 are consequently redundant.
- `_is_echo_line()` (~line 351) — where the structural guards live. Guards
  A–I are present; the two new classes need J and K.
- The diff-hunk exemptions in Guards C and G (`not line.startswith(("+","-"))`)
  do **not** cover Class 1: pnpm's summary marker *is* `+`/`-`, so the shape
  must be matched precisely rather than exempted.

Ledger evidence (`.claude/data/deprecations.jsonl`): 4 redundant live entries
against 3 genuine concerns.

## Migration path

Two additive guards in `_is_echo_line()`, both regex-verified against real log
bytes plus adversarial negatives (6/6 and 3/3 positives, 9/9 and 4/4 negatives,
zero leaks).

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

Worth folding in while there: `\bDEPRECATED\b` + `re.IGNORECASE` makes the
bare word `deprecated` a catch-all and renders lines 62–66 dead. Guard G's own
comment states the intent — "real deprecations are lowercase prose … Vitest
`DEPRECATED: <prose>`" — i.e. `\bDEPRECATED\b` was meant to be the
case-*sensitive* Vitest banner. Tightening that is a broader behavioral change
than the two guards and should be measured separately, not bundled.

## Current decision

**BLOCKED — needs an operator-approved edit to a protected configuration
surface.** `.claude/hooks/deprecation-sentinel.py` is hook configuration; the
`Edit` was denied by the permission classifier, and a background triage agent
must not work around a permission denial or self-authorize a config change.
The patch above is verified and paste-ready; landing it needs an operator in
the loop.

**This entry deliberately canonicalizes ZERO ledger fingerprints**
(`fingerprints: []`). The four redundant surfaces are listed under
`evidence_fingerprints` instead, and their ledger lines cite the *package*
concern each is a surface of — authored independently by the concurrent triage
run that owned the primary surfaces:

| Evidence fp | Redundant surface of | Ledger cites (owner entry) |
|---|---|---|
| `bba59aabdf63` | `819aa7c6f6bd` eslint@8.57.1 EOL | `deprecation-sophia-eslint-8-eol-flat-config-migration.md` |
| `b1561f3d429d` | `819aa7c6f6bd` eslint@8.57.1 EOL | `deprecation-sophia-eslint-8-eol-flat-config-migration.md` |
| `9d31ba938515` | `011f5406331d` jquery@2.1.1 XSS family | `security-jquery-2-1-1-shipped-in-sophia-umd-bundle.md` |
| `010ff5a7bfb5` | `50aa3734f6b0` intersection-observer@0.12.2 | `deprecation-sophia-intersection-observer-dead-declaration.md` |

That routing is deliberate: a redundant surface carries no independent concern,
so on re-encounter the sentinel should cite what the line is *about* (eslint 8
is EOL, blocked) rather than a hook defect. Those three owner entries already
list these fps, so the mapping is N:1 onto concerns with no duplicate claim.

None of the four is deleted. Deleting them would be wrong while the guards are
unlanded — each deleted line would read as NEW on the next `pnpm install` and
re-fire a dispatch, which is exactly the cost this entry exists to remove. They
become deletable when Guards J/K land and the class is collapsed at source.

## Verification

Not fixed — no verification to record. The regexes are verified in isolation
against real captured bytes (see Migration path); the guards themselves are
unlanded, so the sentinel's live behavior is unchanged.

Re-check trigger for the stasis sweep: once Guards J/K land in
`.claude/hooks/deprecation-sentinel.py`, confirm a fresh `pnpm install` in a
changed pnpm workspace mints exactly ONE fingerprint per deprecated package
(not two), then delete the four `evidence_fingerprints` lines from the ledger
(their owner entries keep the primary fps) and delete this entry.

Self-demonstrating footnote: `b1561f3d429d`, `9d31ba938515`, and
`010ff5a7bfb5` were minted *by this triage run*, when a `cat -A` inspection of
the already-captured install log re-emitted three warnings the ledger was
already holding. The defect cost three dispatch directives inside the very run
that diagnosed it.
