---
id: placement-c6-report
title: "C6 — TypeScript: verifyAddress, HttpTier and placement status, the service worker's CacheTier"
status: DONE_WITH_CONCERNS
class: architecture
gap: plans__2026-10-20-placement-contract-architecture-plan#29
actor: agent:implementer@codex
habits: [materialize-what-is-read]
commits: []
cites:
  - "placement-c6-brief | C6 brief | sha256:5540ad4a3292a972 | path: genesis/docs/superpowers/plans/task-placement-c6-brief.md"
  - "placement-contract-interfaces-design | The placement contract's interfaces | sha256:de0208f9b6e426db | path: genesis/docs/superpowers/specs/2026-10-20-placement-contract-interfaces-design.md"
---

# C6 — verifyAddress, HttpTier, placement status, CacheTier

Item C6, gap `#29`. Nothing is staged, committed or pushed, as the brief requires. No cargo was run, so
no cargo berth was claimed.

## Gate evidence

**Superseded.** This block records round 0 only. The current gate evidence is under "Gate
evidence (final installed dependency graph)" in "Codex takeover integration — 2026-10-21" below;
the round-0 `EXIT=1` was cured in round 1.

- `just gate elohim-app` (the gate `epr flow context app/elohim-app/src/apps-sw.ts` printed): **`EXIT=1`**.
  The lint, lint-routes, lint-a11y and deps steps passed: eslint reported 0 errors and 295 warnings, and
  the storage-client `tsc` build passed. The `build` step (`ng build`) then failed with `TS2307: Cannot find
  module '@elohim/storage-client/generated/ResilienceView'`, in four files that are unchanged since
  `ea437a1ae` (2026-10-15). The failure has nothing to do with this lane. The app's tsconfig maps
  `@elohim/storage-client/generated/*` to `./node_modules/@elohim/storage-client/dist/...`, and this
  workspace has no app-local `node_modules`, because the install is hoisted. So the deep import resolves
  only through the package `exports`, which have no `./generated/*` entry. `dist/generated/ResilienceView.d.ts`
  does exist. The build does not compile `apps-sw.ts`; esbuild's `build:sw` does that.
- The gate's remaining step, run alone (`just test` in app/elohim-app): **`EXIT=0`**, 234 files and
  4716 tests passed. That includes the new `src/apps-sw.spec.ts` (6 tests) and the unchanged
  `raw-cid-verify.spec.ts` (5).
- `eslint src/apps-sw.ts`: **`EXIT=0`**, with 0 errors and 4 warnings. All 4 warnings were already there
  (`resp.json()` and `event.data` are `any`).
- R-T2 proof: I linted a mutant `apps-sw.ts` in place and then restored it (byte-identical, checked with
  `cmp`). The mutant had a stray `cache.put`, a `cache.addAll`, a `new ArchiveWriter` outside
  `extractZip` and an `archive.put` outside `extractZip`. Result: **`EXIT=1`**, with all four reported as
  R-T2 `no-restricted-syntax` errors.
- The SW bundle, `esbuild src/apps-sw.ts --bundle --format=iife` written to scratch: **`EXIT=0`** (203 kB).
- epr-ts `vitest run`: **`EXIT=0`**, 7 files and 36 tests. `tsc --noEmit -p tsconfig.json`: **`EXIT=0`**.
- storage-client-ts `vitest run`: **`EXIT=0`**, 9 files and 122 tests. `tsc -p tsconfig.typecheck.json`
  (new files included): **`EXIT=0`**. `tsc --noEmit -p tsconfig.json` (the build config): **`EXIT=0`**.
- eprfs-ts `vitest run`, re-run against the changed epr-ts: **`EXIT=0`**, 3 files and 27 tests.
- epr-ts and storage-client-ts have no eslint config or lint script. `tsc` is the lint step for both, and
  I added no eslint config to them.

## Files changed

- `elohim/sdk/epr-ts/src/address.ts` (new): `verifyAddress(address: CID | string, bytes): Promise<boolean>`,
  `addressForm`, `RAW_CODEC`, `DAG_CBOR_CODEC`.
- `elohim/sdk/epr-ts/src/cbor.ts`: added `cborItemLength` and `decodeCanonicalSequence`, an RFC 8742
  framer.
- `elohim/sdk/epr-ts/src/index.ts`: exports the additions above.
- `elohim/sdk/epr-ts/tests/address.test.ts` (new, 10 tests).
- `elohim/sdk/storage-client-ts/src/tiers/http-tier.ts` (new): `HttpTier`, `HttpConnector`, `AgentSigner`,
  `BATCH_ROUTE`, `SIGNATURE_HEADERS`, `admissibleFromSecureContext`.
- `elohim/sdk/storage-client-ts/src/placement.ts` (new): `NeedRef`, `NeedStatus`, `PendingStatus`,
  `PlacementExplanation`, `Placement`, `DevicePlacement`, `placementFor`, `placementRoutes`.
- `elohim/sdk/storage-client-ts/tsconfig.json`: excludes the two new sources from the published build
  (see concern 1). `tsconfig.typecheck.json` (new) checks them instead.
- `elohim/sdk/storage-client-ts/vitest.config.ts`: aliases `@elohim/eprfs` and `@elohim/epr` to their
  sources.
- `elohim/sdk/storage-client-ts/tests/{http-tier,placement}.test.ts` (new, 10 and 8 tests).
- `app/elohim-app/src/apps-sw.ts`: adds `CacheTier`, `ArchiveWriter`, `rememberHead`, `EPRFS_CACHE_NAME`
  (`eprfs-objects`) and `staleAppCaches`. `extractZip` now writes through `ArchiveWriter` and
  `rememberHead`.
- `app/elohim-app/eslint.config.js`: the SSR selectors are hoisted into `SSR_RESTRICTED_SYNTAX` with no
  change to the rule. A new `src/apps-sw.ts` block adds R-T2 (`SW_VERIFIED_WRITES_ONLY`) and replaces
  the pillar deep-relative import pattern there with a narrowed one: anything leaving the app is an
  error except `epr-ts/src/index` and `eprfs-ts/src/(index|wire)` (round 1, finding 9).
- `app/elohim-app/src/apps-sw.spec.ts` (new, 6 tests). This is a test file beside the write-set file and
  sits outside the brief's literal write-set.

## Behaviour

Behaviour is unchanged. The fetch path, `getDeliveryPeers` (R6a, untouched), scoring, the zip path and
invalidation work as before. The only runtime difference is that activate's purge filter now goes
through `staleAppCaches`, which applies the same predicate. `CacheTier` is defined and tested, but
nothing routes through it yet, because no bundle is published as a tree.

## Where the code and the interfaces spec disagree (for the spec to absorb)

1. **Who verifies in HttpTier.** §A.6 says `HttpTier` "calls `sink.arrived` … after it has verified the
   bytes", while §H.2 says "Verification stays in TierChain". The TS code follows §H.2: `HttpTier` passes
   bytes through, and the chain verifies them and counts `integrity` against the tier. The spec should
   pick one rule for both languages.
2. **`AgentSigner` and the A7 headers are not defined in the spec.** I chose
   `{ agent: string; sign(m): Promise<Uint8Array> }`. The headers are `X-Eprfs-Agent`, `X-Eprfs-Nonce`
   and `X-Eprfs-Signature` (base64url). The signed message is the dag-cbor encoding of
   `{root, items: [cid…], nonce}`. Nothing serves these yet.
3. **`HttpConnector` options.** Beyond `{signer}`, the constructor also takes `timeoutMs` and
   `secureContext`. From a secure context the connector refuses `http:` endpoints that are not loopback,
   which enforces H11.
4. **Batch-route parsing choices.** A trailer is buffered and applied only when the stream ends cleanly,
   so anything after the trailer voids it (every unanswered item becomes `unreachable`). An arrived CID
   answers every requested item with that CID. A `via` entry that is not a `TierId` (`own`) is dropped
   before sending.
5. **`DevicePlacement` gains `declared(): Promise<NeedRef[]>`.** `status()` needs to know which needs the
   app declares. Without that, a network need the route fails to answer would vanish rather than read
   `unreachable`. `status()` rejects when the worker cannot answer, instead of returning an empty list.
6. **The storage routes for placement are not named in the spec, and the server does not serve them.**
   I chose `GET /api/v1/apps/{appId}/placement`, answering `{needs: NeedStatus[]}`, and
   `GET …/placement/why?field=&contentType=|scope=`, answering `PlacementExplanationView`. Their failures
   map onto `AnswerReason`: 401 and 403 give `refused`, a timeout gives `timeout`, anything else gives
   `transport_error`, a declared need the route does not answer gives `not_yet_delivered`, and an entry
   that cannot be read gives `unverifiable`.
7. **`placementFor` signature.** The `client` parameter is typed `Pick<StorageClient, 'getJson'>`. An
   optional fourth `options` parameter (`pollMs`, `now`, `sleep`) feeds `watch`.
8. **`watch` mapping.** `present` maps to `answered` (terminal). `absent` maps to `unanswered`, and
   `watch` waits until `comeBack.at` when one is given. `unreachable` maps to `retrying-at` at now plus
   the poll interval. When the need is no longer declared, the result is `withdrawn`. `queued` is never
   emitted.
9. **The device-plane `why` is derived from `device.status`.** `kept` means present and `deferred` means
   absent. The spec never defines `deferred`.
10. **`NeedRef` has no `signing` variant**, although the operator ruled on 2026-10-20 that the app
    declares signing as a need. Its shape waits on the signing design (§19 Q4). I added nothing for it.
11. **`verifyAddress` details.** Legacy hex is accepted in lowercase only, matching `raw-cid-verify`.
    The digest length must be 32. CIDv0, dag-pb, dag-json and the identity hash all fail closed.
12. **`@elohim/epr` gained a CBOR-sequence framer**, which the spec does not list. `@ipld/dag-cbor`
    9.2.5 has no `decodeFirst`, and `cborg` would be an undeclared (phantom) dependency.
13. **ArchiveWriter's precondition.** `ArchiveWriter.put` runs after `verifyRawSha256`, not after a direct
    `verifyAddress` call as H1 states. This preserves behaviour. The two become the same once
    `raw-cid-verify.ts` wraps `verifyAddress` (concern 2).
14. **`CacheTier.put` verifies before it writes**, and throws on a mismatch, writing nothing;
    `TierChain` catches a failed write-back and still delivers (contract 7). Round 1 dropped this
    check; round 2 restored it, because §H.3's contract and R-T2's name both say the worker
    *caches* only verified bytes, not only that it serves them. Hits are verified as well (H5).
    So an own-tier write-back is hashed twice today, once by the chain and once by `put`.

## Concerns

1. **Packaging blocks the by-name dependency.** `@elohim/eprfs` is not in `pnpm-workspace.yaml`, and no
   prebuild builds `@elohim/epr`'s `dist`. Changing either, or `pnpm-lock.yaml`, is outside this
   write-set. As a result:
   - `HttpTier` and `placement.ts` are excluded from the storage-client build and are not exported from
     its index. They are typechecked through `tsconfig.typecheck.json` and the vitest aliases instead.
   - `apps-sw.ts` imports `verifyAddress` from `epr-ts/src/index`, every eprfs type from
     `eprfs-ts/src/index`, and its one eprfs value, `absence`, from `eprfs-ts/src/wire`, all by
     relative workspace path (round 1, finding 9).
   - The app spec cannot load `TierChain`, because eprfs-ts's `tree.ts` imports `@elohim/epr` by name.
     The chain over a verified own tier is covered in storage-client's `http-tier.test.ts` instead, with a
     `MemoryTier` own tier.

   Follow-on: add eprfs-ts to the workspace, build epr-ts in the app prebuild, depend on both by name,
   then drop the exclusions and aliases.
2. **R-T3 is not met yet: TypeScript has more than one verifier.** `raw-cid-verify.ts` is outside the
   write-set and still carries its own check. So does `eprfs-ts/src/verify.ts`, which is C4's file.
   Follow-on: make `verifyRawSha256` check `addressForm(a) ∈ {raw, legacy}` and then call
   `verifyAddress`, keeping `expectedSha256Hex`. Replace eprfs-ts's `verifiesAddress` with an import of
   `verifyAddress`.
3. **Most of the H.3 table is still to do.** `CacheTier` is not wired into the fetch path. These items
   are not done:
   - the head-plane split and the `X-Tree-Root` header (H10);
   - the `residentView` 404 rule;
   - `navigator.storage.persist` for depth 3 (H19);
   - the `{type:'tiers'}` BroadcastChannel message (H8);
   - replacing `scorePeerForSw` with `proximityOrder` (H17);
   - moving delivery peers onto `HttpTier`.

   That last item would change behaviour now, because no server serves `POST /eprfs/v1/blobs` yet.
4. **The app gate's build step is red in this workspace** for the pre-existing `ResilienceView`
   resolution described above. Someone who owns `app/elohim-app/tsconfig.json` or the install shape
   should fix it. It is not this lane's.
5. **The habit atom was not updated.** I wrote no DELTA in
   `elohim/eprfs/.epr-meta/materialize-what-is-read.habit.md`, which is outside this write-set. The
   habit stays red: no remote tier is served yet.
6. **R-T2 is a lint rule, not a build test.** It runs in the app's `lint` script, which the gate runs. A
   page writing Cache Storage directly is covered at runtime by `CacheTier`'s hit verification (H5), and
   that is tested.

## Codex review 1 fixes

I checked each of the nine findings against the code. Five hold and are fixed. Four hold in part:
the parts inside this lane's write-set are fixed, and the parts outside it are refused, with the
reason given below. Status stays `DONE_WITH_CONCERNS`. Nothing is staged, committed or pushed.

### Gate evidence (round 1)

The cargo berth was claimed before the gates (`berth claim cargo --class verify --ttl 3600`,
`CLAIM_EXIT=0`) and released after them (`berth release cargo`, `RELEASE_EXIT=0`). No cargo ran.

- `just gate epr-ts`: **`EXIT=0`**, 8 files and 40 tests. This is the first time the gate itself
  ran for this lane, and it was red before this round. TypeScript 6.0.3 refuses an `outDir` with no
  `rootDir` (`TS5011`), so `pnpm build` failed. Fix: `"rootDir": "./src"` in
  `elohim/sdk/epr-ts/tsconfig.json`. This prerequisite was broken before this lane, and its file
  is inside the write-set.
- `just gate storage-client-ts` (the new gate project, see finding 7): **`EXIT=0`**. The build,
  `typecheck` and `test` steps each printed `EXIT=0`, with 9 files and 128 tests.
- `just gate elohim-app`: **`EXIT=0`**. Lint gave 0 errors and 285 warnings, the build passed, and
  the tests ran 234 files and 4716 tests. Report concern 4, the pre-existing `ResilienceView`
  build red, is resolved. The root cause was in this write-set: `@elohim/storage-client`'s
  `exports` had no `./generated/*` subpath, so the hoisted install could not resolve
  `@elohim/storage-client/generated/ResilienceView`. I added `"./generated/*"` →
  `dist/generated/*.{d.ts,js}`.
- The app's vitest over `src/apps-sw.spec.ts` and `raw-cid-verify.spec.ts`: **`EXIT=0`**, 11 tests.
  `eslint src/apps-sw.ts`: **`EXIT=0`**, 0 errors and 4 pre-existing `any` warnings.
- The SW bundle, `esbuild src/apps-sw.ts --bundle --format=iife --target=es2020` written to
  scratch: **`EXIT=0`**, 202.9 kB. This is the same size as before the switch to the epr entry
  (see finding 9).
- eprfs-ts against the changed epr-ts (`node_modules/.bin/vitest run --root elohim/eprfs/eprfs-ts`):
  **`EXIT=0`**, 3 files and 27 tests. `pnpm exec` cannot run there, because eprfs-ts is not in the
  workspace.
- `node --test` over `gate-runner.test.mjs`, `pipeline-registry.test.mjs` and the manifest and
  schema tests in `genesis/orchestrator`: **`EXIT=0`**, 38 passed and 1 skipped. The new
  `build-manifest.json` loads and registers.

### Fixed

1. **Findings 1 and 5: `cborItemLength` threw on valid DAG-CBOR.** Both findings hold. The head's
   argument is now read as a number only for majors 2-5, where it is a length or a count. For
   majors 0, 1, 6 and 7 the framer skips `width` bytes and does not read them. The safe-integer
   check stays only where a length is read. Vectors added to `tests/address.test.ts`:
   - `{x:1.5}` (float64, `fb 3ff8…`), `{n:2^60}`, `{n:-2^63}`, `2^64-1`, and an array that mixes
     floats with a large integer. Each frames at exactly its length, every truncated prefix of each
     answers `null`, and the sequence decodes round-trip.
   - Lengths past 2^53: a byte-string length and an array count still throw `length too large`.
2. **Findings 2 and 4: `watch` busy-looped on a past `comeBack.at`.** Both findings hold, and there
   was a second way into the same loop. The old shallow check let `comeBack: {when:'at'}` through
   with no `epochSecs`, so the wait was `NaN`, which `setTimeout` treats as 0.
   - Fix: the wait is now `until > 0 ? max(minWaitMs, until) : pollMs`. A time that is already
     past waits the poll interval, and a time in the future waits at least `minWaitMs` (a new
     option, default 1 s).
   - `absent` entries are read through `isUnanswered` (finding 8), so a malformed come-back time
     now reads as `unreachable`/`unverifiable` and never reaches the wait.
   - Tests:
     - A route that keeps naming the same past time waits `[7000, 7000, 7000, 1000]` on a fake
       clock: two past answers, one exactly now, and one 200 ms ahead.
     - A come-back time with no `epochSecs`, and an unknown `why.reason`, each read as
       `unreachable`/`unverifiable`.
3. **Finding 3: `readSequence` buffered without bound and did quadratic work.** The finding holds.
   - **Buffer.** A `Pending` buffer holds the bytes as start and end offsets. It compacts in place
     only into at most half its size and otherwise doubles, so each byte moves O(1) times amortized,
     however the chunks fall.
   - **Framing.** Each pass frames from the current item's start. For an incomplete item that costs
     one walk of its heads, not of its bytes.
   - **Item cap.** `maxItemBytes` is new, with a default of 64 MiB. An item that is still incomplete
     past the cap, or complete but larger than it, voids the rest of the batch as `unreachable`.
   - **Cancel.** The reader cancels the stream whenever it gives up.
   - **Timeout.** `HttpTier` and `HttpConnector` now default to `DEFAULT_TIMEOUT_MS` (120 s), and
     `Infinity` opts out.
   - **Aliasing.** I checked that `@ipld/dag-cbor`'s decode copies byte strings out of the input,
     so compacting after delivery cannot change delivered bytes. A test pins this.
   - Tests:
     - A 300 kB leaf arrives in 4 KiB chunks, more than 70 reads, after a small item. Both are
       delivered byte-exact.
     - A byte-string head claims 2^40 bytes and the filler never ends, with `maxItemBytes` at
       8 KiB. The result is `unreachable`, the stream is cancelled, and fewer than 16 chunks are
       read.
     - A connector built with no timeout ends a stalled fetch at `DEFAULT_TIMEOUT_MS` (fake timers).
4. **Finding 7: nothing failed a build for the HttpTier and placement surface.** The finding holds,
   and the fix is inside the write-set.
   - New gate project `storage-client-ts`, from a gate-only `elohim/sdk/storage-client-ts/build-manifest.json`
     (no `jenkinsPath`, so nothing is dispatched; `bridges/jenkins` has the same shape) and a
     package `justfile` `gate` recipe. The recipe runs `pnpm run build`, `pnpm run typecheck` (the
     new script, `tsc -p tsconfig.typecheck.json`, which covers `src/tiers/**` and `placement.ts`)
     and `pnpm run test`, and echoes `EXIT=` after each step.
   - Its inputs include `elohim/eprfs/eprfs-ts/src/**`, `elohim/sdk/epr-ts/src/**` and, since
     round 2 (fix 7), the two schema sources whose generated types placement imports. A change to `eprfs-ts/src/tier.ts` or to
     `epr-ts/src/cbor.ts` now selects `storage-client-ts`, checked with
     `gate-runner.mjs --changed-file-list --names` in both oracle modes.

### Fixed in part; the rest refused

5. **Finding 6: R-T3, TypeScript has five verifiers.** Inside the write-set I fixed four things.
   - `verifyCid` now delegates to `verifyAddress`, as `addressForm(c) === 'dag-cbor' &&
     verifyAddress(c, b)`, and is marked `@deprecated`. Its behaviour is unchanged: it still
     refuses a raw CID over the same digest, and a test pins that.
   - `DAG_CBOR_CODEC` has one home in `@elohim/epr`, `address.ts`; `cid.ts` imports it.
   - `verifyBlobHash` is documented as the three-answer reader of a declared row, not the
     admission verifier.
   - *(Superseded by round 2, fix 1: `put` verifies again.)* `CacheTier.put` no longer hashed
     again, on the grounds that `TierChain` writes back only bytes it has just verified.
   - Source-scan rails (widened in round 2, fix 6): `epr-ts/tests/one-verifier.test.ts` fixes the set of files in `@elohim/epr`
     that take a sha2-256 digest. That set is `address.ts`, `cid.ts` and `blob-hash.ts`, each with
     its reason. In storage-client, `http-tier.test.ts` asserts that the package takes no digest at
     all. A sixth verifier in either package fails its gate.

   Refused, because each sits outside the write-set:
   - `raw-cid-verify.ts` (`app/elohim-app/src/app/elohim/utils/`) is still its own
     `crypto.subtle` check. `address.ts`'s header used to claim it was already a wrapper; that
     claim is corrected to name the follow-on.
   - `eprfs-ts/src/verify.ts` and `TierChain`'s re-verify of an own-tier hit (`tier.ts:267`)
     belong to C4. Each own-tier hit is therefore still hashed twice, once by `CacheTier` (H5,
     required because a same-origin page can forge an entry) and once by the chain.
   - **For the spec:** either a tier declares that it verifies its hits and the chain skips them,
     or the chain alone verifies and `CacheTier`'s hit check is dropped. Dropping it reopens H5 for
     any reader that calls `CacheTier` outside the chain.
   - A repo-wide scan would need an allowlist of about ten legitimate digest sites (minting in the
     seeder, the loader, session-key proofs). It belongs to whoever owns that rail's scope, so the
     two package-local scans above stand in for it.
6. **Finding 8: `placement.ts` breaks the SDK source-of-record rules.** Inside the write-set I
   fixed two things.
   - `absent` is now read with `@elohim/eprfs`'s `isUnanswered` plus a `NotCarriedReason` tag
     guard, the same strictness as the HTTP tier's trailer. The cast is gone.
   - `ANSWER_REASONS` is now `Record<AnswerReason, true>`, and the new `NOT_CARRIED_REASONS` is
     `Record<NotCarriedReason['reason'], true>`. Adding, renaming or removing a value in either
     generated union fails the typecheck, which the new gate runs, until the table matches. A
     hand list can no longer drift silently.

   Refused, because each sits outside the write-set:
   - The schema itself: `views/placement-status-view.schema.json` belongs in `elohim/sdk/schemas/v1/views/`,
     and its Rust struct in elohim-storage.
   - The codegen: an `answer-reason` `_codegen.constant`, and a `GENERATED_OUTPUT_DIRS` entry for
     `storage-client-ts/src/generated` in `elohim/sdk/schemas/scripts/codegen-ts.mjs`.
   - The relative `../../schemas/generated-ts` import therefore stays until that distribution
     lands. It is type-only and works under the typecheck config, but it would hit `TS6059` once
     the file rejoins the build's `rootDir: ./src`.
   - The status route's shape is still this client's revision (report revision 6), not a schema.
     That makes it the first schema-first item for the lane that owns the views.
7. **Finding 9: the worker deep-imports package internals, and the lint rule is off.** Inside the
   write-set I fixed four things.
   - `apps-sw.ts` imports `verifyAddress` from `epr-ts/src/index` and every eprfs type from
     `eprfs-ts/src/index`.
   - The one value taken from eprfs, `absence`, still comes from `src/wire`. The eprfs entry loads
     `tree.ts`, which imports `@elohim/epr` by name, and the app can neither bundle nor test that
     until the by-name dependency lands. The import's comment says so.
   - `no-restricted-imports` is back on for `src/apps-sw.ts`, with a narrowed `regex` pattern: any
     import that goes three levels up and is not `epr-ts/src/index` or `eprfs-ts/src/(index|wire)`
     is an error.
     - Proof: a mutant that imported `eprfs-ts/src/tier` and `epr-ts/src/cbor` linted
       **`EXIT=1`**, with both reported as `no-restricted-imports`. The file was restored
       byte-identical (`cmp`, `RESTORED=0`).
   - `"sideEffects": false` is added to `@elohim/epr`'s `package.json`. Without it the entry import
     pulled `@noble/ed25519` and dag-cbor into the worker, 276 kB against 203 kB. With it the
     bundle is 202.9 kB.
     - The one top-level statement in epr-ts is `proof.ts`'s `ed.etc.sha512Sync` setup. It still
       runs whenever `verifyEd25519` is imported. It is dropped only when nothing from `proof.ts`
       is used.

   Refused, because each sits outside the write-set:
   - Counting relative edges that leave `app/`, or recording them, needs changes to
     `app/scripts/lint-workspace-imports.mjs` and its baseline.
   - A `tsc --noEmit` step over `src/apps-sw.ts` needs a change to the app gate, in
     `app/elohim-app/build-manifest.json` and its justfile.
   - Meanwhile `CacheTier implements WritableTier` is still checked only where the vitest spec
     compiles it.

### Concerns after round 1

- Report concern 4 (the app build red) is resolved.
- Concerns 1, 2, 3 and 5 stand, narrowed as described above.
- New concern: two changes reach consumers outside this lane.
  - The `./generated/*` export in `@elohim/storage-client` widens what the package resolves. It
    adds subpaths and removes none.
  - `"sideEffects": false` in `@elohim/epr` lets bundlers drop unused modules. Any consumer that
    relied on importing epr-ts for `proof.ts`'s global `ed.etc.sha512Sync` setup, without
    importing anything from `proof.ts`, would lose it. `git grep` finds no import of
    `@elohim/epr` by name outside `elohim/sdk/`. The one other `@noble/ed25519` user,
    `genesis/seeder/src/seed-epr-atom.ts`, does not import epr-ts and uses only the async API.
    This rests on the search; no test covers it.

## Codex review 2 fixes

I checked each of the nine findings against the code. Findings 2 and 6 are the same defect, so
there are eight defects in all. Six are fixed in full. Two are fixed in part: the parts inside this
lane's write-set are fixed, and the rest is refused with its reason. Status stays
`DONE_WITH_CONCERNS`. Nothing is staged, committed or pushed.

### Gate evidence (round 2)

The cargo berth was claimed before the gates (`berth claim cargo --class verify --ttl 3600`,
`CLAIM_EXIT=0`) and released after them (`berth release cargo`, `RELEASE_EXIT=0`). No cargo ran.

- `just gate epr-ts`: **`EXIT=0`**, 8 files and 43 tests (40 before: the CID-like forgery test and
  two framer tests are new).
- `just gate storage-client-ts`: **`EXIT=0`**. The build, `typecheck` and `test` steps each printed
  `EXIT=0`, with 9 files and 130 tests (128 before: the tiny-element framing test and the
  moving-clock watch test are new).
- `just gate elohim-app`: **`EXIT=0`**. Lint gave 0 errors and 285 warnings, the build passed, and
  the tests ran 234 files and 4716 tests.
- The app's vitest over `src/apps-sw.spec.ts` and `raw-cid-verify.spec.ts`: **`EXIT=0`**, 11 tests.
  `eslint src/apps-sw.ts`: **`EXIT=0`**, 0 errors and the 4 pre-existing `any` warnings.
- The SW bundle, `esbuild src/apps-sw.ts --bundle --format=iife --target=es2020` written to
  scratch: **`EXIT=0`**, 203.3 kB.
- eprfs-ts against the changed epr-ts (`node_modules/.bin/vitest run --root elohim/eprfs/eprfs-ts`):
  **`EXIT=0`**, 3 files and 27 tests.
- Selection: `gate-runner.mjs --changed-file-list --names` (`GATE_ORACLE=path`) selects
  `storage-client-ts` for a change to `v1/views/placement-explanation-view.schema.json`, and for
  one to `epr-ts/src/cbor.ts`: **`EXIT=0`** each.

### Fixed

1. **Finding 1: `CacheTier.put` no longer verified, which breaks §H.3 and R-T2.** The finding
   holds. Round 1 removed the check on the grounds that its caller verifies, but no caller in the
   worker exists yet, and the contract is about what is *cached*, not only what is served.
   - `put` runs `verifyAddress(cid, bytes)` first and throws `eprfs-integrity: … not cached`
     on a mismatch, writing nothing. `TierChain` already catches a failed write-back
     (`tier.ts:331`, `.catch(() => undefined)`) and still delivers.
   - The class comment now says `put` verifies whoever calls it. The R-T2 lint comment and
     message ("verifies against its CID") are true again, so they are unchanged.
   - Spec test: a put of bytes that do not match the CID rejects with `eprfs-integrity`, and the
     cache stays empty. It replaces the round-1 test that pinned the defect.
   - Cost: each own-tier write-back is hashed twice, by the chain and by `put`. Report revision
     14 says so.
2. **Findings 2 and 6: `watch()` reported an unchanged unreachable need on every poll.** Both
   hold. The dedupe compared the derived `retrying-at` value, whose `epochSecs` moves with the
   clock, and the test froze the clock.
   - `watch` now compares a *reading* instead (`readingOf`). For `retrying-at` the reading is
     the unreachable reason, or `declared-unknown` when the worker did not answer. For any other
     status it is the report itself. A new `retrying-at` is yielded only when that reading
     changes.
   - The existing test "five reads, three reports" now advances its clock inside `sleep`.
   - New test, on a moving clock: four refused reads, two where the worker does not answer, two
     more refused reads, then present. Nine reads give four reports: refused, worker-down,
     refused, answered. Under the old dedupe it would have given nine.
3. **Finding 3: the rail and the report described pre-round-1 code.** The finding holds.
   - The lint comment and message match the code again, by fix 1.
   - Report revision 14 is rewritten to the current behaviour and its history.
   - Concern 1's import bullet now names `epr-ts/src/index`, `eprfs-ts/src/index` and
     `eprfs-ts/src/wire`.
   - The Files-changed line no longer says the import rule is switched off; it names the
     narrowed pattern.
   - The round-0 gate block is marked superseded, and so is the round-1 fix 5 bullet on `put`.
4. **Finding 4: `verifyAddress` trusted a CID-like object's `multihash` over its `bytes`.** The
   finding holds, and I reproduced it: `CID.asCID` rebuilt the forged object with the target's
   `toString()` and the forged digest.
   - Fix: for a non-string address, `parse` decodes `CID.decode(cid.bytes)` and requires
     `canon.equals(cid)`. An object whose `multihash` disagrees with its `bytes` fails closed,
     and the digest checked is always the one the CID's text names.
   - Test: the forged object from the finding answers `false` against both the forged bytes and
     the target's bytes, and `addressForm` answers `null`. A consistent CID-like object still
     verifies.
5. **Finding 5: re-framing an incomplete item was quadratic in its heads.** The finding holds.
   - New `CborItemFramer` in `@elohim/epr`'s `cbor.ts`, exported from its index. It keeps its
     place (`at` and items still owed) after the last complete head, from the item's start, so
     each `frame(bytesSoFar)` call resumes there. A head that arrives only in part, or a string
     whose body is not all in yet, is re-read once on the next call. An item of `n` heads in `k`
     chunks costs at most `n + k` head visits. `cborItemLength` is now a one-shot framer, with
     unchanged answers.
   - `HttpTier.readSequence` holds one framer and resets it after each decoded item. The offsets
     it keeps are relative to the item's start, which `Pending`'s compaction preserves (it never
     consumes mid-item).
   - Tests:
     - epr-ts: the framer fed byte by byte agrees with a fresh walk at every prefix of an item
       with a link, byte strings, floats and a large integer. An array of 2^18 one-byte elements
       fed in 1 KiB steps completes within `n + 1 + calls` visits.
     - storage-client: an arrived item whose `paths` array holds 2^20 one-byte elements,
       streamed in 4 KiB chunks and then ended one byte short, answers `unreachable` within
       `n + 3 + reads` visits over more than 250 reads. Before this fix that would be about
       2^28 visits.
6. **Finding 7, the rail part: the R-T3 scan was flat, cwd-relative and package-local.** The
   finding holds.
   - `epr-ts/tests/one-verifier.test.ts` now scans recursively, resolved from `import.meta.url`,
     over `epr-ts/src`, `eprfs-ts/src`, `storage-client-ts/src` and `app/elohim-app/src`. It skips
     tests and specs.
   - The allowlist names every digest site found, each with its reason. Two are marked **DEBT**,
     second verifiers still to go: `eprfs-ts/src/verify.ts` (C4) and `raw-cid-verify.ts`. The
     others mint a CID and check nothing (`tree.ts` `cidOf`, `epr-codec.ts`), are the reader
     (`blob-hash.ts`), or mention a digest only in a comment (three app files, one of them
     generated).
   - A new digest site in any of the four trees now fails the epr-ts gate.
7. **Finding 8, the gate part: the gate depended on a gitignored cache.** The finding holds.
   - The manifest no longer lists the two cache files as inputs. It lists their schema sources
     instead (`v1/enums/answer-reason.schema.json`, `v1/views/placement-explanation-view.schema.json`),
     so a schema change selects this gate (checked above).
   - The gate recipe refuses before launch when either cache file is missing, printing
     `REFUSED: … Cure: pnpm run schema:codegen:ts at the repo root` and `EXIT=2`, instead of
     failing the typecheck as a missing module. I did not exercise that branch: proving it
     would mean removing another lane's generated cache.

### Fixed in part; the rest refused

8. **Finding 7, the code part: three TypeScript verifiers remain.** Refused, because each file
   sits outside this write-set: `eprfs-ts/src/verify.ts` and `tier.ts` belong to C4, and
   `raw-cid-verify.ts` sits in `app/elohim-app/src/app/elohim/utils/`. The rail now names both
   as DEBT, so either deletion shrinks the list, and a fourth verifier fails. The cure is the
   finding's: import `verifyAddress` in `tier.ts` and delete `verify.ts`; make
   `verifyRawSha256` check `addressForm ∈ {raw, legacy}` and then call `verifyAddress`.
9. **Finding 8, the distribution part: placement's types are still read from the cache.**
   Refused. Distributing `placement-explanation-view` and `answer-reason` into a tracked
   `storage-client-ts/src/generated/` needs an `INTERFACE_FILES` entry and a
   `GENERATED_OUTPUT_DIRS` entry, or a ts-rs export from elohim-storage. Each sits in
   `elohim/sdk/schemas/scripts/codegen-ts.mjs` or the Rust crate. The schema itself is another
   lane's untracked file. Hand-copying the generated types into `src/generated/` would break
   the rule that generated files are never hand-edited. The `rootDir: ./src` obstacle stands
   until that distribution lands.
10. **Finding 9: the TypeScript arrows exist only in test and typecheck aliases.** Refused. Each
    step in the fix needs a file outside this write-set:
    - `@elohim/eprfs` needs an entry in `pnpm-workspace.yaml`;
    - `@elohim/epr` needs to be a dependency of `app/elohim-app/package.json`, and its build
      needs adding to that file's prebuild;
    - `pnpm-lock.yaml` needs to be regenerated.

    A `workspace:*` dependency added to storage-client's own `package.json` would be inside the
    write-set, but it would leave `pnpm-lock.yaml` out of step with it, and a frozen-lockfile
    install, pnpm's default under CI, refuses that. The finding is right that `@elohim/epr` is already a workspace package, so concern 1's
    wording "packaging blocks" overstated it for epr. The block is this write-set, not the
    workspace. Deleting the exclusions, aliases, relative imports and the eslint allowance has
    to land in the same change as those manifests, so I left all of them in place.

### Concerns after round 2

- Concerns 1, 2, 3 and 5 stand as narrowed in round 1. Concern 2 is now visible in a rail: two
  verifiers are named as DEBT.
- `CacheTier.put` and `TierChain` both hash a write-back. That is the price of R-T2 holding
  without a caller. The spec should decide whether a tier declares that it verifies, letting
  the chain skip the check (see round 1, fix 5).
- The R-T3 rail lives in the epr-ts gate, but it reads the app and eprfs trees. The epr-ts
  manifest's inputs do not list those trees, so a new digest site in the app fails the next
  epr-ts gate run, not the app's own gate. Listing them as inputs means changing
  `elohim/epr/build-manifest.json`, which is outside this write-set.
- The storage-client gate's refusal branch is untested (fix 7).

## Independent Codex handoff review — 2026-10-21

The independent review of the current working tree requests changes. Its durable receipt is
`genesis/local-dev/placement-codex-handoff/c6-review.md`. Four Important findings remain: ship
HttpTier and placement through the public SDK/workspace build; finish R-T3 by replacing the two
remaining verifiers and wiring the rail's source inputs; publish the placement schema types rather
than importing the gitignored schema cache; and make CacheTier body-read failures return a named
absence without aborting the rest of a batch.

Fresh verification: `just gate epr-ts`, **`EXIT=0`** (43 tests); `just gate storage-client-ts`,
**`EXIT=0`** (130 tests, build and typecheck passed); focused app vitest over `src/apps-sw.spec.ts`
and `src/app/elohim/utils/raw-cid-verify.spec.ts`, **`EXIT=0`** (11 tests). No cargo, full app build,
mesh operation, staging or commit was performed by this reviewer. These unit/build results do not
establish H.3's full worker routing or justify changing the habit to green. The round-3 fallback
cause remains unverified; no rate-limit cause is inferred from the missing disposition.

## Codex takeover integration — 2026-10-21

The user authorized completion of the interrupted workflow, including the integration prerequisites
outside C6's original lane write-set. This addendum supersedes the package-distribution and R-T3
concerns in rounds 0–2. No staging, commits, pushes, cargo or mesh operations were performed by this
seat. The household remains stopped. The resuming actor is `agent:implementer@codex`.

### Handoff attribution

The user-provided transcript identifies the original C5 termination as an HTTP 429 weekly-limit
error. It says the original C6 third review round used Claude fallback for all three lenses; the
exact reason Codex was unavailable for that round has not been established from a saved log.
No rate-limit cause is inferred for C6.

### Closed findings

1. **The public APIs ship.** eprfs-ts is a pnpm workspace package; eprfs builds epr before itself,
   and storage-client builds eprfs before itself. The app declares both dependencies and its existing
   storage-client prebuild now reaches that dependency chain. HttpTier and placement compile into
   storage-client's normal `dist`, export from its barrel, and have framework-free `./tiers/http-tier`
   and `./placement` package subpaths. Test/typecheck source aliases and build exclusions are removed.
   epr-ts's relative runtime imports now include `.js`, allowing plain Node ESM to load the built
   dependency graph. The lockfile has only 37 added lines for the new workspace imports; unrelated
   pnpm snapshot refreshes were removed, then the final lock was reinstalled offline and frozen.
2. **R-T3 has one admission verifier.** The app raw wrapper checks raw/legacy address form and
   delegates to `verifyAddress`, retaining `expectedSha256Hex` and the existing DAG-CBOR refusal.
   eprfs's chain, walk and vector tests use that same verifier; its private duplicate is deleted.
   The two DEBT allowances are removed. The epr-ts gate's inputs include the scanned consumer
   sources, while CI's ordinary ts-test source scope remains unchanged.
3. **Placement types are published generated outputs.** The existing schema generator accepts
   per-interface output directories and writes AnswerReason and PlacementExplanationView into
   `storage-client-ts/src/generated/placement/`. Both writing and `--verify` use the same output map.
   Placement imports these package-local generated modules, not the gitignored schema cache.
   The SDK gate watches the generator and all upstream package metadata. A packed tarball contains
   the new JavaScript, public declarations, and both generated schema declarations.
4. **Cache body failures are named absences.** CacheTier catches body-read and verification
   exceptions as `unreachable('own')`. Successfully read bytes with the wrong digest still produce
   `integrity` and are deleted. A regression makes the first cached body's `arrayBuffer` reject,
   then proves both that item's named absence and the next item's correct bytes reach the batch sink.

The worker now imports only public epr/eprfs package names; the temporary relative-import ESLint
allowance is removed. The app gate also typechecks `apps-sw.ts` with a dedicated WebWorker tsconfig,
so the WritableTier implementation is checked even though Angular's entry graph excludes it.

### Gate evidence (final installed dependency graph)

- `just gate epr-ts`: **`EXIT=0`**, build plus 8 files / 43 tests.
  Receipt: `genesis/local-dev/placement-codex-handoff/c6-epr-ts-final.log`.
- `just gate eprfs-ts`: **`EXIT=0`**, build/typecheck/purity lint and 3 files / 27 tests.
  Receipt: `genesis/local-dev/placement-codex-handoff/c6-eprfs-ts-final.log`.
- `just gate storage-client-ts`: **`EXIT=0`**, build/typecheck, 10 files / 131 tests, plus the
  plain Node ESM public-subpath smoke (1 test). The barrel consumer is included in the package
  typecheck and vitest suite, resolving `dist` without source aliases.
  Receipt: `genesis/local-dev/placement-codex-handoff/c6-storage-client-ts-final.log`.
- From `app/elohim-app`, `just typecheck-sw`: **`EXIT=0`**;
  `pnpm exec vitest run --config vite.config.ts src/apps-sw.spec.ts src/app/elohim/utils/raw-cid-verify.spec.ts`:
  **`EXIT=0`**, 2 files / 12 tests. Receipts: `c6-sw-typecheck.log`, `c6-sw-focused-final.log`.
- Focused ESLint over the worker, its spec and raw wrapper: **`EXIT=0`**, 0 errors / 6 warnings
  (four existing worker warnings, one existing raw-parser warning, and the ignored spec notice).
  esbuild's SW bundle: **`EXIT=0`**, 203.0 kB. Receipts: `c6-sw-lint.log`, `c6-sw-bundle.log`.
- `pnpm pack --pack-destination /projects/elohim/genesis/local-dev/placement-codex-handoff`
  from storage-client-ts: **`EXIT=0`**. Archive inspection confirmed both runtime entry files,
  both declaration entry files, and the AnswerReason/PlacementExplanationView declarations.
- `node elohim/sdk/schemas/scripts/codegen-ts.mjs --verify`: **`EXIT=0`** (root-executed;
  270 interfaces fresh). `pnpm run schema:test`: **`EXIT=0`** (root-executed after its separate
  fixture-only ContentView correction; no schema/production API change). Receipts:
  `c6-codegen-verify.log` and `schema-test-3.log` under the handoff directory.
- Final `pnpm install --offline --frozen-lockfile --ignore-scripts`: **`EXIT=0`** (root-executed),
  receipt `c6-pnpm-install-final.log`. Child-side installation/codegen verification encountered
  sandbox EACCES; root reran the same commands with the needed filesystem access. These failed
  attempts are superseded by the recorded successful commands.
- Gate-selection probes: **`EXIT=0`** each. An eprfs package manifest change selects eprfs-ts and
  storage-client-ts; an app raw-verifier change selects epr-ts and elohim-app; a schema generator
  change selects storage-client-ts. `git diff --check` over the scoped changes: **`EXIT=0`**.
- `just gate elohim-app`: **`EXIT=0`** (root-executed with host Node). All lint, route, SSR,
  workspace, accessibility, dependency, worker typecheck, Angular build and test steps passed;
  234 files / 4,717 tests, with 0 lint errors and 285 warnings.
  Receipt: `genesis/local-dev/placement-codex-handoff/c6-app-gate-final.log`.

### Remaining scope and status

Status is **DONE_WITH_CONCERNS** for the architecture/first-adapter lane. Package wiring, generated
ownership, shared verification and the four handoff findings are resolved. This does not implement
or prove the full browser runtime in H.3: CacheTier is not yet the fetch path, and head-plane/root
headers/residentView, persistence-granted depth, tier BroadcastChannel updates, proximity ordering,
delivery-peer TierChain routing and the live placement HTTP routes remain later runtime integration.
The worker continues asking its ingress doorway for delivery peers (R6a). The materialization habit
must remain red until its real scenario passes; root owns its evidence delta and projection.

Scoped commits: none (the architecture brief forbids staging/commits/pushes).
