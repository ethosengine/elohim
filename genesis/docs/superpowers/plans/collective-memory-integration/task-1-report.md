---
id: collective-memory-task-1-report
status: DONE_WITH_CONCERNS
cites: []
gap: plans__2026-09-09-collective-memory-integration#1
actor: agent:implementer@gpt-6
---

# Native collective memory delivery

The native local slice is implemented over existing governed files, BlobCid and REA observations. The repository collective is declared in `.epr-meta/collective.json`; shared memory is a capability of `collective:ethosengine/elohim`, not a separate memory collective. The declaration's local participation rule uses the existing actor store for mutable session attribution. This is an honor-system local act policy, not authentication, notarized membership or peer authorization.

No commits or pushes were made. Base supplied by orchestration: `6d46714ddb936071e1a2847fd6c595ea25883733`. Scoped commits: none, per operator instruction. Existing unrelated working-tree edits remain intact.

## Interfaces

`epr flow memory collective|pin|contribute|project|feedback|graduate [--input PATH] [--session ID] --root DIR --json`.

- `collective` validates the static declaration and exposes its exact resource pin, local stewardship relationship, policy defaults and discoverable strict input guidance.
- `pin` computes an exact raw file BlobCid including metadata, alongside the existing canonical-body `flowResourceCid`. Paths are locators; the two address meanings are explicitly distinct.
- `contribute` validates the versioned assertion file, exact source versions, declaration pin, restrictions, source/lineage links and session-claimed author; records an existing attributed observation. It does not copy source documents into a database or accept the assertion.
- `project` selects only explicitly supplied, recorded contribution versions. It preserves qualifications, contradictory references, author/steward/scope/reach and selection omissions. Its canonical JSON receipt has a raw BlobCid and implementation-source method pin. Population outside explicit selection remains unknown; output is not automatically saved.
- `feedback` requires saved exact target bytes and the challenged passage; it records an existing correction referring to the pinned request and target. Request and output metadata inherit target/request reach restrictions.
- `graduate` is a read-only repository reach rehearsal. It requires available exact evidence, allowed source/request audience, an independent approved native verdict on the exact contribution, and no later contrary verdict. It neither executes publication nor establishes experiential acceptance.

The native adapter contract is `eprfs-agent/src/memory.rs`. No new core EprKind, DHT entry, coordinator function, network head, membership registry, mutable memory queue or private runtime-store writer was added. Existing Qahal charter/steward concepts and role/kind vocabulary inform the local declaration; its shape is not represented as a notarized Qahal Membership record.

## Verification

Gate evidence: `env GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=safe.directory GIT_CONFIG_VALUE_0=/projects/elohim CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 just gate eprfs` — `EXIT=0`.

Final owning gate passed 353 tests, including 11 new memory integration tests, fmt and clippy. Log: `/tmp/collective-memory-native-gate.log`. Existing ts-rs attribute-parser warnings remain; there was no new gate failure in the final run.

Build evidence: `env RUSTFLAGS= CARGO_TARGET_DIR=/tmp/eprfs-gate-target CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 cargo build --manifest-path elohim/eprfs/Cargo.toml -p elohim-epr-cli` — `EXIT=0`. Log: `/tmp/collective-memory-native-build.log`.

The frozen executable is `/tmp/collective-memory-native-epr`, SHA256 `a354c33a8fa329b2e0b47da5d8fb1ad98736b1cc11fc5137b5c6b202c39afe73`. Its real repository `flow memory collective` invocation returned `EXIT=0`; declaration raw CID: `bafkreih4v23ncuo4m4im7huis5wsskexml55bs7yq4p7kieicmhn6obc64`. View: `/tmp/collective-memory-declaration-view.json`. The shared Cargo berth was normally released after verification.

Seam registry JSON Schema validation passed (`EXIT=0`). Path-scoped `git diff --check` passed. Fixtures prove two actor roles, concurrent identical retry, contradictory versions without last-writer resolution, exact feedback, independent rehearsal, source and metadata reach refusal, unknown fields, stale collective/source pins, missing saved bytes, malicious/symlink paths, byte/log limits, and raw-codec/frontmatter preservation.

Independent source review identified two reach defects during implementation: projection-request metadata could exceed its own source policy, and feedback metadata lacked inherited restrictions. Both were corrected and covered by a direct request/saved-output laundering regression before this final gate. Final audit also corrected raw-byte CID codec selection and bounded the existing note log reader before writes. A final same-session persona-switch race was also fixed: the shared note writer validates and emits from one resolved actor snapshot, retains its actor-claim CID, and memory output derives its author/claim from that same outcome. The deterministic regression switches the session persona between validation and emission, then checks event provider, claim link and returned identity. Existing unguarded note behavior remains unchanged; its output gains an optional actor_claim mirroring the existing event link. These earlier states are not the verified binary.

## Concerns and remaining scope

This task establishes the native local slice, not completed ceremony integration, full toolkit retirement, network enforcement or token savings. The fresh integrated journey is owned by station 3. Local technical review does not confer appointed experiential acceptance.

Inputs are bounded to 32 unique files/256KiB, explicit selection to 16 contributions, projection receipt content to 24KiB, and existing native sidecar reads to a 32MiB per-file preflight. Source hashing loads complete bounded files. Write-boundary target rechecks and existing actor/note reader work are additional bounded work, separately disclosed; the caller must impose total subprocess time/output limits. These are local operational bounds, not a guarantee against an adversarial concurrent filesystem mutator.

Exact versions need retrievable bytes. This slice deliberately does not store copies of source files or automatically retain projection bodies. Missing/changed evidence produces refusal, not fabricated recovery. Consequential saved projections and feedback requests must be preserved by the retention owner. The receipt method CID pins this implementation slice; the full executable hash above pins the tested native binary. Native observation timestamps retain the existing Git-HEAD dating semantics, not measurement wall-clock sampling.

Direct source/governance inspections and Cargo verification occurred outside the bounded recall executor and are not evidence of executor token economy. No production source assertions were contributed by these tests; synthetic roots own all integration-test writes.

## Final source checksums

- `.epr-meta/collective.json`: `fcaeb6d151dc6710cf9e88976d29289762fbd0cbf8871ff52088130edf3822f7`
- `elohim/eprfs/eprfs-agent/src/memory.rs`: `9a98911a6d4f7a67cdf05c6e23d71b18c19c837f3892a67aaf943c1e0171db20`
- `elohim/eprfs/eprfs-agent/src/lib.rs`: `809f5fe17fb2eb96a7d01c8af38358d661e41d24654c7919fafbe6e7e84c2d16`
- `elohim/eprfs/epr-cli/src/flow/memory/mod.rs`: `8844c6f113130d9e991e6099876b5abd44bfae8c2ab9d3352ad03c1ddc547ff9`
- `elohim/eprfs/epr-cli/src/flow/memory/validation.rs`: `f114de947c910f4f3256e57ac0eda9259edc8b03e970087eb59d89a384725f4b`
- `elohim/eprfs/epr-cli/src/flow/memory/guide.rs`: `15ed6100732199f882026abd69ed0c9657e40a1c15cf977aaec814c1798b088f`
- `elohim/eprfs/epr-cli/src/flow/mod.rs`: `613ce57f25f48c038436719110c03963251680e72784a55603094a9650e139fc`
- `elohim/eprfs/epr-cli/tests/flow_memory.rs`: `d1c100e0b7e65f89b36c91f631263e52db733f75dab123dafbf8b812f750170f`
- `elohim/eprfs/epr-cli/seam-registry.yaml`: `533a7237b2469127f052bf6d00e55d54bf4cd5059a80bf7e023eac84b4cc6f4d`
- `elohim/eprfs/epr-cli/Cargo.toml`: `96e487374058b2c49c4739d20fd8d3a72a729bf7f9718ba90d60e1c2b51d9e6c`
- `elohim/eprfs/Cargo.lock`: `b85f77e71e8cb27a55d68957e96b8f48427a64cf6cdc41a7b8046aa89a6e3ee1`
- `elohim/eprfs/epr-cli/src/flow/note.rs`: `fd14572abb6b2c5d435eda1212cb7c87a6f2be8d83bde88c8c0b1f7f422570f9`
