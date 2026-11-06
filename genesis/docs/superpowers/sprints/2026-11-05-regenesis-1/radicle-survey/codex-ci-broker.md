Legend: ✅ directly verified in source; ◐ inferred from the inspected design or absence of a mechanism.

## 1. Architecture

- ✅ `radicle-node` receives repository updates from peers and emits events after storing them; `cib` receives and filters those events, then spawns an adapter subprocess for each selected CI system. The adapter reports success or failure back to the broker (`genesis/research/repos/radicle-ci-broker/doc/overview.md:3-16`).

- ✅ Concretely, the broker subscribes to the local node’s control socket. It converts `RefsFetched` and `LocalRefsAnnounced` events into branch, tag, and patch CI events by diffing signed refs; unrelated known node events are ignored (`genesis/research/repos/radicle-ci-broker/src/node_event_source.rs:22-42`, `genesis/research/repos/radicle-ci-broker/src/ci_event.rs:289-369`).

- ✅ Converted CI events first enter a persistent SQLite queue. Filtering happens when queued events are processed, not when they are inserted (`genesis/research/repos/radicle-ci-broker/src/queueadd.rs:43-72`, `genesis/research/repos/radicle-ci-broker/NEWS.md:538-550`).

- ✅ A global filter selects the default adapter; independently, each configured trigger has its own filter and named adapter. Every matching adapter runs, sequentially for that event, even if an earlier adapter fails (`genesis/research/repos/radicle-ci-broker/src/queueproc.rs:337-363`, `genesis/research/repos/radicle-ci-broker/doc/userguide.md:569-650`).

- ✅ Adapters are external executables with a common two-message JSON protocol. The broker sends repository and exact commit information over stdin; the adapter first returns a run ID and optional information URL, then `success` or `failure` (`genesis/research/repos/radicle-ci-broker/doc/adapter-impl.md:3-60`). For native CI the adapter itself is the engine; for external CI it mediates with another service (`genesis/research/repos/radicle-ci-broker/doc/architecture.md:140-151`). Known adapters include native CI, Ambient, Concourse, GitHub Actions, webhooks, Container CI, and Tekton (`genesis/research/repos/radicle-ci-broker/README.md:90-113`).

- ✅ As execution progresses, the broker records triggered/running/finished state in SQLite, consumes the adapter’s result, and updates the associated job COB (`genesis/research/repos/radicle-ci-broker/src/adapter.rs:147-178`, `genesis/research/repos/radicle-ci-broker/src/adapter.rs:261-339`).

## 2. Where results live

- ✅ The local durable record is SQLite: `ci_event_queue` stores queued events and `ci_runs` stores each serialized run under a broker-generated ID (`genesis/research/repos/radicle-ci-broker/src/db.rs:29-62`). A run contains repository identity, branch or patch provenance, exact commit, local and adapter run IDs, optional information URL, state, result, job ID, and timeout state (`genesis/research/repos/radicle-ci-broker/src/run.rs:77-94`, `genesis/research/repos/radicle-ci-broker/src/run.rs:214-255`).

- ✅ HTML, JSON, and RSS reports are projections generated from the local database, not independent attestations. The report worker reloads all runs and queued public events, then writes `index.html`, repository pages, `status.json`, and feeds (`genesis/research/repos/radicle-ci-broker/src/pages.rs:964-1063`). A per-run log remains wherever the adapter’s optional URL points (`genesis/research/repos/radicle-ci-broker/doc/architecture.md:173-187`).

- ✅ The network-visible result is a Radicle job COB. A repository has one job object per commit; repeated CI executions add runs to that object rather than creating another job (`genesis/research/repos/radicle-ci-broker/src/cob.rs:92-130`, `genesis/research/repos/radicle-ci-broker/NEWS.md:200-210`).

- ✅ COB operations are written using the local Radicle profile’s device signer. Completion records `Succeeded` or `Failed`, then announces the repository’s local-node namespace with a replication requirement of reaching at least one seed (`genesis/research/repos/radicle-ci-broker/src/cob.rs:199-231`, `genesis/research/repos/radicle-ci-broker/src/cob.rs:249-281`). Thus final results are signed and intended to replicate over Radicle’s P2P repository network.

- ✅ COB publication is optional and deliberately non-fatal: it defaults on, can be disabled, and failures updating the COB are logged without failing or repeating CI (`genesis/research/repos/radicle-ci-broker/src/config.rs:61-75`, `genesis/research/repos/radicle-ci-broker/NEWS.md:241-249`).

- ◐ No patch-comment result path exists in the scoped implementation. Current result surfaces are the job COB, SQLite record, generated reports, and adapter URL; the only “comment” match is documentation prose.

## 3. Artifacts

- ✅ Artifact publication is explicitly a future goal. The initial network contract announces only run metadata—commit and result—not build logs or build artifacts (`genesis/research/repos/radicle-ci-broker/doc/requirements.md:27-31`, `genesis/research/repos/radicle-ci-broker/doc/requirements.md:45-54`, `genesis/research/repos/radicle-ci-broker/doc/requirements.md:81-86`).

- ✅ Neither the broker nor adapter protocol has an artifact field. Adapter output contains only its run ID, optional informational URL, and binary success/failure result (`genesis/research/repos/radicle-ci-broker/adapterlib/src/lib.rs:435-455`).

- ◐ There is therefore no standardized output name, digest/CID, size limit, retention policy, replication promise, or content-addressing model. An individual adapter may publish something independently, but the broker cannot discover, bind, or vouch for it through this protocol.

## 4. Trust model

- ✅ A result is bound to an exact Git commit OID: the job is created for that OID, lookup compares it directly, and the local run’s branch/patch provenance contains the same commit (`genesis/research/repos/radicle-ci-broker/src/cob.rs:105-128`, `genesis/research/repos/radicle-ci-broker/src/cob.rs:234-246`, `genesis/research/repos/radicle-ci-broker/src/run.rs:214-255`).

- ◐ Workflow bytes are only indirectly bound when the workflow file lives inside that Git commit and the adapter actually checks out and honors it. Native CI conventionally uses `.radicle/native.yaml` (`genesis/research/repos/radicle-ci-broker/README.md:98-100`), but neither the result nor job update includes a workflow digest, adapter binary digest, local adapter configuration, environment, or execution closure.

- ◐ Nothing cryptographically proves that the signed node actually performed the run. Any conforming adapter can emit `{"result":"success"}`, after which the broker signs that assertion into its job COB (`genesis/research/repos/radicle-ci-broker/doc/adapter-impl.md:37-55`, `genesis/research/repos/radicle-ci-broker/src/adapter.rs:314-334`). The signature provides attribution and tamper evidence, not proof of execution.

- ✅ Radicle’s social answer is explicit trust selection: each node operator chooses what to run, and each project chooses which CI nodes it trusts (`genesis/research/repos/radicle-ci-broker/doc/whatci.md:66-72`).

- ◐ Runner identity is not distinguished from node identity in the attestation. The COB is signed by the node profile’s device signer; the adapter contributes only an opaque run ID, URL, and result. There is no separate runner key or signature in the response schema (`genesis/research/repos/radicle-ci-broker/src/cob.rs:166-173`, `genesis/research/repos/radicle-ci-broker/adapterlib/src/lib.rs:435-455`).

## 5. Operational lessons from `NEWS.md`

- ✅ Job COB handling repeatedly needed hardening: reuse one job per commit, cache lookups—reducing 100-run lookup time from 600 to 40 seconds—allow suppression of noisy test COBs, and fix announcements to the local-node namespace (`genesis/research/repos/radicle-ci-broker/NEWS.md:124-126`, `genesis/research/repos/radicle-ci-broker/NEWS.md:200-210`, `genesis/research/repos/radicle-ci-broker/NEWS.md:81-88`, `genesis/research/repos/radicle-ci-broker/NEWS.md:11-15`).

- ✅ Adapter process management was recurrent pain: excessive output could deadlock the broker, while fewer than two protocol lines could defeat timeout handling (`genesis/research/repos/radicle-ci-broker/NEWS.md:676-689`, `genesis/research/repos/radicle-ci-broker/NEWS.md:198-204`).

- ✅ Poison events caused restart loops, so processing deliberately changed to at-most-once: remove an event before execution and leave recovery to humans rather than retry forever (`genesis/research/repos/radicle-ci-broker/NEWS.md:401-409`, `genesis/research/repos/radicle-ci-broker/NEWS.md:577-592`).

- ✅ Event semantics and ref normalization recurred as defects: fetched-from node was confused with originating node, and branch, qualified, and namespaced refs were mixed (`genesis/research/repos/radicle-ci-broker/NEWS.md:621-637`, `genesis/research/repos/radicle-ci-broker/NEWS.md:411-418`).

- ✅ Several choices were reversed or retired: a queued shutdown sentinel was removed because it killed the restarted broker; scheduled report generation became event-driven; old broker events were replaced by CI events; and the helper API was ultimately split into a lightweight adapter crate (`genesis/research/repos/radicle-ci-broker/NEWS.md:746-752`, `genesis/research/repos/radicle-ci-broker/NEWS.md:709-714`, `genesis/research/repos/radicle-ci-broker/NEWS.md:719-739`, `genesis/research/repos/radicle-ci-broker/NEWS.md:30-36`).

## Lessons for build-output artifacts on a blob plane

- TAKE · ◐ Keep the signed, replicated verdict small and bind it to the exact closure plus `output_cid`; Radicle’s commit-keyed COB is a strong control-plane precedent (`genesis/research/repos/radicle-ci-broker/src/cob.rs:105-128`).
- TAKE · ◐ Separate artifact bytes from the attestation, but make the attestation carry the CID rather than an arbitrary log URL (`genesis/research/repos/radicle-ci-broker/adapterlib/src/lib.rs:435-455`).
- TAKE · ◐ Preserve local SQLite/HTML as rebuildable operational projections, never as the source of truth (`genesis/research/repos/radicle-ci-broker/src/pages.rs:964-1048`).
- LEAVE · ◐ Do not accept an unsigned adapter `success` as execution proof; require the runner/agent signature over closure, workflow digest, environment identity, and output CID.
- LEAVE · ◐ Do not make attestation publication best-effort while artifact/channel visibility depends on it; Radicle explicitly ignores COB-update failures (`genesis/research/repos/radicle-ci-broker/NEWS.md:247-249`).
- WATCH · ◐ Avoid blind retries, but improve on Radicle’s at-most-once event loss with idempotency keys and durable failure states (`genesis/research/repos/radicle-ci-broker/NEWS.md:401-409`).
- WATCH · ◐ Treat channel-head movement separately from artifact production: a successful immutable artifact attestation should not itself move “latest,” just as Radicle aggregates runs under a commit rather than defining a mutable release head (`genesis/research/repos/radicle-ci-broker/NEWS.md:209-210`).