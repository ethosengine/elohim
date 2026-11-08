---
id: artifact-store-brief-f2-sibling-fetch
status: active
---

# Brief F2 — the sibling fetch: producer (F2a) and consumer (F2b) (Opus 5.5; integrity class: key computation, record acceptance, verify-before-install)

Worktree `as-f2`, branch `artifact-store/f2`, based on `artifact-store/e3` (scoped serving, verified). Read `common.md`, then `design.md` §3 in full — **§3 is your specification** — and §1 for the serving side you build on. Read the E3 report (`genesis/docs/superpowers/plans/task-as-e3-report.md`) for what actually landed, especially:
- the serving session closes with its connection, so **draw one artifact over one connection** (not one `tier_call` per chunk);
- local custody is indexed only from a custody Produce whose **receiver** is this device and which carries no `via:` slot;
- the newest designated derivation per output is the only one served.

**Write-set:** `elohim/eprfs/epr-cli/src/{artifact.rs (new), peer.rs (the `artifacts` verb and the fetch queue only), main.rs}`, `elohim/eprfs/eprfs-peer/src/{fetch.rs (new), daemon.rs (the fetch-queue tick only), lib.rs}`, `elohim/eprfs/eprfs-peer/tests/fetch.rs (new)`, `elohim/eprfs/epr-cli/tests/artifact.rs (new)`, `app/elohim-app/scripts/hc-mesh.sh` (`_mesh_build_one` ~:1996-2032 and a new `mesh_fetch`), `justfile` (mesh build recipe ~:420-429 only), `genesis/agentic/pool-policy.json` (`berth.channel.crosses` gains `artifact-produce@1: [produce]`). Not E4's files (`crates/elohim-blob-store`, `epr-cli/src/payload.rs`).

**Deliverables (design §3):**
- **F2a producer.** The `mesh-build` MethodDescriptor (§3.a.1); the DerivationKey (§3.a.2: `KeyInput::Projected` if Track A's `project_view` is usable on this tree, otherwise the labelled interim `KeyInput::Raw(<epr bin source-state closureCid>)` with slot `basis-key:git-tree` — say which you used); `epr artifact put` (§3.a.3: descriptor stored under its CID, `brit-build-ref build put --step mesh-build:<project> --output <bafkr4i…> --inputs-hash <DerivationKey CID>`, the local put whose Produce carries `measure: artifact-produce@1`, `reason: produce` so it is both local custody and the crossing record); `hc-mesh.sh` calls it after `install -D` + `cmp`, and a failure prints `[mesh build] unrecorded: <reason>` and writes `<slot>/mesh-bin/.unrecorded` (the build still succeeds).
- **F2b consumer.** `epr peer artifacts --key <cid> --json` (§3.b.1, the `verdicts` filter); the daemon-owned fetch queue (§3.b.2); `fetch::fetch_artifact` with `FetchBudget` and `FetchGap` exactly as §3.b.3 (manifest first, capped; `record.key == key && record.output == output`; resume = only missing chunks; race across holders; verify each chunk; store the manifest last; the install read is `stream_local_blob`, the whole-blob verify that writes the `Use`); `just mesh build` tries the fetch first (`MESH_BUILD_FETCH=0` opts out) and falls through to `cargo build` on any `FetchGap`, printing its reason; transport modes per §3.b.5 as typed gaps.
- **Follow-ups owned here:** F-E2-1 (the manifest path decodes the nested `DerivationRecord`/key without their `ext`, so a manifest with record extensions changes CID on re-encode — fix by decoding through the retained-record wrapper in `eprfs-core/src/derive.rs` ~:364, or record why it stays) and F-E2-2 (a peer-level test that a held manifest records an `unavailable` Use).

**Tests:** every test named in design §3.e (`eprfs-peer/tests/fetch.rs` and `epr-cli/tests/artifact.rs`). Plus `the_fetch_draws_one_artifact_over_one_connection`.

**Gates:** `just gate eprfs` (one build at a time on the device; a storage gate may hold the slot for ~85 min — wait, never kill), `just gate orchestrator` with `EPR_BIN` set per common.md (pool-policy and hc-mesh.sh are in its inputs). **Do not run the mesh or a 454 MB receipt**: the orchestrator runs F2r with both devices.

**Reviewer:** Codex astra read leg + orchestrator hunk review of `fetch.rs` acceptance and `artifact.rs` key computation. Report: `genesis/docs/superpowers/plans/task-as-f2-report.md` (`cites: []` in frontmatter).
