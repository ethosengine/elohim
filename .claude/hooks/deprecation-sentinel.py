#!/usr/bin/env python3
"""deprecation-sentinel — PostToolUse(Bash) hook.

Watches Bash tool output for deprecation warnings AND security-
vulnerability reports emitted in flight (Vitest "DEPRECATED ...",
npm "npm warn deprecated", Node DeprecationWarning, Rust "use of
deprecated"; pnpm/npm install + audit vulnerability summaries, GitHub
push banners "found N vulnerabilities", CVE-/GHSA-/RUSTSEC advisories)
against two stores with crisp roles:

  1. Ledger = the EXISTING-POSITIVES CHECK SURFACE (decides firing):
     .claude/data/deprecations.jsonl — one line per LIVE fingerprint
     {ts, fp, line, cmd, status, backlog?}. Status: open → triaged →
     blocked. A fingerprint PRESENT here suppresses dispatch; a
     fingerprint ABSENT fires the dev. FIXED items are DELETED from
     the ledger at close (full memory decomposition — the git commit
     is the record), so a reintroduced deprecation reads as NEW and
     correctly re-fires: regression handling for free.
  2. Canonical backlog = the CLOSE-OF-TRIAGE DECISION SURFACE:
     genesis/data/timeline/backlog/deprecation-*.md — timeline-
     CONVENTIONS-conformant entries holding the live trajectory
     ("Current decision" is the citation line for blocked items).
     Fixed-no-work-left entries are DELETED (rarely graduated to
     timeline/chronicle/ when genuinely meaningful) — everything in
     the backlog has a trajectory or a status, or it's not there.

Behavior:
  * NEW fingerprint  → append to ledger + inject a dispatch directive:
    the session launches the `deprecation-triage` agent (Opus) in the
    BACKGROUND — flag → scope → canonicalize → fix|block — and carries
    on with its current task.
  * KNOWN fingerprint (live: open/triaged/blocked) → once per session,
    inject a one-line deterministic citation of the current decision
    (backlog path + status). No agent dispatch — blocked-and-
    canonicalized items never re-fire automation; the
    deprecation-stasis sweep re-checks blockers deliberately.
  * Command itself mentions deprecation (greps, ledger edits, this
    hook's own tooling) → skip entirely (false-positive guard).

Fail-safe: any internal error exits 0 silently — the sentinel must
never break a session.
"""

# The intervenor's removal condition (Meadows' shifting-the-burden trap;
# counted by _lib/intervenor_census.py). A condition, never a date.
RETIRE_WHEN = (
    "when new deprecation warnings fail a CI gate directly rather than being scraped out of "
    "Bash output in flight, AND the ledger holds at zero open fingerprints across a full "
    "dependency-bump cycle. This hook is a stand-in for a gate that does not exist yet; the "
    "gate is the exit."
)

import hashlib
import json
import os
import re
import sys
from datetime import datetime, timezone

MAX_SCAN_BYTES = 200_000  # bound the regex scan on huge outputs
MAX_NEW_PER_CALL = 5  # bound dispatch-directive noise from one command
MAX_CITED_PER_CALL = 3  # bound re-encounter citation noise
LINE_TRUNC = 300

# Deprecation signatures across the toolchains in this repo
# (Vitest/Vite, npm/pnpm, Node, Angular, Rust, Python, generic).
DEPRECATION_PATTERNS = re.compile(
    r"(?:"
    r"\bDEPRECATED\b"
    r"|\bDeprecationWarning\b"
    r"|npm warn deprecated"
    r"|\buse of deprecated\b"
    r"|\b(?:is|are|has been|was|were) deprecated\b"
    r"|\bdeprecated (?:and|API|option|in|since)\b"
    r"|\bwill be removed in (?:a future|the next|version|v?\d)"
    r")",
    re.IGNORECASE,
)

# Security-vulnerability signatures from dependency pull/install/audit stages
# (pnpm/npm install + audit summaries, GitHub push banners, cargo-audit
# RUSTSEC advisories, GHSA/CVE identifiers).
SECURITY_PATTERNS = re.compile(
    r"(?:"
    r"\b(?!0 )\d+ vulnerabilit(?:y|ies)\b"  # zero-count audit lines are CLEAN results, not findings
    r"|\bCVE-\d{4}-\d+\b"
    r"|\bGHSA-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{4}\b"
    r"|\bRUSTSEC-\d{4}-\d+\b"
    r"|\bsecurity advisor(?:y|ies)\b"
    r"|\b(?:critical|high|moderate|low) severity vulnerab"
    r")",
    re.IGNORECASE,
)

# Count-bearing security SUMMARY lines ("found 191 vulnerabilities (1
# critical, 113 high...)"): counts drift run-to-run, so their fingerprint is
# digit-normalized — one live concern, stable across count churn. Advisory-ID
# lines (CVE-/GHSA-/RUSTSEC-) are NOT normalized: distinct advisories must
# stay distinct fingerprints.
SECURITY_SUMMARY = re.compile(
    r"\d+ vulnerabilit|\(\d+ (?:critical|high|moderate|low)", re.IGNORECASE
)

# Node prefixes its own process warnings with the EMITTING PROCESS ID:
#   (node:810849) [DEP0040] DeprecationWarning: The `punycode` module ...
# The pid is pure run-identity, never concern-identity, so leaving it in the
# hashed string mints one fingerprint PER PROCESS — a single test suite can
# mint triage dispatches faster than triage can retire them, forever, for one
# long-known upstream warning. Collapsing the pid makes the concern the unit.
# Deliberately narrow: only the literal `(node:<digits>)` wrapper is touched,
# so DEP-codes, package versions and message text all stay concern-bearing.
NODE_PID_PREFIX = re.compile(r"\(node:\d+\)")

# Commands that themselves talk about these signals (greps, ledger edits,
# this tooling) are not new in-flight findings.
GUARD_TOKENS = ("deprecat", "vulnerab", "cve-", "ghsa-", "rustsec", "advisor")

ANSI = re.compile(r"\x1b\[[0-9;]*[A-Za-z]")

# ── Anti-echo guards (three deterministic false-positive classes) ─────────────
#
# Guard A — ledger self-capture:
#   A grep that recurses into the repo may hit the ledger file itself and
#   re-fingerprint already-triaged entries.  A grep-with-filename result line
#   looks like "/path/to/deprecations.jsonl:42:{...json...}" so we match any
#   line that contains "deprecations.jsonl" as a source path prefix.
ECHO_LEDGER_PATH = re.compile(r"deprecations\.jsonl", re.IGNORECASE)

# Guard B — managed decision-record / museum / planning prose:
#   Lines sourced from the project's deprecation-narrating managed surfaces:
#     - genesis/docs/content/elohim-protocol/history/**  (museum / arc records)
#     - genesis/data/timeline/backlog/**                 (the canonical backlog
#       decision records THIS sentinel itself writes — every deprecation-*.md
#       carries "deprecated" in its title/tags/author frontmatter by design)
#     - genesis/data/timeline/chronicle/**               (graduated-lesson records)
#     - genesis/docs/superpowers/**                      (plan / spec / brief
#       docs whose task headers narrate work — "### Task 1: Retire the dead
#       Groovy helpers + fix the inverted DEPRECATED label")
#     - .superpowers/**                                  (the SDD working tree:
#       progress.md task-checkbox ledgers, per-task reports, review diffs —
#       "- [x] Task 1: complete … DEPRECATED comment removed …")
#     - CLAUDE.md (root + per-project, e.g. bridges/CLAUDE.md)  (the gospel
#       instruction docs, which narrate deprecated APIs in their Gotchas
#       sections by design — e.g. the root CLAUDE.md line "Swarm event loop
#       uses `StreamExt::next()` (not the deprecated `select_next_event()`)").
#   These narrate (or canonicalize) deprecations; reading them back is never a
#   NEW in-flight finding.  fp-dedupe cannot suppress the class: each new
#   backlog/chronicle/plan/progress edit mints a fresh fingerprint on the next
#   cat/grep/tail, so only a structural guard collapses it (cf. the 2026-06-17
#   + 2026-06-20 backlog-frontmatter captures, and the 2026-06-25 SDD-ledger
#   c4c3eccc7e05 + plan-header d09bbe4004a6 captures during the P6 sprint; and
#   the 2026-07-21 CLAUDE.md-prose capture 97d4865837a9 — a scope grep of the
#   root CLAUDE.md libp2p gotcha matched its prose "the deprecated
#   `select_next_event()`", which is a doc narrating a *past* upstream API
#   deprecation, not a live toolchain warning).
#   They appear in grep-with-filename output as a leading file path, or the
#   command itself directly reads such a file (cat/head/tail/sed/awk of a
#   single doc has no path prefix — the command gate below covers that case).
ECHO_HISTORY_PATH_RE = re.compile(
    r"genesis/docs/content/elohim-protocol/history/"
    r"|genesis/data/timeline/(?:backlog|chronicle)/"
    r"|genesis/docs/superpowers/"
    r"|\.superpowers/"
    r"|(?:^|/)CLAUDE\.md\b"
    r"|(?:^|/)VULNERABILITY_CLUSTER[A-Z0-9_]*\.md\b"  # Guard H1 (see below)
    r"|genesis/research/repos/",  # Guard I (see below)
    re.IGNORECASE,
)

# Guard C — commit-message prose:
#   git log --oneline produces lines like "8f0cb4122 chore(deprecation): …"
#   git show --stat/--format produces commit subject lines verbatim.
#   Heuristic: a line that starts with a short hex run followed by a space is a
#   git log oneline entry.  Additionally, bare commit-subject lines beginning
#   with conventional-commit prefixes (chore(deprecat…)/fix(deprecat…) etc.)
#   are commit message text, not live runtime output.
#   We do NOT gate on the command being a git command alone — a `git show` DIFF
#   hunk (lines starting with + or -) can legitimately add a `#[deprecated]`
#   attribute and should still be captured.  The shape guards below are
#   line-level and do not suppress diff hunks.
# Guard E — agentic tooling-source echo:
#   Review findings, hook/scripts source lines, and workflow reports quote the
#   sentinel's own vocabulary ("deprecated", "N vulnerabilities") while naming
#   `.claude/` tooling paths or review-finding markers. Real dependency
#   install/audit output never references `.claude/` — so any matching line is
#   an echo of our own tooling, not a live finding (3 spurious captures on
#   2026-07-02 came from printing a code-review findings list).
#
# Guard N (2026-08-04) — the SAME agent surfaces under their OTHER two homes.
#   Guard E keyed on `.claude/`, but every one of those surfaces is also
#   stored content-addressed in the EPR package store and re-projected per
#   agent runtime:
#     `.epr-meta/elohim/packages/skills/<name>.json`        (package: body inline)
#     `.epr-meta/elohim/projections/{claude,codex,codexProject}/skills/…`
#     `.codex/{skills,agents,commands}/…`                   (the codex mirror)
#   So one skill body lives in up to four places, and a repo-wide scope grep
#   hits all of them while Guard E dismisses only the `.claude/` one. Live
#   proof: fp `09f2f5632c00` (2026-08-04) — a scope grep for
#   `enable_mdns|enable_relaying` hit `packages/skills/libp2p-transport.json`,
#   whose inlined body carries the prose "not the deprecated
#   `select_next_event()`". That is the SAME sentence Guard B already dismisses
#   from the root CLAUDE.md (fp `97d4865837a9`, 2026-07-21) — a doc narrating a
#   *past* upstream libp2p API deprecation, long since migrated. It cost a full
#   background Opus dispatch to re-derive a dismissal the guards already held.
#   fp-dedupe cannot collapse the class: each package re-seal rewrites the JSON
#   and every projection mints its own distinct text, so only a structural
#   path guard closes it.
#   Deliberately narrow, two ways: the `.epr-meta/` clause requires a trailing
#   slash, so it matches ONLY the root package store and never the
#   directory-local `.epr-meta` compose-gate manifest FILES (`elohim/.epr-meta`,
#   `doorway/doorway-service/.epr-meta` — no slash follows those); and the
#   `.codex/` clause enumerates the agent-surface dirs, so `.codex/config.toml`
#   stays capturable. Zero true-positive risk: a live toolchain warning is
#   never sourced from a checked-in agent-doc projection — the finding, if real,
#   belongs to the code the doc describes.
ECHO_TOOLING_SOURCE_RE = re.compile(
    r"\.claude/(?:hooks|scripts|skills|agents|data|memory)/"
    r"|(?:^|/|\s)\.epr-meta/"  # Guard N: EPR package store + agent projections
    r"|(?:^|/|\s)\.codex/(?:skills|agents|commands)/"  # Guard N: codex mirror
    r"|### FINDING\b"
    r"|\bVERDICT: (?:CONFIRMED|REFUTED|UNCERTAIN)\b",
)

ECHO_GIT_ONELINE_RE = re.compile(r"^[0-9a-f]{7,12} ")
ECHO_COMMIT_SUBJECT_RE = re.compile(
    r"^(?:chore|fix|feat|refactor|docs|test|perf|ci|build|revert)"
    r"\((?:deprecat|security|sentinel)[^)]*\)\s*:",
    re.IGNORECASE,
)

# Guard D — ephemeral-script self-capture:
#   A DeprecationWarning whose SOURCE marker is literally `<string>:N:` or
#   `<stdin>:N:` was emitted by code run via `python3 -c "…"`, `exec()`, or
#   stdin — by Python's own convention these markers NEVER name a checked-in
#   source file (a real file warning carries its path: "foo.py:12:"). These
#   are an agent's own ad-hoc inline scripts (e.g. parsing an MCP tool-result
#   .txt with datetime.utcfromtimestamp), scrolled past in Bash output and
#   re-fingerprinted line-by-line. Every one of the 14 priors of this exact
#   shape (utcfromtimestamp / PIL getdata) was dispositioned false-positive,
#   and because the marker's line number differs each run, fp-dedupe can never
#   suppress the class — only this shape guard can. The marker may sit at
#   line-start OR after a grep line-number prefix ("1110: <string>:1: …"), so
#   we search rather than anchor. Zero true-positive risk: a genuine codebase
#   deprecation is always sourced from a named path or a toolchain prefix.
ECHO_EPHEMERAL_SOURCE_RE = re.compile(r"<(?:string|stdin)>:\d+:")

# Guard F — crates.io `+deprecated` build-metadata self-capture (two forms):
#   A crates.io semver BUILD-METADATA suffix like `+deprecated` (a maintainer's
#   in-band end-of-life signal, e.g. `serde_yaml 0.9.34+deprecated`) surfaces in
#   two structurally identical, ZERO-true-positive self-capture shapes when a
#   scope pass reads the dependency tree:
#     F1) Cargo.lock / Cargo.toml QUOTED version literal — `version =
#         "0.9.34+deprecated"`. Any grep of a lockfile for that crate re-emits it.
#     F2) A cargo REGISTRY ARTIFACT FILENAME — the download/cache `.crate` file
#         (`serde_yaml-0.9.34+deprecated.crate` from `ls ~/.cargo/registry/cache/`)
#         or the extracted source dir (`serde_yaml-0.9.34+deprecated/` from
#         `ls .../registry/src/`). The `+deprecated` sits in a HYPHEN-joined
#         name-version token (`-<digit>…+deprecated`), optionally `.crate`-suffixed.
#   Both defeat fp-dedupe: `grep -n`/context prefixes (`561-`, `560:`) and each
#   distinct crate/version mint a FRESH fingerprint (same structural problem as
#   Guards B/D). Both are REGISTRY METADATA, not a live in-flight toolchain
#   warning: a genuine build/compile deprecation is emitted UNQUOTED and
#   prose-shaped with a SPACE-`v` version join (`Compiling serde_yaml
#   v0.9.34+deprecated`, `Downloaded serde_yaml v0.9.34+deprecated`, `use of
#   deprecated …`) — never a `version = "…"` assignment nor a `-<digit>…+deprecated`
#   filename token — so it stays capturable (the live build line is already the
#   stable fp 8cc10bf8a03b). Zero true-positive risk. (F1: two triage dispatches on
#   2026-07-06 — 83fcd27c645c / 30de7ef1fec4 — from lockfile scope greps. F2:
#   f7d6c5fa7c1c on 2026-07-18 from an `ls registry/cache/` scope probe. The
#   serde_yaml archived-crate concern is already canonicalized+blocked.)
ECHO_LOCKFILE_VERSION_META_RE = re.compile(
    r'version\s*=\s*"[^"]*\+deprecated"'          # F1: Cargo.lock/toml version literal
    r'|-\d[0-9A-Za-z.]*\+deprecated(?:\.crate)?\b',  # F2: registry cache/src artifact filename
    re.IGNORECASE,
)

# Guard P (2026-08-07) — EXTRACTED `+deprecated` BUILD-METADATA READOUT: the
#   general form of Guard F, keyed on the invariant F already rests on rather
#   than on two hard-coded syntaxes.
#   Guard F dismisses a `+deprecated` build-metadata token in exactly two
#   shapes — the QUOTED toml literal (F1) and the HYPHEN-joined registry
#   filename (F2). Both are the token as the *toolchain* stores it. But the
#   moment an agent EXTRACTS that version and reprints it, the shape changes
#   and the guard stops seeing it, while the semantics are identical: still
#   registry metadata, still not a live warning. A scope pass comparing lock
#   versions across workspaces — the exact diagnostic that triaging a blocked
#   dependency concern requires — emits
#     `holochain_sqlite: storage=0.7.0-dev.19 steward=0.7.0+deprecated`
#   and every join (`=`, `:`, tab, table cell, JSON, python list-repr) plus
#   every version pair mints a FRESH fingerprint, so fp-dedupe can never
#   collapse the class (same defeat as Guards B/D/F/G/L/O).
#   Measured live 2026-08-07: SEVEN fingerprints for ONE already-`blocked`
#   concern inside ten minutes — f7f949929c67 (main session's version-compare
#   loop), e898eca36448 (its python re-do, 98s later, list-repr shape), and
#   five more (f4d8b5b3c714 / c40d6cb35607 / 8db0f74af09c / 1249d4932448 /
#   9ebdacdc42e2) from the triage run's own verification harness. Two dispatch
#   directives requested for readouts of a decision already written down.
#   The guard is stated as an INVARIANT with an explicit live-channel carve-out
#   and a residual-signal safety net, in three steps:
#     1. the line carries a semver build-metadata `+deprecated` token; AND
#     2. that token is NOT in cargo's live prose shape — a SPACE-`v` version
#        join (`Compiling serde_yaml v0.9.34+deprecated`, `Updating … ->
#        v0.7.0+deprecated`). This is the carve-out that keeps the real channel
#        open, and it is exactly the invariant Guard F's own comment asserts;
#        AND
#     3. with every build-metadata token REMOVED, no deprecation signal
#        remains in the line. This is the safety net: a readout that also
#        carries a genuine warning still captures, so the guard can only ever
#        dismiss a line whose ONLY deprecation signal is the metadata token.
#   Deprecation class only — a security-class advisory is never dismissed here.
#   Deliberately NOT diff-exempt (same call as Guards H2/M/O): a readout is a
#   readout whether it is being added to a doc or merely re-printed, and the
#   `+`-prefixed add is a dominant shape because writing the comparison table
#   is what the triage work DOES.
BUILD_META_DEPRECATED_RE = re.compile(r"\d[0-9A-Za-z.\-]*\+deprecated\b", re.IGNORECASE)
CARGO_VERSION_JOIN_RE = re.compile(
    r"(?:^|[\s>=(\[,])v\d[0-9A-Za-z.\-]*\+deprecated\b", re.IGNORECASE
)

# Guard G — in-source DEPRECATED section-header / doc-comment self-capture:
#   A scope-pass grep or cat of the codebase's OWN source surfaces comment
#   banners and doc-comments that NARRATE a deliberate, migration-retained
#   deprecation. content_store/src/lib.rs, for example, carries section
#   headers `// Learning Path Operations (DEPRECATED)` and doc-comments
#   `/// DEPRECATED: Use create_content with content_type "path" instead.`
#   above zome fns that INTENTIONALLY return a deprecation error and are
#   retained by design for reading existing DHT entries during migration
#   (see elohim/holochain/dna/CLAUDE.md — the DHT is a notary; deprecated
#   write paths error, read paths stay). These lines are the deprecation-
#   IMPLEMENTATION surface, not a live in-flight toolchain warning. Any
#   re-scan re-emits them, and `grep -n` / `-A` context prefixes (plus sed
#   rewrites that strip the leading `//`, as the capturing pipeline did) mint
#   a FRESH line-number-bearing fingerprint each run, so fp-dedupe can never
#   collapse the class — only a structural guard can (same defeat as Guards
#   B/D/F). Two shapes, both ZERO true-positive because live toolchain
#   deprecations are lowercase prose (`use of deprecated`, `npm warn
#   deprecated`, Vitest `DEPRECATED: <prose>`, `DeprecationWarning`) and NEVER
#   a trailing all-caps `(DEPRECATED)` section label nor a `//`-prefixed
#   source-comment line:
#     G1) a trailing all-caps parenthesized section label `(DEPRECATED)$`
#         — covers `4085  Learning Path Operations (DEPRECATED)` from the
#         section-header grep, and a cat'd `// Chapter Operations (DEPRECATED)`.
#     G2) a source line-comment / doc-comment (`//`, `///`, `//!`) that
#         mentions DEPRECATED — covers a `grep -rn DEPRECATED lib.rs` scope
#         pass emitting `lib.rs:4164:/// DEPRECATED: Use create_content …`.
#   Diff hunks (lines starting `+` / `-`) are exempt at the call site — a
#   commit that ADDS a #[deprecated] attribute or a deprecation doc-comment
#   must still capture. (Three triage dispatches on 2026-07-10 —
#   2730dafbdcc2 / e06ddf1806ad / c1065020cbb6 — were this class, from a
#   content_store section-header scope grep; the fns are intentionally
#   deprecated-and-retained, so there was no live concern to canonicalize.)
# Guard J — pnpm PACKAGE-CHANGES-SUMMARY redundant surface:
#   pnpm reports a deprecated dependency TWICE in one install, from two stages.
#   The resolution stage emits the real warning, carrying the upstream message
#   and URL (` WARN  deprecated eslint@8.57.1: This version is no longer
#   supported. Please see https://eslint.org/version-support …`). The
#   end-of-install lockfile-diff listing then emits a bare marker with no
#   message, no URL, no workspace prefix (`+ eslint 8.57.1 deprecated`). They
#   differ textually, so they hash differently and fp-dedupe cannot collapse
#   them — fps 819aa7c6f6bd (resolution WARN) and bba59aabdf63 (summary twin)
#   cost TWO background triage dispatches for ONE concern on 2026-07-30.
#   Safe because the summary surface is a strict SUBSET of the resolution
#   surface: the resolution WARN fires on every install (it re-reads the
#   lockfile), while the summary block only prints when the package set
#   changes. Collapsing the summary can never lose a finding.
#   The `deprecated\s*$` anchor keeps it narrow: a real diff hunk adding
#   `#[deprecated(note = "…")]`, a `version = "0.9.34+deprecated"` literal, or
#   any prose sentence carries punctuation, quotes, or trailing words and
#   cannot match. Diff markers are matched precisely rather than exempted —
#   pnpm's summary marker IS `+`/`-`, so Guard C/G's hunk exemption misses it.
ECHO_PNPM_CHANGE_SUMMARY_RE = re.compile(
    r"^\s*(?:[^\s:]+:\d+[:-])?\s*(?:\d+[:-])?\s*"  # optional grep path:line: / -n prefix
    r"[+-]\s+"  # pnpm add/remove marker
    r"(?:@[A-Za-z0-9._-]+/)?[A-Za-z0-9._-]+\s+"  # package name (optionally scoped)
    r"\d[A-Za-z0-9.+-]*\s+"  # version (starts with a digit)
    r"deprecated\s*$",  # bare marker, nothing after it
    re.IGNORECASE,
)

# Guard K — ESCAPE-RENDERED mangling self-capture:
#   The sentinel already normalizes the U+2009 THIN SPACE that pnpm pads its
#   `WARN` prefix with. But when an agent re-reads that same captured log
#   through an escape-rendering tool (`cat -A`, `od -c`), U+2009 renders as the
#   literal ASCII `M-bM-^@M-^I` — a different byte string, so it hashes
#   differently and re-mints. Three fps (b1561f3d429d, 9d31ba938515,
#   010ff5a7bfb5) were minted this way on 2026-07-30 for warnings the ledger
#   already held. No real toolchain emits `cat -A`'s escape vocabulary; a
#   mangled rendering is never a live warning.
ECHO_ESCAPE_RENDERED_RE = re.compile(r"M-[A-Za-z]M-\^?[@-_A-Za-z]|M-\^[@-_]")

ECHO_SRC_DEPRECATED_BANNER_RE = re.compile(r"\(DEPRECATED\)\s*$")
ECHO_SRC_DEPRECATED_COMMENT_RE = re.compile(
    r"^(?:[^\s:]+:)?\d*[:-]?\s*//[/!]?.*\bDEPRECATED\b", re.IGNORECASE
)

# Guard H — REMEDIATION-ANNOTATION self-capture (the security-class analog of
#   Guards B/G). A vulnerability fix does not just change a version — it
#   ANNOTATES why, naming the advisory it closes. Those annotations then read
#   back as fresh "security findings" every time anyone greps or diffs the
#   remediation surface, which is exactly what the fix work does constantly.
#   Two shapes, both ZERO true-positive risk:
#     H1) The repo-root `VULNERABILITY_CLUSTER_*.md` remediation ledgers — the
#         per-workspace decision records for the Dependabot alert lanes. They
#         are structurally identical to the Guard-B surfaces (backlog /
#         chronicle / superpowers prose) and were simply not in that path list:
#         every "Resolved alerts" table row narrates an advisory by design
#         (`| #482 | protobuf | Upgraded prometheus 0.13.4 → 0.14.0 … RUSTSEC-2024-0437 …`).
#     H2) A COMMENT line whose security signal is an ADVISORY IDENTIFIER
#         (CVE-/GHSA-/RUSTSEC-). A manifest/source comment explaining a pin or
#         a fix — `# 0.14 carries protobuf >=3.7.2, fixing RUSTSEC-2024-0437.`,
#         `// origin isolation (GHSA-824h-7x5x-wfmf).` — is the remediation's
#         own paper trail. Unlike Guards C/G, diff hunks are NOT exempt here: a
#         comment is annotation whether it is being ADDED or merely re-read, and
#         a `+`-prefixed comment-add is the single most common shape (a
#         `git diff Cargo.toml` during in-flight fix work). The attribute form
#         `#[deprecated…]` is explicitly NOT a comment and stays capturable
#         (the `#(?!\[)` clause), and the live security channel is never
#         comment-shaped: cargo-audit emits `ID:      RUSTSEC-…` / `Crate: …`,
#         npm/pnpm audit emit table+prose, GitHub push banners emit `remote:
#         GitHub found N vulnerabilities …` — none of which open with `#`/`//`.
#   fp-dedupe cannot collapse either class: each new cluster-doc row, each
#   `grep -n` prefix, and each re-worded annotation mints a fresh fingerprint
#   (same defeat as Guards B/D/F/G). Both classes cost a triage dispatch on
#   2026-07-29 while the Rust vulnerability-cluster lanes were mid-remediation:
#   929b7f99229f (H2 — a `git diff Cargo.toml` of cluster 09's own
#   prometheus-0.14 remediation comment) and 5a3e9e45a634 (H1 — a grep hit on
#   cluster 09's resolved-alerts table). A third, 0e0f81127d39 on 2026-07-27,
#   was H2 in `//` form. None named a live concern the fix lanes did not
#   already own.
# Guard I — VENDORED THIRD-PARTY RESEARCH CLONE self-capture:
#   `genesis/research/repos/**` holds read-only upstream clones (freenet-core,
#   hypercore, polis, civic-ai, … — enumerated in
#   genesis/research/research-manifest.json) kept for the cross-pollination
#   surveys. The tree is GITIGNORED (.gitignore:92) and is never built, tested,
#   or shipped by this repo. Every deprecation and every advisory annotation in
#   it belongs to ANOTHER project: it can never be a live in-flight finding
#   here, and it is not our surface to fix. A survey pass that greps a vendored
#   repo for patterns would otherwise mint one triage dispatch per upstream
#   annotation it happens to scroll past — fp-dedupe cannot collapse that (each
#   upstream file/line mints a fresh fingerprint). Zero true-positive risk:
#   nothing under this path is in any workspace, Dockerfile COPY, or gate.
#   (One dispatch on 2026-07-27 — 0e0f81127d39 — was this class: freenet-core's
#   own `// origin isolation (GHSA-824h-7x5x-wfmf).` mitigation comment, read
#   during a survey. It was H2-shaped too, but the vendored-path guard is the
#   root-cause collapse for the whole tree.)
ECHO_VULN_CLUSTER_PATH_RE = re.compile(r"VULNERABILITY_CLUSTER[A-Z0-9_]*\.md", re.IGNORECASE)
ADVISORY_ID_RE = re.compile(
    r"\b(?:CVE-\d{4}-\d+"
    r"|GHSA-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{4}"
    r"|RUSTSEC-\d{4}-\d+)\b",
    re.IGNORECASE,
)
ECHO_COMMENT_OPENER_RE = re.compile(
    r"^[+-]?\s*"  # optional diff marker
    r"(?:[^\s:]+:\d+[:-])?\s*"  # optional grep `path:line:` prefix
    r"(?:\d+[:-])?\s*"  # optional bare `-n` line-number prefix
    # comment opener: // /// //! or # — but NOT `#[attr]`, and NOT a Docker
    # BuildKit progress prefix. BuildKit prints `#<step> <elapsed> <text>`
    # (`#12 3.456 npm warn deprecated glob@7.2.3: …`), which is a LIVE warning
    # wearing a comment costume: verified 2026-08-07 to match the pre-narrowing
    # regex AND classify as `deprecation`. `#` followed by a digit is therefore
    # never treated as a comment opener — every CI build in this repo runs
    # inside Docker, so this shape is the dominant live channel for cargo/npm
    # warnings. The exclusion also closes the same latent hole in Guard H2
    # (a `#12 3.456 … RUSTSEC-2024-0437` line was dismissible as a remediation
    # annotation). Narrowing an echo guard can only ever LOSE a dismissal, so
    # it errs toward capture.
    r"(?://[/!]?|\#(?!\[)(?!\d))"
)

# Guard O (2026-08-07) — FIRST-PARTY DEPRECATION-COMMENT self-capture: the
#   deprecation-class analog of H2, and the general form of Guard G.
#   Guard G already establishes that an in-source comment NARRATING a
#   deprecation is the deprecation-IMPLEMENTATION/DOCUMENTATION surface, not a
#   live toolchain warning. But G is anchored three ways that leak:
#     - `//`-openers only, so `#` comments (Cargo.toml, shell, YAML, Dockerfile)
#       and markdown prose escape;
#     - ALL-CAPS `DEPRECATED` only, so ordinary lowercase prose escapes;
#     - diff-EXEMPT, so the `+`-prefixed comment-ADD escapes — and that is the
#       dominant shape, because writing the guard comment is what fix work DOES.
#   The resulting class is self-amplifying in the worst way: a comment written
#   to DOCUMENT an already-canonicalized deprecation becomes a perpetual
#   dispatch generator, because every subsequent `git diff` or scope `grep` of
#   the file it guards re-emits it under a fresh line-number/diff prefix that
#   fp-dedupe cannot collapse (same defeat as Guards B/D/F/G/L).
#   Measured live 2026-08-07: **33 of 286 ledger rows (11.5%)** are this class.
#   Their dispositions are the zero-true-positive argument, and it is empirical
#   rather than regex reasoning: 5 already hand-marked `false-positive`, 12
#   `blocked` against concerns already canonicalized (learning-path zome
#   surface, doorway warm-projection cache, holochain_sqlite tombstone, devfile
#   dead command), 16 `open` — and **not one row in 286 was ever `triaged`**,
#   i.e. no comment-line capture in the ledger's whole history ever became an
#   actionable fix. The 16 open rows are Rust doc-comments (`/// deprecated in
#   favour of …`), `#![allow(deprecated)]`, a shell comment, a markdown task
#   heading, and a Cargo.toml pin-guard block — first-party prose, every one.
#   Deliberately NOT diff-exempt (same call as H2 and M). The live deprecation
#   channel is never comment-shaped — `npm warn deprecated`, ` WARN  deprecated`,
#   `warning: use of deprecated`, `(node:N) DeprecationWarning`, Vitest
#   `DEPRECATED: <prose>` all open with the tool's own token — and the one live
#   channel that DOES open with `#` (Docker BuildKit `#12 3.456 …`) is excluded
#   in the opener regex above. `#[deprecated…]` is an attribute, not a comment,
#   and stays capturable.
#   (This guard was itself dispatched by its own class: fps 4e4f2598ff5c /
#   93a86c9e5657 / 1f681f440327 were a `git diff` of the holochain_sqlite
#   TOMBSTONE GUARD comment — a comment whose entire purpose is to document a
#   concern already canonicalized and blocked — and the triage run's own
#   verification `grep` of that same comment minted three MORE, 6dd68a9c0f58 /
#   07364812bedc / 022dae188a5f, requesting a second dispatch mid-run.)

# Guard L — DERIVED/GENERATED ARTIFACT TREE self-capture:
#   A grep/cat/find that recurses into a BUILD OUTPUT or dependency tree reads
#   back annotations that are, by construction, copies of something else:
#   `node_modules/**` (another project's source — the Guard-I rationale applied
#   to installed deps), and the derived trees `dist/**`, `.angular/cache/**`,
#   `.nx/cache/**`, `.vite/**`, `vite/deps/**`, `coverage/**`,
#   `target/{debug,release,wasm32*}/**` (our OWN source, compiled/cached/bundled).
#   Nothing in these trees is authored or fixed here: a finding in `dist/` is
#   the `src/` line that produced it, and a finding in `node_modules/` belongs
#   upstream. fp-dedupe cannot collapse the class — cache filenames are
#   content-hashed (`.angular/cache/22.1.0/ng-packagr/<sha256>.json`) so every
#   rebuild mints a FRESH path and therefore a fresh fingerprint, unboundedly
#   (same defeat as Guards B/D/F/G).
#   Keyed at LINE level on a grep-with-filename PATH PREFIX (`<path>:` at line
#   start), deliberately NOT on the command string. That is what makes it safe
#   against the over-suppression risk recorded in
#   `deprecation-lit-context-upstream-dts-jsdoc-noise.md`: a real warning from a
#   tool INVOKED out of `node_modules/.bin/` (vitest, ng, eslint) is emitted as
#   the tool's own prose with no path prefix, and a DeprecationWarning carrying
#   a `node_modules/` STACK FRAME mid-line is not prefix-shaped either — both
#   stay capturable (verified TP7/TP15).
#   (2026-07-30: fp c3259594f97a + the three self-minted d6938f2c29af /
#   0efde47788fa / 678210c3381a all came from ONE
#   `app/elohim-library/.angular/cache/**/ng-packagr/<sha>.json` blob.)
ECHO_DERIVED_ARTIFACT_PATH_RE = re.compile(
    r"^[+-]?\s*"
    r"(?:[^\s:]*/)?"
    r"(?:node_modules|dist|coverage|\.angular/cache|\.nx/cache|\.vite"
    r"|vite/deps|target/(?:debug|release|wasm32[^/]*))"
    r"/[^\s:]*:",
)

# Guard M — JSDoc `@deprecated` TAG self-capture (the TS/JS analog of Guard G):
#   Guard G collapses the deprecation-IMPLEMENTATION surface for `//`-comment
#   languages, but the TypeScript/JavaScript form of the same deliberate,
#   migration-retained annotation is a JSDoc BLOCK comment, which G's
#   `//`-anchored regex cannot see:
#       /**
#        * Trigger an upgrade prompt.
#        * @deprecated Activity signals now flow via EconomicEvents to the Rust
#        * substrate. This stub is retained for call-site compatibility.
#        */
#   A scope grep re-emits those ` * @deprecated …` lines with a fresh
#   `grep -n` line-number prefix every time the file shifts, so fp-dedupe can
#   never collapse them — only a shape guard can.
#   Keyed on the JSDoc TAG `@deprecated` (with its literal `@`) on a
#   COMMENT-SHAPED line (`/**`, `/*`, `*`, `//`, `///`, `//!`). Zero
#   true-positive risk: the live TS deprecation channel is the ESLint
#   `@typescript-eslint/no-deprecated` rule, whose output is prose
#   ("`LearningPath` is deprecated. Use PathView instead") and contains the
#   substring `no-deprecated`, never `@deprecated`; npm's channel is
#   `npm warn deprecated <pkg>@<ver>` (the `@` follows the package name, never
#   precedes `deprecated`); Vitest/Node/Rust emit bare-word prose. Verified
#   against 19 live-channel negatives, zero over-suppression.
#   Deliberately NOT diff-exempt (unlike Guards C/G, like Guard H2): a JSDoc
#   tag is an annotation whether it is being ADDED in a `git diff` or merely
#   re-read, and `+ * @deprecated …` mid-migration is a dominant shape. The
#   Rust attribute form `#[deprecated…]` is not comment-shaped and still
#   captures (verified TP11/TP18).
ECHO_JSDOC_DEPRECATED_TAG_RE = re.compile(
    r"^\s*[+-]?\s*"
    r"(?:[^\s:]+:\d+[:-])?\s*"  # optional grep `path:line:` prefix
    r"(?:\d+[:-])?\s*"  # optional bare `-n` line-number prefix
    r"(?:/\*\*?|\*|//[/!]?)"  # JSDoc/blockcomment opener: /** /* * // /// //!
    r".*@deprecated",
    re.IGNORECASE,
)

# Command-level gates for echo classes B and C: if the command itself is a pure
# git history read (git log / git show without a -p / --patch flag) or reads
# directly from the history / planning / SDD prose trees, ALL lines from that
# output are echo candidates — the command gate is a cheap early exit before
# per-line work (cat/tail/grep of a single doc carries no path prefix per line,
# so the command string is the only signal of source).
_CMD_GIT_HISTORY_RE = re.compile(
    r"\bgit\s+(?:log|show)\b(?!.*(?:\s-p\b|\s--patch\b|\s--diff\b))",
    re.IGNORECASE,
)
_CMD_HISTORY_TREE_RE = re.compile(
    r"genesis/docs/content/elohim-protocol/history/"
    r"|genesis/data/timeline/(?:backlog|chronicle)/"
    r"|genesis/docs/superpowers/"
    r"|\.superpowers/"
    r"|(?:^|/|\s)CLAUDE\.md\b"
    r"|VULNERABILITY_CLUSTER[A-Z0-9_]*\.md\b"  # Guard H1 (see below)
    r"|genesis/research/repos/",  # Guard I (see below)
    re.IGNORECASE,
)

# Guard Q (2026-08-11) — AGENT-TOOLING SOURCE read through a SINGLE-FILE command.
#   Guard E dismisses first-party tooling source keyed on a `.claude/…` path
#   appearing IN THE LINE. But that path only appears when grep is given
#   MULTIPLE files (or -H): `grep -n <pat> .claude/scripts/foo.py` on ONE file
#   emits `73:<code>` with no filename at all. So the identical content Guard E
#   dismisses from a repo-wide scope grep is CAPTURED when an agent greps the
#   single file directly — and reading one tooling file directly is the more
#   common act.
#   This is the exact gap `_CMD_HISTORY_TREE_RE` was introduced to close for the
#   prose trees, quoted from its own comment: "cat/tail/grep of a single doc
#   carries no path prefix per line, so the command string is the only signal of
#   source." Guard Q applies that reasoning to the tooling tree Guard E owns.
#   Live proof, and it is the capture that dispatched the run which found it:
#   fp `3054d0cb4bd7` (2026-08-11) — `grep -n "…DEAD_WORDS\s*=…"
#   .claude/scripts/memory-kit/placement-audit.py` captured
#   `73:DEAD_WORDS = {"superseded", …, "deprecated", "retired"}`, a plain Python
#   STATUS-VOCABULARY set literal. The same constant was captured at line 72 as
#   `802862c393b2` (hand-marked false-positive 2026-06-06) and with no prefix at
#   all as `5723985e3232` — three fingerprints, one string literal that has
#   never been a warning. fp-dedupe cannot collapse the class: the `grep -n`
#   prefix moves whenever the file is edited (Class 3), so every edit re-mints.
#   Measured live 2026-08-11: 6 of 275 rows are this class. Dispositions are the
#   zero-true-positive argument, empirical as in Guards O/P: 2 already
#   hand-marked `false-positive`, 4 `open`, and **0 `triaged`** — no
#   tooling-source read in the ledger's history ever became an actionable fix.
#   READ-GATED, deliberately. The command must use a READ utility on the tooling
#   path; EXECUTING one (`python3 .claude/scripts/memory-kit/…py`) is left
#   capturable, because our own Python tools genuinely can emit a real
#   DeprecationWarning and that IS a first-party finding worth having. This is
#   the Guard-O BuildKit lesson applied before the fact: name the live channel
#   the guard sits next to, then carve it out.
_CMD_TOOLING_SOURCE_READ_RE = re.compile(
    r"\b(?:grep|egrep|fgrep|rg|cat|head|tail|nl|sed|awk|cut|less|more|wc"
    r"|git\s+(?:diff|show)|diff)\b"
    # Same COMMAND only — never across `;`, `&&`, `&`, or a newline. Pipes are
    # deliberately ALLOWED: a grep alternation carries literal `\|` (and `-E`
    # patterns carry `|`) between the utility and its file operand, so excluding
    # `|` made the guard blind to its own dispatching capture —
    # `grep -n "ACTIVE_WORDS\s*=\|…\|DEAD_WORDS\s*=" .claude/scripts/…py`.
    # Allowing pipes widens the gate to `cargo build | grep … .claude/…`; that
    # is what the live-channel residual below is for.
    r"[^;&\n]*?"
    r"(?:\.claude/(?:hooks|scripts|skills|agents|data|memory)/"
    r"|(?:^|/|\s)\.epr-meta/"
    r"|(?:^|/|\s)\.codex/(?:skills|agents|commands)/)",
)

# Guard A2 — the FINDINGS LEDGERS read through the command, in ANY word order.
#   Guard A keys on the literal `deprecations.jsonl` appearing in the LINE, but
#   a ledger ROW read back out is a JSON object that does not contain its own
#   filename — so `head -1` of the ledger yields a line carrying the ORIGINAL
#   captured warning text and nothing identifying it as stored data. fp
#   `18ddd0fb6e64` is exactly that: the GitHub 191-vulnerabilities banner,
#   re-minted as a fresh security finding by an agent inspecting the ledger
#   files it had just been dispatched to triage.
#   Deliberately NOT segment-ordered like Guard Q: the dominant shape is a shell
#   loop (`for f in .claude/data/*.jsonl; do head -1 $f; done`) which places the
#   PATH BEFORE the reader, so an ordered regex misses it. Order-insensitivity
#   is safe here in a way it would not be for Guard Q, because reading a stored
#   JSONL data file emits no toolchain output of its own — there is no live
#   channel in this command to protect. It still carries Guard Q's residual, so
#   a compound `pnpm install && cat .claude/data/x.jsonl` keeps its real
#   warnings.
_CMD_FINDINGS_LEDGER_RE = re.compile(r"\.claude/data/[^\s;&|]*\.jsonl")

# The live toolchain channels, as they actually arrive. Guard Q's residual (the
# Guard-P step-3 property): even under a tooling-read command, a line carrying a
# real tool's own token is a real warning and must still capture. This is what
# makes Guard Q safe against a COMPOUND command — `cargo build && grep -n x
# .claude/scripts/y.py` matches the command gate, but cargo's genuine
# `warning: use of deprecated …` in that same output survives on its own shape.
_LIVE_TOOLCHAIN_CHANNEL_RE = re.compile(
    r"npm\s+warn\s+deprecated"
    r"|\bwarn\b\s+deprecated"  # pnpm ` WARN  deprecated <pkg>@<ver>`
    r"|warning:\s*use of deprecated"  # rustc/clippy
    r"|\bDeprecationWarning\b"
    r"|\bExperimentalWarning\b"
    r"|\bDEP\d{4}\b"  # Node DEP codes
    r"|^\s*#\d+\s+\d+\.\d+\s",  # Docker BuildKit `#12 3.456 …`
    re.IGNORECASE,
)
# Vitest's banner is the one live channel that is only distinguishable by CASE
# (`DEPRECATED: <prose>`), so it cannot ride the IGNORECASE regex above without
# re-admitting every lowercase prose mention the guard exists to dismiss.
_LIVE_VITEST_BANNER_RE = re.compile(r"\bDEPRECATED:\s")
# Note: the SECURITY live channels (cargo audit / npm audit / the GitHub push
# banner) are deliberately absent from the residual. None of them is emitted by
# a file-READ command, so they need no protection under Guard Q — and including
# the `N vulnerabilities` summary shape would re-admit Guard A2's dominant row,
# a stored ledger entry quoting that very banner back.

# Guard R (2026-08-11) — Guard D generalized from the MARKER to the INVARIANT.
#   Guard D dismisses an agent's own ephemeral script output keyed on Python's
#   `<string>:N:` / `<stdin>:N:` source markers — i.e. `python3 -c "…"` and
#   stdin. But the moment the agent writes that same script to a scratch FILE
#   and runs it, the marker becomes a real path and Guard D goes blind, while
#   the semantics are unchanged: it is still the agent's own ad-hoc code, still
#   not a live finding about this repo.
#   The dominant shape is the sharpest one available, because it is
#   SELF-AMPLIFYING and it targets this very hook: a deprecation-triage run
#   VERIFYING an echo guard must print an adversarial fixture corpus, and that
#   corpus is by construction full of warning-shaped lines. Every harness run
#   therefore mints fingerprints for warnings that do not exist. Measured live
#   2026-08-11: 7 of 275 rows, every one a triage harness printing its own test
#   vectors — `[PASS] diff-add of #[deprecated] attr still classifies…`,
#   `text: let deprecated: Vec<u32> = vec![];`, `text: DEAD_WORDS = {…}`.
#   Dispositions: 5 `false-positive`, 2 `blocked`, and **0 `triaged`**.
#   This is Class 8's lesson restated at a new seam: Guard D was keyed on the
#   syntax it first met (`<string>:`), not on the property that makes the class
#   dismissible (the code is EPHEMERAL and AGENT-AUTHORED). Keying on the scratch
#   tree closes it for every interpreter at once.
#   Unconditional, exactly as Guard D is, and on Guard D's own argument: a
#   genuine codebase deprecation is emitted by a real toolchain run against the
#   repo, never by a throwaway script in /tmp. Deliberately NO live-channel
#   residual here (unlike Guard Q) — a verification harness's whole job is to
#   print live-channel-shaped fixtures, so a residual would re-admit precisely
#   the rows this guard exists to stop.
_CMD_EPHEMERAL_SCRIPT_RE = re.compile(
    r"\b(?:python3?|node|bash|sh|uv\s+run)\s+[^\s;&|]*"
    r"(?:/tmp/|/var/tmp/|/private/tmp/|/scratchpad/)"
    r"[^\s;&|]*\.(?:py|mjs|js|sh)\b"
)


def _is_echo_line(
    line: str,
    cmd_is_git_history: bool,
    cmd_is_history_tree: bool,
    cls: str | None = None,
    cmd_is_tooling_read: bool = False,
    cmd_is_ephemeral_script: bool = False,
) -> bool:
    """Return True if *line* is a known echo false-positive and must be skipped.

    Called after classify() returns non-None to prevent false echo entries
    from being fingerprinted and appended to the ledger.

    The guards (A–O):
      A) Line sourced from the ledger file (self-capture via recursive grep).
      B) Line sourced from the history/museum prose tree.
      C) Line that is git commit-message text (log oneline entry, or any
         subject/body line from a pure git history read), NOT a diff hunk
         (diff hunks start with + or - so carry real signal).
      D) DeprecationWarning sourced from `<string>:N:`/`<stdin>:N:` — an
         agent's own ephemeral `python3 -c`/stdin script, never a real file.
      E) Agentic tooling-source echo — `.claude/` paths or review-finding
         markers in the line (our own governance tooling narrating findings).
      F) crates.io `+deprecated` build-metadata in either a Cargo lock/manifest
         version literal (`version = "…+deprecated"`) or a registry artifact
         filename (`serde_yaml-0.9.34+deprecated.crate`) — a semver
         build-metadata suffix, registry metadata not a live toolchain warning;
         `grep -n`/`ls` re-emission defeats fp-dedupe.
      P) The same `+deprecated` build-metadata token after an agent EXTRACTED
         and REPRINTED it (`holochain_sqlite: storage=0.7.0-dev.19
         steward=0.7.0+deprecated`) — Guard F generalized past its two
         hard-coded syntaxes to the invariant F rests on: dismissed iff the
         token is the line's ONLY deprecation signal AND is not in cargo's
         live SPACE-`v` prose shape. Deprecation class only.
      G) In-source DEPRECATED section-header / doc-comment surfaced by a
         grep/cat of the codebase's own source — a trailing all-caps
         `(DEPRECATED)` section label or a `//`/`///` comment mentioning
         DEPRECATED. The deprecation-IMPLEMENTATION surface (deliberate,
         migration-retained), not a live warning; `grep -n` prefixes defeat
         fp-dedupe. Diff hunks (+/-) are exempt so a real added annotation
         still captures.
      H) Remediation-annotation echo (security class only) — the repo-root
         `VULNERABILITY_CLUSTER_*.md` alert-lane decision records (H1, folded
         into Guard B's path/command gates), or a COMMENT line whose security
         signal is an advisory identifier (H2: `# … fixing RUSTSEC-2024-0437.`,
         `// origin isolation (GHSA-…)`). The fix's own paper trail, not a live
         finding. Diff hunks are NOT exempt for H2 — a `+`-prefixed
         comment-add during in-flight fix work is the dominant shape — but
         `#[deprecated…]` is not a comment and still captures.
      I) Vendored third-party research clone (`genesis/research/repos/**`,
         gitignored, never built) — another project's deprecations/advisories,
         folded into Guard B's path/command gates.
      J) pnpm package-changes-summary twin (`+ eslint 8.57.1 deprecated`) — the
         end-of-install lockfile-diff marker for a package whose real warning
         the resolution stage already emitted with message + URL. A strict
         subset of the resolution surface, so collapsing it loses nothing.
      K) Escape-rendered mangling (`M-bM-^@M-^I` from a `cat -A`/`od -c`
         re-read of a captured log) — a byte-mangled rendering of a warning
         already held, never a live emission.
      L) Derived/generated artifact tree — a grep-with-filename PATH PREFIX
         into `node_modules/`, `dist/`, `.angular/cache/`, `.nx/cache/`,
         `.vite/`, `vite/deps/`, `coverage/`, `target/{debug,release,wasm32*}/`.
         Build output or installed dependency: the finding belongs to the
         `src/` line that produced it or to the upstream project. Keyed on the
         path prefix, NOT the command, so a tool invoked from
         `node_modules/.bin/` still captures.
      M) JSDoc `@deprecated` tag on a comment-shaped line — the TS/JS analog of
         Guard G's `//`-comment collapse, covering the block-comment form
         (`/** … * @deprecated … */`) that G's regex cannot see. The live TS
         channel is ESLint `@typescript-eslint/no-deprecated` prose, which
         never contains the literal `@deprecated`. Not diff-exempt (like H2).
      N) The same agent surfaces under their OTHER homes — the EPR package
         store and its per-runtime projections (`.epr-meta/…`) and the codex
         mirror (`.codex/{skills,agents,commands}/…`). Guard E keyed on
         `.claude/` only, so one skill body stored in four places was
         dismissed in one and dispatched from the other three. Folded into
         Guard E's regex; see its comment block for the narrowness argument
         (the `.epr-meta/` trailing slash excludes the directory-local
         compose-gate manifest FILES).
      O) First-party deprecation-COMMENT self-capture (deprecation class only)
         — Guard G generalized past its three leaky anchors (`//`-openers
         only, ALL-CAPS only, diff-exempt). A comment narrating a deprecation
         is the documentation surface, not a live warning; writing one to
         guard an already-canonicalized concern otherwise turns that concern
         into a perpetual dispatch generator. Not diff-exempt (like H2/M).
         Docker BuildKit `#12 3.456 …` is excluded in the opener regex — it
         is a live warning wearing a comment costume.
      Q) Guard E at COMMAND level — a single-file READ of agent-tooling source
         (`grep -n <pat> .claude/scripts/foo.py`) emits no path prefix per
         line, so E's line-keyed regex is blind to content it would dismiss
         from a repo-wide grep. Read-gated: EXECUTING a `.claude/` script is
         left capturable (our Python tools can emit a real DeprecationWarning).
         Carries a live-channel residual so a compound command cannot swallow
         a genuine warning.
      R) Guard D at TREE level — an agent-authored throwaway script run from
         /tmp or a scratchpad. D was keyed on Python's `<string>:`/`<stdin>:`
         marker; writing the same code to a scratch file defeats it. Dominant
         shape is self-amplifying: a triage run verifying an echo guard must
         print a warning-shaped fixture corpus. Unconditional, residual-free.
    """
    # Guard A — ledger self-capture
    if ECHO_LEDGER_PATH.search(line):
        return True

    # Guard D — ephemeral-script self-capture (<string>:N: / <stdin>:N:).
    # Placed early: cheapest deterministic skip, no command-flag dependency.
    if ECHO_EPHEMERAL_SOURCE_RE.search(line):
        return True

    # Guard R — Guard D's invariant, keyed on the scratch TREE rather than on
    # Python's `<string>:`/`<stdin>:` marker: an interpreter running an
    # agent-authored throwaway script out of /tmp or a scratchpad. Unconditional
    # (as D is) and deliberately residual-free: a verification harness prints
    # warning-shaped fixtures by design, so a live-channel carve-out would
    # re-admit the exact rows this guard exists to stop.
    if cmd_is_ephemeral_script:
        return True

    # Guard F — crates.io `+deprecated` build-metadata self-capture: a
    # Cargo lock/manifest version literal (`version = "…+deprecated"`) OR a
    # registry artifact filename (`…-0.9.34+deprecated.crate`): registry
    # metadata, not a live warning. Deterministic, no command-flag dep — like D.
    if ECHO_LOCKFILE_VERSION_META_RE.search(line):
        return True

    # Guard P — EXTRACTED `+deprecated` build-metadata readout (Guard F
    # generalized past its two hard-coded syntaxes to the invariant F rests
    # on). Dismiss iff the line's ONLY deprecation signal is a semver
    # build-metadata token that is not in cargo's live SPACE-`v` prose shape.
    # Deprecation class only; not diff-exempt (per H2/M/O).
    if (
        cls == "deprecation"
        and BUILD_META_DEPRECATED_RE.search(line)
        and not CARGO_VERSION_JOIN_RE.search(line)
        and not DEPRECATION_PATTERNS.search(BUILD_META_DEPRECATED_RE.sub("", line))
    ):
        return True

    # Guard J — pnpm package-changes-summary twin (`+ eslint 8.57.1 deprecated`):
    # a bare marker carrying no message/URL the resolution WARN lacks.
    if ECHO_PNPM_CHANGE_SUMMARY_RE.match(line):
        return True

    # Guard K — escape-rendered mangling (`cat -A`/`od -c` re-read of a captured
    # log): never a live warning.
    if ECHO_ESCAPE_RENDERED_RE.search(line):
        return True

    # Guard E — agentic tooling-source echo (.claude/ paths, review-finding
    # markers): our own governance tooling talking about deprecations/vulns.
    if ECHO_TOOLING_SOURCE_RE.search(line):
        return True

    # Guard Q — Guard E's COMMAND-level counterpart. A single-file read of an
    # agent-tooling file emits no path prefix per line, so Guard E cannot see
    # the source; the command string is the only signal (the `cmd_is_history_
    # tree` argument, applied to the tooling tree). Residual per Guard P step 3:
    # a line carrying a real toolchain's own token still captures, which is what
    # keeps a compound `cargo build && grep … .claude/scripts/x.py` honest.
    if (
        cmd_is_tooling_read
        and not _LIVE_TOOLCHAIN_CHANNEL_RE.search(line)
        and not _LIVE_VITEST_BANNER_RE.search(line)
    ):
        return True

    # Guard G — in-source DEPRECATED section-header / doc-comment self-capture.
    # Exempt diff hunks (+/-) so a commit ADDING a real annotation still captures.
    if not line.startswith(("+", "-")) and (
        ECHO_SRC_DEPRECATED_BANNER_RE.search(line)
        or ECHO_SRC_DEPRECATED_COMMENT_RE.search(line)
    ):
        return True

    # Guard H2 — remediation annotation: a COMMENT line whose security signal
    # is an advisory identifier (CVE-/GHSA-/RUSTSEC-). Security class only, and
    # deliberately NOT diff-exempt: `+# … fixing RUSTSEC-2024-0437.` from a
    # `git diff Cargo.toml` mid-remediation is the dominant capture shape.
    if (
        cls == "security"
        and ADVISORY_ID_RE.search(line)
        and ECHO_COMMENT_OPENER_RE.match(line)
    ):
        return True

    # Guard O — first-party deprecation-comment self-capture: a COMMENT line in
    # the deprecation class is the deprecation-documentation surface (Guard G
    # generalized past its `//`-only / ALL-CAPS-only / diff-exempt anchors).
    # Deliberately NOT diff-exempt: the `+`-prefixed comment-ADD from a
    # `git diff` during fix work is the dominant shape. The one live channel
    # that opens with `#` (Docker BuildKit `#12 3.456 …`) is excluded in
    # ECHO_COMMENT_OPENER_RE; `#[deprecated…]` is an attribute and still captures.
    if cls == "deprecation" and ECHO_COMMENT_OPENER_RE.match(line):
        return True

    # Guard L — derived/generated artifact tree (node_modules, dist, .angular/
    # cache, .nx/cache, .vite, vite/deps, coverage, target/{debug,release,wasm32*}).
    # Line-level path-prefix key only — a tool run FROM node_modules/.bin emits
    # prose with no path prefix and stays capturable.
    if ECHO_DERIVED_ARTIFACT_PATH_RE.match(line):
        return True

    # Guard M — JSDoc `@deprecated` tag on a comment-shaped line: the TS/JS
    # deprecation-IMPLEMENTATION surface (Guard G's analog for block comments).
    # Not diff-exempt: an annotation is an annotation added or re-read.
    if ECHO_JSDOC_DEPRECATED_TAG_RE.match(line):
        return True

    # Guard B (+ H1) — history / remediation-ledger prose (grep output carries
    # the file path in the line; if the command itself reads from the tree,
    # every line is echo output)
    if ECHO_HISTORY_PATH_RE.search(line) or cmd_is_history_tree:
        return True

    # Guard C — commit-message text (both git log oneline entries and all
    # lines of commit message bodies from git log/show history reads).
    # Preserve diff hunks: lines starting with + or - are hunk content that
    # can legitimately add a real #[deprecated] attribute and must still capture.
    if not line.startswith(("+", "-")):
        # git log --oneline shape: "8f0cb4122 commit subject…" — always echo.
        if ECHO_GIT_ONELINE_RE.match(line):
            return True
        # Any non-hunk line from a pure git history read (git log/git show
        # without -p/--patch) is commit message prose — subject OR body.
        # ECHO_COMMIT_SUBJECT_RE was too narrow (subject-only); the command-
        # level flag is the authoritative gate for the whole message body.
        if cmd_is_git_history:
            return True

    return False


def collect_strings(node, out, budget):
    """Walk the tool_response JSON collecting string values, bounded."""
    if budget[0] <= 0:
        return
    if isinstance(node, str):
        out.append(node[: budget[0]])
        budget[0] -= len(node)
    elif isinstance(node, dict):
        for v in node.values():
            collect_strings(v, out, budget)
    elif isinstance(node, list):
        for v in node:
            collect_strings(v, out, budget)


def classify(line: str) -> str | None:
    """Return 'deprecation' | 'security' | None for a stripped line."""
    if DEPRECATION_PATTERNS.search(line):
        return "deprecation"
    if SECURITY_PATTERNS.search(line):
        return "security"
    return None


def fingerprint(line: str, cls: str) -> str:
    norm = re.sub(r"\s+", " ", line).strip().lower()
    # Run-identity stripping: the emitting pid is not part of the concern.
    norm = NODE_PID_PREFIX.sub("(node:#)", norm)
    if cls == "security" and SECURITY_SUMMARY.search(line):
        # Count-churn stability: digit runs collapse so "191 vulnerabilities
        # (1 critical, 113 high)" and next week's counts share one concern.
        norm = re.sub(r"\d+", "#", norm)
    return hashlib.sha256(norm.encode()).hexdigest()[:12]


def session_cited_path(session_id: str) -> str:
    sid = re.sub(r"[^A-Za-z0-9]", "", session_id)[:12] or "nosession"
    return f"/tmp/claude-dep-cited-{sid}"


def load_session_cited(path: str) -> set:
    try:
        with open(path, encoding="utf-8") as fh:
            return set(fh.read().split())
    except OSError:
        return set()


def main() -> None:
    payload = json.load(sys.stdin)
    if payload.get("tool_name") != "Bash":
        return

    command = (payload.get("tool_input") or {}).get("command", "")
    # Guard the grep/echo false-positive class: a command that itself
    # talks about deprecation/vulnerabilities (searching the codebase,
    # reading this ledger, editing this tooling) is not a NEW in-flight
    # finding. Real sources (pnpm install, pnpm audit, cargo audit, git
    # push banners) don't carry these tokens in the command string.
    cmd_lower = command.lower()
    if any(tok in cmd_lower for tok in GUARD_TOKENS):
        return

    texts: list = []
    collect_strings(payload.get("tool_response"), texts, [MAX_SCAN_BYTES])
    if not texts:
        return

    project = os.environ.get("CLAUDE_PROJECT_DIR", ".")
    ledger = os.path.join(project, ".claude", "data", "deprecations.jsonl")

    known = {}  # fp -> entry
    if os.path.exists(ledger):
        with open(ledger, encoding="utf-8", errors="replace") as fh:
            for raw in fh:
                try:
                    entry = json.loads(raw)
                    known[entry["fp"]] = entry
                except (json.JSONDecodeError, KeyError):
                    continue

    # Pre-compute command-level echo flags (cheap, done once per call).
    cmd_is_git_history = bool(_CMD_GIT_HISTORY_RE.search(command))
    cmd_is_history_tree = bool(_CMD_HISTORY_TREE_RE.search(command))
    cmd_is_tooling_read = bool(
        _CMD_TOOLING_SOURCE_READ_RE.search(command)
        or _CMD_FINDINGS_LEDGER_RE.search(command)  # Guard A2
    )
    cmd_is_ephemeral_script = bool(_CMD_EPHEMERAL_SCRIPT_RE.search(command))

    new_entries = []
    reencountered = []  # known, live
    matched_this_call = set()
    for text in texts:
        for line in ANSI.sub("", text).splitlines():
            line = line.strip()
            if not line:
                continue
            cls = classify(line)
            if cls is None:
                continue
            # Anti-echo guards: skip deterministic false-positive line shapes
            # before fingerprinting so they never enter the ledger.
            if _is_echo_line(
                line,
                cmd_is_git_history,
                cmd_is_history_tree,
                cls,
                cmd_is_tooling_read,
                cmd_is_ephemeral_script,
            ):
                continue
            fp = fingerprint(line, cls)
            if fp in matched_this_call:
                continue
            matched_this_call.add(fp)
            entry = known.get(fp)
            if entry is None:
                if len(new_entries) < MAX_NEW_PER_CALL:
                    new_entries.append(
                        {
                            "ts": datetime.now(timezone.utc).isoformat(timespec="seconds"),
                            "fp": fp,
                            "class": cls,
                            "line": line[:LINE_TRUNC],
                            "cmd": command[:160],
                            "status": "open",
                        }
                    )
            else:
                # Any ledger presence is a LIVE positive (fixed items are
                # deleted at close, never parked) — cite, don't re-fire.
                reencountered.append(entry)

    context_parts = []
    system_parts = []

    if new_entries:
        os.makedirs(os.path.dirname(ledger), exist_ok=True)
        with open(ledger, "a", encoding="utf-8") as fh:
            for entry in new_entries:
                fh.write(json.dumps(entry, ensure_ascii=False) + "\n")
        fps = ", ".join(e["fp"] for e in new_entries)
        lines = " | ".join(f'{e["fp"]} [{e["class"]}]: "{e["line"][:110]}"' for e in new_entries)
        context_parts.append(
            f"[deprecation-sentinel] {len(new_entries)} NEW finding(s) "
            f"captured to .claude/data/deprecations.jsonl — {lines}. "
            f"DISPATCH NOW (do not derail the current task): launch the "
            f"`deprecation-triage` agent via the Agent tool with "
            f"run_in_background: true and the prompt 'Triage ledger "
            f"fingerprint(s) {fps} per your agent definition "
            f"(.claude/agents/deprecation-triage.md). Your goal is the "
            f"largest genuine step toward stasis this run supports — "
            f"canonicalize by concern, land what is bounded, document "
            f"live trajectories for the rest.' "
            f"If the Agent tool lacks the deprecation-triage type this "
            f"session, use general-purpose with the same prompt. Then "
            f"continue your current task."
        )
        system_parts.append(f"+{len(new_entries)} new → deprecation-triage dispatch")

    if reencountered:
        # Deterministic backlog citation: once per session per fingerprint.
        cited_path = session_cited_path(str(payload.get("session_id", "")))
        cited = load_session_cited(cited_path)
        fresh = [e for e in reencountered if e["fp"] not in cited][:MAX_CITED_PER_CALL]
        if fresh:
            try:
                with open(cited_path, "a", encoding="utf-8") as fh:
                    for e in fresh:
                        fh.write(e["fp"] + "\n")
            except OSError:
                pass
            cites = "; ".join(
                f'{e["fp"]} status={e.get("status", "open")}'
                + (f' decision={e["backlog"]}' if e.get("backlog") else " (untriaged)")
                for e in fresh
            )
            context_parts.append(
                f"[deprecation-sentinel] known deprecation(s) re-encountered — "
                f"current decision(s): {cites}. No action needed; the "
                f"deprecation-stasis sweep owns re-checks."
            )

    if not context_parts:
        return

    print(
        json.dumps(
            {
                "systemMessage": "deprecation-sentinel: " + "; ".join(system_parts or ["known re-encounter cited"]),
                "hookSpecificOutput": {
                    "hookEventName": "PostToolUse",
                    "additionalContext": " ".join(context_parts),
                },
            }
        )
    )


if __name__ == "__main__":
    try:
        main()
    except Exception:  # noqa: BLE001 — sentinel must never break a session
        pass
    sys.exit(0)
