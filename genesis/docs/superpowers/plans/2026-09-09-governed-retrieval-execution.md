---
title: Governed retrieval execution and progressive discovery
id: governed-retrieval-execution
status: Active
class: devflow
serves: dev-system-equilibrium
date: 2026-09-09
cites: []
---

The ceremony needs executable limits and repeatable discovery, while agents retain authority to
continue useful evidence gathering. The algorithm is an authored content artifact carried by
existing EPRFS content addressing and dependency seals, not a new core EprKind. The existing
recall-contract.json owns composition, source scope, limits, fitness questions and retirement.
The CLI is its local executor; procedure text alone cannot enforce these properties.

- [ ] An agent discovers candidate sources by subject/category, reads bounded evidence, and continues for an explicit question with cumulative execution accounting under the same pinned algorithm.

Composition: declared source scope → bounded metadata discovery or bounded semantic provider
→ exact category/subject filtering where metadata exists → grouping over returned candidates
→ explicit selected source/excerpt → independent fitness judgment. Semantic provider output
is unverified candidate text, never exact tag membership or authority. Group counts describe
only the measured candidate window. File, scan, result, output and time bounds name incomplete
frontiers; no silent truncation is accepted as a complete source. All CLI operations require a
session and question. The session binds contract and executable identities and accumulates
usage before emitting evidence. Failed operations remain charged; method changes require
an explicitly new execution session. Direct shell/MCP reads remain outside this executor.

P2P gate: local authored algorithm bytes are private authoring B; execution counters are local
B operational accounting and returned candidate/group views C. Published algorithm content can
use existing Content (content_store_integrity), with existing governance and attestation
mechanisms; this slice publishes nothing, adds no DHT entry/link/head, coordinator, signal,
SQLite or Automerge projection, HTTP route, DNA-hash change or peer authority. Identity uses
content-derived fingerprints and existing EPRFS seals; session labels are local accounting
locators, not agent identity. Byte transport is local; future transport affinity is unchanged.
Seed/year-one additional network heads = 0. No stage weakens source containment or honest absence.

Concern answers: C0 local context selection; C1/C2/C9/C13 n-a no authority/identity change;
C3 bounded time and explicit continuation; C4 named incomplete frontier; C5 candidate != authority;
C6a byte/file/result/scan/time bounds; C6b reads plus locked counter updates, no content mutation;
C7 executable contract tested; C8 receipts identify method, operation and cumulative cost;
C10 method change refuses in-session reuse; C11 caller-selected demand remains within bounds;
C12 repository scope and explicit invocation; C14 unknown ranking/freshness/acceptance retained.
These are implementation requirements, not yet acceptance evidence. Python tooling introduces
no Rust crate decision point; focused adversarial tests are the contract check.

Verification: adversarial fixtures for oversize/escaping sources, filtered partial discovery,
semantic output overflow/timeouts, session continuity and method drift, UTF-8 errors and
progressive excerpt bounds. Independent reviewer must use the real CLI and judge whether
continued useful work remains possible without pretending the entire corpus was searched.
Retire this local executor when native EPR execution provides equivalent tested behavior and
migrates session accounting without losing continuity.
