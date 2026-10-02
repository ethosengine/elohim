/** Test-only orchestration of the maintained one-item publisher and exact witness ceremony. */
import { strict as assert } from 'node:assert';
import { execFile } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';

import { decodeHashFromBase64, encodeHashToBase64 } from '@holochain/client';

import { buildContentInput } from '../../../seeder/src/content-input.js';
import {
  checkClosingPublication,
  checkClosingStage,
} from '../../steps/lamad/commons-path-steward-publish.helpers.js';

import { connectConductor } from './steward-conductor.js';
import { writeCustodyJson } from './steward-credential.js';
import { loadRepoItem } from './steward-items.js';

import type { ConductorOptions } from './steward-conductor.js';
import type { CredentialPlan } from './steward-credential.js';
import type { PublicationReceipt } from './steward-publication-receipt.js';
import type { ClosingPublicationContext } from '../../steps/lamad/commons-path-steward-publish.helpers.js';

const execute = promisify(execFile);
const credentialScript = 'steward-credential';
const a2o = fileURLToPath(new URL('../../', import.meta.url));
export interface ClosingPublicationFixture extends ClosingPublicationContext {
  /** Root/controller transport and existing custody are verified before enabling this fixture. */
  cheConnection: string;
  storage: string;
  binding: string;
  canonicalRoots: string;
  delegations: string;
  grantorConnections: string;
  ownerDescriptor: string;
  nativeReceivers: string;
  witnessTemplate: string;
  witnessExerciseConnection: ConductorOptions;
  witnessIssuer: string;
  witnessDna: string;
  dataDirs: [string, string];
  receiptDir: string;
  privateRunDir: string;
}
function json<T>(path: string): T {
  return JSON.parse(readFileSync(path, 'utf8')) as T;
}
function remaining(deadline: number): number {
  const left = deadline - Date.now();
  assert.ok(left > 0, 'shared 75-second publication deadline elapsed; no further station allowed');
  return left;
}
async function cli(
  script: 'steward-publish' | 'steward-credential',
  args: string[],
  deadline: number
) {
  try {
    return await execute(process.execPath, ['--import', 'tsx', `scripts/${script}.ts`, ...args], {
      cwd: a2o,
      timeout: remaining(deadline),
      maxBuffer: 1_048_576,
      encoding: 'utf8',
    });
  } catch (error) {
    const result = error as { code?: number; stdout?: string; stderr?: string };
    // Keep credential/transport details out of Cucumber's public failure text.
    return { code: result.code ?? -1, stdout: result.stdout ?? '', stderr: result.stderr ?? '' };
  }
}
function argsFor(f: ClosingPublicationFixture, dataDir: string, deadline: number): string[] {
  return [
    f.id,
    '--connection',
    f.cheConnection,
    '--storage',
    f.storage,
    '--data-dir',
    dataDir,
    '--device-agent',
    f.che,
    '--dna-hash',
    f.dna,
    '--binding',
    f.binding,
    '--canonical-roots',
    f.canonicalRoots,
    '--delegations',
    f.delegations,
    '--grantor-connections',
    f.grantorConnections,
    '--receipt-dir',
    f.receiptDir,
    '--await-timeout',
    String(Math.max(1, Math.floor(remaining(deadline) / 1000))),
  ];
}

/** There is no arbitrary command fixture, new root, device, or public grant operation here. */
export async function runClosingPublication(f: ClosingPublicationFixture): Promise<string[]> {
  assert.equal(f.dataDirs.length, 2);
  const first = loadRepoItem(f.dataDirs[0], f.id);
  const second = loadRepoItem(f.dataDirs[1], f.id);
  assert.ok(first.item && second.item, 'two single-item source files required');
  assert.equal(first.item.json.reach, 'commons');
  assert.equal(second.item.json.reach, 'commons');
  assert.equal(first.item.kind, 'content');
  assert.equal(second.item.kind, 'content');
  assert.notEqual(
    buildContentInput(first.item.json).contentBody,
    buildContentInput(second.item.json).contentBody,
    'consecutive updates must change the authored body'
  );
  const che = json<ConductorOptions & { connection?: ConductorOptions }>(f.cheConnection);
  assert.equal((che.connection ?? che).expectedAgent, f.che);
  assert.equal((che.connection ?? che).expectedDna, f.dna);
  const receivers = json<Record<string, ConductorOptions>>(f.nativeReceivers);
  assert.ok(receivers.adam, 'one designated independent receiver is required');
  for (const receiver of Object.values(receivers)) {
    assert.equal(receiver.expectedDna, f.dna);
    assert.notEqual(receiver.expectedAgent, f.che);
  }
  assert.equal(receivers.adam.expectedAgent, f.adam);
  assert.equal(receivers.adam.expectedDna, f.dna);
  const owner = json<{
    connection: ConductorOptions;
    mandate: {
      issuer: string;
      dna: string;
      delegate: string;
      binding: string;
      subjects: { id: string; root: string }[];
    };
  }>(f.ownerDescriptor);
  assert.equal(owner.connection.expectedAgent, f.matthew);
  assert.equal(owner.connection.expectedDna, f.dna);
  assert.equal(owner.mandate.issuer, f.matthew);
  assert.equal(owner.mandate.dna, f.dna);
  assert.equal(owner.mandate.delegate, f.che);
  assert.equal(owner.mandate.binding, f.binding);
  assert.deepEqual(owner.mandate.subjects, [{ id: f.id, root: f.root }]);
  const template = json<CredentialPlan>(f.witnessTemplate);
  assert.equal(template.connection.expectedAgent, f.witnessIssuer);
  assert.equal(template.connection.expectedDna, f.witnessDna);
  assert.equal(f.witnessExerciseConnection.expectedAgent, f.witnessIssuer);
  assert.equal(f.witnessExerciseConnection.expectedDna, f.witnessDna);
  assert.equal(template.mandate.delegate, null);
  assert.deepEqual(template.mandate.subjects, []);
  assert.deepEqual(template.mandate.operations, ['witness_device_publication']);
  assert.equal(template.mandate.binding, f.binding);
  mkdirSync(f.privateRunDir, { recursive: true, mode: 0o700 });
  const heads: string[] = [];
  const source = await connectConductor(che.connection ?? che);
  try {
    const response = await fetch(`${f.storage}/db/content/${encodeURIComponent(f.id)}`, {
      signal: AbortSignal.timeout(5000),
    });
    assert.equal(
      response.status,
      200,
      'originalChe must already hold the actual signed root; no create/bulk bootstrap allowed'
    );
    const row = (await response.json()) as { id: string; dhtAnchorHash?: string };
    assert.equal(row.id, f.id);
    assert.ok(row.dhtAnchorHash);
    const lineage = await source.call<{
      content_id: string;
      root_action_hash: Uint8Array;
      root_author: Uint8Array;
      truncated: boolean;
    }>(
      'get_content_lineage',
      { action_hash: decodeHashFromBase64(row.dhtAnchorHash), local: true },
      5000
    );
    assert.equal(lineage.content_id, f.id);
    assert.equal(lineage.truncated, false);
    assert.equal(encodeHashToBase64(lineage.root_action_hash), f.root);
    assert.equal(encodeHashToBase64(lineage.root_author), f.matthew);
  } finally {
    await source.close();
  }
  const adam = await connectConductor(receivers.adam);
  try {
    for (const [round, dataDir] of f.dataDirs.entries()) {
      const startedAt = Date.now(),
        deadline = startedAt + 75_000;
      const roundDir = join(f.privateRunDir, `round-${round + 1}`);
      mkdirSync(roundDir, { mode: 0o700 });
      const args = argsFor(f, dataDir, deadline);
      const stage = await cli('steward-publish', [...args, '--stage-local'], deadline);
      writeCustodyJson(join(roundDir, 'stage-output.json'), stage);
      const receiptPath = join(
        f.receiptDir,
        `${createHash('sha256').update(f.id).digest('hex')}.json`
      );
      const pending = json<PublicationReceipt>(receiptPath);
      assert.ok(!pending.declaredAt && Date.parse(pending.authoredAt) >= startedAt);
      assert.equal(pending.delegation?.rootActionHash, f.root);
      assert.equal(pending.delegation?.grantor, f.matthew);
      assert.equal(pending.delegation?.delegate, f.che);
      assert.equal(pending.agent, f.che);
      assert.ok(pending.head && pending.head !== heads.at(-1));
      checkClosingStage(stage, f.id, pending.head);
      const approval = await cli(
        credentialScript,
        ['stage-head', f.ownerDescriptor, f.id, pending.head],
        deadline
      );
      writeCustodyJson(join(roundDir, 'approval-output.json'), approval);
      assert.ok(!('code' in approval), 'exact root-author provisional approval failed');
      const approvals = json<Record<string, { head: string; rootAcceptance: string }>>(
        join(resolve(f.ownerDescriptor, '..'), 'root-approvals.json')
      );
      assert.equal(approvals[f.id]?.head, pending.head);
      const bytes = (hash: string) => Array.from(decodeHashFromBase64(hash));
      const payload = {
        device: {
          binding: bytes(f.binding),
          expected_device: bytes(f.che),
          expected_content_dna: bytes(f.dna),
        },
        content_root: bytes(f.root),
        content_head: bytes(pending.head),
        root_acceptance: bytes(approvals[f.id].rootAcceptance),
      };
      const plan: CredentialPlan = {
        ...template,
        outputDir: join(roundDir, 'human-witness'),
        mandate: {
          ...template.mandate,
          valid_until: Math.min(template.mandate.valid_until, (Date.now() + 900_000) * 1000),
          exact_payload_json: JSON.stringify(payload),
        },
      };
      assert.ok(plan.mandate.valid_until > Date.now() * 1000, 'fixed controller template expired');
      const planPath = join(roundDir, 'witness-plan.json');
      writeCustodyJson(planPath, plan, true);
      const issued = await cli(credentialScript, ['issue', planPath], deadline);
      writeCustodyJson(join(roundDir, 'issuance-output.json'), issued);
      assert.ok(!('code' in issued), 'exact controller witness invocation issuance failed');
      const descriptorPath = join(plan.outputDir, 'descriptor.json');
      const descriptor = json<{ connection: ConductorOptions; mandate: unknown }>(descriptorPath);
      descriptor.connection = {
        ...f.witnessExerciseConnection,
        signingCredentialsDir: plan.outputDir,
      };
      writeCustodyJson(descriptorPath, descriptor);
      const exercised = await cli(
        credentialScript,
        ['exercise-ceremony', descriptorPath],
        deadline
      );
      writeCustodyJson(join(roundDir, 'witness-output.json'), exercised);
      assert.ok(!('code' in exercised), 'exact Human witness exercise failed');
      const witness = json<{ action_hash: number[] }>(join(plan.outputDir, 'ceremony-result.json'));
      const witnessesPath = join(roundDir, 'device-witnesses.json');
      writeCustodyJson(witnessesPath, {
        [f.id]: {
          head: pending.head,
          witness: encodeHashToBase64(new Uint8Array(witness.action_hash)),
        },
      });
      const resumed = await cli(
        'steward-publish',
        [
          ...argsFor(f, dataDir, deadline),
          '--device-witnesses',
          witnessesPath,
          '--native-receivers',
          f.nativeReceivers,
          ...f.doorways.flatMap(peer => ['--await-peer', peer]),
        ],
        deadline
      );
      writeCustodyJson(join(roundDir, 'publication-output.json'), resumed);
      assert.ok(
        !('code' in resumed),
        'same-head publication did not close within the shared deadline'
      );
      const receipt = json<PublicationReceipt>(receiptPath);
      assert.equal(
        receipt.head,
        pending.head,
        'resume must declare the original exact authored action'
      );
      const checked = checkClosingPublication(f, receipt, resumed.stdout, startedAt, heads.at(-1));
      const lineage = await adam.call<{
        root_author: Uint8Array;
        root_action_hash: Uint8Array;
        content_id: string;
        truncated: boolean;
        candidates: {
          action_hash: Uint8Array;
          predecessor: Uint8Array | null;
          author: Uint8Array | null;
          in_root: boolean;
          fetch_outcome: string;
        }[];
      }>(
        'get_content_lineage',
        { action_hash: decodeHashFromBase64(receipt.head), local: true },
        remaining(deadline)
      );
      assert.equal(lineage.content_id, f.id);
      assert.equal(lineage.truncated, false);
      assert.equal(encodeHashToBase64(lineage.root_action_hash), f.root);
      assert.equal(encodeHashToBase64(lineage.root_author), f.matthew);
      const update = lineage.candidates.find(
        candidate => encodeHashToBase64(candidate.action_hash) === receipt.head
      );
      assert.ok(
        update?.in_root && update.predecessor && update.author,
        'exact signed Update must have a predecessor'
      );
      assert.equal(encodeHashToBase64(update.author), f.che);
      if (heads.length) assert.equal(encodeHashToBase64(update.predecessor), heads.at(-1));
      assert.ok(Date.now() <= deadline, 'full ceremony exceeded 75 seconds');
      writeCustodyJson(join(roundDir, 'closing-proof.json'), {
        startedAt,
        deadline,
        head: checked.head,
      });
      heads.push(checked.head);
    }
    return heads;
  } finally {
    await adam.close();
  }
}
