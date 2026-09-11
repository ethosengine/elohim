---
id: memory-kit-replacement-task-3-report
title: Station three — cite tooling on brit, audits absorbed by the package verifier
status: DONE_WITH_CONCERNS
class: devflow
gap: plans__2026-09-10-memory-kit-replacement-finish#3
actor: agent:implementer@claude-opus-5
commits: []
cites:
  - "memory-kit-replacement-finish | The plan this station drains | sha256:8c5418194e8c0c7e | status: stale — target content moved on; re-verify | path: genesis/docs/superpowers/plans/2026-09-10-memory-kit-replacement-finish.md"
  - "parity-inventory-2026-09-10 | The inventory ledger whose cite and audit rows this station corrects | sha256:739db2976d95a003 | path: genesis/docs/superpowers/plans/memory-kit-replacement/parity-inventory-2026-09-10.md"
---

# Station three — cite tooling on brit; audits absorbed by the package verifier

Two of the station's six deliverables rest on premises that do not hold. Both are
stated up front with the evidence, because the rest of the station was implemented
around them.

## Headline

| # | Deliverable | Outcome |
|---|---|---|
| 1 | brit tool pin + resolver | **BLOCKED** — `brit-build-ref` cannot be built here: two of its transitive deps live only on the auth-gated `elohim` Nexus registry and no credential is provisioned in this container |
| 2 | Byte-parity proof, brit seal vs `cite-gen.py --seal` | **NOT POSSIBLE** — brit has no cite-sealing verb. What *is* provable was proved: `cite-gen --seal` fixed-point digests recorded for all ten docs, and brit's fingerprint recipe cross-checked against the Python oracle over 14 cite edges with 0 divergence |
| 3 | Switch invocations to the brit verb | **NOT DONE, deliberately** — there is nothing to switch to. Every invocation left on `cite-gen.py`; the full file:line list is below |
| 4 | Contract for `epr flow concerns --stamp <doc>` | **DONE** — §4; `cite-propagate.py` left in place, inventory row marked `pending-native` |
| 5 | `cites-migrate.py` no-op check, then delete | **NOT DELETED** — it is *not* a no-op: the dry run proposes 225 `id:` assignments and 10 envelope conversions across 7 docs |
| 6 | agent-audit / skill-audit → `package-projections.mjs verify` | **DONE** — floors refuse, advisories print, both scripts deleted, references rewired package-first |
| 7 | Tests and gates | **DONE** — all green; `just gate memory-ceremony` `EXIT=0` |

## Gate evidence

```
just gate memory-ceremony
… 60 recall tests OK · ceremony 5 scenarios / 22 steps passed · collective-memory 4 scenarios / 17 steps passed
EXIT=0
```

```
node elohim/sdk/domains/elohim-agent/scripts/package-projections.mjs verify
89 packages: 74 package-first, 0 source-fidelity, 15 native
elohim-agent package checks passed: 1998 passed
EXIT=0
```

Baseline before this seat was **1919 passed / 0 failed**; the delta is **+79**: 9 bound/fixture
legs plus one description-floor assertion per markdown package (45 skills + 25 agents).

```
python3 -m unittest discover -s .claude/scripts/memory-kit/__tests__ -p 'cite*_test.py'
25 assertions passed ✅
EXIT=5
```
```
python3 -m unittest discover -s .claude/scripts/_lib/__tests__ -p 'cite*_test.py'
59 assertions passed ✅
EXIT=5
```
`EXIT=5` is unittest's *no TestCase collected* code — these are script-style test files that
assert and self-report at import. Run directly, they return real exit codes:
```
cite_gen_test.py EXIT=0
cite_cid_parity_test.py EXIT=0
cite_graph_test.py EXIT=0
```

```
python3 -m unittest discover -s .claude/scripts/_lib/__tests__ -p 'managed_surfaces_test.py'
52 assertions passed ✅
EXIT=5   (same script-style collection; no failures)
```

---

## 1. The brit tool pin — BLOCKED on a missing registry credential

Submodule pin: `elohim/brit` @ `221a1b056380a97d42a4cf408a7fbc46f987a068`
(`gitoxide-core-v0.55.0-259-g221a1b0563`), so the install path would have been
`/projects/.claude-config/tools/brit-221a1b056380/bin/brit-build-ref`.
The binary is named `brit-build-ref`, not `brit`.

Berth discipline was observed: `berth claim cargo` before the build, `berth release cargo` after
(both `EXIT=0`). Pool slot resolved from inside the submodule:
`CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/brit/projects__elohim__elohim__brit/dev`.

```
cd elohim/brit && env RUSTFLAGS="" CARGO_TARGET_DIR=<pool slot> CARGO_BUILD_JOBS=2 \
  cargo build --release -p brit-build-ref
error: failed to get `rakia-brit` as a dependency of package `brit-cli v0.1.0`
Caused by: unable to update registry `elohim`
Caused by: authenticated registries require a credential-provider to be configured
EXIT=101
```

Three escalations, all refused:

1. `CARGO_REGISTRIES_ELOHIM_TOKEN="Bearer anonymous"` alone — same "credential-provider" error.
2. `+ CARGO_REGISTRIES_ELOHIM_CREDENTIAL_PROVIDER="cargo:token"` — the provider configures, then
   `HTTP 401` from `nexus.ethosengine.com/repository/cargo-internal/ra/ki/rakia-brit`.
   `EXIT=101`. The committed `elohim/brit/.cargo/config.toml` documents this exactly: Nexus
   advertises `auth-required:true` even though it serves anonymously, so a read token is required.
3. `--offline` — different, and worse: `no matching package named 'elohim-epr' found`. So the
   blocker is not only the sibling workspace member. **`brit-epr` itself** (the crate
   `brit-build-ref` links) takes `elohim-epr = { version = "0.1", optional = true }`, enabled by
   `brit-epr`'s default feature `elohim-protocol`. Neither `elohim-epr` nor `rakia-brit` is in
   `/opt/rust/cargo/registry/cache/` (only `index.crates.io-…` is present), and
   `$CARGO_HOME` (`/opt/rust/cargo`) holds no `credentials.toml`.

I did not go looking for a token; provisioning a registry credential is an operator act.

**What unblocks it:** export `CARGO_REGISTRIES_ELOHIM_TOKEN="Bearer <read-only NpmToken>"` plus
`CARGO_REGISTRIES_ELOHIM_CREDENTIAL_PROVIDER="cargo:token"` (or drop a `credentials.toml` into
`$CARGO_HOME`), then the build command above. No resolver (`genesis/agentic/bin/brit-bin`,
`BRIT_BIN`) was added — a resolver pointing at a binary that cannot exist is worse than its
absence, and per §3 there is nothing for the hooks to resolve *to* yet.

**Submodule pointer concern (as instructed, reported not acted on):** nothing in
`elohim/brit/**` was modified. Had the credential existed, the whole-workspace resolution of
`brit-cli`'s `rakia-brit` would still have to succeed for a `-p brit-build-ref` build — that is a
property of the pinned submodule's workspace layout, and if the pin is ever expected to build in
a credential-free container it needs `elohim-protocol` off by default or `brit-cli` out of the
default members. That is a change in brit, on `brit-dev`, for the operator to integrate.

## 2. Parity — what brit actually has, and what was proved

**brit has no cite-sealing verb.** This is the load-bearing correction to the plan and to the
inventory row, both of which name `meta seal cites`. Read at the pin:

- `elohim/brit/brit-build-ref/src/main.rs:57-75` — `MetaCmd` has exactly three variants:
  `Seal { --dir }`, `Verify { --cid }`, `Status { --dir }`. There is no `cites` subcommand.
- `elohim/brit/brit-build-ref/src/meta_cmd.rs:14-80` — `meta seal --dir` collects a directory's
  immediate files, computes an `EprMeta`, and prints/stores its CID. It **reads** each `.md`'s
  frontmatter to populate `imports`/`exports`; it writes to the git object store and never to a doc.
- `elohim/brit/brit-build-ref/src/meta_cmd.rs:96-113` — `meta status --dir` prints one verdict
  line per doc-cite. Read-only, exit 0, advisory.
- `elohim/brit/brit-epr/src/engine/cite.rs` — the whole module is readers: `extract_id`,
  `extract_cites`, `SlugIndex::build`, `resolve`. The only `fs::write` in the file is at `:108`,
  inside a `#[cfg(test)]` fixture.
- `elohim/brit/brit-cli` (binary `rakia`) has no cite verb; `grep -rn cite brit-cli/src/` is empty.

`cite-gen.py --seal` is a *composite writer*: `--assign-id` → `--into` → `--verify`, rewriting the
target's frontmatter (`id:`, `cites:` envelopes with `slug | desc | sha256:hex16 | path:`). Nothing
in brit does any part of that. `elohim/brit/brit-epr/tests/cite_parity.rs` — the test the inventory
cites as "parity-proven" — asserts **verdict-label** equality against the Python oracle
(`format!("{:?}", verdict(&edge, &idx))`), not envelope bytes. The inventory row overstated it.

### 2a. `cite-gen --seal` baseline digests (ten docs, mixed classes)

`--seal` was run against each real path (idempotent: the corpus is already at its fixed point, so
the tree was not mutated — `diff -rq before after` → identical, and `git status` on the ten shows
only the two files another seat had already left dirty). These sha256 digests are the fixed
baseline for whoever later builds the brit side.

| doc | sha256 (post-seal == pre-seal) |
|---|---|
| `genesis/docs/superpowers/specs/2026-05-28-in-flight-memory-coherence-design.md` | `8a9a51a757fc4e38d5918defe1041b1e39a2f79ecaa6dfcd273d8b522261da14` |
| `genesis/docs/superpowers/specs/2026-06-02-semantic-computable-links-design.md` | `9ed38bbede03272c7c5fe104050e9601ef50a03b955bdb5d1a55667e0ddaf35a` |
| `genesis/docs/superpowers/specs/2026-06-05-omnibar-consolidation-epr-native-links-design.md` | `1de47a7e43f0e3242be6b529f5d73ef25067588978e6956dcfee611c85f46d80` |
| `genesis/docs/superpowers/plans/2026-05-10-iroh-gossip-dual-publish.md` | `a4f6fedcd93e948f5a1d64b4246218e004bb2300b4d3326da822c560c0178d85` |
| `genesis/docs/superpowers/plans/2026-05-11-tiered-quilt-delivery-master.md` | `3135cd780875fa2226fd36c5b5dd74baedd7286a53f91b760924def4677cf253` |
| `genesis/docs/superpowers/plans/2026-05-23-spa-blob-deploy-drift.md` | `eb32e2097191ae2836378b821d7cada09f0a24ee331db0b406e9a4c52e02340f` |
| `.claude/memory/feedback_codegen_prettier_oscillation.md` | `ef8080891f8d2280f79ef87e7b46cc7619fe355b58d1e1b78d1e0b3f3a5dc9f3` |
| `.claude/memory/feedback_diesel_migration_timestamp_collision.md` | `db09c81ced1d016435053ed17b0b4b5d23d292f3143caf6fd07c5da0e14ac5e5` |
| `.claude/memory/feedback_lint_autofix_string_scan_poison.md` | `d9d4c5b271b845f606e61e419d1d865a627335a088291b61b158a1efb626c75d` |
| `app/elohim-app/src/app/qahal/CLAUDE.md` | `363f8ae962bf6f213add0d2c7c97d13e6c41962b3884c973cff9a29a9d9e963a` |

Eight sealed clean; two exited 1 on pre-existing `DEAD-CITE` targets
(`2026-05-08-iroh-libp2p-complementarity.md`, `2026-05-11-tiered-quilt-stewardship-design.md`) —
both are legacy path-cites to files that no longer exist, unrelated to this station.

### 2b. Read-side recipe parity (the half brit *does* implement)

`elohim/brit/brit-epr/src/engine/frontmatter.rs::drift_fingerprint` was transcribed to Python
line-for-line and run against `_lib/cite_graph.fingerprint` over every envelope edge in the ten docs:

```
edges checked=14 diverged=0 unresolved=5
EXIT=0
```

The 5 unresolved are legacy path-cites at non-`.md` targets (`codegen-ts.mjs`, `db/mod.rs`,
`elohim-core/`, two `.claude/` paths) — outside brit's `SlugIndex`, which walks `*.md` only.
One drift note surfaced and is **not** mine: `2026-06-02-semantic-computable-links-design.md`
declares `sha256:99100efd20d10129` for `unified-memory-loop-design` whose live body is
`sha256:07e941a325cc49c2` — a stale envelope in a file another seat has dirty.

**One residual divergence class, unmeasured because no writer exists to measure it:** the two
`split_frontmatter` implementations differ on malformed input. Rust requires a literal `---\n`
prefix and finds `\n---\n`; Python takes `lines[0].strip() == "---"` and any line whose `strip()`
is `---`, then re-joins with `\n`. A delimiter line with trailing whitespace, or CRLF endings,
would split differently and therefore fingerprint differently. Nothing in the current corpus trips
it; it is the first thing to fixture when the brit writer lands.

## 3. Invocations NOT switched, and why

Switching these would point the hooks, commands and the surface registry at a verb that does not
exist. Every one stays on `cite-gen.py`. Full list, so the next seat has the diff pre-computed:

| file:line | invocation |
|---|---|
| `.claude/hooks/cite-seal-signal.py:8` | doc comment naming `cite-gen --seal <doc>` |
| `.claude/hooks/cite-seal-signal.py:99` | remediation string `python3 .claude/scripts/memory-kit/cite-gen.py --seal {rel}` |
| `.claude/hooks/cite-seal-signal.py:101` | remediation string naming `cite-describe.py` |
| `.claude/commands/plan.md:128` | `cite-gen.py --seal <new-plan-path>` |
| `.claude/commands/plan.md:129` | `cite-describe.py <plan> …` |
| `.claude/commands/brainstorm.md:198` | `cite-gen.py --seal <new-spec-path>` |
| `.claude/commands/brainstorm.md:202` | `cite-describe.py <doc> …` |
| `.claude/commands/shift.md:86` | `cite-gen.py --seal-all` |
| `.claude/commands/shift.md:89` | `cite-describe.py` |
| `.claude/scripts/_lib/managed_surfaces.py:13` | module doc: `cite-gen --seal-all / cites-migrate` sweep scope |
| `.claude/scripts/_lib/managed_surfaces.py:65,67,68` | doc-root surface discipline + two `tools:` entries |
| `.claude/scripts/_lib/managed_surfaces.py:77,88,100,113,122,131` | six further `tools:` entries naming `cite-gen.py --seal <file>` |
| `.claude/skills/semantic-links/SKILL.md:54-64,74,79,82-83,87,96,105-108` | the whole verb table + tool list |
| `.claude/skills/plant-eprfs-agentdoc/SKILL.md:3,17-36,49,65-66` | the "composes with cite-gen" section and its `--verify` step |

`--verify` semantics: `cite-gen.py --verify <doc>` is **kept explicitly** and remains the
dissolution gate. brit's nearest read is `brit-build-ref meta status --dir <dir>`, which prints
advisory verdict lines and exits 0 — it is a reporter, not a gate, and it has no per-doc mode.
Even with the binary built, `--verify` could not be swapped without adding a non-zero exit path
to `meta status` in brit.

## 4. Contract — `epr flow concerns --stamp <doc>`

Written, not implemented: `elohim/eprfs/**` is the native seat's write set. `cite-propagate.py`
stays in place and its inventory row now reads `pending-native`. Current corpus state, for the
implementer's fixture:

```
python3 .claude/scripts/memory-kit/cite-propagate.py        # dry run
cite-propagate [DRY-RUN]: 895 docs
  status stamped: 72 {'stale': 52, 'dead': 20} | cleared: 0 | path: refreshed: 0 | docs touched: 48
EXIT=0
```

**Purpose.** Fan each cited target's verdict back out onto every citing edge as inline
`status:` / `path:` segments, so link health travels *with* the doc in git rather than living in
a report. The verdict must be the same one `epr flow concerns` displays — one computation, two
renderings — which is the whole reason this is a `concerns` flag and not a new verb.

**Signature.**
```
epr flow concerns --stamp <doc>...        # write the doc(s) named
epr flow concerns --stamp --all           # the whole corpus
epr flow concerns --stamp --dry-run …     # default when neither --write nor --apply is given
```
Default is dry-run, matching `cite-propagate.py`. Writing requires an explicit `--write`.

**Inputs.**
1. Target docs — explicit paths, or the corpus: doc roots (`genesis/docs`, `.claude/memory`)
   plus `id:`-declaring gospel `CLAUDE.md`s repo-wide. Skip set: `CLAUDE.md`, `MEMORY.md`,
   `INDEX.md`, `README.md`, `TRAJECTORY.md`, `claude.md` inside doc roots (a gospel that
   declares cites re-enters through the gospel walk), plus `/_state/` and `/memory-kit/`.
2. The slug index, built over the doc roots **and** `genesis/docs/superpowers/held/` — held
   targets must resolve, or every held cite reads `dead` instead of `held`.
3. Each cite envelope's declared `fingerprint`.

**Outputs — the five verdicts and their exact stamps.** Strings are load-bearing: the corpus
already carries them and any change is a corpus-wide diff.

| verdict | condition | `status:` written |
|---|---|---|
| `ok` | slug resolves, not under `held/`, fingerprint matches the live body digest | *field removed* |
| `held` | slug resolves to a path containing `/held/` | `held — target sequestered (see cluster-state.yaml)` |
| `stale` | slug resolves, declared fingerprint ≠ recomputed | `stale — target content moved on; re-verify` |
| `remote` | slug does not resolve **and** the fingerprint slot holds a full CID (`baf…`) | `remote — resolvable on the substrate, absent locally` |
| `dead` | slug does not resolve and the fingerprint is short-form or absent | `dead — target no longer resolves` |

`path:` is refreshed on every envelope from the live resolution. A dead slug **keeps** its last
known path as a forensic breadcrumb — never blanked. Legacy path-string cites are skipped
entirely: their ref *is* the path, they carry no status.

**Comparison rule (do not reimplement the digest).** For a short-form `sha256:hex16` token,
compare against the first 16 hex of `sha2-256(canonical_body)`. For a full-CID token, **decode the
CID's digest** and compare raw bytes to the same `sha2-256` — never re-encode a CID to compare
strings. `canonical_body` is the frontmatter-excluded, trimmed body. An unreadable target is
`ok`, not `stale`: an I/O failure is not fingerprint-drift evidence.

**Idempotence.** Running twice writes once. The second run must report `0 stamped, 0 cleared,
0 paths refreshed, 0 docs touched` and leave every byte identical. The write path rewrites only
the `cites:` block via the frontmatter list rewriter; frontmatter key order, body, and trailing
newline are preserved. An envelope carrying a `: ` (which every `status:`/`path:` one does) must
be emitted double-quoted or it parses as a mapping mid-scalar and hard-blocks the evaluator.

**Refusal cases — the verb must refuse, not guess.**
1. Doc has no frontmatter block → refuse naming the path. Never synthesize one.
2. Doc has frontmatter but no `cites:` key → skip silently (not an error; most docs).
3. The `cites:` block cannot be rewritten (nested mapping, unterminated frontmatter) → refuse
   that doc, name it, continue the corpus, and exit non-zero at the end. Partial rewrite of a
   doc is forbidden: write the whole reassembled file or none of it.
4. A target path resolves but is unreadable → verdict `ok`, and count it in an
   `unreadable: N` line so the silence is visible.
5. Two docs declaring the same `id:` → refuse the whole run naming both; a colliding slug index
   makes every verdict untrustworthy.
6. `--stamp` without `--write`/`--apply` never touches disk, including on refusal paths.

**Evidence.** Emit the same shape `cite-propagate.py` prints, so a run is diffable against the
Python it replaces: `docs`, `status stamped: N {verdict: count …}`, `cleared`,
`path: stamped/refreshed`, `docs touched`. Parity is proven when a `--stamp --all --write` on a
tree where `cite-propagate.py --apply` has already run produces zero changes, and the reverse.

## 5. `cites-migrate.py` — NOT retirable

```
python3 .claude/scripts/memory-kit/cites-migrate.py       # dry run, default
cites-migrate [DRY-RUN]: 895 docs
  pass 1: 225 id: slugs assigned (575 already had one)
  pass 2: 10 cites converted to envelopes across 7 docs
EXIT=0
```

The inventory's `retire — already applied, re-run is a no-op` is false as of today. 225 of 800
corpus docs carry no `id:` at all. `cite-gen --seal` assigns an id only to the doc it is pointed
at, so docs authored since the 2026-06 migration have accumulated un-slugged. `cites-migrate.py`
is the only corpus-wide sweep that closes that gap, and deleting it would silently drop live
hygiene. **Left in place.** Retiring it needs a `--seal-all`-style native sweep first, or an
`--apply` run to actually drain the 225 — a corpus-wide frontmatter write, which is an operator
decision, not a station-three side effect.

## 6. The audits, absorbed by the package verifier

New module `elohim/sdk/domains/elohim-agent/scripts/package-quality.mjs` (the house pattern —
`hook-package.mjs`, `command-packages.mjs`, `mcp-packages.mjs`, `agent-doc-packages.mjs` are all
siblings of the verifier rather than inlined into its 2 300 lines).

**Bounds are read, not hardcoded.** `loadQualityBounds()` scans
`.claude/epr-meta/measures.yaml` for the lens rows and their `hard:` values, and the verifier
asserts that the read succeeded rather than silently falling back:

```
PASS: quality bounds read from the declared middot rows in .claude/epr-meta/measures.yaml
      (got {"skillDescriptionFloor":"skill-description-floor@1",
            "agentDescriptionFloor":"agent-description-floor@1",
            "triggerOverlapThreshold":"trigger-overlap-ceiling@1",
            "dynamicStopwordFraction":"dynamic-stopword-fraction-ceiling@1"})
```

Live values: 60 (skills), 80 (agents), 3 shared words, 0.25 stopword fraction — the rows station
one declared. Literal fallbacks exist for an unreadable registry and each is annotated with the
row id that authorizes it.

**What refuses vs. what prints.**

- **Refuses** (hard assert, one per markdown package): `description-missing` and
  `description-too-short`. The corpus is clean today — 0 of 45 skills below 60, 0 of 25 agents
  below 80 — so the floor cost nothing to arm.
- **Prints only** (advisory, never fails): weak trigger phrasing (`description-no-when`,
  `description-no-triggers`) and trigger overlap. Overlap **cannot** be a gate on this corpus:

```
quality (advisory — never refuses; --quality for detail):
  skills: 45 described · 4 advisory description finding(s) · 42 trigger-overlap pair(s) above 3 shared words
  agents: 25 described · 0 advisory description finding(s) · 68 trigger-overlap pair(s) above 3 shared words
```

110 pairs sit above the declared ceiling, and the largest are the mechanism working as designed:
`plant-eprfs-agentdoc ~ plant-eprfs-hook` shares 30 words because each member of that family
*names its five siblings* in its own description, which is precisely the disambiguation the
overlap check was meant to encourage. `deprecation-triage ~ runtime-triage` shares 49 for the same
reason. Making it hard would refuse the corpus for doing the right thing; making it invisible
would lose the signal. It prints a count, and `--quality` expands to the pair list.

The 4 skill advisories are `holochain-hdk-0-7`, `seed-workflow`, `valueflow-authoring`,
`valueflow-reviewer` — descriptions with no trigger word. Their packages are outside this seat's
write set; they are named here for the owning seat.

**Staleness is NOT ported.** `surface-stale-mtime-days@1` (hard 90) has no home in the verifier.
Package verification is content-based: a projection's mtime records the last `project` run, not
anything about the capability, so an mtime bound would fire on every fresh checkout and stay
silent on a genuinely stale package that happened to be reprojected. If age is wanted it is a
fold over authored history, which belongs to `epr flow report`. This reasoning is written into
`package-quality.mjs`'s module header so it does not have to be re-derived.

**Fixtures run inside `verify`, not only `selftest`.** The corpus passing tells you nothing about
whether the refusal still works, so the boundary is proved on every run:

```
PASS: description-floor fixture is 40 chars (got 40)
PASS: description-floor fixture is 80 chars (got 80)
PASS: description floor REFUSES a 40-char SkillPackage description (floor 60)
PASS: description floor ACCEPTS an 80-char SkillPackage description (floor 60)
PASS: description floor REFUSES an empty SkillPackage description (floor 60)
PASS: description floor REFUSES a 40-char AgentPackage description (floor 80)
PASS: description floor ACCEPTS an 80-char AgentPackage description (floor 80)
PASS: description floor REFUSES an empty AgentPackage description (floor 80)
```

### Deletions

| path | status |
|---|---|
| `.claude/scripts/memory-kit/agent-audit.py` | deleted (460 lines) |
| `.claude/scripts/memory-kit/skill-audit.py` | deleted (459 lines) |

Left as unstaged worktree deletions (`git rm` then `git reset` on the two paths) so the index
stays clean for the no-commit rule. No test, no `justfile` recipe and no
`genesis/build-manifest.json` input referenced either script.

### References rewired — package-first, then `just codegen agents write`

| package | change |
|---|---|
| `.epr-meta/elohim/packages/skills/memory-kit.json` | §6 and §7 (the two audit sections) replaced by one section on the verifier; the Review row and the monthly-sweep bullet re-pointed |
| `.epr-meta/elohim/packages/agents/librarian.json` | toolkit table rows merged into one verifier row; "`agent-audit.py` is your tool" → the verifier, with the package-edit-then-reproject discipline named; the `/converge` handoff line and the full-pass list updated |
| `.epr-meta/elohim/packages/skills/converge.json` | hygiene-tool list re-pointed |
| `.claude/scripts/_lib/managed_surfaces.py` | the `skill` and `agent` surface entries now name the declared floors and the verifier command instead of the two scripts |

Package diffs are minimal (`memory-kit.json` 12 lines, `librarian.json` 4, `converge.json` 2);
`just codegen agents write` reprojected them and `verify` is green at 1998.

**Residual references, outside this seat's write set** — each names a now-deleted script:
`.claude/scripts/memory-kit/CLAUDE.md` and `.claude/scripts/memory-kit/LIFECYCLE.md` (inside the
kit directory station six removes wholesale, and `CLAUDE.md` is currently dirty from another
seat), and the `procedure:` / `provenance:` strings on six rows of `.claude/epr-meta/measures.yaml`
(`package-description-chars@1`, `surface-stale-mtime-days@1`, `trigger-overlap@1`,
`dynamic-stopword-fraction@1`, and two `agent-audit.py:76` rationale rows). The measures strings
are historical provenance — they record where a number came from — so they are arguably correct
as-is, but the owning seat should decide.

## Files written by this seat

| path | change |
|---|---|
| `elohim/sdk/domains/elohim-agent/scripts/package-quality.mjs` | new — bounds reader, description diagnosis, overlap analysis |
| `elohim/sdk/domains/elohim-agent/scripts/package-projections.mjs` | import + `MEASURES_PATH` + `--quality` flag + `verifyDescriptionFloorFixtures` + `reportQualityAdvisories` + the per-package floor assert in `verifyPackage` |
| `.claude/scripts/_lib/managed_surfaces.py` | `skill` and `agent` surface entries re-pointed |
| `.epr-meta/elohim/packages/{skills/memory-kit,skills/converge,agents/librarian}.json` | audit references rewired |
| `genesis/docs/superpowers/plans/memory-kit-replacement/parity-inventory-2026-09-10.md` | six rows corrected (cite-gen, cite-describe, cite-propagate, cites-migrate, agent-audit, skill-audit); frontmatter added — it had none, so its slug did not resolve and this report could not cite it |
| `genesis/docs/superpowers/plans/memory-kit-replacement/task-3-report.md` | this report |
| `.claude/scripts/memory-kit/{agent-audit,skill-audit}.py` | deleted |

Not touched: `elohim/brit/**`, `elohim/eprfs/**`, `.claude/epr-meta/*.yaml`,
`.claude/hooks/_observation.py`, `.claude/hooks/cite-seal-signal.py`,
`.claude/commands/{brainstorm,plan,shift}.md`, `genesis/agentic/**`.

## Concerns

1. **The plan's station-three premise does not hold.** brit cannot seal cites, and the inventory
   row that said it could conflated verdict-label parity with envelope-byte parity. Stations that
   depend on "cite tooling runs on brit" need re-planning around building the writer in brit
   first — the read side (fingerprint recipe, slug index, verdicts) is genuinely ready.
2. **The tool pin needs a Nexus credential** that is not provisioned in this container.
3. **`cites-migrate.py` is live, not spent** — 225 docs lack an `id:`. Worth its own decision.
4. **72 cite edges are already stamped-stale/dead in dry run** (52 stale, 20 dead, 48 docs).
   `cite-propagate.py --apply` has not been run recently. Not this seat's to apply.
5. **`just codegen agents write` reprojected other seats' dirty packages.** 28 packages were
   already modified in the worktree when this seat started; the write command projects *all*
   packages, so `.codex/agents/*.md` and `.agents/skills/*` moved for packages this seat never
   authored. The projections now match their packages, which is the correct end state, but the
   diff is wider than this seat's edits.
6. **One unmeasured `split_frontmatter` divergence class** between the Rust and Python recipes
   (trailing-whitespace delimiters, CRLF). Fixture it when the brit writer exists.
