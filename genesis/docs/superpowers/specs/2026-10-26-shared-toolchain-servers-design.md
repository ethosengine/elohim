---
title: Shared toolchain servers with per-session diagnostic attribution
id: shared-toolchain-servers-design
status: Draft
class: architecture
serves: edit-diagnostics-reach-every-session
date: 2026-10-26
context-tier: disclosed
steward: human:matthew
tags: [toolchain, lsp, rust-analyzer, session, berth, eprfs]
graduation-trigger: >-
  Rust slice 1 passes the owning gates and the edit-diagnostics-share-one-server scenario:
  concurrent sessions share one server for one root and pin, receive the C7 attribution views,
  yield flycheck to cargo leases and foreground builds, and retire idle children after 900 seconds
  with no clients while the resident and its device-wide lease remain live. The habit atom carries the measured evidence and the spec's cites are sealed.
cites:
  - elohim/lvi/docs/specs/2026-10-10-workspace-arrival-as-declared-device.md
  - "compute-envelope-tevah | why the resident is an own child of the device root envelope, not a guest: guests fail closed below Enforced | sha256:d427a367f3b01a6e | path: genesis/docs/superpowers/specs/2026-09-02-compute-envelope-tevah-design.md"
  - genesis/agentic/.epr-meta/edit-diagnostics-reach-every-session.habit.md
  - genesis/a2o/features/devflow/edit-diagnostics-share-one-server.feature
---

# Shared toolchain servers with per-session diagnostic attribution

| Scheme | Meaning | Home section |
|---|---|---|
| C-n (C1–C9, plus C6b) | Interface contracts, including C9 — Berth presence. A lettered sub-contract such as C6b has its parent's authority | Appendix B (authoritative) |
| Rule N | The numbered rules of `.claude/hooks/cargo-disk-guard.py`. Only Rule 4's yield is specified here (C6b); Rules 1–3 and 5 already exist and are outside this spec | `.claude/hooks/cargo-disk-guard.py`; C6b |
| CC-n | Concern-canon criteria from `.claude/epr-meta/concerns.yaml`, with evaluable criteria in `.claude/epr-meta/policies.yaml` | Appendix A maps their applicability here |
| D-n (D1–D22) | Design decisions | §Decisions taken |

This specifies the Rust slice approved by the steward, `human:matthew`, per the frontmatter.
Appendix B is authoritative for the wording of C1–C9: where body and appendix differ on a
contract, the appendix wins. Body sections stay normative for everything no C-n covers, and
for the examples and detail a contract delegates to them (for C7, the worked example in §5
and D15/D16's flicker and fold-text rules). An implementer reads both. **Superseded working copy:**
`genesis/local-dev/shared-toolchain-servers/contracts.md` is superseded in its entirety;
no section of it is authoritative. Appendix B replaces all of it, and the file is retired
at landing.
Paths and line numbers below identify source definitions and recorded evidence. Absolute
paths name device-local evidence outside this worktree.

## Vocabulary

### Terms this spec's contracts use

The berth terms below follow `genesis/agentic/bin/berth` (command help, `claim_preflight`
and `holder_is_live`); the pool terms follow `genesis/agentic/bin/pool-lib.sh`
(`POOL_ROOT`, `detect_family` and `slot_path`).

| Term | Meaning and source |
|---|---|
| flat-workspace | The Cargo workspace's repository-relative path flattened for a pool slot: `flatten_path` first replaces each `/` with `_`, then replaces every run of one or more underscores (including a single `_`) with exactly `__` (`sed 's/__*/__/g'`), then strips all leading underscores and all trailing underscores, in that order. Thus `elohim/elohim-storage` → `elohim_elohim-storage` → `elohim__elohim-storage`; trimming leaves that result unchanged (`genesis/agentic/bin/pool-lib.sh:65`). |
| LSP terms | This spec assumes Language Server Protocol terms such as `didOpen`, `publishDiagnostics` and `textDocumentSync`; see the [Language Server Protocol reference](https://microsoft.github.io/language-server-protocol/). |
| bin-launch | Native registry-backed executable launcher, sourced at `elohim/eprfs/device-runtime/src/bin/bin-launch.rs`; it discovers the binary registry and delegates execution to `bin_store::launch_os`, so the selected target becomes the running executable. The attach shim uses `/projects/.claude-config/tools/bin-store/bin/bin-launch` (`genesis/agentic/bin/lsp-attach:16`). |
| WASM root | A root under `elohim/holochain/dna/`, whose Rust code compiles to WebAssembly for Holochain. The LSP policy unsets the native toolchain pin there; ordinary DNA packing needs its in-tree `./target` (`genesis/agentic/bin/lsp-flycheck`, workspace case; `genesis/agentic/bin/pool-lib.sh:76`). |
| gate | The manifest-declared quality checks selected by `just gate` for changed files or an explicit project/path; `genesis/orchestrator/gate-runner.mjs` selects and executes them from `build-manifest.json` (`justfile:18`; `genesis/orchestrator/gate-runner.mjs:1`). |
| a2o | Executable Gherkin acceptance scenarios in `genesis/a2o`; a habit's `checks:` and the scenario's `@concern:` tag join its claim to runnable proof (`CLAUDE.md`, “What to work on, and how you know it's done”). |
| habit | A declared behavior bound to a runnable check in `<dir>/.epr-meta/<id>.habit.md`, with an evidence ledger and `retire-when:` condition; status changes require evidence (`CLAUDE.md`, “What to work on, and how you know it's done”). |
| DELTA | A one-line evidence-backed change recorded in the habit's own atom on completion, followed by re-projecting the habit register (`CLAUDE.md`, “What to work on, and how you know it's done”). |
| cite sealing | `epr flow cites seal <doc>` assigns an ID if absent, converts eligible `cites:` references to content-addressed envelopes, and verifies them; slugs, fingerprints and locators are tool-managed (`.claude/skills/semantic-links/SKILL.md`). |
| epr / eprfs | `eprfs` is the native filesystem projection layer; its `elohim/eprfs` workspace also builds the `epr` device CLI (`elohim/eprfs/README.md`, `elohim/eprfs/epr-cli/Cargo.toml`). Shared hooks resolve it through `.claude/hooks/_observation.py` → `.claude/scripts/_lib/epr_client.py` (`EPR_BIN`, otherwise `epr bin status --tool epr --json`); the edit hook uses `EPR_BIN` or PATH (`.claude/hooks/lsp-edit-event.py`), while the attach shim tries `EPR_BIN`, `bin-launch`, pool debug binary, then PATH (`genesis/agentic/bin/lsp-attach`). |
| berth | The workspace's capacity register: it tracks present sessions, resource holders and their notes. `berth status` shows moorings, leases and capacity. |
| mux | The multiplexer inside the resident: it accepts each session's attach connection, routes LSP traffic between sessions and the one server child for that root, and applies attribution and delivery (§4, §5). |
| mooring / synthetic mooring | A mooring registers a session with its PID and process start time, refreshed by a heartbeat so dead or reused PIDs cannot retain ownership. A synthetic mooring gives a background process its own session label and PID instead of borrowing an editor's lifetime, using the same `berth moor --session … --pid …` interface (precedent: `.husky/pre-push.bash`, `berth_moor_hook`). |
| lease | A record that one session holds one unit of a named resource. Releasing or unmooring gives it back; expiry or a dead mooring permits takeover. |
| claim | `berth claim` requests a lease: refusal exits 3 without queueing; no resolvable session (also an inaccessible store) exits 4. Only exit 4 lets a wrapper proceed unleased, subject here to C6's run lock and yield checks. |
| capacity | The number of simultaneous lease holders allowed for a resource, declared in `genesis/agentic/pool-policy.json` under `berth.resources`. The cargo resource currently has capacity 2; this is distinct from an artifact directory. |
| class | A resource-specific lease policy chosen with `--class`, such as `verify` or `lsp`. Its row in `berth.classes` controls admission and lifetime. |
| TTL | Time to live, in seconds: a mooring expires after its last touch and a bounded lease after its last claim or renewal. A lease class can supply a default and a maximum. |
| daemon class | A lease class marked `daemon: true` has no lease TTL and is reclaimed when its mooring dies. Each root's child separately has a 900-second idle timer; its retirement leaves the resident lease held (`elohim/eprfs/epr-lsp/src/daemon.rs:748`; `elohim/eprfs/epr-cli/src/lsp.rs:387`). |
| cargo pool | Shared local directories for reusable Cargo build artifacts, selected through `CARGO_TARGET_DIR`. The default root is `/projects/.cargo-target-pool`. |
| family | The pool's grouping of related worktrees, selected by environment override, a `.family` file, then a branch-name heuristic or worktree-name fallback. Worktrees in one family can reuse artifacts. |
| slot | One artifact directory at `<pool>/family/<family>/<flattened-workspace>/<profile>`. This design uses an `lsp` profile directory beside `dev`; a directory does not grant a cargo lease. |
| flycheck | rust-analyzer's background Cargo check. The `lsp-flycheck` wrapper serves two modes (C6): a save-triggered package check that defers when capacity is busy, and `--build-scripts`, a workspace-wide build-script/proc-macro run that waits for admission instead. This repository selects `lsp-flycheck` as its check command with `checkOnSave: true` in `genesis/agentic/pool-policy.json`. |
| anchor / anchor URI | Publication location of a fold (a grouped Information-severity summary of other errors/warnings): package manifest, root manifest, or the diagnostic's own file, independent of receiver (§5; `elohim/eprfs/epr-lsp/src/deliver.rs:73`). A null server answers without an analyzer; its single status notice follows §3's first-open rule (`elohim/eprfs/epr-lsp/src/attach.rs:149`). |
| RAM guard | `genesis/agentic/bin/ram-guard` keeps the workspace below its container memory limit through kernel steering and shedding non-critical processes before a workspace-wide OOM kill (module docstring). |
| I/O guard | `genesis/agentic/bin/io-guard` monitors host I/O pressure and container write rate, observing or enforcing a disk-write budget by pausing, resuming or shedding build processes (module docstring). |
| short8 | The first eight characters of a session ID, used only for display. Defined at `elohim/eprfs/epr-lsp/src/attrib.rs:58`; ownership compares full session IDs (`elohim/eprfs/epr-lsp/src/attrib.rs:47`). |
| ark | The process envelope in `elohim/ark`, which starts and reaps declared children and carries their resource-bound declarations (`elohim/ark/CLAUDE.md`; `elohim/ark/supervisor/src/native.rs`). |
| resident / device mux | A long-lived device process sharing editor connections; it starts language-server children lazily and retires idle children while remaining up (`elohim/eprfs/epr-lsp/src/daemon.rs:569`, `elohim/eprfs/epr-lsp/src/daemon.rs:748`). |
| device supervisor | ark acting as the process envelope for the device's own declared children; the foreground entry point is `elohim/eprfs/epr-cli/src/lsp.rs:349`. |

### Repository terms used for classification (§6, Appendix A)

| Term | Meaning and source |
|---|---|
| EPR (protocol) | Elohim Protocol Record, also read as Reference during composition: the protocol's content-addressed unit carries knowledge, value and governance together. This is distinct from the lowercase `epr` CLI (`genesis/docs/content/elohim-protocol/protocol-specification.md`, Executive Summary). |
| DHT | Distributed hash table; Holochain's agent-centric notary substrate, where peers validate entries against their shared DNA rules without global consensus (`genesis/docs/content/elohim-protocol/protocol-specification.md`, Scope). |
| DNA (Holochain) | An application's rule set, divided into zomes; its hash defines a distinct network, and a cell is one participant's running instance (`genesis/docs/content/elohim-protocol/protocol-specification.md`, Scope). |
| CID | Content identifier derived from bytes; EPR uses CIDv1 over canonical DAG-CBOR with SHA-256 (`elohim/epr/README.md`, What this crate provides; `README.md`, Architecture: Codec). |
| integrity / coordinator zome | Integrity defines entry/link types and validation rules committed to the DNA hash; coordinator implements operations and signals and can be hot-swapped (`.claude/skills/holochain-hdk-0-7/references/architecture.md`, Coordinator vs. Integrity Zomes). |
| concern-canon | Versioned, evidence-backed classes of recurring seam concerns. Non-evaluable classes live in `.claude/epr-meta/concerns.yaml`; evaluable enforcement rows live in `.claude/epr-meta/policies.yaml`. Both feed the generated `elohim/sdk/schemas/v1/registries/concern-classes.json` (the two homes and named consumers in `.claude/epr-meta/concerns.yaml:13`). |
| DevspaceSeed | lvi's declared arrival lifecycle: what the workspace is, what persists and which health/readiness conditions establish arrival (`elohim/lvi/docs/specs/2026-10-10-workspace-arrival-as-declared-device.md:70`). |
| head-plane cost | The recurring per-item work of reconciling heads, refreshing discovery records, electing holders and making conductor calls as the item count grows (`.claude/skills/p2p-design-gate/SKILL.md`, “Head-Plane Cost Budget”). |
| gradient-reading | Produces a reading that separates repeated cost which sharing between trusted parties can remove from cost still owed on every use. Here, sessions authorized for the same checkout share indexing, removing duplicated indexing; per-edit analysis stays owed. The method asks about mechanism, relationship, cost kind, custody, plane, multiplier, phase and waiting party; it does not measure performance (`.claude/skills/gradient-reading/SKILL.md`, “The eight questions” and “Reading the answers”). |

## 1. Measured failure class

Independent per-session language servers duplicate indexing and background compiler work,
compete with foreground builds, and deliver diagnostics without identifying whose edit
preceded them. The recorded observations, their sources and measurement limits are retained
verbatim in Verification (b), Historical evidence. The resident contract addresses server
sharing, build yielding and advisory attribution; Verification (a) is the complete current
acceptance checklist.

## 2. Topology

The approved shape is a **device resident under ark**: one foreground
`epr lsp serve --device` mux on one device socket. Internally each `(lang, root)` has
one active generation on its policy pin. The first attach starts its child; a different
pin in a later attach plan replaces it. The Rust policy defaults to `idle_ttl_s:900`;
idle retirement leaves the resident up
(`elohim/eprfs/epr-cli/src/lsp.rs:349`;
`elohim/eprfs/epr-lsp/src/daemon.rs:569`,
`elohim/eprfs/epr-lsp/src/daemon.rs:748`;
`elohim/eprfs/epr-lsp/src/server.rs:36`).

The operator-approved declaration is:

```text
epr device run epr --ready-line "epr-lsp: listening" --restart permanent -- lsp serve --device
```

Launch flags (`--ready-line`, `--restart`) come before `--`; everything after `--` is the
resident's own argument list.

On this branch alone, before the merge into `feat/gate-live`, start the resident directly
from the checkout with `epr lsp serve --device` and keep it running in the foreground; `--root <dir>` selects
an explicit root. The command binds the socket, requires berth admission, then prints and
flushes `epr-lsp: listening <socket>` (`elohim/eprfs/epr-cli/src/lsp.rs:349 (serve)`).

The approved supervised declaration is an **own child of the device root envelope**,
with restart `Permanent`, not a guest. Guests fail closed below `Enforced` (tevah §4,
`genesis/docs/superpowers/specs/2026-09-02-compute-envelope-tevah-design.md:326`);
ark reports `EffectiveTier::None` today
(`elohim/ark/supervisor/src/native.rs:83`). Supervision belongs to this slice;
enforcement remains later work. Readiness uses ark's existing `StdoutLine`
(the expected stdout line appeared) and `ExecutableIdentity`
(the running executable matches the declared artifact) probes
(`elohim/ark/supervisor/src/supervisor.rs:779`).
`Permanent` is ark's default child restart policy
(`elohim/ark/core/src/manifest.rs:329`).

**Integration status:** the declaration form is implemented on `feat/gate-live` in
06dd6c51c (`elohim/eprfs/epr-cli/src/device/declaration.rs` there); this branch alone predates
it. The LSP side prints and flushes `epr-lsp: listening <socket>` after bind and successful
berth claim (`elohim/eprfs/epr-cli/src/lsp.rs:379`). The ready line is required: as of
`feat/gate-live` a3dfff3c7, ark never marks a resident declared without one ready, and the
arrival line counts such a child as waiting only while it has no live process.
Verification records the installed, supervised run as PENDING.

```text
ark device root envelope
  own child: epr lsp serve --device  [Permanent]
    device.sock <- attach A / attach B / attach C
      rust + root A + current pin -> rust-analyzer (lazy; idle retirement)
      rust + root B + current pin -> rust-analyzer (lazy; idle retirement)
      per-root attribution snapshot -> epr lsp show
```

Worktrees remain distinct analysis roots: attach canonicalizes its root; the resident
resolves policy fresh for each attach and indexes children by language/root
(`elohim/eprfs/epr-cli/src/lsp.rs:163`,
`elohim/eprfs/epr-cli/src/lsp.rs:294`;
`elohim/eprfs/epr-lsp/src/daemon.rs:569`).
Two independent repositories on one device can have separate residents at the socket/lock
level: `main_checkout` takes the parent of Git's absolute common directory (falling back to
the selected root), and `runtime_dir` appends `.eprfs/status/lsp`, so a main checkout and
its worktrees share `device.sock` while unrelated repositories get different paths
(`elohim/eprfs/epr-cli/src/lsp.rs:189 (main_checkout)`,
`elohim/eprfs/epr-cli/src/lsp.rs:205 (runtime_dir)`,
`elohim/eprfs/epr-cli/src/lsp.rs:349 (serve)`).
Startup still requires each resident's berth admission: `lsp.rs` does not create a
repository-specific berth store or mooring name, and both use `lsp-device` / `toolchain:lsp`
(`elohim/eprfs/epr-cli/src/lsp.rs:558 (Berth::claim)`,
`elohim/eprfs/epr-cli/src/lsp.rs:632 (berth_call)`).

**C3 — runtime directory.** `<main checkout>/.eprfs/status/lsp/` holds `device.sock`,
`device.lock`, `edits.jsonl`, per-root `<lang>-<root12>.json` records and
`attrib-<lang>-<root12>.json` snapshots.
`root12` is the first 12 SHA-256 hex digits of the canonical absolute root.
Registry fields remain
`{"pid","startTime","root","lang","toolchain","targetDir","startedAt","sock"}`:
PID/start identify the **mux**, and sock is the device socket
(`elohim/eprfs/epr-lsp/src/socket.rs:28`,
`elohim/eprfs/epr-lsp/src/socket.rs:40`,
`elohim/eprfs/epr-lsp/src/socket.rs:84`,
`elohim/eprfs/epr-lsp/src/socket.rs:100`).
There are no per-root socket/log files; child stderr inherits mux stderr
(`elohim/eprfs/epr-lsp/src/server.rs:259`). Appendix B specifies file lifetimes.

**D1 — pin replacement.** There is no drain handshake. A fresh attach plan with a different
pin queues `toolchain pin changed <old> → <new>; restarting` as a showMessage and
Information diagnostic to that root's clients, closes their connections, retires the old
child and starts the replacement. Retirement is asynchronous, so old and new processes may
briefly overlap. Other roots, the resident socket and lease remain live
(`elohim/eprfs/epr-lsp/src/daemon.rs:569`,
`elohim/eprfs/epr-lsp/src/daemon.rs:673`;
`elohim/eprfs/epr-lsp/src/mux.rs:1085`).
A mux EOF without client exit makes attach return 1, enabling the configured editor
restart-on-crash to reattach (`elohim/eprfs/epr-cli/src/lsp.rs:260`).

## 3. Attach path

Use the repo-scoped `elohim-lsp` plugin with a local marketplace. The branch's auto-loaded
`.lsp.json` launch surface is
`.claude/marketplace/plugins/elohim-lsp/.lsp.json:1`.
`${CLAUDE_PROJECT_DIR}` is the project directory Claude Code substitutes in this command:

```json
{
  "rust": {
    "command": "${CLAUDE_PROJECT_DIR}/genesis/agentic/bin/lsp-attach",
    "args": ["rust"],
    "extensionToLanguage": {".rs": "rust"},
    "startupTimeout": 15000,
    "restartOnCrash": true,
    "maxRestarts": 3
  }
}
```

The plugin points to an attach endpoint. It does not install a PATH replacement named
`rust-analyzer`. The shim resolves `epr` through `EPR_BIN`, then `bin-launch`, then the pool's
debug `epr`, then PATH (`genesis/agentic/bin/lsp-attach`). If none resolves, the shim
reports `no diagnostics: epr unresolved` over LSP as specified below. Repairing the existing
epr PATH shadow remains outside this slice; the `workspace-device-arrival` habit owns it.

The plugin schema accepts either `.lsp.json` at the plugin root or `lspServers` in
`plugin.json`. It requires `command` and `extensionToLanguage`, and also accepts `args`,
`transport`, `env`, `initializationOptions`, `settings`, `workspaceFolder`, timeouts, restart
controls and `diagnostics` (default true). The launch uses stdio. A bare command resolves on
PATH; an absolute path is allowed. `${CLAUDE_PLUGIN_ROOT}`, `${CLAUDE_PLUGIN_DATA}` and
`${CLAUDE_PROJECT_DIR}` are substituted in command, args, env and workspaceFolder, so the
project-root command above resolves independently of a nested launch cwd. The daemon retains
configuration ownership. The server starts lazily on the first edit of a matching extension,
and diagnostics reach the model automatically after edits
(`genesis/local-dev/shared-toolchain-servers/facts-plugin-schema.md:2`).
[Claude Code plugin manifest reference](https://code.claude.com/docs/en/plugins-reference#lspservers).
The installed marketplace's equivalent Rust launch was opened at
`/projects/.claude-config/plugins/marketplaces/claude-plugins-official/.claude-plugin/marketplace.json:3336`.

Project settings declare `extraKnownMarketplaces.elohim-workspace.source` as
`{"source":"directory","path":"./.claude/marketplace"}` and enable
`elohim-lsp@elohim-workspace` (`.claude/settings.json:437`). The marketplace names
`elohim-workspace`, its owner, and plugin source `./plugins/elohim-lsp`
(`.claude/marketplace/.claude-plugin/marketplace.json:2`).

**Bring-up today** (once in each workspace):

1. Install the plugin: `claude plugin install elohim-lsp@elohim-workspace --scope project`.
2. Trust the workspace folder.
3. Start the resident with `epr lsp serve --device` and keep it in the foreground until
   `epr device supervise` lands (§2 and C4).

A committed `enabledPlugins` entry does not download/install the plugin. Scope precedence
is local > project > user, so the project-level
`"rust-analyzer-lsp@claude-plugins-official": false` (`.claude/settings.json:445`) overrides
the user-level true without editing user settings
(`genesis/local-dev/shared-toolchain-servers/facts-plugin-schema.md:6`).

**C4 — CLI.** The native forms are
(`elohim/eprfs/epr-cli/src/lsp.rs:73`):

```text
epr lsp attach <lang> [--root <dir>]
epr lsp serve --device [--root <dir>]
epr lsp show [<lang>] [--root <dir>] [--session <id>] [--json]
```

Attach defaults to the checkout around cwd (cwd itself outside Git), canonicalizes its root
and connects to the device socket; it never starts a daemon
(`elohim/eprfs/epr-cli/src/lsp.rs:163`,
`elohim/eprfs/epr-cli/src/lsp.rs:233`).
The first socket bytes are a newline-terminated hello carrying identity, root and language,
then raw LSP frames. There is no drain prelude
(`elohim/eprfs/epr-lsp/src/socket.rs:183`,
`elohim/eprfs/epr-lsp/src/socket.rs:200`):

```text
EPR-LSP-HELLO {"v":1,"session":…,"ownerPid":…,"ownerStartTime":…,"root":"<absolute root>","lang":"rust"}\n
```

With an absent/refused resident socket, attach answers as a null server:
`no diagnostics: device supervisor not running — epr device supervise`.
Initialize gets `textDocumentSync:{"openClose":true,"change":0}`, then a type-2
showMessage; one Information diagnostic goes on the first opened URI. A change/save
before any open instead uses `<root>/Cargo.toml`. It answers until client exit/EOF and
returns success on this path, leaving editing available
(`elohim/eprfs/epr-cli/src/lsp.rs:38`,
`elohim/eprfs/epr-cli/src/lsp.rs:270`;
`elohim/eprfs/epr-lsp/src/attach.rs:113`).
`feat/gate-live` at commit `06dd6c51c` is the device-floor integration branch, into which
this branch merges before landing (Appendix B, C4). It contributes four things this spec
depends on and adds no contract to C1–C9:
`ManifestAct::Run` gains arguments, a ready line and a restart policy, so the resident can be
declared as specified in §2; `epr device supervise` runs one ark per data root under a
`device-ark` daemon lease and adds the `device supervisor` arrival line; the resident
declaration itself; and gate-live's producer for C8, which publishes gate reds with source
`gate-live`. The same branch carries the pre-push exemption in §8.
This branch alone does not dispatch it (`elohim/eprfs/epr-cli/src/device.rs:143 (run)`);
the interim start is foreground `epr lsp serve --device`.

A connected attach returns 1 for `ServerClosed`: mux EOF before client exit, or a mux
socket error such as a reset or broken pipe before client exit. Client exit/EOF returns 0;
client-stdio I/O errors also return 0. Socket failures therefore trigger the editor's
restart-on-crash path (`elohim/eprfs/epr-lsp/src/attach.rs:37 (proxy)`,
`elohim/eprfs/epr-cli/src/lsp.rs:233 (attach)`).
`show` reads snapshots and prints `mine`, `caused-by-me` and `others`.
No snapshot means an explicit no-server message in text, or `"servers":[]` in JSON
(`elohim/eprfs/epr-cli/src/lsp.rs:667`).

**D10 — plugin packaging boundary.** Slice 1 ships the hand-authored marketplace and
`elohim-lsp` plugin under `.claude/marketplace/`. Choosing its Elohim package kind and
adding its projection backend are S5 work, recorded as package-governance debt in §9.
Open Question 1 is closed for slice 1; resolving that debt is not a slice-1 prerequisite.

## 4. Mux semantics

Transport IDs are distinct from session IDs. Client requests and cancellations use remapped
upstream IDs; replies return to the original connection and request ID
(`elohim/eprfs/epr-lsp/src/mux.rs:412`,
`elohim/eprfs/epr-lsp/src/mux.rs:523`,
`elohim/eprfs/epr-lsp/src/mux.rs:764`).

**D11 — initialize.** Rewrite the first client's parameters with the mux PID, forced
root URI/path and workspace folder, and policy options. Remove client `positionEncodings`
and pull-diagnostic capability to use the default UTF-16 convention and push diagnostics
through attribution (`elohim/eprfs/epr-lsp/src/mux.rs:462 (initialize_params)`).
The cached result changes an existing `textDocumentSync` capability to full sync,
`TextDocumentSyncKind.Full` (value 1): an object gets `change:1`, otherwise the capability
becomes `1`. It does not add one the server omitted. Later clients receive the cached
result (`elohim/eprfs/epr-lsp/src/mux.rs:426 (client_initialize)`,
`elohim/eprfs/epr-lsp/src/mux.rs:788 (initialize_response)`).

**D12 — server→client requests.** No original server request is routed to a client
for its answer. Configuration is answered from policy; workspace folders from the root;
progress-token creation is acknowledged. Registrations/unregistrations are accepted and
broadcast using mux IDs; retained registrations replay on late-client `initialized`.
Requests ending in `/refresh` are acknowledged and broadcast; broadcast responses are
swallowed. `workspace/applyEdit` returns `applied:false`; other requests get
method-not-found. Server `$/progress` is dropped, diagnostics enter attribution, and
other notifications are broadcast. Client `didChangeConfiguration` is dropped
(`elohim/eprfs/epr-lsp/src/mux.rs:359 (on_client)`,
`elohim/eprfs/epr-lsp/src/mux.rs:502 (client_notification)`,
`elohim/eprfs/epr-lsp/src/mux.rs:536 (client_notification)`,
`elohim/eprfs/epr-lsp/src/mux.rs:745 (on_server)`,
`elohim/eprfs/epr-lsp/src/mux.rs:825 (server_request)`).

**D13 — disk authority.** Only first open and last close reach the child. Initial open
uses disk text, falling back to the client's buffer if no readable UTF-8 text is available.
Later changes require an opened document and readable disk text; otherwise they are dropped.
Save refreshes an open document from disk if readable, strips client text, and forwards.
Versions advance through the mux's shared document counter
(`elohim/eprfs/epr-lsp/src/mux.rs:549`,
`elohim/eprfs/epr-lsp/src/mux.rs:604`,
`elohim/eprfs/epr-lsp/src/mux.rs:633`,
`elohim/eprfs/epr-lsp/src/mux.rs:683`,
`elohim/eprfs/epr-lsp/src/mux.rs:1146`;
`elohim/eprfs/epr-lsp/src/deliver.rs:46`).
“Only when not on disk” therefore has a qualification: the code tests readability, not
existence, and only initial open uses the fallback buffer.

Edit-interest means the URIs in C2 with that full session ID; accepted LSP fallback edits
also join the retained set. Open interest lasts while any transport of the session holds
the document (`elohim/eprfs/epr-lsp/src/attrib.rs:271`;
`elohim/eprfs/epr-lsp/src/mux.rs:994`).
Shutdown is answered locally; exit/disconnect releases only that client's references.
Child death queues explicit notices and closes that root's clients
(`elohim/eprfs/epr-lsp/src/mux.rs:384`,
`elohim/eprfs/epr-lsp/src/mux.rs:1034`;
`elohim/eprfs/epr-lsp/src/daemon.rs:534`).

### D9 — transport bounds and child shutdown

- Frame body limit: **64 MiB** from client or server
  (`elohim/eprfs/epr-lsp/src/limits.rs:12`).
- Client outbound queue: **1024 messages or 32 MiB**. Overflow queues
  `epr lsp: too slow; reconnecting` and closes that client; the warning can only be
  delivered if the client reads it
  (`elohim/eprfs/epr-lsp/src/limits.rs:15`,
  `elohim/eprfs/epr-lsp/src/limits.rs:18`,
  `elohim/eprfs/epr-lsp/src/limits.rs:27`;
  `elohim/eprfs/epr-lsp/src/daemon.rs:841`).
- No pending server→client request bound: D12 server-request handling answers/refuses
  original requests locally and drops broadcast responses (`elohim/eprfs/epr-lsp/src/mux.rs:359 (on_client)`,
  `elohim/eprfs/epr-lsp/src/mux.rs:825 (server_request)`).
- An initialized child gets `shutdown`, up to **5 s** for a reply, `exit`, up to **2 s**,
  then SIGKILL if still alive. An uninitialized child starts at exit
  (`elohim/eprfs/epr-lsp/src/limits.rs:21`,
  `elohim/eprfs/epr-lsp/src/limits.rs:24`;
  `elohim/eprfs/epr-lsp/src/daemon.rs:707`,
  `elohim/eprfs/epr-lsp/src/daemon.rs:438`;
  `elohim/eprfs/epr-lsp/src/server.rs:269`).
  The 900-second idle threshold **starts retirement**, not an exact-time process-death
  guarantee (`elohim/eprfs/epr-lsp/src/daemon.rs:748`).

## 5. Attribution through epr session

### C1 — identity

Attach uses the shared session resolver and carries
`SessionKey { session, owner_pid, owner_start_time }`. Unresolved identity becomes
`anon-<pid>` with current-process metadata
(`elohim/eprfs/epr-cli/src/lsp.rs:209`).
**D17 — sessions.** Ownership and delivery compare full session ID only. PID/start remain
metadata; differences in them do not make two attribution owners. `short8` is the first
eight characters for display
(`elohim/eprfs/epr-lsp/src/attrib.rs:19`,
`elohim/eprfs/epr-lsp/src/attrib.rs:47`,
`elohim/eprfs/epr-lsp/src/attrib.rs:58`,
`elohim/eprfs/epr-lsp/src/attrib.rs:381`).

### C2 — edit events

The shared log is `<main checkout>/.eprfs/status/lsp/edits.jsonl`, resolved through the Git
common directory as in §2. Every line has this schema:

```json
{"v":1,"at":"<RFC3339 UTC ms>","session":"<id>","ownerPid":123,"ownerStartTime":"<str>","path":"<absolute path>","tool":"Edit|Write|MultiEdit|NotebookEdit"}
```

`ownerPid` is an integer; the example value 123 is illustrative. `tool` is one of the four
listed names. The writer's retention contract is Appendix B, C2: cooperating writers
serialize append/prune on a stable inode, but unread events may be pruned and torn lines
remain possible; this is best-effort evidence, not a lossless reader queue
(`.claude/hooks/lsp-edit-event.py:60 (append_event)`).
The reader takes no lock, resets only when length falls below its offset, buffers a trailing
partial line, and deduplicates retained event identities; a same-length or larger rewrite
can escape detection (`elohim/eprfs/epr-lsp/src/daemon.rs:267 (EditTail::poll)`,
`elohim/eprfs/epr-lsp/src/attrib.rs:139 (EditLines::feed)`,
`elohim/eprfs/epr-lsp/src/attrib.rs:271 (Attributor::record_edit)`).

The PostToolUse hook — a callback after an agent's tool completes — records successful edits, remains fail-open and has a timeout of at most
2 seconds. Its log is private,
device-local operational evidence. Under the C2 amendment, the mux and gate-live may read
it as non-deciding readers, apart from the writer's own pruning read. `show` reads the snapshot. No gate verdict, ownership permission or reputation decision may come from this log;
the same limit is explicit at `genesis/orchestrator/gate-live.mjs:1 (module contract)`
in integration commit `06dd6c51c`.

**D14 — C2 authority.** The device polls C2 before client changes/saves and server diagnostic
publishes. Each root filters logged paths to its subtree, ignoring sibling-worktree edits
(`elohim/eprfs/epr-lsp/src/daemon.rs:498`;
`elohim/eprfs/epr-lsp/src/mux.rs:313`).
A disk-changing `didChange` or a `didSave` contributes fallback evidence only when C2
does not already name that file within the inclusive window. Mirroring another session's
logged disk edit does not make a client a co-owner. Both fallback kinds can supply an owner;
neither writes an invented tool name into C2
(`elohim/eprfs/epr-lsp/src/mux.rs:628`,
`elohim/eprfs/epr-lsp/src/mux.rs:659`,
`elohim/eprfs/epr-lsp/src/mux.rs:713`;
`elohim/eprfs/epr-lsp/src/attrib.rs:294`).

### C7 — ownership and delivery

For each `publishDiagnostics` URI, compare with that URI's previous publish. A diagnostic
matches by **range + message + code**, before adding display prefixes. Only a new diagnostic
gets a new ownership decision. Collect distinct full session IDs with C2 or accepted `didChange`/`didSave` fallback events in
the inclusive interval `[publish_at - attribution_window_ms, publish_at]`; C5 sets this to
30,000 ms. The window covers edits in the served root, not only the diagnostic's URI: an edit
in one crate can break another. Zero candidates means unattributed; one means that owner;
more than one means ambiguous with **all** candidates. Never reduce the owner set by recency.

**D15 — flicker.** Ownership survives unrelated edits and expiry of its original window.
After a clear it is retained for one further attribution window; a matching return by that
expiry keeps its owners. A later return is new
(`elohim/eprfs/epr-lsp/src/attrib.rs:319`, `elohim/eprfs/epr-lsp/src/attrib.rs:399`). The exact
record is:

```text
{"owner": [SessionKey…], "ambiguous": bool, "firstSeen": ts, "edit": {path, at}}
```

**D8 — representative edit.** `edit` holds the latest candidate edit inside the attribution
window (maximum `at`); a tie goes to the lexicographically smaller full session id. The
record's `owner` array retains every candidate, and the delivered `data.epr.owners` lists
every candidate's display id. `ambiguous` alone determines ambiguity; selecting `edit`
never chooses a sole owner or reduces that candidate set.
Matching and D8's candidate/order rule are implemented at
`elohim/eprfs/epr-lsp/src/attrib.rs:208` and `elohim/eprfs/epr-lsp/src/attrib.rs:366`.
This temporal attribution is evidence of possible cause, not compiler-proven causality.
Slow analysis outside the window can be unattributed, and unrelated concurrent edits can make
it ambiguous. These limitations follow from the fixed C7 rule
(Appendix B, C7).

Full delivery is the union of:

1. all diagnostics on URIs the session opened with `didOpen`;
2. all diagnostics on retained edit-interest URIs for that full session ID, including
   accepted LSP fallback edits;
3. every diagnostic whose attribution names that session, wherever it lands.

**D16 — folds and data.** Other errors/warnings fold once per (area, full owner-ID set),
at Information severity. Missing or non-numeric severity counts as an error; Information
and hints neither count nor create folds. Full diagnostics are excluded from the recipient's
fold counts. Genuine diagnostics and folds share the anchor's published list, and a cleared
bucket clears its fold
(`elohim/eprfs/epr-lsp/src/deliver.rs:361`,
`elohim/eprfs/epr-lsp/src/deliver.rs:370`,
`elohim/eprfs/epr-lsp/src/deliver.rs:427`,
`elohim/eprfs/epr-lsp/src/deliver.rs:447`).
The message is `"<area>: <n> errors, <m> warnings from session <short8> (in flight)"`
or `"<area>: <n> errors, <m> warnings (unattributed)"`.
Multiple owners join with comma-space; folds have no `[by …]` prefix
(`elohim/eprfs/epr-lsp/src/deliver.rs:221`).

**D22 — area.** For a file under the root, choose the nearest ancestor manifest declaring
`[package]`, without searching above the root. If no package exists but the root has a
readable `Cargo.toml` (including workspace-only), use the root and that manifest.
Otherwise the file is its own area/anchor; non-file URIs anchor themselves
(`elohim/eprfs/epr-lsp/src/deliver.rs:73`,
`elohim/eprfs/epr-lsp/src/deliver.rs:103`,
`elohim/eprfs/epr-lsp/src/deliver.rs:125`).
The fork is deliberate: the fold anchor is receiver-independent, while the null server's
single Information diagnostic sits on the session's first opened URI because no server
and no area exists then; a change/save before open instead uses the root manifest URI
(`elohim/eprfs/epr-lsp/src/attach.rs:149`).

Full diagnostics get
`data.epr = {"attribution":"mine|caused-by-me|other|ambiguous|unattributed","owners":[short8…]}`
when data is absent/null or an object. Object keys survive except a preexisting `epr`
key, which is replaced. Non-null non-object data is left alone. Foreign or ambiguous full
diagnostics get `[by <short8,…>] ` in the message.
`mine` means sole ownership on an interested URI; `caused-by-me` means sole ownership
elsewhere; ambiguous/unattributed labels remain explicit
(`elohim/eprfs/epr-lsp/src/deliver.rs:269`,
`elohim/eprfs/epr-lsp/src/deliver.rs:289`).
Code-action contexts run through `unstamp`: marked folds are removed; marked full
diagnostics lose their mux prefix and `data.epr`. Without `data.epr` they are returned
unchanged, including any prefix
(`elohim/eprfs/epr-lsp/src/mux.rs:404`;
`elohim/eprfs/epr-lsp/src/deliver.rs:325`).

### Worked example: A changes eprfs-core and eprfs-cli breaks

This is a specified fixture, not a test result. The dependency is real:
`elohim/eprfs/eprfs-cli/Cargo.toml:16` depends on `eprfs-core`, whose public exports are at
`elohim/eprfs/eprfs-core/src/lib.rs:38`.

A, B and C share one root and pin. Their display IDs are `aaaaaaaa`, `bbbbbbbb`, and
`cccccccc`, backed by distinct full session IDs and their SessionKey metadata. A opens and edits
`elohim/eprfs/eprfs-core/src/lib.rs`. B has opened `elohim/eprfs/eprfs-cli/src/main.rs`.
C has opened neither. At time T, A's edit is logged; at T+2 seconds the child publishes one
new type error in B's file. No other session edited anywhere in this root in that 30-second
window. The new diagnostic therefore has owner `[A]`, `ambiguous: false`.

There is also one unchanged, previously attributed B error in another file in the same
`eprfs-cli` package. B edited that file before the current window; A and C never opened it.
Its owner remains B. On the publish, the views are:

| Receiver | New error caused by A | Existing B error |
|---|---|---|
| A | Full diagnostic at the actual `eprfs-cli` URI, `data.epr.attribution = "caused-by-me"`, owners `["aaaaaaaa"]`; no foreign-owner prefix. | One Information fold on `elohim/eprfs/eprfs-cli/Cargo.toml`: `elohim/eprfs/eprfs-cli: 1 errors, 0 warnings from session bbbbbbbb (in flight)`. |
| B | Full diagnostic because they opened the URI; prefix `[by aaaaaaaa] `, attribution `other`, owners `["aaaaaaaa"]`. Opening a file does not transfer ownership. | Full diagnostic because they edited its URI; attribution `mine`, owners `["bbbbbbbb"]`. |
| C | One Information fold on the package manifest: `elohim/eprfs/eprfs-cli: 1 errors, 0 warnings from session aaaaaaaa (in flight)`. | A second fold on the same manifest, for owner B, with the B message above. There are two owner buckets, not one package-wide count. |

`epr lsp show --session aaaaaaaa` places the cross-crate error in `caused-by-me` and B's fold
in `others`. B's `mine` section includes their opened/edited URIs, including the full error
labelled as A's; `mine` as a section describes relevance, not a claim that B caused it. C sees
only the two folds in `others` for these errors.

In the ambiguity variant, B also edits within the same window before the new publish. The new
error's owners are `[A, B]` and its attribution is `ambiguous` for both; both receive it in full.
C gets one ambiguous-owner bucket for that error, preserving both candidates in
`data.epr.owners`, plus the separate older B bucket. The record has `ambiguous: true`;
D8 selects its representative `edit` without changing either owner. A later edit by C
cannot take ownership of the existing error.

The example requires an actual downstream diagnostic to be published. C6's package-scoped
Cargo check does not itself prove that reverse-dependent crates are checked. The fake server
can prove routing; the live scenario must separately prove that rust-analyzer emits this error.

### External diagnostic sets (C8)

Gate-live is a local projection of gate runs that wants its reds visible in the editor
(`genesis/orchestrator/gate-live.mjs:143 (openRun)` and
`genesis/orchestrator/gate-live.mjs:299 (readRun)` at integration commit `06dd6c51c`).
Its intended publication path is through the mux, so gate failures receive the same C7
attribution, session delivery and folds as compiler diagnostics instead of requiring a
second editor channel (`elohim/eprfs/epr-lsp/src/mux.rs:277 (Mux::handle)`).
External sets never replace the server's set; full diagnostics carry `data.epr.source`
where C7/D16 permits stamping, with opaque data and folds retaining C8's exceptions
(`elohim/eprfs/epr-lsp/src/deliver.rs:289 (stamp)`). Sets for a root with no running
server are dropped without starting one (`elohim/eprfs/epr-lsp/src/daemon.rs:482 (Device::on_event)`).
Appendix B, C8 retains the API and set-lifetime contract. The producer is landing on
`feat/gate-live` as an additive `gatefeed` module in epr-lsp: it tails
`<main checkout>/.eprfs/status/gate-live/runs/*.jsonl` and publishes reds whose locus is a `.rs`
file through `ExternalFeed::publish(lang, root, "gate-live", uri, diags)`, with attribution in
`data.epr`. Only the `--device` mux runs it.

## 6. P2P design gate

Edit events, attribution and runtime snapshots are **Ephemeral (C)**, device-local
operational state (C2; `elohim/eprfs/device-runtime/src/session.rs`): disk remains source truth,
analysis is rebuilt from it, and lost ownership stays unknown until new evidence arrives.
They add no DHT entry, link, coordinator call, network projection or head-plane cost at seed
or one year; C3 is a local cache only. There is no signing, HTTP route or peer dependency,
and losing the daemon leaves editing available. Appendix A retains the integrity-zome
locations and applicable concern-canon criteria; Verification records their evidence gaps.

## 7. Rust specifics

**C5 — policy.** Appendix B, C5 contains the authoritative JSON extracts from
`genesis/agentic/pool-policy.json`: the top-level `lsp` member (including literal
`$saved_file`) and the complete `berth.classes.cargo` table. They are extracts to retain
within the existing policy, not a replacement policy file.

Native roots use 1.96.1, consistent with `genesis/agentic/pool-policy.json:97`. A WASM root
is a root under `elohim/holochain/dna/`; its override **unsets** `RUSTUP_TOOLCHAIN` because
null means unset, leaving its normal stable selection. The reason for this distinction is
recorded at `/projects/.claude-config/projects/-projects-elohim/memory/manual-cargo-in-this-workspace.md:16`.
Do not reinterpret null as an empty toolchain name or apply the native pin to a DNA build.

**D18 — target slot.** Rust computes the slot because `cargo-pool key` refuses the
checkout top without a Cargo manifest above it. An empty workspace path yields
`<pool>/family/<family>/lsp`; other roots include the flattened workspace directory
(`elohim/eprfs/epr-cli/src/lsp.rs:405`).
Resolve `target_slot: "lsp"` as the sibling of the root family's `dev` slot:
`CARGO_TARGET_DIR = <cargo-pool dev-slot path>/../lsp`. The existing layout is
`<pool>/family/<family>/<flat-workspace>/<profile>`
(`genesis/agentic/bin/pool-lib.sh:41` and `genesis/agentic/bin/pool-lib.sh:69`). This isolates
LSP artifacts from gate artifacts. It does not promise separate disk caches for worktrees in
the same family. Existing DNA packing remains unchanged; its in-tree target requirement is
documented at `genesis/agentic/bin/pool-lib.sh:76`.

**D2 — target directory and build scripts.** Set `cargo.targetDir: false`: the child's
`CARGO_TARGET_DIR` (the LSP slot) governs every rust-analyzer Cargo invocation. The
`cargo.buildScripts.overrideCommand` above routes build-script and proc-macro work through
`lsp-flycheck rust --build-scripts` under the same environment and slot.

**D19 — override paths.** A relative `overrideCommand` program containing a slash resolves
against the root if present there, otherwise against its containing checkout. Bare names
remain bare (`elohim/eprfs/epr-lsp/src/server.rs:188`).
The policy leaves `$saved_file` for rust-analyzer to expand. Build-script overrides run once per Cargo workspace from that workspace's root
(the directory containing its `Cargo.toml`). These command and working-directory contracts
are documented in the [rust-analyzer configuration reference](https://rust-analyzer.github.io/book/configuration.html#rust-analyzer.cargo.buildScripts.overrideCommand).

**C6 — flycheck.** `genesis/agentic/bin/lsp-flycheck <lang> <saved_file>` finds the nearest
ancestor `Cargo.toml` containing `[package]` and obtains its package name. After taking
D4's device-wide run lock, it requests
`berth claim cargo --class lsp --ttl 600 --note "lsp flycheck <pkg>"`, then checks for
competing work before compiling. On claim success it resolves its own holder ID and
excludes that holder from `berth who cargo`; any other live Cargo lease or non-flycheck
heavy Cargo/rustc process blocks compilation even when a second Cargo slot is free.
It releases its lease and run lock before deferring. Claim exit 3 defers, exit 4 proceeds
unleased with the run lock held only after the same contention check, and other claim
errors exit non-zero
(`genesis/agentic/bin/lsp-flycheck:97 (busy_builds)`,
`genesis/agentic/bin/lsp-flycheck:173 (release_lease)`,
`genesis/agentic/bin/lsp-flycheck:237 (top-level admission loop)`).

Save-triggered deferral emits exactly one Cargo JSON message to stdout and exits 0,
without queuing or retrying. Deferring when the run lock or a lease is busy is intended,
not a liveness defect: the next save retries. `<holder>` names
`another lsp flycheck (<path>)` for the held lock, the live lease holder or blocking process
PID, or the claim refusal detail when neither is found:

```json
{"reason":"compiler-message","package_id":"<pkg>","manifest_path":"<path>","target":{"kind":["lib"],"name":"<pkg>","src_path":"<saved_file>"},"message":{"message":"flycheck deferred: build slot held by <holder>","level":"warning","spans":[],"children":[],"code":null,"rendered":"flycheck deferred: build slot held by <holder>\n"}}
```

After admission, run `cargo check --message-format=json -p <pkg> --all-targets` with C5's
environment and the LSP slot, exporting `ELOHIM_LSP_FLYCHECK=1` to the Cargo tree. The
wrapper releases its claimed lease on exit, failure and handled signals. The JSON deferral
must become visible to attached sessions; a spanless message discarded by the child
would fail live acceptance (`genesis/agentic/bin/lsp-flycheck:118 (warning)`,
`genesis/agentic/bin/lsp-flycheck:132 (defer)`,
`genesis/agentic/bin/lsp-flycheck:181 (cleanup)`,
`genesis/agentic/bin/lsp-flycheck:283 (top-level Cargo launch)`).

**Build-scripts mode (D2).** `lsp-flycheck rust --build-scripts` runs
`cargo check --workspace --message-format=json --all-targets` from the Cargo workspace root.
It waits instead of deferring: a held run lock, claim refusal or post-claim contention
returns to the admission loop, with 10-second polls and a 30-minute waiting limit. Any
acquired lease and run lock are released before waiting. Stderr names the lock path,
holder/process or claim refusal detail; timeout exits non-zero. `LSP_FLYCHECK_POLL_S` and
`LSP_FLYCHECK_WAIT_MAX_S` permit shorter tests, including a 0.1-second poll. Both modes
use the same marker and cleanup (`genesis/agentic/bin/lsp-flycheck:156 (wait_or_defer)`,
`genesis/agentic/bin/lsp-flycheck:181 (cleanup)`,
`genesis/agentic/bin/lsp-flycheck:237 (top-level admission loop)`).

Both modes bound compilation to the remaining lease budget: default TTL 600 seconds,
minus a 30-second margin and time spent claiming/checking contention, with a 10-second
TERM-to-KILL grace. Exhaustion emits `flycheck exceeded its lease; stopped`. Overrides are
`LSP_FLYCHECK_TTL_S` and `LSP_FLYCHECK_MARGIN_S`, requiring `10 < margin < TTL`
(`genesis/agentic/bin/lsp-flycheck:83 (top-level budget validation)`,
`genesis/agentic/bin/lsp-flycheck:272 (top-level budget and Cargo launch)`).

**D4 — single-run safety and foreground admission.** A non-blocking `flock` on
`<main checkout>/.eprfs/status/lsp/flycheck.lock` enforces at most one flycheck or
build-scripts run per device for every caller, including the shared `lsp-device` holder
and unleased (exit 4) callers. The main checkout is the dirname of
`git rev-parse --path-format=absolute --git-common-dir`, so worktrees share one lock.
The wrapper takes it before the berth claim, releases it before every wait or defer,
and holds it for the run because Cargo inherits the descriptor
(`genesis/agentic/bin/lsp-flycheck:229 (run_lock / flock block)`). The regression test is
`test_a_second_run_from_the_same_session_defers_while_the_first_compiles`
(`genesis/agentic/lsp_flycheck_test.py:267`): the second invocation defers without
starting Cargo while the first compiles.

Admission against foreground builds is unchanged: the berth lease plus the post-claim
contention scan checks other holders and non-flycheck heavy processes, and Rule 4 yields
to an admitted foreground build. Exit 4 retains the contention scan under the run lock
(`genesis/agentic/bin/lsp-flycheck:97 (busy_builds)`,
`genesis/agentic/bin/lsp-flycheck:237 (top-level admission loop)`; Appendix B, C6/C6b).

A live concurrent-arrival probe under the real resident (two roots, one device) remains
an acceptance item. The budget assumption remains one check of about 2–5 GB, never stacked.

**Rule 4 yield:** Appendix B, C6b defines the complete predicate and cancellation protocol.

**Cargo class compatibility.** C5 records the resolved `"verify": {}` row beside `lsp`
and why classless callers require it; Cargo capacity and other existing class semantics
remain unchanged.

## 8. Berth

The berth presence belongs to the resident, so editor detach and child retirement do not
give back its capacity. Appendix B, **C9 — Berth presence**, specifies the mooring, mandatory
claim, daemon lease, heartbeat, refusal and exit cleanup; cargo admission remains separate
under C6.
Last detach starts only that root's idle timer; another attach clears it.
At 900 seconds retirement begins and its per-root record/snapshot are removed.
The resident, socket, mooring and lease survive with zero roots
(`elohim/eprfs/epr-lsp/src/daemon.rs:601`,
`elohim/eprfs/epr-lsp/src/daemon.rs:816`,
`elohim/eprfs/epr-lsp/src/daemon.rs:748`,
`elohim/eprfs/epr-lsp/src/daemon.rs:688`).

**D20 — environment.** The resolver collects session-variable names from the runtime
registry. The child removes those variables, then receives
`ELOHIM_SESSION_ID=lsp-device` and `ELOHIM_SESSION_OWNER_PID=<mux-pid>`, so its
flychecks claim as the resident rather than an editor. This scrubs registered identity
variables, not arbitrary environment variables
(`elohim/eprfs/epr-cli/src/lsp.rs:363`,
`elohim/eprfs/epr-cli/src/lsp.rs:316`;
`elohim/eprfs/epr-lsp/src/server.rs:157`).

Pre-push skips the resident's rust-analyzers and still pauses strays. This landed on
`feat/gate-live` in 5bb980361: `ra_strays()` in `.husky/pre-push.bash` skips any
rust-analyzer whose `/proc/<pid>/environ` carries `ELOHIM_SESSION_ID=lsp-device`, and leaves
the yield of the resident's flychecks to cargo-disk-guard Rule 4 rather than to a lease.
The mux's child-spawn path establishes the ancestry the peer change must recognize
(`elohim/eprfs/epr-lsp/src/server.rs:259`).
Flycheck admission and Rule 4 yielding are authoritative in Appendix B, C6/C6b.

## 9. Later slices

Ark supervision is this slice's device-resident shape; only resource enforcement stays
later. The foreground mux and readiness line exist
(`elohim/eprfs/epr-cli/src/lsp.rs:349`).
The declaration/supervisor integration gap is PENDING in Verification, not moved to S5.

| Slice | Remaining scope |
|---|---|
| S2 | Measured RAM/I/O classes and bounds, with enforcement when available; today's native tier is `None` (`elohim/ark/supervisor/src/native.rs:83`). The immediate pre-push resident exception is the peer dependency in §8. |
| S3 | TypeScript server policy and persistent installation. |
| S4 | Python server policy, reusing attribution/delivery. |
| S5 | D10's plugin package kind/projection backend, broader language-server arrival health checks and DevspaceSeed mapping. |

Slice 1's implementation includes the `["lsp","serve"]` long-running declaration and
resident declaration; completion must verify their installation.
DevspaceSeed, lvi's declared arrival lifecycle, maps health checks and process readiness at
`elohim/lvi/docs/specs/2026-10-10-workspace-arrival-as-declared-device.md:73`.
That broader mapping does not postpone supervision of an own child.

## Decisions taken

**Design principles.** Editing works offline with one device, regardless of key custody;
source files survive loss of the analysis cache. Sessions already authorized for the same
checkout share cold startup and indexing, while parsing, per-edit analysis, identity checks
and cleanup still cost work (the gradient-reading vocabulary above). Paths, activity times
and attribution stay device-local; they grant no publication, access-control or gate authority.
The visible outcomes are diagnostics, explicit ambiguity or unknown ownership, deferred
flycheck naming its holder, and `no diagnostics: <reason>`. Background checks yield to builds.

- **Scope:** Rust slice 1 includes Rule 4 flycheck yield and D2 build-script admission. TypeScript,
  Python, RAM/I/O enforcement and broader arrival integration remain later; resident ark
  supervision is this slice's shape (§2, §9).
- **Mux:** our native implementation in `elohim/eprfs/epr-lsp`, exposed by `epr lsp` and a
  repo-scoped attach plugin. No replacement binary named `rust-analyzer` on PATH.
- **Lifetime:** a child starts on attach and retires after 900 idle seconds; its resident
  keeps the device mooring and lease (§2, §8).
- **Interfaces:** C1–C9 describe the resident contract. In particular, ownership considers all sessions in the
  window, and folds group by area **and** owner.

The approved scope, mux and lifetime are stated above; Appendix B, C5 fixes 900 seconds.

- **D1 — pin replacement:** a fresh attach plan replaces that root's child; its clients
  receive the pin-change notice and reconnect while the resident stays up (§2; C4).
- **D2 — Cargo configuration:** `cargo.targetDir: false` preserves the LSP slot; build scripts wait for admission with a 10-second poll and 30-minute limit (§7; Appendix B, C5/C6).
- **D3 — spec authority:** Appendix B is authoritative; `contracts.md` is the working copy retired at landing.
- **D4 — single flycheck:** the device-wide run lock enforces at most one
  flycheck/build-scripts run for all callers, including shared `lsp-device` and unleased
  callers. Foreground admission remains the berth lease plus contention scan and Rule 4
  yield. The same-session regression test covers serialization; a live concurrent-arrival
  probe under the real resident (two roots, one device) remains an acceptance item
  (§7; Appendix B, C6/C6b;
  `genesis/agentic/bin/lsp-flycheck:229 (run_lock / flock block)`,
  `genesis/agentic/lsp_flycheck_test.py:267 (same-session regression test)`).
- **D5 — document status:** keep implementation status and command results only in Verification.
- **D6 — document vocabulary:** define technical terms at first use with source-grounded glosses.
- **D7 — question closure:** close Open Question 3 through D2; retain live downstream delivery as a Verification check.
- **D8 — edit field:** select maximum `at`, breaking ties by lexicographically smaller session id; retain every candidate and let `ambiguous` alone determine ambiguity (§5; Appendix B, C7).
- **D9 — bounds:** 64 MiB frames, 1024-message/32 MiB queues, 5-second shutdown and
  2-second exit waits; no pending reverse-request bound (§4; C4).
- **D10 — plugin packaging:** slice 1 ships the hand-authored plugin under `.claude/marketplace/`; its Elohim package kind and projection backend are S5 package-governance debt, closing Open Question 1 for slice 1 (§3; §9).
- **D11 — initialize:** mux PID/root/policy, UTF-16 default and push diagnostics; full sync
  only for an existing capability (§4; `elohim/eprfs/epr-lsp/src/mux.rs:462`,
  `elohim/eprfs/epr-lsp/src/mux.rs:805`).
- **D12 — server-request handling:** local answers, registration/refresh broadcasts, progress drop and explicit
  refusal (§4; `elohim/eprfs/epr-lsp/src/mux.rs:825 (server_request)`).
- **D13 — disk:** initial open alone falls back to buffer when disk text cannot be read
  (§4; `elohim/eprfs/epr-lsp/src/mux.rs:549`).
- **D14 — C2:** logged authorship suppresses mirrored LSP ownership (§5;
  `elohim/eprfs/epr-lsp/src/mux.rs:713`).
- **D15 — flicker:** returning within one window of clear retains owners (§5;
  `elohim/eprfs/epr-lsp/src/attrib.rs:352`).
- **D16 — folds/data:** count errors/warnings; preserve data with the `epr` exception;
  unstamp marked code-action diagnostics (§5; `elohim/eprfs/epr-lsp/src/deliver.rs:289`,
  `elohim/eprfs/epr-lsp/src/deliver.rs:325`, `elohim/eprfs/epr-lsp/src/deliver.rs:361`).
- **D17 — sessions:** full-ID comparison, eight-character display (§5;
  `elohim/eprfs/epr-lsp/src/attrib.rs:47`).
- **D18 — slot:** Rust computation covers the checkout top (§7;
  `elohim/eprfs/epr-cli/src/lsp.rs:405`).
- **D19 — overrides:** root-relative program path, then checkout fallback (§7;
  `elohim/eprfs/epr-lsp/src/server.rs:188`).
- **D20 — environment:** scrub registered session variables; supply resident identity
  (§8; `elohim/eprfs/epr-lsp/src/server.rs:157`).
- **D21 — exit:** mux EOF or socket failure before client exit returns 1; client closure,
  observed client exit and client-stdio errors return 0 (§3;
  `elohim/eprfs/epr-lsp/src/attach.rs:37 (proxy)`,
  `elohim/eprfs/epr-cli/src/lsp.rs:233 (attach)`).
- **D22 — area:** receiver-independent manifest/file anchors, distinct from null notice
  (§5; `elohim/eprfs/epr-lsp/src/deliver.rs:73`,
  `elohim/eprfs/epr-lsp/src/attach.rs:149`).

## Open questions

1. **Plugin package ownership — closed for slice 1 by D10 (§3; §9).** Slice 1 ships the
   hand-authored marketplace, plugin manifest and `.lsp.json` under `.claude/marketplace/`.
   Its Elohim package kind and projection backend remain explicitly recorded S5 debt;
   they do not block slice 1. Schema and launch-path substitution follow §3; Verification
   includes a live nested `--root` check of the C5 wrapper path.
2. **Ambiguous presentation — closed by D8 (§5; Appendix B, C7).** `edit` displays the
   latest candidate edit in the window (maximum `at`), with ties going to the lexicographically
   smaller session id. `owner` retains all candidate keys and `data.epr.owners` lists all
   their display ids; `ambiguous` alone determines ambiguity. One representative edit does
   not turn multiple candidates into a sole owner.
3. **Rust coverage and admission — closed by D2 (§7; Appendix B, C5/C6).**
   `cargo.targetDir: false` leaves every invocation in the child's LSP slot. The build-scripts
   override runs a workspace-wide all-targets check from the Cargo workspace root, with
   bounded admission waiting; saved-file flycheck stays package-scoped. Live downstream
   diagnostic delivery is an acceptance check in Verification, not an unresolved design choice.
4. **Transport bounds — closed by D9 (§4; Appendix B, C4).** Frames, per-client queues,
   and child shutdown have explicit limits and overflow behavior; D12 server-request handling
   keeps no pending server→client routes. Boundary, slow-client and shutdown tests remain acceptance checks in Verification.

## Verification and outstanding evidence

### (a) Current acceptance checklist

This is the consolidated list of obligations in force for the resident Rust slice.
Historical observations and command results in (b) do not satisfy these obligations by
themselves; (c) names implementation qualifications and outstanding integration. S2 resource
enforcement and S5 package/arrival work remain later slices (§9).

**[G]** marks items carrying the frontmatter graduation trigger's explicit requirements:
owning gates/scenario, shared servers, C7 views, yielding, idle retirement with the resident
lease retained, and measured habit evidence/cite sealing. Where an item contains more,
the marker applies to those clauses. Unmarked items remain slice-1 acceptance obligations.

No item has recorded acceptance evidence yet; all 23 items are listed under (c) as PENDING.

#### Entry points and installation (1–3)

1. **[G] Owning checks.** Run the unit suites, fake LSP fixture, wrapper, cargo-disk-guard,
   berth-compatibility and native daemon suites, `just gate eprfs`, `just gate rust-style`,
   and the live `genesis/a2o/features/devflow/edit-diagnostics-share-one-server.feature`
   scenario with `@concern:edit-diagnostics-reach-every-session`. Record actual results;
   none of the obligations below is a claimed pass.
2. **Installation and root selection (§3, D10).** Trust the workspace and run
   `claude plugin install elohim-lsp@elohim-workspace --scope project` once there. Verify
   the enabled local plugin and project-level disabling of the user-scope Rust plugin;
   settings alone do not install it. Exercise a nested launch cwd and explicit nested
   `--root`; confirm the analyzed root and resolved C5 wrapper path. Exercise the shim's
   `EPR_BIN`, launcher, pool and PATH resolution branches. The executable launcher is
   an `exec` branch, so a launcher failure does not resume the later fallbacks
   (`genesis/agentic/bin/lsp-attach:15 (top-level resolver)`).
3. **Foreground entry and supervision (§2, §9).** Exercise today's foreground
   `epr lsp serve --device`, including rejection without `--device`, a refused berth
   claim and readiness only after bind/claim. Complete and verify the approved
   `epr device run epr --ready-line "epr-lsp: listening" --restart permanent -- lsp serve --device` integration,
   the `["lsp","serve"]` long-running declaration and `epr device supervise` dispatch.
   Prove installation as an own child with `Permanent` restart plus `StdoutLine` and
   `ExecutableIdentity` readiness. Native resource enforcement is later scope.

#### Resident lifetime and LSP transport (4–11)

4. **[G] Discovery and resident ownership (C1, C3, C4, C9, §8).** Prove one socket, lock and
   `toolchain:lsp` lease across a main checkout's roots/worktrees, with lazy children for
   distinct `(lang, canonical root)` keys. Test independent repositories' different
   runtime/socket paths subject to berth admission, duplicate-mux refusal and stale-socket
   recovery by the lock holder. Verify mux PID/start metadata, `root12`, per-root records
   and atomic snapshots; no per-root socket/log is implied. Check mandatory claim failure,
   best-effort mooring, 15-minute heartbeat/warnings and local socket access permissions.
5. **[G] Idle and resident shutdown (C9, §8, D9).** Last detach starts only that root's idle
   timer; reattach clears it. At 900 seconds retirement begins, then allow the bounded
   shutdown waits and loop ticks before asserting child death. Remove that root's record
   and snapshot while the resident/socket/mooring/lease survive, including at zero roots.
   Resident shutdown retires all children, removes the socket, releases the lease and
   unmoors; test these separately from idle-child retirement.
6. **Pin replacement (D1).** A fresh attach plan with a changed pin queues the specified
   notice to that root's clients, disconnects them, retires the old child and starts its
   replacement. Allow temporary overlap during retirement; prove existing clients can
   restart/reattach and other roots, the resident socket and lease stay live.
7. **Absence, failure and attach exits (C4, D21).** With the resident absent/refusing,
   verify the exact supervisor-down reason, open/close-only null initialization, type-2
   showMessage and one Information diagnostic on first open, or root `Cargo.toml` when
   change/save precedes open, with editing available and exit 0. Verify missing/invalid
   policy uses null fallback, child-spawn failure gives a showMessage and closes, and
   child death notifies/closes only its root. The Python fallback is
   `genesis/agentic/bin/lsp-null.py`, reached by `lsp-attach` when no resolver branch
   selects an executable (also on missing language): it publishes once at the first
   valid URI in an open/change/save notification
   (`genesis/agentic/bin/lsp-attach:9 (top-level argument check)`,
   `genesis/agentic/bin/lsp-attach:30 (top-level fallback)`,
   `genesis/agentic/bin/lsp-null.py:30 (main)`). Test mux EOF/socket failure before client
   exit returning 1, client closure/exit returning 0, and client-stdio I/O errors returning
   0 (`elohim/eprfs/epr-lsp/src/attach.rs:37 (proxy)`,
   `elohim/eprfs/epr-cli/src/lsp.rs:233 (attach)`). Missing snapshots must give a text
   no-server notice or JSON `servers:[]`.
8. **Transport and shutdown bounds (D9).** Test at and above 64 MiB frame bodies and
   1024-message/32 MiB client outbound queues. Check oversized-frame logging, slow-client
   notification/disconnection/reattachment and uninterrupted delivery to other clients;
   the slow-client warning can reach only a client that reads it. Test initialized-child
   shutdown: up to 5 seconds for reply, then exit and up to 2 seconds before kill;
   uninitialized children start at exit. Exercise the five-second hello deadline.
9. **Initialization and request routing (D11).** Verify mux PID, forced root URI/path
   and workspace folder, policy options, default UTF-16 and push diagnostics. Verify full
   sync only rewrites an existing capability and later clients receive the cached result.
   Test request-ID and cancellation remapping, original-client responses, local shutdown
   and per-transport detach cleanup.
10. **Server-request handling (D12).** Exercise the complete matrix: policy configuration,
    root workspace folders, progress-token acknowledgement, registration/unregistration
    broadcasts and late-client replay, `/refresh` acknowledgement/broadcast, discarded
    broadcast responses, `workspace/applyEdit` returning `applied:false`, and
    method-not-found for other requests. Check dropped server progress and client
    configuration changes, attributed diagnostics and other-notification broadcasts.
    There are no pending reverse routes to bound.
11. **Disk authority (D13).** Verify only first open/last close reach the child, disk text
    supplies shared full sync and versions advance through the mux counter. Initial open
    alone falls back to the buffer when UTF-8 disk text is unreadable; later unopened or
    unreadable-disk changes are dropped. Save refreshes open disk text when readable,
    strips client text and forwards. A later open may resync changed disk without
    assigning authorship (`elohim/eprfs/epr-lsp/src/mux.rs:549 (did_open)`).

#### Edit evidence and attribution views (12–15)

12. **C2 writer/reader contract (§5, Appendix B).** Test stable-inode writer serialization,
    bounded lock contention/drop, append/prune retention, unread-event loss and short/torn
    writes. Verify fail-open hook behavior within its configured two-second timeout,
    length-below-offset reset, buffered trailing bytes, malformed/unknown-line skipping,
    retained-event deduplication and reconnect/replay. Do not assert that concurrent
    pruning is lossless or that every rewrite is detected. Verify root-prefix filtering
    and the nested-worktree qualification; log privacy and non-deciding readers must hold.
13. **[G] Attribution identity and time (C1, C7, D8, D14, D15, D17).** Test full session IDs
    despite PID/start variations and short8 collisions; unresolved identity stays anonymous.
    Match diagnostics by publisher/URI and range/message/code; use every candidate in the
    inclusive window across the root, preserving ambiguity and unattributed states. Check
    D8's latest-edit/tie selection without dropping owners, C2 suppression of mirrored
    change/save authorship, saved-only fallback, ownership stability on unchanged publishes,
    clear/reappearance within one window, and new ownership after that retention expires.
14. **[G] Delivery, data and anchors (C7, D16, D22).** Test the union of open interest across
    transports, retained edit interest and named ownership. Verify per-area/full-owner-set
    folds, error/warning counts (absent severity counts as error), Information/hint exclusion,
    cleared folds and real diagnostics sharing anchors. Check prefixes, labels, every
    `data.epr` field, null/object/opaque data and code-action unstamp exceptions. Anchors
    must be receiver-independent: nearest package manifest, readable root manifest, then
    file/non-file URI. Compare delivered views with `show`'s `mine`, `caused-by-me`, `others`.
15. **[G] Live downstream delivery (§5).** In concurrent Claude sessions on
    `elohim/elohim-storage`, plant `let x: u32 = "s";` in A's file to prove `mine`, then
    cause a downstream error in B's file: A sees `caused-by-me`, B sees attribution to A,
    and B's unrelated error reaches A only as a fold. Cover the third-session and ambiguity
    variants in the fake fixture. Separately prove live rust-analyzer publishes the
    downstream error; package-scoped Cargo and fake-server routing cannot establish that.
    Count servers for the tested root/pin, rather than using a global process count alone.

#### Rust policy, admission and foreground yielding (16–19)

16. **Rust policy and environment (C5, D2, D18–D20).** Verify native 1.96.1, one Cargo job,
    the LSP slot in the Cargo child's environment, `cargo.targetDir:false`; cache priming
    disabled; proc macros enabled; check on save enabled; and both `overrideCommand`s present. Test WASM toolchain unsetting,
    checkout-top and nested/family slots, relative override resolution then checkout
    fallback, and unchanged bare names/`$saved_file`. Verify registered session-variable
    removal and resident identity injection. Preserve classless Cargo behavior with the
    `verify` class alongside `lsp`; no capacity change is authorized.
17. **[G] Saved-file admission and budget (C6, D4).** Verify the owning package and all-targets
    command, run-lock acquisition before the berth claim, claim/refusal/error paths,
    post-claim contention even with one of two Cargo slots occupied, lease and lock release
    before deferral, and exit-4 unleased contention checks with the run lock held.
    A refusal must compile nothing and its single spanless JSON warning must become visible
    to attached sessions. Run-lock serialization is covered by
    `test_a_second_run_from_the_same_session_defers_while_the_first_compiles`
    (`genesis/agentic/lsp_flycheck_test.py:267`). Still outstanding as live acceptance:
    a concurrent-arrival probe under the real resident with two roots on one device,
    checking serialization, shared-lease lifetime and visible deferral. Include distinct
    holders and standalone/store-failure unleased paths in acceptance coverage. Verify an
    intact resident identity does not take the no-session fallback. Test the compilation
    budget, timeout warning and cleanup on success, error and handled signals.
18. **[G] Build-script admission (C6, D2).** Verify one workspace-root all-targets invocation
    after contention clears; a held run lock, held leases, heavy processes and claim exit 3
    cause bounded waiting with the named blocker, not saved-file deferral. Verify release
    of any acquired lease and run lock before waiting. Test poll/wait overrides,
    non-zero waiting timeout without compilation, exit-4 contention checks and the same
    environment, lease-budget watchdog and cleanup as saved-file mode.
19. **[G] Rule 4 yielding (C6b).** Test both arrival orders. A foreground build already
    present blocks flycheck compilation; flycheck-first arrival exports the marker and
    allows an otherwise admitted foreground build to SIGTERM each deduplicated yieldable
    root once. Verify marked ancestry, direct/bash wrapper recognition, marker/wrapper and
    start-time revalidation, zombies/exits, non-yieldable-only and mixed sets, PID-count
    threshold and Node-launched gate runners. Denial by any existing guard rule must
    cancel nothing. Check the shared three-second deadline, confirmed-stop advisory,
    signal-error/still-stopping advisories, no Rule 4 SIGKILL escalation and no new denial
    from yield failure. Separately prove wrapper descendant cleanup and lease release;
    the timeout advisory is not proof they finished.

#### External delivery, governance and completion evidence (20–23)

20. **External feed (C8).** Test the in-process handle, source/URI set isolation, empty-set
    clearing, full-diagnostic source tags where stampable, opaque-data preservation and
    untagged folds. Verify root paths must match, missing-root sets are dropped without
    spawning, and the bool reports enqueue success only. Prove the `gatefeed`
    producer (landing on `feat/gate-live`) uses the same attribution/delivery path.
21. **Governance and observability (§6, Appendix A).** Run native governance evaluation
    and exercise the required evidence paths for every applicable Appendix A criterion:
    CC-0, CC-3, CC-4, CC-5, CC-6a, CC-6b, CC-7, CC-8, CC-10, CC-11 and CC-14. Compare each
    result with that row's stated requirement; record the exercised path, expected and
    actual outcome, command/exit code and evidence location. Include typed reason counters
    for success, refusal and failure, skipped-line counts, census/sample semantics, bounded
    fallback and backpressure, and witnessed residual failures with a recorded disposition.
    Prove private edit/snapshot state never enters network projection, grants authority or
    decides gates/reputation. Missing instrumentation or execution remains PENDING.
22. **Measurement (§1).** Collect post-warm-up RSS using the repeat procedure and
    cwd-selection caveat in
    `genesis/local-dev/shared-toolchain-servers/evidence-2026-10-26-live-ra.md`. Check the
    reported live-session count and Python-server absence with a process census; retain
    the historical sample's timing and pressure-report limitations in (b).
23. **[G] Completion artifacts (D3, D5–D7, §9).** Keep implementation status/results here.
    Verify the arrival-spec backlink when integration lands, retire the working
    `contracts.md` at landing and retain S5 package-governance debt without reopening D10.
    The habit `genesis/agentic/.epr-meta/edit-diagnostics-reach-every-session.habit.md`
    becomes red when runnable, green only with evidence; completion needs its measured
    DELTA, `.claude/scripts/habits-project.py` projection and a passing
    `.claude/scripts/habits-project.py --check`, plus cite sealing. Its successor remains
    DevspaceSeed arrival with servers as own children under ark.

### (b) Historical evidence (pre-resident, unchanged)

The following observations and command results are preserved verbatim. They record earlier
work; the current resident obligations are exclusively the checklist in (a).

The current user configuration enables `rust-analyzer-lsp@claude-plugins-official`
(`/projects/.claude-config/settings.json:3`). Its marketplace entry launches
`rust-analyzer` directly, with an extension mapping and no arguments or environment overrides
(`/projects/.claude-config/plugins/marketplaces/claude-plugins-official/.claude-plugin/marketplace.json:3336`).
Each session can therefore launch an independent server.

The recorded failure is competition for memory, compiler work and disk:

| Observation | Verified evidence and limits |
|---|---|
| Several rust-analyzers accompanied a workspace at 77–82% committed memory; the RAM guard killed a gate's clippy or test step at 80%. | `genesis/docs/superpowers/plans/task-placement-c4-report.md:175`. These are that report's observations, not a new measurement or an allocation of all pressure to rust-analyzer. |
| A session's server ran `cargo check --workspace` into `elohim/elohim-storage/target` on unpinned stable. | `/projects/.claude-config/projects/-projects-elohim/memory/manual-cargo-in-this-workspace.md:13`, an observation dated 2026-10-06. |
| The heavy-build guard refuses competing work. | The one-build-at-a-time rule (Rule 4 in cargo-disk-guard), `.claude/hooks/cargo-disk-guard.py`, is stated in §7. Its heavy-process limit and berth's current `cargo` capacity of 2 (`genesis/agentic/pool-policy.json:136`) are different controls. |
| A plain `rust-analyzer` process falls through to `unknown`, which the RAM guard never sheds. | `genesis/agentic/bin/ram-guard:238` and `genesis/agentic/bin/ram-guard:271`. Its Cargo children can still classify as compiler work. |
| Pre-push finds processes by a broad name match, stops them, then resumes them on exit. | `.husky/pre-push.bash:946`, `.husky/pre-push.bash:951`, `.husky/pre-push.bash:986`. SIGSTOP is not proof that resident memory was freed. |

The initial report counted seven plugin-use markers for three live sessions; that count
remains an unverified historical observation. A marker contains a PID and
process start time, for example
`/projects/.claude-config/plugins/cache/claude-plugins-official/rust-analyzer-lsp/1.0.0/.in_use/221727:1`.
A marker is not a live-server census; the historical three-session count was not independently
verified. TypeScript is disabled in `.claude/settings.json:444`; the inspected diff for
commit `0743a1278` confirms that switch without explaining it. Python's reported lack of a
server is also unverified; no live Python-server census was taken.

The live observation on 2026-10-26 recorded rust-analyzer PID **336405**, parent `claude`,
at **2156 MB RSS and 2156 MB VmHWM**, with an elapsed time of **1m48s**. This is **one sample
taken before the server finished warming up**, not a steady-state measurement or a memory
bound (`genesis/local-dev/shared-toolchain-servers/evidence-2026-10-26-live-ra.md:8`).
Rust edits in `.worktrees/shared-toolchain-servers` triggered the parent Claude process's
user-scope server, whose cwd was `/projects/elohim` and whose toolchain was
`stable-x86_64-unknown-linux-gnu`, not 1.96.1
(`genesis/local-dev/shared-toolchain-servers/evidence-2026-10-26-live-ra.md:3`). It analyzed the
main checkout rather than the edited worktree. Errors in `content_safety_integration.rs` and
`phase4_e2e.rs` (`gate_client` test helpers cfg'd out), plus warnings in `stack.rs`,
`verification.rs`, `config.rs`, `consensus.rs` and `subsidiarity.rs`, reached the orchestrator
without attribution, newness or in-flight status; the orchestrator had touched none of those
crates (`genesis/local-dev/shared-toolchain-servers/evidence-2026-10-26-live-ra.md:9`). This is
direct evidence for C7's others' and unattributed views.

Repeat-measurement script and cwd-selection caveat: `genesis/local-dev/shared-toolchain-servers/evidence-2026-10-26-live-ra.md`, “Repeat measurement procedure”.

Recorded command results:

- `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest genesis.agentic.lsp_flycheck_test`:
  **13 passed, exit 0 (2026-10-26)**. Stub Cargo and real berth cover held-lease waiting
  followed by one workspace-root build-script invocation, non-zero timeout without
  compilation, heavy-process waiting, environment/slot selection, Cargo failure cleanup
  and saved-file flycheck. This does not exercise live rust-analyzer.
- `python3 -c 'import json;json.load(open("genesis/agentic/pool-policy.json"))'`:
  **exit 0 (2026-10-26)**.
- `bash -n genesis/agentic/bin/lsp-flycheck`: **exit 0 (2026-10-26)**.
- `epr flow cites seal genesis/docs/superpowers/specs/2026-10-26-shared-toolchain-servers-design.md`:
  **exit 0 (2026-10-26)**. Existing ID retained; zero `cites:` entries converted or stamped.
  This command checks frontmatter citations, not the inline prose references.

Historically, “ONE BUILD AT A TIME” counted all heavy-process PIDs without a yieldability filter.

Additional recorded limitations:

The native evaluator is unavailable; its pass
  and runtime acceptance remain unverified.

The main-checkout gate-live source is absent from this worktree.

### (c) Prompt/code differences and PENDING

- The declaration form (launch flags before `--`) and `epr device supervise` are supplied by
  `feat/gate-live` 06dd6c51c; this branch alone has neither (C4)
  (`elohim/eprfs/epr-cli/src/device.rs:143` here).
- Socket discovery is main-checkout/worktree scoped, not global across independent
  repositories (`elohim/eprfs/epr-cli/src/lsp.rs:187`). C2's filter is path-prefix based:
  sibling roots are excluded, but a nested worktree beneath the served root is not
  separately excluded (`elohim/eprfs/epr-lsp/src/mux.rs:313`).
- Server-request handling (D12) also answers workspace folders and acknowledges progress-token creation;
  applyEdit returns `applied:false` rather than a method error
  (`elohim/eprfs/epr-lsp/src/mux.rs:825 (server_request)`).
- The full sync rewrite affects only an existing capability (`elohim/eprfs/epr-lsp/src/mux.rs:788 (initialize_response)`).
- Initial-buffer fallback tests readable UTF-8 disk text, not file existence; later
  no-disk changes are dropped (`elohim/eprfs/epr-lsp/src/deliver.rs:46`;
  `elohim/eprfs/epr-lsp/src/mux.rs:549`, `elohim/eprfs/epr-lsp/src/mux.rs:620`).
- Object data replaces any preexisting `epr` key; null data is stamped; other non-object
  data is preserved and therefore lacks attribution/source tags. Unstamp without
  `data.epr` leaves even a mux prefix unchanged. Fold summaries have no source tag
  (`elohim/eprfs/epr-lsp/src/deliver.rs:289`,
  `elohim/eprfs/epr-lsp/src/deliver.rs:325`,
  `elohim/eprfs/epr-lsp/src/deliver.rs:236`).
- A null notice uses first open, but change/save before open anchors on root Cargo.toml
  (`elohim/eprfs/epr-lsp/src/attach.rs:149`).
- Idle expiry starts shutdown; child exit can follow the 5 s + 2 s waits and loop ticks.
  Pin retirement can overlap replacement
  (`elohim/eprfs/epr-lsp/src/daemon.rs:748`,
  `elohim/eprfs/epr-lsp/src/daemon.rs:569`;
  `elohim/eprfs/epr-lsp/src/limits.rs:21`).
- Attach returns 1 for mux EOF, socket reset or broken pipe before client exit;
  client closure/exit and client-stdio I/O errors return 0
  (`elohim/eprfs/epr-lsp/src/attach.rs:37 (proxy)`,
  `elohim/eprfs/epr-cli/src/lsp.rs:233 (attach)`).
- C8 is an in-process API. True means enqueued, even when a missing-root set is later
  dropped; the gate-live producer (`gatefeed`) is landing on `feat/gate-live`, not on this branch
  (`elohim/eprfs/epr-lsp/src/daemon.rs:195`,
  `elohim/eprfs/epr-lsp/src/daemon.rs:561`).
- The pre-push resident exemption landed on `feat/gate-live` (5bb980361, `ra_strays()`);
  this branch alone still has the unfiltered match (`.husky/pre-push.bash:948`).
- C2 serializes writers but does not coordinate pruning with readers; it cannot promise
  lossless retention or absence of torn lines (Appendix B, C2).
- Commit `97ea46685` adds the device-wide run lock before flycheck's berth claim, enforcing
  at most one flycheck/build-scripts run for all callers, including shared `lsp-device`
  and unleased callers. The same-session regression test is
  `test_a_second_run_from_the_same_session_defers_while_the_first_compiles`
  (`genesis/agentic/lsp_flycheck_test.py:267`). Foreground admission remains the berth lease
  plus contention scan and Rule 4 yield; compilation stays inside its lease budget.
  D4's live concurrent-arrival probe under the real resident (two roots, one device)
  remains an acceptance item (Appendix B, C6/C6b).
- Rule 4 also recognizes a `node`/`nodejs`-launched gate runner, using the first non-option
  argument as the script path (Appendix B, C6b).

**PENDING:** all acceptance items (a) 1–23; none has recorded acceptance evidence yet.
Outstanding integration includes: merge the device-floor integration supplying supervise dispatch (C4), prove the installed
declaration, restart and readiness under `epr device supervise`; the `gatefeed` producer landing; live idle/lease/pin and
attribution probes; D4 live concurrent-arrival acceptance under the real resident
(two roots, one device);
native gates and owning acceptance execution.

## Appendix A. Integrity zomes and concern-canon criteria

Integrity entry enum locations recorded for the classification are
`elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs:3672`,
`elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/lib.rs:892`,
`elohim/holochain/dna/infrastructure/zomes/infrastructure_integrity/src/lib.rs:227`,
`elohim/holochain/dna/mishpat/zomes/mishpat_integrity/src/lib.rs:309`, and both configurations at
`elohim/holochain/dna/node-registry/zomes/node_registry_integrity/src/lib.rs:220` and
`elohim/holochain/dna/node-registry/zomes/node_registry_integrity/src/lib.rs:421`.
No type is needed for this operational cache; entry-type capacity is not a reason to notarize
it. There is no integrity change, coordinator call, return hash or DNA-hash move.

Concern-canon criteria are separate from interface contracts C1–C9. The statements come from
`.claude/epr-meta/concerns.yaml:54` onward and the enforcement rows at
`.claude/epr-meta/policies.yaml:688` and `.claude/epr-meta/policies.yaml:777`.
Each applicable row states its requirement as a close paraphrase of the named canon
statement, followed by the evidence required here. CC-6a uses the active policy version 2,
not the superseded concern row. Verification (a), item 21 applies these requirements;
their verification status is recorded in (c).

| Concern | Criterion requirement and source | Applicability and required evidence |
|---|---|---|
| CC-0 plane location | Before binding a concern, name custody/availability, reach/audience or head/version and distinguish truth from projection; a cure on the wrong plane is a defect (`.claude/epr-meta/concerns.yaml:60`, `c0-plane-location@1`). | Record the plane and truth/projection classification of each decision path; device-local observation is explicit, and prove it never enters network projection. |
| CC-1 anti-self-election; CC-2 monotonic authority | n-a here. | n-a: no head election or authority upgrade. Temporal ownership is advisory, including D15's short retention after clear (`elohim/eprfs/epr-lsp/src/attrib.rs:352`). |
| CC-3 liveness | Every reachable non-terminal state must enable at least one automated transition; a human action or deploy as the only exit is a liveness hole (`.claude/epr-meta/concerns.yaml:145`, `c3-liveness@1`). | Exercise bounded attach, fallback and reap with a dead child or absent binary; identify the enabled automated transition from each non-terminal state, including combined refusals. |
| CC-4 honest absence | Absent, unreachable, refused and unverifiable must remain distinct answers; unreadable or unmeasured must never mean zero or complete (`.claude/epr-meta/concerns.yaml:185`, `c4-honest-absence@1`). | Test dead child, absent binary, refused attach, and missing/unreadable snapshot paths; record each reason and identify any collapsed or unmeasured outcome as an evidence gap. |
| CC-5 evidence-not-authority | A transferred claim confers only what the receiver re-derives, including action signature and entry/action binding where relevant; unevaluable evidence confers nothing and neither hardens nor softens authority (`.claude/epr-meta/concerns.yaml:227`, `c5-evidence-not-authority@1`). | Attribution grants no authority; test that unavailable evidence remains unattributed and that local edit/snapshot evidence cannot decide gates or reputation. |
| CC-6a bounded work | Every loop, sweep and retry ladder, including retries against uncancellable calls, must provably respect a declared budget; new pacing must reuse the existing drain kernel (`.claude/epr-meta/policies.yaml:783`, `c6a-bounded-work@2`). | C4 has no daemon-spawn wait; test the resident's 5-second hello deadline (`elohim/eprfs/epr-lsp/src/daemon.rs:1009 (connection)`). Exercise D9's frame, client-queue and child-retirement bounds; server-request handling (D12) has no pending reverse routes. Verify hook/log/flycheck bounds in C2/C6 and berth-call deadlines in C9. |
| CC-6b idempotent effect | Replaying a decision against settled state must mint nothing and double-apply nothing, while reconciliation stays eager; quiescence concerns effect, not effort (`.claude/epr-meta/concerns.yaml:313`, `c6b-idempotent-effect@1`). | Repeated publish preserves ownership and counts; test reconnect and log replay. |
| CC-7 advertise/serve symmetry | What a surface advertises must equal what it serves, including capabilities, inventory, validators and coverage (`.claude/epr-meta/concerns.yaml:349`, `c7-advertise-serve-symmetry@1`). | Daemon configuration and cached capabilities must match; test incompatible attach and missing server. |
| CC-8 observability per decision | Every outcome must increment a labeled counter through an enum-derived typed reason; count failures beside successes, use no structurally constant labels, and declare every meter as census or sample (`.claude/epr-meta/concerns.yaml:385`, `c8-observability-per-decision@1`). | Exercise C3/C4/C9 decision outcomes and inspect counter deltas, reason types and meter semantics for success, refusal and failure; missing counters remain an evidence gap. |
| CC-9 identity-lineage continuity | n-a here. | n-a for cryptographic re-keying; C1 carries PID/start metadata but attribution compares full session IDs. Test ID-based ownership, not PID-incarnation isolation (`elohim/eprfs/epr-lsp/src/attrib.rs:47`). |
| CC-10 contract evolution | A wire/config/schema change must be rejected or observed, never silently defaulted: an unknown or renamed field must fail parsing or increment a counter (`.claude/epr-meta/concerns.yaml:459`, `c10-contract-evolution-honesty@1`). | C2 skips unknown lines as contracted; inject malformed/unknown lines and record skipped-line counts rather than silently treating them as accepted events. |
| CC-11 imposed backpressure | Under externally chosen load, degrade by a declared, counted policy with a reason: defer with retry-after, shed or decline, never unbounded queueing or OOM (`.claude/epr-meta/concerns.yaml:493`, `c11-externally-imposed-backpressure@1`). | Defer saved-file flycheck without a queue; build scripts wait within D2's bound. Test D9's bounded delivery and slow-client disconnection/reconnection; record reason-counter deltas for deferral, waiting and disconnection. |
| CC-12 consent/authorization | n-a for notarized grants here. | n-a for notarized grants: no remote action or disclosure widening. Local socket permissions must respect the device's existing access boundary. |
| CC-13 graduated authority | n-a here. | n-a: this is not an authority scaffold or a social-tier bypass. |
| CC-14 witnessed residual | Every decision needs a witnessed residual arm: capture unclassified failures with inputs digest, decision state and relevant clocks/lags/peer states for root-cause analysis, deliver the evidence to the developer-facing report, and record a disposition of cure, declared variance or new concern class (`.claude/epr-meta/concerns.yaml:603`, `c14-witnessed-residual@1`). | Inject an unclassified failure and trace its context through local diagnostics/logs to a local developer-facing report and disposition; never export private edit events to satisfy an observability convention. |

Slice 1 registers its decision points and contract tests following
the birth rule in `.claude/skills/p2p-design-gate/SKILL.md`, “Concern-Canon Answer”.

## Appendix B. Interface contracts

**D10 — plugin packaging boundary.** Slice 1 ships the hand-authored marketplace and
`elohim-lsp` plugin under `.claude/marketplace/`. Its Elohim package kind and projection
backend are S5 package-governance debt, outside slice 1; Open Question 1 is closed for
slice 1 on that basis.

## C1. Session identity

Attach uses the shared session resolver, carrying
`SessionKey { session, owner_pid, owner_start_time }`; unresolved identity is
`anon-<pid>`. Ownership compares full session ID only; PID/start are metadata and
`short8` is the first eight characters (D17)
(`elohim/eprfs/epr-cli/src/lsp.rs:209`;
`elohim/eprfs/epr-lsp/src/attrib.rs:19`,
`elohim/eprfs/epr-lsp/src/attrib.rs:47`,
`elohim/eprfs/epr-lsp/src/attrib.rs:58`).

## C2. Edit-event log (Ephemeral C, device-local; read only by non-deciding readers: the mux and gate-live — amended 2026-10-26)

Path: `<main checkout>/.eprfs/status/lsp/edits.jsonl` (resolve the main checkout via
`git rev-parse --git-common-dir`, as gate-live.mjs `liveRoot` does, so worktrees share it).
One JSON object per line:
`{"v":1,"at":"<RFC3339 UTC ms>","session":"<id>","ownerPid":<int>,"ownerStartTime":"<str>","path":"<absolute path>","tool":"Edit|Write|MultiEdit|NotebookEdit"}`

**Writer guarantee and limits.** `append_event` opens the log with `O_APPEND` and takes
an exclusive advisory `flock`, retrying every 5 ms for up to 150 ms; contention past that
budget drops this event. While holding the lock it submits one `os.write()` for the JSON
line plus newline. If the file exceeds 2 MiB, it reads the last 10,000 lines, truncates the
same inode to zero, and writes that tail before closing the descriptor and releasing the
lock (`.claude/hooks/lsp-edit-event.py:60 (append_event)`).

For cooperating writers, the lock serializes append and prune, and keeping the inode
stable prevents an append from landing in an unlinked file. With complete successful
writes, pruning retains the last 10,000 lines; it is not a hard 2 MiB size cap. There is
no reader acknowledgement or retention watermark: **a concurrent prune can discard a
line a reader has not read yet**. Readers take no lock and may see the truncate/rewrite
interval. **Torn lines are possible**: write return lengths are not checked or retried,
and a short write or interruption can leave partial bytes; no `fsync` or crash-durability
guarantee is provided. Hook errors are swallowed, so neither recording every edit nor
lossless delivery is guaranteed (`.claude/hooks/lsp-edit-event.py:60 (append_event)`,
`.claude/hooks/lsp-edit-event.py:87 (main)`).

**Reader behavior.** The mux tails from a byte offset without locking; it resets the
offset and partial buffer only when file length is less than that offset. A same-length
or larger rewrite can therefore go unnoticed. It buffers trailing bytes until a newline,
skips malformed/unknown-version or invalid-UTF-8 completed lines, and deduplicates retained
event identities by timestamp, full session ID and path. The root mux then filters by path
prefix; sibling-worktree edits are excluded, while a nested worktree beneath the served
root is not separately excluded
(`elohim/eprfs/epr-lsp/src/daemon.rs:267 (EditTail::poll)`,
`elohim/eprfs/epr-lsp/src/attrib.rs:111 (parse_edit_line)`,
`elohim/eprfs/epr-lsp/src/attrib.rs:139 (EditLines::feed)`,
`elohim/eprfs/epr-lsp/src/attrib.rs:271 (Attributor::record_edit)`,
`elohim/eprfs/epr-lsp/src/attrib.rs:399 (Attributor::prune)`,
`elohim/eprfs/epr-lsp/src/mux.rs:277 (Mux::handle)`).

## C3. Runtime directory

`<main checkout>/.eprfs/status/lsp/` is derived from Git's absolute common directory,
so worktrees share it (`elohim/eprfs/epr-cli/src/lsp.rs:187`).

- `device.sock`: one socket; `device.lock`: exclusive nonblocking flock for the mux's
  life. Only the lock holder removes a stale socket before bind
  (`elohim/eprfs/epr-lsp/src/socket.rs:40`,
  `elohim/eprfs/epr-lsp/src/socket.rs:131`;
  `elohim/eprfs/epr-lsp/src/daemon.rs:320`).
- `<lang>-<root12>.json`: per-root record
  `{"pid","startTime","root","lang","toolchain","targetDir","startedAt","sock"}`.
  PID/start identify the mux; sock is the device socket. Written atomically on child
  start; removed when the root closes
  (`elohim/eprfs/epr-lsp/src/socket.rs:100`;
  `elohim/eprfs/epr-lsp/src/daemon.rs:641`,
  `elohim/eprfs/epr-lsp/src/daemon.rs:688`).
- `attrib-<lang>-<root12>.json`: per-root snapshot, atomically rewritten after state
  changes including publishes; removed on root closure
  (`elohim/eprfs/epr-lsp/src/socket.rs:92`;
  `elohim/eprfs/epr-lsp/src/daemon.rs:877`,
  `elohim/eprfs/epr-lsp/src/daemon.rs:688`).
- `edits.jsonl`: shared C2 log (`elohim/eprfs/epr-lsp/src/socket.rs:52`).

`root12` is the first 12 SHA-256 hex digits of the canonical absolute root
(`elohim/eprfs/epr-lsp/src/socket.rs:28`).
No per-root socket/log or separate device registry JSON is created; stderr is inherited
(`elohim/eprfs/epr-lsp/src/socket.rs:38`;
`elohim/eprfs/epr-lsp/src/server.rs:259`).
This implementation discovers one socket across a main checkout and its worktrees, not
automatically across unrelated repositories.

## C4. CLI

- `epr lsp attach <lang> [--root <dir>]`: connect to the resident, never spawn it.
  Root defaults to canonical checkout around cwd, or cwd outside Git
  (`elohim/eprfs/epr-cli/src/lsp.rs:163`,
  `elohim/eprfs/epr-cli/src/lsp.rs:233`).
- `epr lsp serve --device [--root <dir>]`: foreground mux; berth claim precedes the
  flushed `epr-lsp: listening <socket>` line. Serve without `--device` is rejected.
  Idle roots retire without ending it (`elohim/eprfs/epr-cli/src/lsp.rs:349`;
  `elohim/eprfs/epr-lsp/src/daemon.rs:748`).
- `epr lsp show [<lang>] [--root <dir>] [--session <id>] [--json]`: snapshot views
  `mine`, `caused-by-me`, `others`; no state gives a text no-server notice or JSON
  `servers:[]` (`elohim/eprfs/epr-cli/src/lsp.rs:667`).

Hello is
`EPR-LSP-HELLO {"v":1,"session":…,"ownerPid":…,"ownerStartTime":…,"root":…,"lang":…}\n`,
followed by raw frames; there is no drain prelude
(`elohim/eprfs/epr-lsp/src/socket.rs:183`,
`elohim/eprfs/epr-lsp/src/socket.rs:200`).
An unavailable resident yields
`no diagnostics: device supervisor not running — epr device supervise`.
The null server initializes open/close-only sync, sends a type-2 showMessage and one
Information diagnostic on first open (root manifest if change/save arrives first).
Missing resident alone returns 0
(`elohim/eprfs/epr-cli/src/lsp.rs:38 (SUPERVISOR_DOWN)`,
`elohim/eprfs/epr-cli/src/lsp.rs:233 (attach)`;
`elohim/eprfs/epr-lsp/src/attach.rs:117 (null_server)`).
The message's remedy is supplied by the device-floor integration branch `feat/gate-live`,
commit `06dd6c51c`, into which this branch merges before landing: that commit dispatches
`epr device supervise` and runs one ark per data root
(`elohim/eprfs/epr-cli/src/device.rs:149 (run)` and
`elohim/eprfs/epr-cli/src/device/supervise.rs:96 (run)`, both at `06dd6c51c`).
On this branch alone the named command is absent
(`elohim/eprfs/epr-cli/src/device.rs:143 (run)`); use foreground
`epr lsp serve --device` as the interim start
(`elohim/eprfs/epr-cli/src/lsp.rs:349 (serve)`).

**D21 — exit.** `ServerClosed` means mux EOF before client exit, or a mux socket failure
such as a reset or broken pipe before client exit; attach returns 1 so the editor can
restart it. Client exit/EOF and client-stdio I/O errors return 0
(`elohim/eprfs/epr-lsp/src/attach.rs:37 (proxy)`,
`elohim/eprfs/epr-cli/src/lsp.rs:233 (attach)`).

**D11 — full sync.** The cached initialize response rewrites an existing
`textDocumentSync` to `TextDocumentSyncKind.Full` (value 1): object form gets `change:1`,
otherwise the value becomes `1`; an absent capability stays absent. Later clients receive
that cached response (`elohim/eprfs/epr-lsp/src/mux.rs:788 (initialize_response)`,
`elohim/eprfs/epr-lsp/src/mux.rs:426 (client_initialize)`).

**D1 — replacement.** Compare the running root pin to the new attach plan. Mismatch
notifies that root's clients with
`toolchain pin changed <old> → <new>; restarting`, closes them and replaces the child.
Retirement may overlap the new generation; there is no resident restart or lease release
(`elohim/eprfs/epr-lsp/src/daemon.rs:569`,
`elohim/eprfs/epr-lsp/src/daemon.rs:673`;
`elohim/eprfs/epr-lsp/src/mux.rs:1085`).

**D9 — bounds.** Frame body **64 MiB**; queue **1024 messages / 32 MiB**; overflow queues
`epr lsp: too slow; reconnecting` and disconnects.
Initialized-child shutdown waits **5 s**, then exit waits **2 s** before kill;
uninitialized children skip to exit
(`elohim/eprfs/epr-lsp/src/limits.rs:12`,
`elohim/eprfs/epr-lsp/src/limits.rs:15`,
`elohim/eprfs/epr-lsp/src/limits.rs:18`,
`elohim/eprfs/epr-lsp/src/limits.rs:21`,
`elohim/eprfs/epr-lsp/src/limits.rs:24`,
`elohim/eprfs/epr-lsp/src/limits.rs:27`;
`elohim/eprfs/epr-lsp/src/daemon.rs:707`).
There is no pending reverse-request limit: D12 server-request handling (§4) answers/refuses
original requests locally and discards broadcast replies (`elohim/eprfs/epr-lsp/src/mux.rs:359 (on_client)`,
`elohim/eprfs/epr-lsp/src/mux.rs:825 (server_request)`).

## C5. Policy (genesis/agentic/pool-policy.json)

These are JSON extracts from the existing policy, not a replacement policy file. The
first selects its top-level `lsp` member, including literal `$saved_file`
(`genesis/agentic/pool-policy.json:110 (lsp)`):

```json
{
  "lsp": {
    "idle_ttl_s": 900,
    "attribution_window_ms": 30000,
    "servers": {
      "rust": {
        "command": ["rust-analyzer"],
        "env": {"RUSTUP_TOOLCHAIN": "1.96.1", "CARGO_BUILD_JOBS": "1"},
        "env_wasm_roots": {"RUSTUP_TOOLCHAIN": null},
        "target_slot": "lsp",
        "initialization_options": {
          "cargo": {
            "targetDir": false,
            "buildScripts": {
              "overrideCommand": ["genesis/agentic/bin/lsp-flycheck", "rust", "--build-scripts"]
            }
          },
          "check": {
            "workspace": false,
            "overrideCommand": ["genesis/agentic/bin/lsp-flycheck", "rust", "$saved_file"]
          },
          "checkOnSave": true,
          "cachePriming": {"enable": false},
          "procMacro": {"enable": true}
        }
      }
    }
  }
}
```

`target_slot: "lsp"` is computed in Rust, including the checkout-top case (D18;
`elohim/eprfs/epr-cli/src/lsp.rs:405`).
Null environment values unset variables; roots beneath `elohim/holochain/dna/` receive
WASM overrides (`elohim/eprfs/epr-lsp/src/server.rs:139`,
`elohim/eprfs/epr-lsp/src/server.rs:270`).
Relative path programs in `overrideCommand` resolve against root if present, else
checkout; bare names and `$saved_file` stay unchanged (D19;
`elohim/eprfs/epr-lsp/src/server.rs:188`).
**D2 — target directory.** `cargo.targetDir: false` leaves the child's `CARGO_TARGET_DIR`
(the LSP slot) in control of every rust-analyzer Cargo invocation. Build scripts and
proc macros use the override in C5 and the admission behavior in C6.

The second extract is the complete `berth.classes.cargo` table
(`genesis/agentic/pool-policy.json:160 (berth.classes.cargo)`):

```json
{
  "verify": {},
  "lsp": {"default_ttl_s": 600, "max_ttl_s": 900}
}
```

The empty `verify` row preserves classless Cargo claims and their no-default-TTL behavior.
`DEFAULT_CLASS` is `verify`; `claim_preflight` supplies it before `class_spec` rejects any
class absent from an existing table, so an `lsp`-only table would break those callers
(`genesis/agentic/bin/berth:104 (DEFAULT_CLASS)`,
`genesis/agentic/bin/berth:856 (Berth.claim_preflight)`,
`genesis/agentic/bin/berth:789 (Berth.class_spec)`). The empty row adds neither a TTL default
nor a cap (`genesis/agentic/bin/berth:913 (Berth._claim_live)`). Cargo capacity remains 2
(`genesis/agentic/pool-policy.json:142 (berth.resources.cargo)`); other resource/class
semantics are unchanged.

## C6. Flycheck wrapper (genesis/agentic/bin/lsp-flycheck <lang> <saved_file>|--build-scripts)

1. Find the Cargo package owning `saved_file` (nearest Cargo.toml with `[package]`).
   Build-scripts mode instead takes the workspace manifest in its working directory
   (`genesis/agentic/bin/lsp-flycheck:16 (top-level metadata resolver)`).
2. Acquire D4's non-blocking device-wide run lock before requesting
   `berth claim cargo --class lsp --ttl 600 --note "lsp flycheck <pkg>"`.
   On success, resolve the wrapper's session holder and check `berth who cargo`, excluding
   that holder, plus `running_heavy`'s non-flycheck processes. Any competing holder or process
   blocks compilation even with spare capacity; release the acquired lease and run lock
   before deferring or waiting. A held run lock or claim exit 3 also blocks; exit 4 permits
   an unleased run with the lock held only after the same contention check; other claim
   errors exit non-zero. Saved-file mode responds to any admission block with
   one Cargo JSON message, then exits 0 without retrying:
   `{"reason":"compiler-message","package_id":"<pkg>","manifest_path":"<path>","target":{"kind":["lib"],"name":"<pkg>","src_path":"<saved_file>"},"message":{"message":"flycheck deferred: build slot held by <holder>","level":"warning","spans":[],"children":[],"code":null,"rendered":"flycheck deferred: build slot held by <holder>\n"}}`
   The placeholder names `another lsp flycheck (<path>)` for a held run lock,
   a blocking holder/PID or the claim refusal detail
   (`genesis/agentic/bin/lsp-flycheck:97 (busy_builds)`,
   `genesis/agentic/bin/lsp-flycheck:132 (defer)`,
   `genesis/agentic/bin/lsp-flycheck:173 (release_lease)`,
   `genesis/agentic/bin/lsp-flycheck:237 (top-level admission loop)`).
3. After admission run `cargo check --message-format=json -p <pkg> --all-targets` with
   C5's environment, the LSP slot and `ELOHIM_LSP_FLYCHECK=1`. Cleanup terminates marked
   descendants, waits for its driver and releases a claimed lease on exit, failure or
   handled signals (`genesis/agentic/bin/lsp-flycheck:181 (cleanup)`,
   `genesis/agentic/bin/lsp-flycheck:283 (top-level Cargo launch)`).

**D2 — build scripts.** `lsp-flycheck rust --build-scripts` runs from the Cargo workspace
root with `cargo check --workspace --message-format=json --all-targets`, the same environment
and LSP slot, and the note `lsp build-scripts <workspace>`. Instead of saved-file deferral,
it waits on a held run lock, claim exit 3 or post-claim contention, polling every 10 seconds
for up to 30 minutes. Any acquired lease and run lock are released before waiting. Exit 4
retains the lock and the same contention check; waiting names the lock path, holder/process
or claim refusal detail on stderr, and timeout exits non-zero. `LSP_FLYCHECK_POLL_S` and
`LSP_FLYCHECK_WAIT_MAX_S` override these durations
(`genesis/agentic/bin/lsp-flycheck:156 (wait_or_defer)`,
`genesis/agentic/bin/lsp-flycheck:237 (top-level admission loop)`).

Both modes enforce the remaining compilation budget with `timeout`: TTL 600 seconds minus
a 30-second margin and admission time, then at most 10 seconds between TERM and KILL.
No remaining budget, or timeout status 124/137, emits `flycheck exceeded its lease; stopped`
and exits 0 through cleanup. `LSP_FLYCHECK_TTL_S` / `LSP_FLYCHECK_MARGIN_S` override the
budget, requiring `10 < margin < TTL`
(`genesis/agentic/bin/lsp-flycheck:83 (top-level budget validation)`,
`genesis/agentic/bin/lsp-flycheck:272 (top-level budget and Cargo launch)`).

**D4 — single-run safety and live acceptance.** The wrapper opens
`<main checkout>/.eprfs/status/lsp/flycheck.lock` as `run_lock` and takes a non-blocking
`flock -n` before the berth claim. The main checkout is the dirname of
`git rev-parse --path-format=absolute --git-common-dir`, so all worktrees share this lock.
It is released before every wait or defer and held for the run because Cargo inherits
the descriptor. This enforces at most one flycheck or build-scripts run per device for
every caller: distinct holders, children sharing `lsp-device`, and unleased exit-4 callers
(`genesis/agentic/bin/lsp-flycheck:229 (run_lock / flock block)`). A save that finds it held
emits `flycheck deferred: build slot held by another lsp flycheck (<path>)`.

The regression test
`test_a_second_run_from_the_same_session_defers_while_the_first_compiles`
(`genesis/agentic/lsp_flycheck_test.py:267`) starts one check, waits for its Cargo process,
then asserts that a second check with the same session emits the lock deferral and never
starts Cargo. It also checks the first run's completion and lease release. The lock
prevents a second resident check from renewing or releasing the first check's shared
lease; unleased callers retain the same mutual exclusion independently of berth.

Admission against foreground builds is unchanged: the berth lease plus the post-claim
contention scan excludes the wrapper's own holder and marked flychecks, but blocks on
other holders and non-flycheck heavy processes even with spare capacity. Exit 4 keeps
this scan while holding the run lock. Rule 4 yields to an admitted foreground build
(`genesis/agentic/bin/lsp-flycheck:97 (busy_builds)`,
`genesis/agentic/bin/lsp-flycheck:237 (top-level admission loop)`; C6b).

A live concurrent-arrival probe under the real resident, with two roots on one device,
remains outstanding as an acceptance item: verify serialization, shared-lease lifetime
and delivery of the deferral to attached sessions. The run-lock implementation and its
same-session regression test establish the safety mechanism; this probe exercises the
resident integration.

### C6b — cargo-disk-guard Rule 4 yield

For a heavy Cargo command or `just gate` recognized by the hook, after its DNA/WASM
exemption, **steps 1–4 below are the whole of Rule 4's predicate and yield sequence**.
Rules 1–3 already exist and are outside this spec:

- Rule 1 — disk hard ceiling: deny heavy work when `/projects` usage reaches `volume_hard_pct` (`.claude/hooks/cargo-disk-guard.py:470 (main)`).
- Rule 2 — pool discipline: for a recognized native Cargo workspace, deny when the command text lacks `CARGO_TARGET_DIR`; `just gate` is exempt (`.claude/hooks/cargo-disk-guard.py:373 (native_ws_for)`, `.claude/hooks/cargo-disk-guard.py:470 (main)`).
- Rule 3 — host I/O pressure: deny when `full avg10` or `some avg60` reaches its policy threshold (`.claude/hooks/cargo-disk-guard.py:140 (host_psi_io)`, `.claude/hooks/cargo-disk-guard.py:470 (main)`).

Rule 5 (jobs cap) also already exists outside this spec and must pass before yielding
(`.claude/hooks/cargo-disk-guard.py:280 (has_jobs_cap)`,
`.claude/hooks/cargo-disk-guard.py:470 (main)`).

1. Collect heavy processes from `/proc`, excluding only the hook's own PID and its direct
   parent PID. Match the basename of the first command-line argument: `cargo` with a heavy
   verb (the first argument after any `+toolchain` selector), or `rustc`, `rust-lld`,
   `ld.lld`, `cc1`, or `gate-runner.mjs`. Heavy verbs are `build`, `b`, `test`, `t`, `check`,
   `c`, `clippy`, `run`, `r`, `nextest`, `bench`, `doc`, and `install`. For `node` or
   `nodejs`, also match when the first subsequent non-option argument's basename is
   `gate-runner.mjs` (`.claude/hooks/cargo-disk-guard.py:251 (running_heavy)`).
2. A collected process is yieldable if and only if its environment contains the exact
   entry `ELOHIM_LSP_FLYCHECK=1`; an unreadable environment makes it non-yieldable
   (`.claude/hooks/cargo-disk-guard.py:168 (flycheck_marked)`).
3. Let N be the number of distinct non-yieldable PIDs and M be
   `io.max_concurrent_heavy` from `genesis/agentic/pool-policy.json` (default 1). Rule 4
   denies exactly when **N > 0 and N ≥ M**; otherwise it allows. This test occurs before
   any yield signal. The hook also requires the disk-ceiling, pool, I/O-pressure and jobs-cap
   rules to pass before cancelling flychecks; a denied command cancels none
   (`.claude/hooks/cargo-disk-guard.py:470 (main)`).
4. For an admitted command, deduplicate yieldable process-tree roots by walking upward
   through consecutive marked ancestors or an `lsp-flycheck` wrapper (including a wrapper
   invoked by `bash`, whose own environment need not expose the marker). Revalidate each
   root's marker or wrapper identity and process start time, skip exited or zombie roots,
   and send SIGTERM once to each remaining root. Wait for all signalled roots under one
   shared deadline of at most **3 s**. Emit `yielded: stopped lsp flycheck pid N for this build`
   only when that PID disappears, becomes a zombie or has a different start time. A signal
   failure other than `ProcessLookupError` emits
   `flycheck yield: could not stop pid N: <error>`; an already-gone PID is skipped; expiry emits
   `flycheck yield: pid N still stopping after 3 s`. These are advisories: neither a signal
   error nor a still-stopping root denies the admitted build, and this hook does not
   escalate to SIGKILL (`.claude/hooks/cargo-disk-guard.py:187 (flycheck_wrapper)`,
   `.claude/hooks/cargo-disk-guard.py:197 (flycheck_root)`,
   `.claude/hooks/cargo-disk-guard.py:212 (yield_flychecks)`).

The hook exempts commands/cwds matching its DNA/WASM markers before these rules; its
top-level exception handler fails open (`.claude/hooks/cargo-disk-guard.py:470 (main)`,
`.claude/hooks/cargo-disk-guard.py:575 (module entry point)`). Both flycheck and build-scripts
mode use the same marker and wrapper cleanup path: the wrapper SIGTERMs its marked
descendants, waits for its Cargo driver and releases its own lease
(`genesis/agentic/bin/lsp-flycheck:181 (cleanup)`,
`genesis/agentic/bin/lsp-flycheck:173 (release_lease)`). Non-yieldable heavy processes are
never signalled by Rule 4; its advisory timeout does not prove the wrapper has finished
cleanup or released its lease.

## C7. Diagnostic delivery (per session)

Full delivery joins current open URIs across a session's transports, retained edit-interest
URIs (C2 plus accepted LSP fallback edits), and diagnostics naming the full session ID
(`elohim/eprfs/epr-lsp/src/mux.rs:994`;
`elohim/eprfs/epr-lsp/src/deliver.rs:370`).

For each publisher/URI, match range/message/code. New diagnostics get every distinct
session ID with evidence in the inclusive window, counting edits anywhere in the served root,
not only to the diagnostic's own URI; zero candidates is unattributed, more than one is
ambiguous with all of them.
C2 suppresses change/save fallback stamps for that file within the window; sibling-worktree
C2 paths are ignored
(`elohim/eprfs/epr-lsp/src/attrib.rs:208`,
`elohim/eprfs/epr-lsp/src/attrib.rs:294`,
`elohim/eprfs/epr-lsp/src/attrib.rs:366`;
`elohim/eprfs/epr-lsp/src/mux.rs:313`,
`elohim/eprfs/epr-lsp/src/mux.rs:713`).

Record: `{"owner":[SessionKey…],"ambiguous":bool,"firstSeen":ts,"edit":{path,at}|null}`.
D8 chooses the latest edit, breaking time ties by lexicographically smaller full session
ID, without reducing owners. D15 keeps owners through a clear/reappearance within one
window after the clear
(`elohim/eprfs/epr-lsp/src/attrib.rs:170`,
`elohim/eprfs/epr-lsp/src/attrib.rs:352`,
`elohim/eprfs/epr-lsp/src/attrib.rs:366`).

Other errors/warnings fold per (area, full owner-ID set) at Information severity; absent
severity counts as error. D16 in §5 specifies fold text, clearing, data preservation and
the exact code-action unstamp exceptions
(`elohim/eprfs/epr-lsp/src/deliver.rs:221`,
`elohim/eprfs/epr-lsp/src/deliver.rs:289`,
`elohim/eprfs/epr-lsp/src/deliver.rs:325`,
`elohim/eprfs/epr-lsp/src/deliver.rs:361`,
`elohim/eprfs/epr-lsp/src/deliver.rs:447`).
The attribution payload is
`data.epr={"attribution":"mine|caused-by-me|other|ambiguous|unattributed","owners":[short8…]}`
where data permits stamping; non-null non-object data remains opaque.

D22: inside the root, the nearest package manifest anchors the area; otherwise use a
readable root manifest, else the file itself. The anchor is receiver-independent.
The null server's notice follows the separate first-open rule and pre-open fallback in C4
(`elohim/eprfs/epr-lsp/src/deliver.rs:73`,
`elohim/eprfs/epr-lsp/src/attach.rs:149`).

## C8. External diagnostic feed

An in-process producer obtains `Bound::external_feed()` and awaits
`ExternalFeed::publish(lang, root, source, uri, diagnostics) -> bool`.
This submits that source's full current set for the URI through C7's attribution and
delivery path. Sets are keyed by (source tag, URI); an external set never replaces the
server's own set. An empty set clears only that source's set
(`elohim/eprfs/epr-lsp/src/daemon.rs:353 (Bound::external_feed)`,
`elohim/eprfs/epr-lsp/src/daemon.rs:195 (ExternalFeed::publish)`;
`elohim/eprfs/epr-lsp/src/mux.rs:277 (Mux::handle)`;
`elohim/eprfs/epr-lsp/src/attrib.rs:319 (Attributor::publish)`).

A root with no running server drops the set; publishing starts no server.
The bool is enqueue success, not delivery or root existence: false means the event receiver
is gone. The supplied root is used as-is, so it must match the active root path
(`elohim/eprfs/epr-lsp/src/daemon.rs:195 (ExternalFeed::publish)`,
`elohim/eprfs/epr-lsp/src/daemon.rs:482 (Device::on_event)`).
Full diagnostics carry `data.epr.source` where D16 allows stamping; opaque non-null data
gets no tag. Folds group by area/owner and have no source tag
(`elohim/eprfs/epr-lsp/src/deliver.rs:289`,
`elohim/eprfs/epr-lsp/src/deliver.rs:236`,
`elohim/eprfs/epr-lsp/src/deliver.rs:408`).
Gate-live's `gatefeed` module is the producer, landing on `feat/gate-live`; only the
`--device` mux runs it.
No CLI or external wire protocol is implied.

## C9 — Berth presence

Two different berth claims carry the word `lsp`, and both are needed. The resident holds
resource `toolchain:lsp` for its life (this contract), which proves there is one resident per
device. Each flycheck holds resource `cargo`, class `lsp`, for one check (C6), which shares build
capacity with gates and other builds.

Policy extract (`genesis/agentic/pool-policy.json`, `berth`):

```json
"resources": { "toolchain:lsp": 1 },
"classes": { "toolchain:lsp": { "lsp": { "daemon": true } } },
"mooring_ttl_s": 10800
```

The resident uses synthetic mooring `lsp-device` with its own PID and claims resource
`toolchain:lsp`, class `lsp`, once for its whole serving lifetime. Policy sets capacity
**1** and `daemon: true`: the lease has no TTL and is reclaimable when its mooring dies.
The mooring itself has a 10,800-second TTL
(`elohim/eprfs/epr-cli/src/lsp.rs:40 (MOORING)`,
`elohim/eprfs/epr-cli/src/lsp.rs:42 (RESOURCE)`,
`elohim/eprfs/epr-cli/src/lsp.rs:558 (Berth::claim)`;
`genesis/agentic/pool-policy.json:138 (berth._comment)`,
`genesis/agentic/pool-policy.json:141 (berth.resources)`,
`genesis/agentic/pool-policy.json:149 (berth.mooring_ttl_s)`,
`genesis/agentic/pool-policy.json:159 (berth.classes._comment)`,
`genesis/agentic/pool-policy.json:164 (berth.classes.toolchain:lsp)`).

Mooring is best-effort (a failure warns; claim can establish a missing mooring), but the
claim is mandatory before readiness or serving. A refused claim makes the resident exit
non-zero with berth's stderr detail, naming the holder on a capacity refusal; it never
serves unleased. Each berth invocation has a 60-second deadline
(`elohim/eprfs/epr-cli/src/lsp.rs:46 (BERTH_WAIT)`,
`elohim/eprfs/epr-cli/src/lsp.rs:387 (serve)`,
`elohim/eprfs/epr-cli/src/lsp.rs:558 (Berth::claim)`,
`elohim/eprfs/epr-cli/src/lsp.rs:632 (berth_call)`;
`genesis/agentic/pool-policy.json:138 (berth._comment)`).

Every **15 minutes**, the resident runs
`berth touch --session lsp-device --pid <pid>`, where `<pid>` is the resident's PID;
heartbeat failures warn. Last detach and idle-child retirement leave this mooring and
lease held, even with zero roots. On resident exit, stop the heartbeat, run
`berth release toolchain:lsp --session lsp-device`, then
`berth unmoor --session lsp-device`; cleanup failures warn. An abrupt death is reclaimed
through the daemon class's dead-mooring rule
(`elohim/eprfs/epr-cli/src/lsp.rs:44 (HEARTBEAT)`,
`elohim/eprfs/epr-cli/src/lsp.rs:396 (serve)`,
`elohim/eprfs/epr-cli/src/lsp.rs:593 (Berth::heartbeat)`,
`elohim/eprfs/epr-cli/src/lsp.rs:619 (Berth::down)`;
`genesis/agentic/pool-policy.json:159 (berth.classes._comment)`,
`genesis/agentic/pool-policy.json:164 (berth.classes.toolchain:lsp)`).
