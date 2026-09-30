/* eslint-disable @typescript-eslint/no-floating-promises -- node:test owns test promises. */
import { strict as assert } from 'node:assert';
import { mkdtemp, readFile, readdir, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { it } from 'node:test';

import { encodeHashToBase64 } from '@holochain/client';

import { scopedCredentials } from '../device-ceremony.js';
import { loadSigningCredentials } from '../lib/steward-conductor.js';

import type { ScopedCredentialAdmin, ScopedCredentialTarget } from '../device-ceremony.js';
import type { AppInfo, CapGrantInfo, CellId } from '@holochain/client';

const hash = (prefix: number, byte: number) =>
  new Uint8Array([132, prefix, 36, ...new Uint8Array(36).fill(byte)]);
async function fixture() {
  const dir = await mkdtemp(join(tmpdir(), 'scoped-credentials-'));
  const cell: CellId = [hash(45, 1), hash(32, 2)];
  const app = {
    installed_app_id: 'elohim',
    agent_pub_key: cell[1],
    cell_info: {
      lamad: [{ type: 'provisioned', value: { cell_id: cell } }],
      mishpat: [{ type: 'provisioned', value: { cell_id: [hash(45, 3), cell[1]] } }],
    },
  } as unknown as AppInfo;
  const target: ScopedCredentialTarget = {
    conductor: {
      adminWs: 'ws://localhost:1',
      appWs: 'ws://localhost:2',
      appId: 'elohim',
      role: 'lamad',
      zome: 'content_store',
      expectedAgent: encodeHashToBase64(cell[1]),
      expectedDna: encodeHashToBase64(cell[0]),
      signingCredentialsDir: dir,
    },
    functions: ['get_content_lineage', 'resolve_canonical_election', 'resolve_content_head_local'],
  };
  const grants: CapGrantInfo[] = [];
  let writes = 0,
    loseResponse = false;
  const admin: ScopedCredentialAdmin = {
    listApps: async () => Promise.resolve([app]),
    listGrants: async () => Promise.resolve([[cell, grants]]),
    grant: async request => {
      writes++;
      assert.deepEqual(request.cell_id, cell);
      const pending = await readdir(join(dir, 'pending'));
      assert.equal(pending.length, 1, 'signer must be durable BEFORE grant call');
      const saved = JSON.parse(await readFile(join(dir, 'pending', pending[0]), 'utf8')) as {
        capSecret: string;
      };
      assert.equal(
        Buffer.from(
          request.cap_grant.access.type === 'assigned' ? request.cap_grant.access.value.secret : []
        ).toString('hex'),
        saved.capSecret
      );
      const action = hash(41, writes);
      grants.push({ cap_grant: request.cap_grant, action_hash: action, created_at: 1 });
      if (loseResponse) throw Error('private-network-detail');
      return action;
    },
    close: async () => Promise.resolve(),
  };
  return {
    dir,
    cell,
    app,
    target,
    grants,
    admin,
    writes: () => writes,
    loseResponse: () => {
      loseResponse = true;
    },
    cleanup: async () => rm(dir, { recursive: true, force: true }),
  };
}

it('preflights every target before any grant and excludes identity roles and all-functions grants', async () => {
  const f = await fixture();
  try {
    const other = structuredClone(f.target);
    other.conductor.expectedDna = encodeHashToBase64(hash(45, 8));
    other.conductor.signingCredentialsDir = join(f.dir, 'other');
    await assert.rejects(
      scopedCredentials({ credentialTargets: [f.target, other] }, async () =>
        Promise.resolve(f.admin)
      ),
      /DNA mismatch/
    );
    assert.equal(f.writes(), 0);
    assert.deepEqual(await readdir(f.dir), []);
    const output = await scopedCredentials({ credentialTargets: [f.target] }, async () =>
      Promise.resolve(f.admin)
    );
    assert.equal(f.writes(), 1);
    assert.deepEqual(f.grants[0].cap_grant.functions, {
      type: 'listed',
      value: [
        ['content_store', 'get_content_lineage'],
        ['content_store', 'resolve_canonical_election'],
        ['content_store', 'resolve_content_head_local'],
      ],
    });
    assert.equal(output[0].capActionHash, encodeHashToBase64(f.grants[0].action_hash));
    await scopedCredentials({ credentialTargets: [f.target] }, async () =>
      Promise.resolve(f.admin)
    );
    assert.equal(f.writes(), 1, 'existing native scoped grant is reused');
  } finally {
    await f.cleanup();
  }
});

it('lost grant response stays pending and explicit resume recovers only the exact native action', async () => {
  const f = await fixture();
  try {
    f.loseResponse();
    await assert.rejects(
      scopedCredentials({ credentialTargets: [f.target] }, async () => Promise.resolve(f.admin)),
      /pending signer preserved/
    );
    assert.deepEqual(await readdir(f.dir), ['pending']);
    await assert.rejects(loadSigningCredentials(f.dir, f.cell), /explicit enrollment required/);
    await assert.rejects(
      scopedCredentials({ credentialTargets: [f.target] }, async () => Promise.resolve(f.admin)),
      /explicit resumePending/
    );
    assert.equal(f.writes(), 1);
    const output = await scopedCredentials(
      { credentialTargets: [f.target], resumePending: true },
      async () => Promise.resolve(f.admin)
    );
    assert.equal(f.writes(), 1);
    assert.equal(output[0].capActionHash, encodeHashToBase64(f.grants[0].action_hash));
  } finally {
    await f.cleanup();
  }
});

it('missing, revoked, or wider pending grants never trigger a replacement capability', async () => {
  const f = await fixture();
  try {
    f.loseResponse();
    await assert.rejects(
      scopedCredentials({ credentialTargets: [f.target] }, async () => Promise.resolve(f.admin))
    );
    const original = structuredClone(f.grants[0]);
    const resume = async () =>
      scopedCredentials({ credentialTargets: [f.target], resumePending: true }, async () =>
        Promise.resolve(f.admin)
      );
    f.grants.length = 0;
    await assert.rejects(resume(), /not found/);
    f.grants.push({ ...original, revoked_at: 2 });
    await assert.rejects(resume(), /revoked or differs/);
    f.grants[0] = structuredClone(original);
    f.grants[0].cap_grant.functions = { type: 'all' };
    await assert.rejects(resume(), /revoked or differs/);
    f.grants[0] = structuredClone(original);
    f.grants.push(structuredClone(original));
    await assert.rejects(resume(), /Ambiguous/);
    assert.equal(f.writes(), 1);
    assert.deepEqual(await readdir(f.dir), ['pending']);
  } finally {
    await f.cleanup();
  }
});

it('mismatched app, cell actor, unlisted functions and legacy files fail closed', async () => {
  const f = await fixture();
  try {
    const config = { credentialTargets: [f.target] };
    f.app.agent_pub_key = hash(32, 7);
    await assert.rejects(
      scopedCredentials(config, async () => Promise.resolve(f.admin)),
      /app or role mismatch/
    );
    f.app.agent_pub_key = f.cell[1];
    const info = f.app.cell_info.lamad[0];
    assert.equal(info.type, 'provisioned');
    if (info.type !== 'provisioned') throw Error('fixture role');
    info.value.cell_id = [f.cell[0], hash(32, 8)];
    await assert.rejects(
      scopedCredentials(config, async () => Promise.resolve(f.admin)),
      /cell agent or DNA mismatch/
    );
    info.value.cell_id = f.cell;
    f.target.functions.push('create_self_revocation');
    await assert.rejects(
      scopedCredentials(config, async () => Promise.resolve(f.admin)),
      /listed content functions/
    );
    f.target.functions.pop();
    const name = `${f.cell.map(h => Buffer.from(h).toString('hex')).join('-')}.json`;
    await writeFile(join(f.dir, name), JSON.stringify({ keypair: 'legacy-private-placeholder' }));
    await assert.rejects(
      scopedCredentials(config, async () => Promise.resolve(f.admin)),
      /scope differs/
    );
    assert.equal(f.writes(), 0);
  } finally {
    await f.cleanup();
  }
});

it('explicit resume cannot generate a new signer when durable state is missing', async () => {
  const f = await fixture();
  try {
    await assert.rejects(
      scopedCredentials({ credentialTargets: [f.target], resumePending: true }, async () =>
        Promise.resolve(f.admin)
      ),
      /Missing durable pending state/
    );
    assert.equal(f.writes(), 0);
    assert.deepEqual(await readdir(f.dir), []);
  } finally {
    await f.cleanup();
  }
});
