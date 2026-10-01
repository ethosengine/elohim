/** Local recovery state. These records never confer publishing authority. */
import { createHash } from 'node:crypto';
import {
  closeSync,
  existsSync,
  fsyncSync,
  mkdirSync,
  openSync,
  readFileSync,
  renameSync,
  writeFileSync,
} from 'node:fs';
import { join } from 'node:path';

import type { HeadDelegationDocument } from './steward-delegation.js';

export interface PublicationReceipt {
  id: string;
  seedHash: string;
  agent: string;
  dna: string;
  storage: string;
  head: string;
  authoredAt: string;
  execution?: {
    requester: string;
    executor: string;
    humanId: string;
    humanAction: string;
    binding: string;
  };
  declaredAt?: string;
  /** Portable evidence only; native verification is mandatory on recovery. */
  acceptedDelegation?: HeadDelegationDocument;
  /** Original exact grant, retained before requesting native acceptance. */
  delegation?: HeadDelegationDocument;
}

function receiptPath(dir: string, id: string): string {
  return join(dir, `${createHash('sha256').update(id).digest('hex')}.json`);
}

/** Never silently discard an interrupted publication, even if the source changed. */
export function pendingPublication(
  dir: string,
  context: Pick<PublicationReceipt, 'id' | 'seedHash' | 'agent' | 'dna' | 'storage'>
): PublicationReceipt | undefined {
  const path = receiptPath(dir, context.id);
  if (!existsSync(path)) return undefined;
  const saved = JSON.parse(readFileSync(path, 'utf8')) as PublicationReceipt;
  if (saved.declaredAt) return undefined;
  for (const key of ['id', 'seedHash', 'agent', 'dna', 'storage'] as const) {
    if (saved[key] !== context[key])
      throw new Error(
        `pending publication ${context.id}: ${key} changed; resume the original publication first`
      );
  }
  if (typeof saved.head !== 'string' || !saved.head || typeof saved.authoredAt !== 'string')
    throw new Error(`invalid pending publication receipt for ${context.id}`);
  if (
    saved.acceptedDelegation?.acceptance?.headActionHash !== undefined &&
    saved.acceptedDelegation.acceptance.headActionHash !== saved.head
  )
    throw new Error(`pending publication ${context.id}: acceptance names another head`);
  if (saved.acceptedDelegation && !saved.acceptedDelegation.acceptance)
    throw new Error(`pending publication ${context.id}: signed acceptance missing`);
  return saved;
}

/** Atomic, durable recovery point before declaration; no key material is stored. */
export function savePublication(dir: string, receipt: PublicationReceipt): void {
  mkdirSync(dir, { recursive: true, mode: 0o700 });
  const path = receiptPath(dir, receipt.id);
  const temporary = `${path}.${process.pid}.tmp`;
  const fd = openSync(temporary, 'w', 0o600);
  try {
    writeFileSync(fd, `${JSON.stringify(receipt, null, 2)}\n`);
    fsyncSync(fd);
  } finally {
    closeSync(fd);
  }
  renameSync(temporary, path);
  const directory = openSync(dir, 'r');
  try {
    fsyncSync(directory);
  } finally {
    closeSync(directory);
  }
}
