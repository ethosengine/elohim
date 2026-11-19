**Hold `220b5642f`.** All twelve ordinary-path probes gave the expected result. Three read-only checks exposed cases those probes miss: a protected multi-ref push can receive only a startup warning, a local Git setting can hide the conductor pin, and the batch reader can return an existing blob for a nonexistent path.

I checked `bb2a6ab74..220b5642f` in the context of the full change from `2080441f5`. I ran the evaluator with Git-object output from the named commits and supplied the hypothetical path, policy, and tag changes in memory. No files were modified, and no builds or Cargo commands ran. Below, `guard`, `walker`, and `CI` refer to `genesis/orchestrator/fleet-roll-guard.mjs`, `graph-walker.mjs`, and `Jenkinsfile`.

### Five required fixes

| Prior requirement | Status | Evidence and remainder |
|---|---|---|
| 1. Pushed manifests and destination policy | **PARTIAL** | The graph uses the pushed superproject tree, and policy uses the advertised destination OID. The Brit/Rakia warning still consults the local submodule manifest. Returning an unrelated pipeline name for the same Brit pin changed **WARN** to **no row**. The regression test supplies only a matching checkout. `guard:183–207,340–362,424–432,537–538`; `fleet-roll-guard.test.mjs:273–286` |
| 2. Build-process reads, invalid graphs, omitted `depends` | **PARTIAL** | Missing files and functions mark steps stale, duplicate pipelines, missing steps and cycles produce errors, and the sorter receives normalized dependencies. The object reader can nevertheless hand the walker the wrong file, and a symlinked process file supplies the link text where CI reads the target. `walker:249–252,268–293,302–309,345`; `guard:155–173,204–207`; `build-graph.groovy:307–321` |
| 3. Deploy-only and Edge dispatchability | **PARTIAL** | An ordinary `[deploy-only]` over validation-only work now asks for the declaration even if the already-built filter could remove Edge. Blank `jenkinsPath` is handled. Two exceptions remain. The guard excludes a manual Edge even for explicit `[build:edge]`, while CI’s explicit force checks `jenkinsPath`. Also, skip and validate-only suppress the declaration entirely, contrary to the owner’s **always asks** ruling. CI suppression can still be reported accurately alongside the declaration requirement. `guard:218–248,302–321`; `CI:1886–1893,2828–2844` |
| 4. Older hook and startup failure | **PARTIAL** | With no ref list, the guard reads HEAD’s message and tree and judges the supplied paths. The documented non-HEAD limit is accurate. Ordinary startup failures get the ruled severity. A long ref list can still turn a protected startup failure into an advisory warning. `guard:460–478`; `.husky/pre-push.bash:35,1064–1074` |
| 5. Rows, cures and regression coverage | **PARTIAL** | The named regressions have tests. The baseline text now conditions advancement on green, the declaration text says the reason is unchecked, and a missing destination names the hook’s remote. Combined forcing tags and the symlink cure remain inaccurate. `fleet-roll-guard.test.mjs:124–148,262–286,386–395`; `guard:99,261–279,420,437` |

### Git-object findings that block landing

- **Batch responses are only partially validated.** The reader checks header syntax, payload length, newline, and OID equality for bare-OID requests. It uses LF-delimited requests for `sha:path`, accepts the next blob for such a request, and never checks that it consumed all stdout. With real `git cat-file --batch` output, one path containing an LF shifted the responses: the parser returned **74,464 characters for a nonexistent second path** and discarded its real `missing` response. A missing manifest blob ordinarily becomes an evaluator error, and an ordinary missing process file becomes stale. Neither safeguard works when a different blob has already been assigned to that path. `guard:152–173,197–207`
- **Tree enumeration fails closed on size but loses path identity elsewhere.** `ls-tree -r -z` avoids a per-entry argument limit. The listing and each complete batch have a **64 MiB** buffer cap. Overflow throws and produces a protected refusal under `refuse`, so a sufficiently large valid docs-only push can be refused. Below the cap, the listing is decoded as UTF-8, which collapses distinct invalid byte sequences. The changed-path read separately uses quoted, newline-delimited `diff --name-only` and trims every line. More immediately, I compared the real conductor update `230c06042^..230c06042`: the ordinary diff included the conductor pin, while local `diff.ignoreSubmodules=all` removed it and left only the sprint document. The guard uses that same unqualified diff. `guard:98,106,140–145,184–195,428–432`
- **The destination OID is correct; shallow history is a separate issue.** The current hook passes the advertised destination SHA. A protected destination commit absent locally produces an explicit refusal naming the remote and branch to fetch. A present destination with missing or invalid policy defaults to `refuse`. But two readable endpoint trees do not prove a complete declaration history: `git log base..tip` can terminate successfully at a shallow boundary, hiding a legitimate declaration or, on a merge, failing to exclude older history behind a shallow destination. The code treats the result as conclusive. `.husky/pre-push.bash:1053–1059`; `guard:110–118,415–431`
- **A protected CLI startup failure can pass the fleet check without a decision.** I exercised the hook’s exact `printf | grep -qE` condition with `dev` first and 800 additional refs. `grep` found `dev` and exited zero, but the writer exited nonzero. With the hook’s `pipefail`, this selects the branch that prints “no pushed ref deploys.” If the hook subsequently takes an early pass, this check has recorded no red. The same hook already documents this pipe failure elsewhere. `.husky/pre-push.bash:35,198–199,1067–1081`

### Twelve pushed-tree probes

Each path was supplied alone against `220b5642f`, targeting `dev` with an ordinary tip and no declaration. “No row” means no fleet-roll row.

| Changed path | Observed |
|---|---|
| `elohim/holochain-conductor` | **REFUSED** — conductor image selects storage, Edge image and deploy |
| `elohim/sdk/schemas/.gitignore` | **REFUSED** — storage, Edge image and deploy |
| `elohim/brit` | **WARN** — CI prunes the absent submodule pipeline |
| `elohim/rakia` | **WARN** — same |
| `elohim/eprfs/epr-cli/src/bin.rs` | No row |
| `genesis/orchestrator/gate-runner.mjs` | No row |
| `elohim/Cargo.lock` | **REFUSED** — doorway and storage, then Edge image and deploy |
| `genesis/docs/content/elohim-protocol/manifesto.md` | No row |
| `elohim/holochain/build-manifest.json` | No row for a valid edit preserving the graph |
| `genesis/agentic/pool-policy.json` proposing `off` | No row on the ordinary tip. With `[build:edge]` as a control, **REFUSED**. Both probes requested policy only at `bb2a6ab74`, the supplied destination. |
| `che-devworkspaces` | No row |
| `elohim/eprfs/device-runtime/src/layout.rs` | **REFUSED** — storage, Edge image and deploy |

The selection evidence is in `elohim/conductor-image/build-manifest.json:16`, `elohim/holochain/build-manifest.json:33,80,93,102,132,250,284`, and `guard:352–387,424–432`.

### Rows, cures and follow-up

**Blocking operator text.** `[build:all]` gets the promised “CI forces nothing” explanation only when no independent selection wins. With `[build:all] [build:edge]`, the cure can say to drop `[build:edge]` even though `[build:all]` still requires a declaration. A path-selected push carrying `[build:all]` has the analogous problem. The symlinked-manifest refusal also offers “declare the selection” as a cure, although `loadTree` throws before the declaration can be honored. `guard:246–249,261–279,380–384,431–437`

**Follow-up.** The 64 MiB cap warrants streaming or a size-specific diagnostic; it produces a visible refusal. The hypothetical pin-warning step list also uses *all* changed paths, so a mixed push can attribute unrelated Edge steps to the pin. The stored-hash, already-built, earlier-staleness and older-hook non-HEAD limits are disclosed. The timing follows `gate_on_red` as ruled. `guard:26–35,98,364–370`; `.husky/pre-push.bash:1044–1052`

VERDICT: FIX FIRST

- Remove checkout-dependent pin decisions and make changed-path reads preserve path identity and include gitlinks regardless of local diff settings.
- Make batch requests unambiguous, validate complete response consumption, and resolve or explicitly flag symlinked process inputs.
- Detect incomplete declaration history before treating a shallow range as conclusive.
- Classify protected destinations on CLI startup failure without the early-exiting pipe.
- Apply declaration policy independently of dispatch suppression, match CI’s explicit-force dispatchability, and correct the combined-tag and symlink cures.