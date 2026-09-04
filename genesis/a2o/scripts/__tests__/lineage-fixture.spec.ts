/**
 * Holochain Evolution Epic, Task 10 Part 1 — the a2o lineage fixture's
 * TypeScript helpers (mesh reset route + `hc-mesh.sh` reset are Task 10
 * Part 2, another session's; the mesh itself is not touched by this spec —
 * every packager call below runs with `noPut: true`).
 *
 * Part 1 (pure builders) tests `lineage-commitments.ts`'s payload builders
 * and `signingPayloadCid` against the Task 2 mishpat field list
 * (`elohim/holochain/dna/mishpat/zomes/mishpat/src/commitments.rs`
 * `validate_migrates_lineage` / `validate_sunsets_lineage` /
 * `validate_revokes_commitment` / `validate_lineage_signatures`) — no
 * process, no filesystem, no network.
 *
 * Part 2 (candidate manifest assembly) drives `lineage-candidate.ts`'s
 * `mintLineageCandidate` / `lineageReleaseWithoutParent`, which shell the
 * real `epr-release-package.ts` CLI (same rail
 * `runtime-upgrade-propagation.steps.ts` uses) with `noPut: true` — nothing
 * touches a live storage peer. It needs the `node-registry-v2.dna` fixture
 * (built by the `lineage-witness` feature — `elohim/holochain/dna/
 * node-registry/justfile`'s `build-witness` recipe, needs cargo — NOT run by
 * this spec) and the pinned 0.7 `hc` CLI to pack a one-role `.happ` from it
 * and to hash both DNAs. When the `.dna` fixture is absent this whole part
 * SKIPS LOUDLY (`console.warn` + `it.skip`) rather than failing — Part 1
 * still runs regardless.
 */

import { strict as assert } from 'node:assert';
import { existsSync, mkdtempSync, readFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { describe, it } from 'node:test';

import {
  computeDnaHash,
  lineageReleaseWithoutParent,
  mintLineageCandidate,
  NODE_REGISTRY_V2_DNA,
  packOneRoleHapp,
} from '../../steps/delivery/lineage-candidate.js';
import {
  buildMigratesLineagePayload,
  buildRevocationPayload,
  buildSunsetsLineagePayload,
  signingPayloadCid,
  type LineageEvidence,
} from '../../steps/delivery/lineage-commitments.js';

type JsonObject = Record<string, unknown>;

const MIGRATES_LINEAGE = 'migrates-lineage';

const EVIDENCE: LineageEvidence = {
  soak: [{ peer: 'james', greenSecs: 40 }],
  forecast: { class: 'additive' },
  deliberation: { note: 'household reviewed the DNA diff' },
};

// ---------------------------------------------------------------------------
// Part 1 — pure payload builders + signingPayloadCid
// ---------------------------------------------------------------------------

void describe('lineage-commitments — buildMigratesLineagePayload', () => {
  const input = {
    role: 'node_registry',
    fromDnaHash: 'uhC0kyvKwO2J5u3mf52tjASWe0ryhdpNYalrSeMGJODF3OpUxyeoH',
    toDnaHash: 'uhC0kEKiIscIk5BDdethLGMFGLnvSvP2gRP5o74v0vAvoRnEzbiJ1',
    releaseCid: 'bafkrei' + 'r'.repeat(52),
    constitutionRoot: 'uhC0k' + 'C'.repeat(48),
    rosterCid: 'bafkrei' + 's'.repeat(52),
    evidence: EVIDENCE,
    window: { opensAt: '2026-09-04T00:00:00Z', revertUntil: '2026-09-11T00:00:00Z' },
  };

  void it('produces every field validate_migrates_lineage requires', () => {
    const payload = buildMigratesLineagePayload(input);
    for (const field of [
      'action',
      'role',
      'from_dna_hash',
      'to_dna_hash',
      'release_cid',
      'constitution_root',
      'roster_cid',
      'signing_payload_cid',
      'signatures',
      'evidence',
      'window',
    ]) {
      assert.ok(
        Object.prototype.hasOwnProperty.call(payload, field),
        `missing required field: ${field}`
      );
    }
    assert.equal(payload.action, MIGRATES_LINEAGE);
    assert.equal(payload.from_dna_hash, input.fromDnaHash);
    assert.equal(payload.to_dna_hash, input.toDnaHash);
    assert.deepEqual(payload.signatures, []);
  });

  void it('window.opens_at precedes window.revert_until, both Z-suffixed', () => {
    const payload = buildMigratesLineagePayload(input);
    assert.ok(payload.window.opens_at.endsWith('Z'), 'opens_at must be Z-suffixed RFC3339');
    assert.ok(payload.window.revert_until.endsWith('Z'), 'revert_until must be Z-suffixed RFC3339');
    assert.ok(
      payload.window.opens_at < payload.window.revert_until,
      'opens_at must lexicographically precede revert_until'
    );
  });

  void it('signing_payload_cid is a non-empty stable content id', () => {
    const payload = buildMigratesLineagePayload(input);
    assert.ok(payload.signing_payload_cid.length > 0);
    assert.ok(payload.signing_payload_cid.startsWith('b'), 'expected a multibase base32-lower cid');
  });

  void it('omits required_signatures when not given, includes it when given', () => {
    const withoutIt = buildMigratesLineagePayload(input);
    assert.equal('required_signatures' in withoutIt, false);
    const withIt = buildMigratesLineagePayload({ ...input, requiredSignatures: 2 });
    assert.equal(withIt.required_signatures, 2);
  });

  void it('carries through caller-supplied signatures', () => {
    const sigs = [
      { agent: 'uhCAk' + 'A'.repeat(48), signature: Buffer.alloc(64).toString('base64') },
    ];
    const payload = buildMigratesLineagePayload({ ...input, signatures: sigs });
    assert.deepEqual(payload.signatures, sigs);
  });
});

void describe('lineage-commitments — buildSunsetsLineagePayload', () => {
  const input = {
    role: 'node_registry',
    fromDnaHash: 'uhC0kyvKwO2J5u3mf52tjASWe0ryhdpNYalrSeMGJODF3OpUxyeoH',
    toDnaHash: 'uhC0kEKiIscIk5BDdethLGMFGLnvSvP2gRP5o74v0vAvoRnEzbiJ1',
    migrationCommitmentCid: 'uhCEk' + 'A'.repeat(48),
    evidence: EVIDENCE,
    sunsetsAt: '2026-09-20T00:00:00Z',
  };

  void it('produces every field validate_sunsets_lineage requires', () => {
    const payload = buildSunsetsLineagePayload(input);
    for (const field of [
      'action',
      'role',
      'from_dna_hash',
      'to_dna_hash',
      'migration_commitment_cid',
      'signing_payload_cid',
      'signatures',
      'evidence',
      'window',
    ]) {
      assert.ok(
        Object.prototype.hasOwnProperty.call(payload, field),
        `missing required field: ${field}`
      );
    }
    assert.equal(payload.action, 'sunsets-lineage');
    assert.equal(payload.migration_commitment_cid, input.migrationCommitmentCid);
  });

  void it('window.sunsets_at is Z-suffixed RFC3339', () => {
    const payload = buildSunsetsLineagePayload(input);
    assert.ok(payload.window.sunsets_at.endsWith('Z'));
    assert.equal(payload.window.sunsets_at, input.sunsetsAt);
  });
});

void describe('lineage-commitments — buildRevocationPayload', () => {
  void it('a plain revocation carries only action/target_cid/signed_at', () => {
    const payload = buildRevocationPayload('bafkrei' + 't'.repeat(52));
    assert.equal(payload.action, 'revokes-commitment');
    assert.equal(payload.target_cid, 'bafkrei' + 't'.repeat(52));
    assert.ok(payload.signed_at.length > 0);
    assert.equal(payload.target_action, undefined);
    assert.equal(payload.signing_payload_cid, undefined);
    assert.equal(payload.signatures, undefined);
  });

  void it('revoking a migrates-lineage target gains the same quorum shape a lineage commitment carries', () => {
    const payload = buildRevocationPayload('bafkrei' + 'u'.repeat(52), {
      targetAction: MIGRATES_LINEAGE,
    });
    assert.equal(payload.target_action, MIGRATES_LINEAGE);
    assert.ok(payload.signing_payload_cid, 'must gain a signing_payload_cid');
    assert.deepEqual(payload.signatures, []);
  });

  void it('honors an explicit signedAt and requiredSignatures', () => {
    const payload = buildRevocationPayload('bafkrei' + 'v'.repeat(52), {
      targetAction: 'sunsets-lineage',
      signedAt: '2026-09-05T00:00:00Z',
      requiredSignatures: 2,
    });
    assert.equal(payload.signed_at, '2026-09-05T00:00:00Z');
    assert.equal(payload.required_signatures, 2);
  });
});

void describe('lineage-commitments — signingPayloadCid', () => {
  void it('is deterministic for the same payload', () => {
    const payload = { a: 1, b: { c: 2, d: [3, 4] } };
    assert.equal(signingPayloadCid(payload), signingPayloadCid(payload));
  });

  void it('is independent of field-insertion order (canonicalized)', () => {
    const a = signingPayloadCid({ a: 1, b: 2, c: { x: 1, y: 2 } });
    const b = signingPayloadCid({ c: { y: 2, x: 1 }, b: 2, a: 1 });
    assert.equal(a, b);
  });

  void it('differs when content differs', () => {
    const a = signingPayloadCid({ a: 1 });
    const b = signingPayloadCid({ a: 2 });
    assert.notEqual(a, b);
  });

  void it('strips signing_payload_cid and signatures before hashing — idempotent across build stages', () => {
    const unsigned = { action: MIGRATES_LINEAGE, role: 'node_registry' };
    const cid = signingPayloadCid(unsigned);
    const signed = {
      ...unsigned,
      signing_payload_cid: cid,
      signatures: [{ agent: 'x', signature: 'y' }],
    };
    assert.equal(signingPayloadCid(signed), cid);
  });
});

// ---------------------------------------------------------------------------
// Part 2 — candidate manifest assembly (shells the real packager, noPut: true)
// ---------------------------------------------------------------------------

/** Returns the packed `.happ` path, or `''` when the `node-registry-v2.dna` fixture is absent. */
function resolveV2HappForTest(): string {
  if (!existsSync(NODE_REGISTRY_V2_DNA)) return '';
  const dir = mkdtempSync(path.join(tmpdir(), 'lineage-fixture-happ-'));
  return packOneRoleHapp(NODE_REGISTRY_V2_DNA, dir);
}

const v2HappPath = resolveV2HappForTest();
const STORAGE_BASE_URL = 'http://127.0.0.1:8090';

function tempOutPath(filename: string): string {
  return path.join(mkdtempSync(path.join(tmpdir(), 'lineage-fixture-out-')), filename);
}

function skippedNoFixture(): void {
  console.warn(
    'SKIPPED: node-registry-v2.dna not found at ' +
      NODE_REGISTRY_V2_DNA +
      ' — the lineage-witness feature build (elohim/holochain/dna/node-registry: ' +
      'just build-witness, needs cargo) has not produced it in this workspace. ' +
      'Part 1 (pure builders) still ran; Part 2 (candidate manifest assembly) skipped.'
  );
}

void describe('lineage-candidate — mintLineageCandidate / lineageReleaseWithoutParent', () => {
  if (!v2HappPath) {
    skippedNoFixture();
    void it.skip(
      'candidate manifest assembly (SKIPPED — no node-registry-v2.dna fixture)',
      skippedNoFixture
    );
    return;
  }

  const v2DnaHash = computeDnaHash(NODE_REGISTRY_V2_DNA);
  const v1DnaPath = path.join(path.dirname(NODE_REGISTRY_V2_DNA), 'node-registry-v1.dna');
  const v1DnaHash = existsSync(v1DnaPath) ? computeDnaHash(v1DnaPath) : `uhC0k${'A'.repeat(48)}`;
  const pathCommitmentCid = `uhCEk${'A'.repeat(48)}`;
  const channelId = 'runtime:happ:elohim:lineage-fixture-spec';
  const discipline = { soakSecs: 30, attestationThreshold: 1, canary: 'james' };

  void it('mints a happ-lineage candidate manifest with migrateFrom/lineage/adoptionDiscipline.path', async () => {
    const out = tempOutPath('candidate.json');
    const result = await mintLineageCandidate({
      v2HappPath,
      role: 'node_registry',
      v1DnaHash,
      v2DnaHash,
      pathCommitmentCid,
      channelId,
      storageBaseUrl: STORAGE_BASE_URL,
      out,
      discipline,
      noPut: true,
    });
    assert.equal(result.manifestPath, out);
    const manifest = JSON.parse(readFileSync(out, 'utf8')) as JsonObject;
    assert.equal(manifest['artifactClass'], 'happ-lineage');
    const roles = (manifest['appliesTo'] as JsonObject)['roles'] as JsonObject;
    const nodeRegistry = roles['node_registry'] as JsonObject;
    assert.equal(nodeRegistry['migrateFrom'], v1DnaHash);
    assert.deepEqual(nodeRegistry['lineage'], [v1DnaHash]);
    assert.deepEqual((manifest['adoptionDiscipline'] as JsonObject)['path'], {
      commitmentCid: pathCommitmentCid,
    });
  });

  void it("lineageReleaseWithoutParent's manifest names no migrateFrom (Station 1 negative control)", async () => {
    const out = tempOutPath('no-parent.json');
    const result = await lineageReleaseWithoutParent({
      v2HappPath,
      role: 'node_registry',
      v2DnaHash,
      channelId,
      storageBaseUrl: STORAGE_BASE_URL,
      out,
      discipline,
      noPut: true,
    });
    assert.equal(result.manifestPath, out);
    const manifest = JSON.parse(readFileSync(out, 'utf8')) as JsonObject;
    const roles = (manifest['appliesTo'] as JsonObject)['roles'] as JsonObject;
    const nodeRegistry = roles['node_registry'] as JsonObject;
    assert.equal(nodeRegistry['migrateFrom'], undefined);
    assert.equal(nodeRegistry['lineage'], undefined);
    assert.equal(nodeRegistry['dnaHash'], v2DnaHash);
  });

  void it('refuses to mint a happ-lineage candidate without a path-commitment cid', async () => {
    const out = tempOutPath('no-commitment.json');
    await assert.rejects(async () =>
      mintLineageCandidate({
        v2HappPath,
        role: 'node_registry',
        v1DnaHash,
        v2DnaHash,
        channelId,
        storageBaseUrl: STORAGE_BASE_URL,
        out,
        discipline,
        noPut: true,
      })
    );
  });

  void it('refuses to mint when the declared v2DnaHash does not match the bundle', async () => {
    const out = tempOutPath('bad-hash.json');
    await assert.rejects(async () =>
      mintLineageCandidate({
        v2HappPath,
        role: 'node_registry',
        v1DnaHash,
        v2DnaHash: `uhC0k${'Z'.repeat(48)}`,
        pathCommitmentCid,
        channelId,
        storageBaseUrl: STORAGE_BASE_URL,
        out,
        discipline,
        noPut: true,
      })
    );
  });
});
