---
title: "Re-genesis 1: regression classes and the interfaces that would have caught them"
id: regenesis-1-regression-classes
date: 2026-11-08
status: in-flight
author: agent:orchestrator@claude-opus-5-5 (steward: human:matthew)
habits: [dataplane-convergence]
---

# Re-genesis 1: regression classes and the interfaces that would have caught them

The address crossing reached alpha green in CI and then failed in eight distinct ways during the roll and re-seed. Each failure is listed with the interface shape that let it through, and the change that would have caught it before alpha. "In flight" items are fixed during the overnight shift; the rest are filed follow-ups with an owner surface.

The common thread: **state and contracts that live only at runtime** — in a running store, a hand-copied manifest, a floating tag, a retry counter — cannot be re-derived after a reset and are not checked by any gate. A re-genesis exposes every one of them at once.

## 1. Declared state that only lives in the store

- **What broke:** the reset wiped the steward-declared release channel `runtime:app-bundle:alpha:dev`, so every release-channel app went dark (`/`, `/lamad/`). App #1796 refused its release with `channel-absent`.
- **Interface gap:** the channel is created once by a manual steward act (`[app:channel-create]`), and nothing in the repo says it should exist.
- **Pattern:** declared-state-as-code. Keep a tracked declaration of the channels each environment carries, and have CI reconcile create-if-absent idempotently. The reviewed commit that adds a channel is the steward's act. A reset then restores declared state with no human memory involved.
- **Status:** cured tonight by the existing tag; declaration file + reconciler filed (touches `scripts/ci/app-release-stage.sh`, a CI-gating script, so it needs an operator grant).

## 2. Empty string used as "absent" across the wire

- **What broke:** Genesis #1747 — `imagodei-portal` (blob "populated at deploy") carried `"blobCid": ""`; the seeder passed it through `??`; the crossing's strict byte-CID validator refused the whole batch of 100.
- **Interface gap:** `blobCid?: string` lets `""` type-check. The server validated correctly; the client had no type that could not hold an invalid address.
- **Pattern:** parse, don't validate, at the SDK edge. A branded `ByteCid` type in the TypeScript SDK, constructible only through `ByteCid.parse`, makes an empty or malformed address a compile error, and `Option`-shaped fields make "absent" explicit.
- **Status:** instance fixed in flight (`8a65a8518`, `||` + 4 tests); branded type filed for `elohim/sdk/epr-ts` and the seeder.

## 3. A per-human manifest forked from the shared template

- **What broke:** the `TOKIO_WORKER_THREADS` band-aid edited only `_edgenode-conductor.template.yaml`; adam renders from its own `adam-firstman-conductor.yaml`, so adam did not get it, and the diagnosis briefly went wrong.
- **Interface gap:** two hand-maintained copies of one workload with no parity check.
- **Pattern:** one template plus per-human values rather than a copied manifest; until then, a parity test that the env-var names and container shapes of the template and every per-human conductor manifest agree (allowlisting intended differences).
- **Caught in flight:** the parity check also found adam's STORAGE manifest (`adam-firstman.yaml`) missing nine env vars the template carries — the three seed-lever gates (so every Genesis run seeded network stakes on "2/3 peers — failed (stay Bootstrap): adam"), the stakes scope aliases, the storage-side iroh relay URL and doorway origin, the blob path, the display name and pprof. A genesis peer was running a degraded storage configuration since those were added.
- **Status:** fixed in flight — the nine blocks mirrored into adam's storage manifest, the conductor band-aid and migration intent removed from both conductor manifests, and `genesis/orchestrator/human-manifest-parity.test.mjs` (env-var names and container sets per workload, explicit allowlist, empty) wired into the orchestrator gate. Template-plus-values consolidation filed.

## 4. A lock held across a blocking call in the conductor fork

- **What broke:** every cold install deadlocked: `Spaces::get_or_create_space_ref` held the spaces-map write lock across `Space::new`'s `block_on(open_db)`, while readers parked the CPU-quota-sized worker pool.
- **Interface gap:** a synchronous shared map whose writer blocks; only reproducible with few workers, so household hosts (24 threads) did not see it.
- **Pattern:** do not hold a shared lock across blocking or async work (for the spaces map, held by `conductor::space::tests::creating_a_space_does_not_starve_the_runtime` in the fork); test concurrency primitives under the smallest production worker count.
- **Status:** fixed (`993cf597a` single-flight, regression test on 2 workers). Filed: run the conductor's own tests with a worker count matching the smallest pod quota.

## 5. Crash-loop retries that damage the state they retry against

- **What broke:** each failed install crash-looped the wrapper; every cycle minted a lair key and appended to an un-checkpointed `store_file-wal` (3 MB against a 4 KB store). Two of two churned keystores then failed `install_app` with lair `internal`; fresh keystores installed.
- **Interface gap:** install failure = process exit, so the retry unit is a container restart that kills lair mid-write.
- **Pattern:** an install is an idempotent, in-process reconcile with backoff, not a crash. Treat a timeout as indeterminate (reconnect, `list_apps`, then decide), keep the conductor alive, and reuse one agent key per install intent instead of minting per attempt.
- **Status:** filed (`elohim-storage` `happ_manager::install_fresh` + `process_manager`), with the two forensic keystores as reproduction material.

## 6. A rollout primitive that cannot replace a broken pod

- **What broke:** a StatefulSet under `OrderedReady` does not replace a pod that has not become Ready, so each fixed spec sat unapplied behind adam's and matthew's crash-looping pods until an operator deleted them; the 600 s rollout wait timed out and held the rest of the fleet.
- **Interface gap:** the deploy treats "apply then wait" as a roll.
- **Pattern:** after applying an update, a conductor that is not Ready on the old revision is replaced deliberately (pod delete by the deploy's own identity), and the wait measures the new revision.
- **Status:** filed (`elohim/holochain/Jenkinsfile` conductor phase, CI-gating — needs an operator grant).

## 7. Fixed retry budgets that assume the dependency is already up

- **What broke:** every storage node's binding minter gave up after 6 attempts (~18 min) because the conductors were not installed yet; conductors came up 45-70 min later and nothing retried, so no alpha node holds a cross-signed binding.
- **Interface gap:** attempt-count retry with no readiness signal.
- **Pattern:** retries keyed to a readiness event (app enabled, cell running), with an unbounded slow tail, not a fixed count.
- **Second instance, caught live:** the same shape broke delivery. Release adoption (`main.rs` ~6360) and binding mint (~3646) each snapshot their conductor role client ONCE at boot; the registry reconnects in the background, but those two keep the boot-time `None`. Matthew's and adam's storage booted while their conductors were broken, so both refused every release (`conductor_unavailable`) and both doorways served "App not found" for every release-channel app.
- **Third instance, in the doorway's readiness signal (shem's reading, confirmed in source):** `/health/serving` judges conductor presence by `rolesDiscovered = state.zome_configs.len()` (`doorway/doorway-service/src/routes/health.rs` ~596), a map filled at first discovery; nothing visible empties it on a conductor restart. So the fleet write-readiness probe answers ready straight through a restart — which is exactly why `app-delivery-refuses-fast.feature` station 2 stays red (3 ready answers across a ~10 s restart). The story is right; the signal is a boot snapshot. Pattern: readiness from live liveness (worker pool / app-socket state), not a discovery-time count. Filed on the doorway.
- **Status:** binding-mint retry handed to shem (it gates shem's custody-authority hardening). Boot-snapshot fix in flight: both consumers acquire their client lazily through the registry, with tests; its edge roll restarts every storage node, which also clears matthew without a cluster act.

## 8. Floating artifact tags in a roll

- **What broke (risk, not failure):** `[deploy-only]` rolled conductors on `elohim-happ:dev-latest`; any DNA build landing between the image and the roll would have moved the hApp under a half-rolled fleet.
- **Interface gap:** a deploy that references a mutable tag.
- **Pattern:** rolls reference digests; a tag is only an input to resolving a digest once, at plan time, recorded in the roll plan.
- **Status:** filed (edge Jenkinsfile, CI-gating).

## Found after closure (shakeout, 2026-11-08)

Nothing below turned anything red. Each was found by reading CI that had already "passed", and each is a name or a reference that crossed a boundary no compiler sees. For each one, the cure is placed at the **lowest layer that can see the defect**, with higher layers kept as backstops rather than as the catch. The ladder runs: type/compile, then lint, then contract test, then pipeline stage, then live probe.

## 9. A name spelled on both sides of a language boundary

`substrate-verify.ts` renamed its state files to `*_blob_cid` (`44a8b99a8`), but `genesis/Jenkinsfile` kept `cat`ing `*_blob_hash`. The read failed inside `catchError`, so it only set UNSTABLE among other causes. The env stayed empty, and three stages gated on `when { env.X != '' }` skipped for two builds: custody seeding, propagation and delivery verify. **Cure (compile and test):** the producer owns the names. `UPLOAD_EXPORTS` is a typed map in TS, the script writes `exports.env` from it and refuses on any empty value, and the Jenkinsfile reads that file generically. The self-test pins both the file's keys and the fact that the Jenkinsfile spells no state file.

## 10. A skip that reads as green

A `when` gate on a derived value turns an upstream failure into a skipped stage. Jenkins shows a skip as neither red nor unstable. **Cure (pipeline):** consumer stages no longer gate on the derived value. A missing value is a named `unstable(...)`, or the script's own preflight failure (`propagation.preflight CONTENT_BLOB_HASH unset`).

## 11. A test that inherits the developer's environment

The pin-attestation steps spawned the gate with `...process.env`. Pre-push and agent shells export `GATE_LIVE_RUN`, which hid the runner's own `epr session id` probe. So the scenario passed locally and failed in the Jenkins pod (#1752/#1753, 6 scenarios). **Cure (test):** the spawn is hermetic and strips inherited `GATE_*` and session variables, so local runs model CI. The fake `epr` answers the runner's housekeeping without counting it as an observation.

## 12. A durable reference into a store that was reset

The doorway's Mongo `users` rows survived the alpha re-genesis, but the conductors were re-keyed. The account rows still named agents that no conductor hosts, so login succeeded and `/hc/connect` answered 401 `Session is missing its installed app binding`. The seeder took 409 "already registered" as success. No compiler sees a reference across two stores. **Cure (pipeline):** the seeder verifies the binding resolves and fails `ORPHANED-BINDING human=…` instead of assuming. **Backstop (live probe):** a doorway invariant, orphaned bindings = 0; the `storeVerdict` check covers only the opposite case, store lost and conductors intact. **Procedure:** a re-genesis resets every store that holds a key binding, the doorway Mongo included, as the household `MESH_RESET` already does. The remaining alpha cure is an operator act.

## 13. A whole-tree check selected only by its own file

`address-profile-lint` scans the tree, but its gate declared only the script itself as input, so no ordinary push selected it. It could not be widened either, because 326 identifier hits would have blocked every push. **Cure (lint):** a ratchet against a committed baseline, refusing any new (rule, path) hit while allowing paydown. Its inputs are now the byte-plane surfaces, and the recipe refuses instead of printing WARN-CLASS.

## 14. Wire shapes hand-copied into a client crate

`crates/elohim-storage-client` kept its own `BlobManifest { blob_hash }` and a `PUT /blob/` with no CID, while storage serialises `blob_cid` and requires `PUT /blob/{cid}`. It has no caller and no test, so nothing noticed. **Cure (test, then compile):** the drifted methods had no caller. `get_manifest` and `BlobManifest` are deleted. `put_blob` computes the CID with the address kernel, PUTs `/blob/{cid}` and refuses a receipt whose CID differs. `get_blob` and `blob_exists` take a `ByteCid`. The contract test `tests/blob_wire_contract.rs` serialises the real `PutBlobResponseView` and parses it through the client. No gate ran this crate's tests: `workspace-bridge` lists it as an input but never builds it, which is class 13 again. A new `elohim-storage-client` gate project now runs test, clippy and fmt. **Still owed (compile):** derive `Deserialize` on `PutBlobResponseView` and its sub-views, so the client parses the producer's type and the last hand-copied receipt struct goes away. The general rule: **a hand-written reader of a wire shape is the drift point.** That covers jq and python in shell scripts, wasm `JSON.parse(...).field`, and Rust clients with private structs. Each either consumes generated types or carries a contract test through the producer's serializer.

## 15. A method guard repeated per branch

The doorway's EPR router served the root only for `method == GET`, so HEAD fell through to a 404. The guard was not one site. About 90 `(Method::GET, …)` arms, the relay and name-standing guards, the registry's exact method match and storage's GET-only dispatch all refused HEAD independently. **Cure (compile and test):** `MethodClass` classifies the method once at the single entry point. A HEAD is admitted as a GET, with any Upgrade dropped, and its body is stripped at the single exit; status and headers are kept. Only two declared native HEAD probes stay HEAD. A source-scan test fails on any new `(Method::HEAD, …)` arm that does not guard on `native_head`, an arm the compiler would let sit dead. A real-listener parity test asserts that HEAD and GET agree on status, content-type and `X-Content-Address` across the root projection, a deep link, a registry read, a blob and health. A mutation check confirmed the test fails without the rewrite. Found while there: a blob pantry hit dropped `X-Content-Address`, for GET too. It is fixed and asserted.

## 16. A verifier that judges a path it cannot see

`epr flow cites verify` called a cite DEAD whenever its target was missing from the working tree. In the eprfs CI pod the `elohim/brit` submodule is not checked out, so a cite into brit read as dead while the file existed at the pinned commit. Orchestrator #2260 (and eprfs #103/#104) were red on dev for this alone. **Cure (test):** a target inside a submodule is judged at the gitlink the superproject pins. It is resolvable when the pinned tree holds it, DEAD naming the pin when the pinned tree lacks it, and `UNVERIFIABLE-CITE` when this checkout has none of the submodule's objects, never DEAD. Paths outside every submodule are byte-identical to before. A test replays the live research corpus in the CI shape, with no submodule checked out. The same shape recurs through this list (9, 11, 13, 16): **a check is only as good as what it can see from where it runs.**

## Smaller frictions met while fixing in flight

- **The style ratchet is invisible until push.** The late-bind fix touched `elohim-storage`, whose RS-GOV-04 rule requires every touched crate to pay down at least one finding; the pre-push gate refused after 322 s. Running `just gate rust-style` with the fix would have caught it in 20 s. Filed: the implementer brief template names `just gate rust-style` for any crate with debt.
- **Freshness by directory mtime, not by the inputs that feed the artifact.** The household's stale-hApp check compares a zome's wasm against the newest file under the zome directory. The licence-tier commit (`a8d2af4d1`) touched only each zome's `Cargo.toml` licence field; cargo correctly rebuilt nothing (licence metadata does not feed the wasm), so the check reported every DNA stale forever and even `MESH_REBUILD_STALE_HAPP=1` refused "still stale after just build". Pattern: freshness from cargo's own fingerprint or a content hash of the build inputs. Worked around tonight with the declared `MESH_ALLOW_STALE_HAPP=1`; fix filed against `app/elohim-app/scripts/hc-mesh.sh`'s staleness check.
- **A regex built from a random value.** `worker-start.test.mjs` asserted a secret never leaked with `new RegExp(secret)`. The secret is random base64, so a `+` in the wrong place made the pattern invalid (`Nothing to repeat`) and the orchestrator gate failed at random. shem's L3 and this shakeout's push each hit it once. The cure is literal containment (`includes`); a sweep found no other regex built from a runtime value in the tests. A test that fails at random is a correctness bug in the test until shown otherwise.
- **`main.rs` is 6,967 lines against a 7,000-line hard ceiling**, and the late-bind fix added about 140 of them. Boot wiring for each consumer (binding mint, release adoption, retention) is the obvious seam to extract into its own module. Filed as a bounded modularization, not done mid-fix.

## What a re-genesis should prove before it touches alpha

The household recast proved the crossing's data plane but none of the eight classes above, because the household runs on full hosts, from scratch, with no prior state, no per-human manifests, no CPU quotas and no release channels. A pre-alpha rehearsal that would have caught most of them:

1. Run the household conductors under a CPU quota matching the smallest alpha pod (classes 4, 7).
2. Reset from a store that has history, not from nothing (class 5).
3. Diff rendered manifests for every alpha human, not just the template (class 3).
4. Reconcile declared environment state from the repo and assert it after reset (class 1).
5. Seed through the same typed SDK the apps use (class 2).
