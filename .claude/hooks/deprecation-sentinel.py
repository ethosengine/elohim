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
ECHO_TOOLING_SOURCE_RE = re.compile(
    r"\.claude/(?:hooks|scripts|skills|agents|data|memory)/"
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
    r"(?://[/!]?|\#(?!\[))"  # comment opener: // /// //! or # (NOT #[attr])
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


def _is_echo_line(
    line: str,
    cmd_is_git_history: bool,
    cmd_is_history_tree: bool,
    cls: str | None = None,
) -> bool:
    """Return True if *line* is a known echo false-positive and must be skipped.

    Called after classify() returns non-None to prevent false echo entries
    from being fingerprinted and appended to the ledger.

    The six guards:
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
    """
    # Guard A — ledger self-capture
    if ECHO_LEDGER_PATH.search(line):
        return True

    # Guard D — ephemeral-script self-capture (<string>:N: / <stdin>:N:).
    # Placed early: cheapest deterministic skip, no command-flag dependency.
    if ECHO_EPHEMERAL_SOURCE_RE.search(line):
        return True

    # Guard F — crates.io `+deprecated` build-metadata self-capture: a
    # Cargo lock/manifest version literal (`version = "…+deprecated"`) OR a
    # registry artifact filename (`…-0.9.34+deprecated.crate`): registry
    # metadata, not a live warning. Deterministic, no command-flag dep — like D.
    if ECHO_LOCKFILE_VERSION_META_RE.search(line):
        return True

    # Guard E — agentic tooling-source echo (.claude/ paths, review-finding
    # markers): our own governance tooling talking about deprecations/vulns.
    if ECHO_TOOLING_SOURCE_RE.search(line):
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
            if _is_echo_line(line, cmd_is_git_history, cmd_is_history_tree, cls):
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
