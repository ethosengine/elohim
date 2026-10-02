/** Explicit issuance: node --import tsx scripts/steward-credential.ts issue PLAN.json
 * PLAN names an owner admin connection, existing custody, exact subjects, binding,
 * operations, policy, expiry and a new output directory. It contains no key bytes.
 * Native/doorway clients subsequently reuse descriptor.connection for the same
 * ceremony and publication; they never ask an invocation key to mint authority.
 */
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { pathToFileURL } from 'node:url';

import { decodeHashFromBase64, encodeHashToBase64 } from '@holochain/client';

import { connectConductor } from './lib/steward-conductor.js';
import {
  issueCredential,
  ceremonyResultDocument,
  readCredentialPlan,
  refreshHostedCredential,
  writeCustodyJson,
} from './lib/steward-credential.js';
import {
  delegationDocument,
  delegationWire,
  type HeadDelegationDocument,
  type HeadDelegationWire,
} from './lib/steward-delegation.js';

import type { ConductorOptions } from './lib/steward-conductor.js';
import type { InvocationMandate } from './lib/steward-credential.js';

export async function main(
  argv: string[],
  connect: typeof connectConductor = connectConductor
): Promise<void> {
  if (argv.length === 4 && argv[0] === 'stage-head') {
    const descriptor = JSON.parse(readFileSync(argv[1], 'utf8')) as {
      connection: ConductorOptions;
      mandate: InvocationMandate;
    };
    const grants = JSON.parse(
      readFileSync(join(dirname(argv[1]), 'delegations.json'), 'utf8')
    ) as Record<string, HeadDelegationDocument>;
    const id = argv[2],
      head = argv[3],
      grant = grants[id];
    if (
      !grant ||
      !descriptor.mandate.operations.includes('stage_delegated_head_acceptance') ||
      !descriptor.mandate.subjects.some(s => s.id === id && s.root === grant.rootActionHash)
    )
      throw new Error('credential does not authorize this exact provisional root approval');
    const c = await connect(descriptor.connection);
    try {
      const m = descriptor.mandate;
      if (c.agent !== m.issuer || c.dna !== m.dna || c.requester !== m.requester)
        throw new Error('approval connection differs from the exact credential');
      const approval = await c.call<{ witness_action_hash: Uint8Array; accepted_at: number }>(
        'stage_delegated_head_acceptance',
        { id, head_action_hash: decodeHashFromBase64(head), delegation: delegationWire(grant) }
      );
      const path = join(dirname(argv[1]), 'root-approvals.json');
      const saved = existsSync(path)
        ? (JSON.parse(readFileSync(path, 'utf8')) as Record<string, unknown>)
        : {};
      saved[id] = {
        head,
        rootAcceptance: encodeHashToBase64(approval.witness_action_hash),
        acceptedAt: approval.accepted_at,
      };
      writeCustodyJson(path, saved);
      console.log(
        'exact provisional root approval recorded; controller reconciliation still required'
      );
      return;
    } finally {
      await c.close();
    }
  }
  if (
    (argv[0] === 'grant-heads' && (argv.length === 2 || argv.length === 3)) ||
    (argv[0] === 'exercise-ceremony' && argv.length === 2)
  ) {
    const descriptor = JSON.parse(readFileSync(argv[1], 'utf8')) as {
      connection: ConductorOptions;
      mandate: InvocationMandate;
    };
    const m = descriptor.mandate;
    // Select execution only; the issued mandate and requester custody stay intact.
    const selectedId = argv[0] === 'grant-heads' ? argv[2] : undefined;
    const subjects =
      selectedId === undefined ? m.subjects : m.subjects.filter(s => s.id === selectedId);
    if (selectedId !== undefined && subjects.length !== 1)
      throw new Error('selected subject must name one exact root in the issued mandate');
    const c = await connect(descriptor.connection);
    try {
      if (c.agent !== m.issuer || c.dna !== m.dna || c.requester !== m.requester)
        throw new Error('ceremony connection differs from its exact credential');
      if (argv[0] === 'exercise-ceremony') {
        if (!m.exact_payload_json || m.operations.length !== 1)
          throw new Error('identity ceremony must name one exact operation and payload');
        const result = await c.call(m.operations[0], JSON.parse(m.exact_payload_json));
        writeCustodyJson(
          join(dirname(argv[1]), 'ceremony-result.json'),
          ceremonyResultDocument(result),
          true
        );
        console.log('native identity ceremony recorded');
        return;
      }
      if (!m.delegate || !m.binding || !m.operations.includes('grant_head_delegation'))
        throw new Error('credential does not authorize exact head grants');
      const path = join(dirname(argv[1]), 'delegations.json');
      const grants = (existsSync(path) ? JSON.parse(readFileSync(path, 'utf8')) : {}) as Record<
        string,
        HeadDelegationDocument
      >;
      for (const subject of subjects) {
        if (Object.hasOwn(grants, subject.id)) {
          const saved = grants[subject.id];
          if (
            saved.rootActionHash !== subject.root ||
            saved.delegate !== m.delegate ||
            saved.deviceBinding !== m.binding ||
            saved.dnaHash !== m.dna ||
            saved.validUntil !== m.valid_until
          )
            throw new Error('existing ceremony grant differs; explicit reconciliation required');
          continue;
        }
        const signed = await c.call<HeadDelegationWire>('grant_head_delegation', {
          scope: subject.id,
          root_action_hash: decodeHashFromBase64(subject.root),
          delegate: decodeHashFromBase64(m.delegate),
          valid_until: m.valid_until,
          device_binding: decodeHashFromBase64(m.binding),
        });
        grants[subject.id] = delegationDocument(signed);
        writeCustodyJson(path, grants);
        console.log(`exact content grant issued: ${subject.id}`);
      }
      return;
    } finally {
      await c.close();
    }
  }
  if (argv.length === 2 && argv[0] === 'refresh-hosted') {
    const o = JSON.parse(readFileSync(argv[1], 'utf8')) as ConductorOptions;
    await refreshHostedCredential(o);
    console.log('hosted credential transport refreshed; native authority unchanged');
    return;
  }
  if (argv.length !== 2 || argv[0] !== 'issue')
    throw new Error(
      'usage: steward-credential.ts issue PLAN.json | grant-heads DESCRIPTOR.json [ID] | stage-head DESCRIPTOR.json ID HEAD | exercise-ceremony DESCRIPTOR.json | refresh-hosted CONNECTION.json'
    );
  const issued = await issueCredential(readCredentialPlan(argv[1]));
  console.log(`credential issued: action=${issued.capabilityAction} profile=${issued.profile}`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main(process.argv.slice(2)).catch(() => {
    console.error('credential ceremony failed; inspect the explicit plan and custody state');
    process.exitCode = 1;
  });
}
