---
id: brit-native-source-loop-result-2026-11-02
title: Brit native source loop — local evidence and successor journey
status: In Progress
date: 2026-11-02
serves: brit-native-daily-driver
cites: []
---

# Brit native source loop: local evidence and successor journey

Matthew can retain selected source as candidate A outside Git, continue editing B,
and explicitly execute a pinned native artifact against A. This is the local
filesystem slice of the existing shared-working-versions story. The broader habit
remains **red**, and native acceptance remains **pending**. This document is an
operator journey and partial sprint result, not the immutable fulfillment report.

Original scope: `bafkreiewyy6d4mwppo5g5utkn4zt5z4n4kkxekygev6gptwvwvzsbvqi5e`.
Intent: `bafyreic4ohovm4wcni37tvv7ofk34wdmffc45ls7wsuwvjvuok55cvjfci`.
Enclosing commitment: `bafyreiafhuwxfowok472rdgf6oa725hpxdc5aloaykjuqtdkq7hrpt3kra`.
Original actor claim: `bafyreibojewkr3wxaokjepfrhi2fup6dzfvj2b67cjpz7l3bbkynj2ga5q`.
The original 2026-11-02 13:44:31.647660Z start and 21:44:31.647660Z deadline
still apply. Recovery grants no additional time.

## Integration repair — 2026-11-03

The steward authorized finishing and integrating this sprint on 2026-11-03,
including permission for Brit to consume `elohim-seam-contracts` 0.1.2 under its
unchanged AGPL-3.0 license. Brit now has a package/version-specific exception;
its other licensing restrictions remain. The full workspace bans, licenses and
sources checks pass. This supersedes the licensing refusal recorded below;
it does not extend or rewrite either expired shift or its immutable evidence.

The Windows repair requests writable directory handles with backup semantics,
verifies NTFS on the opened handle, and refuses unsupported namespace durability.
Verified read-only retrieval remains available; a fresh custody claim requires a
writable handle. Standard-user API probes pass on Windows x64 and ARM
([run 41698810809](https://github.com/ethosengine/brit/actions/runs/41698810809)).
The source passes 23 Linux storage tests, strict Clippy and an independent source
review. Production Windows Rust tests, the released-package consumer journey,
final owner gates and native acceptance are still pending. No physical power-loss
experiment is claimed. The candidate storage version is 0.3.4; published bytes
are never replaced. Durable repair evidence is
`genesis/local-dev/integration-20261103/`.

The discovered precondition between verified object bytes and durable snapshot
closure is now explicit: the host filesystem and handle must support namespace
persistence. Windows NTFS/handle tests preserve that condition alongside the
existing failure-injection and interruption tests.

## Delivery boundary and evidence

Capture and durability already landed in this retained branch at
`a64aa39627170cc7a91de3207ebc3bed30b34da8` and
`34fb6a57db18c36d23c3fbde5d7e1244b731dd04`. Native-task parent revision
`76554ccafc510e2a63226733fe2a0eacdc6f94e9` binds Brit
`4730ba4b6b29365e3e9161853d85c0f45994e572`. EPRFS 0.3.3 was published before
Brit consumed its registry packages; no sibling override is delivered.

Current continuation evidence is `.claude/shifts/brit-c1-20261102/`.
Original evidence remains in `.claude/shifts/brit-native-loop-20261102/`.
The continuation's `readiness-note.json` pins the predecessor, and
`operations/review-native-task/codex-1.jsonl` is the current technical review.
Its late-interruption coverage finding was repaired and measured in
`revise-native-task-evidence.json`; predecessor reviews are not current verdicts.

Acceptance-story adds explicit `ARK_BIN` selection, failing without fallback when
the selection is empty or invalid. Absent `ARK_BIN` retains the ordinary PATH
lookup. The public real-Ark journey now runs with empty PATH, so it cannot hide
that regression behind a test-created `ark` symlink. Existing task admission,
supervision and outcome types remain the owners. Additional native CLI coverage
changes argv, environment, output declaration and actual executable bytes one at
a time and requires a new method identity for the same source candidate.

At 20:54Z the unchanged frozen `measure.py` returned numeric **0**, despite process
exit 0: the installed Brit could not execute Ark. Raw evidence is
`acceptance-story-oracle-installed-20261102T205435393476Z.log`.
The source repair needs the host's **second** fixed-pool artifact build pair after
`commit-acceptance-story`. A writable-slot focused binary is not an installed
oracle artifact. No passing frozen measurement is claimed here before those host
operations. Never change the oracle, fixture, PATH to hide its failure, or fixed
binary paths to manufacture a result.

The technical review, exact final owner gates, two stable frozen measurements,
other-runtime rehearsal, immutable report, fulfillment, appointed acceptance and
reconciliation remain host-owned boundaries. Upstream licensing is separately
blocked by the operator's explicit decision; no license exception is applied.

Focused measurements on this operation's source:

| Check | Observed result | Current-shift log |
| --- | --- | --- |
| Local Cucumber stations | 6 scenarios, 28 steps passed after final fixture formatting | `acceptance-story-a2o-20261102T211616239653Z.log` |
| Brit ordinary journeys | 8 passed; real-Ark case selected separately | `acceptance-story-brit-tests-20261102T205635798737Z.log` |
| Brit explicit installed-Ark journey | 1 passed with empty PATH | `acceptance-story-brit-ark-20261102T210704438952Z.log` |
| Ark CLI | 21 unit, 15 smoke, 9 native passed | `acceptance-story-ark-tests-20261102T210702162629Z.log` |
| Brit CLI scoped strict Clippy | exit 0 | `acceptance-story-brit-clippy-20261102T210706458746Z.log` |
| New step file ESLint and strict TypeScript | both exit 0 | `acceptance-story-a2o-lint-20261102T211225668254Z.log`, `acceptance-story-a2o-focused-types-20261102T211227332337Z.log` |
| Rust suppression policy | exit 0, no new debt | `acceptance-story-style-20261102T210643195854Z.log` |
| Documented manual broken/corrected journey | expected refusal, then corrected success after final fixture formatting | `acceptance-story-manual-20261102T211706689592Z/evidence.json` |

The manual journey was checked in the implementation runtime only; it does not
substitute for the fresh other-runtime successor. An earlier full a2o TypeScript
check failed in untouched `portable-app-lifecycle.steps.ts` because installed
storage-client declarations lacked `availableOffline`, `UnsupportedLifecycle`,
`UnsupportedCapabilityError` and `AppResidencyView`. No unrelated repair is made.
The locked dependency files match the existing checkout; its installed packages
were reused read-only after the offline package-store lookup failed. The focused
step check above is narrower than a green owner gate. Failed attempts and their
logs remain intact in the evidence directory.

**Verification addendum from the current continuation review:** the earlier
declaration failure is resolved locally. The host rebuilt the owning ignored
dependency artifacts in 17.517 seconds, then `pnpm run typecheck` in `genesis/a2o`
passed in 40.877 seconds, exit 0. Evidence:
`a2o-storage-client-rebuild-20261102T210627Z.json` and its retained build/typecheck
logs. No source, configuration or lockfile changed in that repair. This supersedes
the earlier typecheck blocker without deleting its failed evidence; the final
complete `genesis-a2o` owner gate is still required.
The `revise-acceptance-story` rerun independently passed full typechecking in
35.135 seconds and all 6 local scenarios / 28 steps in 6.136 seconds. Its logs are
`revise-acceptance-a2o-typecheck-20261102T212411455534Z.log` and
`revise-acceptance-a2o-20261102T212411535622Z.log` in the current shift directory.

The current `operations/review-acceptance-story/codex-1.jsonl` also identifies an
unresolved **Windows compatibility blocker** in published EPRFS 0.3.3:
`DirectoryStorage::create` opens directories using `File::open` before syncing,
and deduplicated puts sync a read-only object handle. Retained Windows CI job
125528586847 fails the existing `snapshot seal` journey with Access Denied before
reaching Linux-only restore. Diagnosis and raw failure evidence remain in
`windows-storage-diagnosis-20261102T205356Z.json` and
`brit-ci-4730ba4b-20261102T205010Z-job-125528586847.log`. A verified Windows
namespace-durability implementation, unchanged cross-platform journey passing,
and a new immutable EPRFS release before consumer upgrade are required. No Windows
repair or new release is established by the passing Linux checks. This blocker
and the separate licensing refusal prevent closure.

Native citation sealing passes for this result and both backlog notes when run
from the repository root. Earlier nested-directory invocations could not resolve
root-relative legacy references; the root-scoped rerun resolves them without
rewriting fingerprints. The habit projector passes and leaves its generated root
register unchanged; Brit's scoped
DELTA preserves `status: red` and `active: false`.

## Run the local acceptance stations

Prerequisites: Linux; Rust 1.96.1 installed; the repository's locked JavaScript
dependencies; exact delivered Brit and Ark executables. No conductor, doorway,
Git repository, helper executable inside the task, or network service is needed.
This proves local operation with no peer configured, not a hostile network sandbox.

From the repository root, set the binaries explicitly. Under this bounded shift
the host installs the two fixed paths only after committed source; elsewhere use
the exact binaries delivered by your operator. The toolchain command resolves the
compiler's absolute path, so the test does not search PATH during compilation.

```bash
export BRIT_BIN=/projects/.cargo-target-pool/family/sprint/elohim__brit/dev/debug/brit
export ARK_BIN=/projects/.cargo-target-pool/family/dev/elohim/dev/debug/ark
export RUSTC_BIN="$(rustc +1.96.1 --print sysroot)/bin/rustc"
cd genesis/a2o
pnpm exec cucumber-js --config scripts/brit-native-source-loop.mjs
```

Expected: **6 scenarios passed**, no skipped or pending local steps. The dedicated
profile selects `@local-native` and loads only its own steps. The enclosing feature
retains `@wip`; all Avodah, peer sharing, freshness, commons and publication stories
remain pending. Test directories are disposable and cleaned; retain the command
log in the current shift directory when measuring a delivery.

## Manual journey for an independent successor

Use an empty private rehearsal directory under the successor's authorized evidence
directory. Save the following as `journey.py` there. Supply
`REPO` as the absolute delivered checkout and keep the three executable variables
above. Run `python3 journey.py broken`, then `python3 journey.py corrected`.
Each invocation creates a disposable fixture under `/tmp`, outside Git, and copies
it back beneath the script directory after completion; retain it and stdout/stderr
as evidence. A successor grant must permit that disposable scratch as well as its
durable evidence directory. Do not restore inside the shared checkout: the existing
Git writer guard correctly applies to a destination inside a repository.
A refusal in the broken run is the expected regression
signal, not a product failure or an acceptance verdict.

```python
import hashlib, json, os, pathlib, shutil, subprocess, sys, tempfile, time
base = pathlib.Path(tempfile.mkdtemp(prefix='brit-journey-', dir='/tmp'))
repo = pathlib.Path(os.environ['REPO'])
brit, ark, rustc = [os.environ[k] for k in ('BRIT_BIN', 'ARK_BIN', 'RUSTC_BIN')]
env = {k: v for k, v in os.environ.items() if not k.startswith('GIT_')}
env['ARK_BIN'] = ark
def call(*args):
    return json.loads(subprocess.check_output([brit, *map(str, args)], cwd=base, env=env, text=True, timeout=60))
source, store = base / 'source', base / 'store'
source.mkdir()
(source / 'value').write_bytes(b'A: captured bytes\n')
(source / 'omitted').write_bytes(b'unselected')
a = call('snapshot', 'capture', '--source', source, '--store', store, '--', 'value')
assert a['published'] is False
(source / 'value').write_bytes(b'B: continuing work\n')
call('snapshot', 'restore', '--root', a['root'], '--store', store, '--destination', base / 'restored')
assert (base / 'restored/value').read_bytes() == b'A: captured bytes\n'
assert not (base / 'restored/omitted').exists()
artifact, run = base / 'fixture', base / 'run'
subprocess.run([rustc, '--edition=2021', '-C', 'opt-level=1', str(repo / 'genesis/a2o/scripts/brit-native-source-fixture.rs'), '-o', str(artifact)], check=True, timeout=60)
digest = hashlib.sha256(artifact.read_bytes()).hexdigest()
mode = sys.argv[1]
assert mode in ('broken', 'corrected')
request = {
    'method': {'name': 'brit-gate:native-source-loop', 'step': 'native-source-loop', 'deterministic': False},
    'child': {'artifact': {'pinned': {'sha256': '0' * 64 if mode == 'broken' else digest}},
              'argv': ['{artifact}', '{input}', '{output}'],
              'env': {'PATH': '', 'HOME': str(run / 'home')}, 'envScrub': True},
    'outputs': ['answer'],
}
request_path = base / 'request.json'
request_path.write_text(json.dumps(request))
with (base / 'stdout').open('w+') as out, (base / 'stderr').open('w+') as err:
    child = subprocess.Popen([brit, 'build', 'run', '--root', a['root'], '--store', str(store), '--request', str(request_path), '--artifact', str(artifact), '--run-dir', str(run)], cwd=base, env=env, stdout=out, stderr=err)
    try:
        if mode == 'broken':
            assert child.wait(timeout=30) != 0
            err.seek(0)
            assert 'digest mismatch' in err.read()
            assert not (run / 'output/ready').exists()
            print('expected wrong-artifact refusal', base)
        else:
            until = time.monotonic() + 30
            while not (run / 'output/ready').exists():
                assert child.poll() is None and time.monotonic() < until, base
                time.sleep(.02)
            (source / 'value').write_bytes(b'B: edited while A ran\n')
            (run / 'output/continue').write_bytes(b'continue')
            assert child.wait(timeout=30) == 0
            out.seek(0)
            result = json.load(out)
            assert result['outcome'] == 'done' and result['record'] is None and result['hit'] is False
            assert result['key']['input'] == {'form': 'projected', 'cid': a['root']}
            call('snapshot', 'restore', '--root', result['output'], '--store', store, '--destination', base / 'answer')
            assert sorted(p.name for p in (base / 'answer').iterdir()) == ['answer']
            assert (base / 'answer/answer').read_bytes() == b'A: captured bytes\n'
            assert (source / 'value').read_bytes() == b'B: edited while A ran\n'
            b = call('snapshot', 'capture', '--source', source, '--store', store, '--', 'value')
            assert b['root'] != result['key']['input']['cid']
            print('corrected A/B journey passed', base)
    finally:
        if child.poll() is None:
            child.terminate()  # Ark owns settlement of its task descendants.
            child.wait(timeout=20)
archive = pathlib.Path(__file__).resolve().parent / base.name
shutil.copytree(base, archive)
print('retained fixture evidence', archive)
shutil.rmtree(base)
```

The successor must be a fresh other-runtime session with no implementation
conversation. Record exact parent and Brit revisions, executable digests, actual
commands, broken refusal, corrected success, elapsed time and interventions.
Do not edit delivered source or use the rehearsal to fulfill or accept work.
The prepared current-shift successor wrapper belongs to the root; this operation
does not launch it. Missing prerequisites remain explicit.

## Coverage boundaries and successor stations

Published EPRFS capture tests cover text/binary/byte names, executable modes,
literal links, explicit selection, source/store overlap, open/read/ancestor races,
concurrent capture, mid-read mutation and interrupted writes. Storage and exact
restore tests cover write/sync/promotion failures, real process interruption,
retained residue, corruption versus absence, and the fixed size limits. The
current native-task review and repair evidence above retain those results; this
operation does not republish or rewrite those packages.

Ark native CLI tests cover fixed A during B edits, replacing the original
executable, wrong digest, unknown placeholders, explicit environment, incompatible
Git/reuse options, non-whole scope, missing/corrupt objects, input inventory
mutation, confined/missing outputs, failed exit and descendant settlement.
Interruption while running and immediately before terminal answer have separate
regressions. The new declaration test separates artifact/argv/env/output identity
changes. Stored executable or hook-shaped files remain inert during capture and
restore; execution is an explicit later act.

Limits remain **64 MiB per object and 1 GiB per expanded closure**. This is a
Linux trusted-parent contract for fresh destinations. It does not promise an
atomic snapshot of externally changing files, hostile-code isolation, detection
of a malicious mutate-and-restore sequence, a hermetic build closure, reusable
attestation, automatic publication or independent acceptance. Failed runs and
interrupted staging may retain inspectable private residue.

Successors, in order:

1. Hydrate root-pinned closures from an explicitly selected serving store; never
   expose the whole private store.
2. Establish large-file custody and cache lifecycle without increasing 16 MiB
   peer frames.
3. Add native working contexts/history through existing EPR/REA records;
   Automerge remains the shared working-version direction.
4. Establish publication standing, authority and peer-held availability.
5. Complete Git/LFS interoperability and full execution closures before reusable
   builds.

Story review: three fresh-context rounds, class counts
`3 correctness / 1 interpretability`, `2 / 1`, then `0 / 0` (no preferences).
The missing trigger actions in the preserved pending stories were clarified;
those stories remain unwired. Final reader: READY. Evidence:
`.claude/shifts/brit-c1-20261102/acceptance-story-blind-review.json`.
The current independent technical review found the two issues documented in the
verification addendum; its original transcript remains unchanged. Native appointed
acceptance remains pending.
Dialogue-blocked duration is unmeasured here; no stasis policy or state was edited.
