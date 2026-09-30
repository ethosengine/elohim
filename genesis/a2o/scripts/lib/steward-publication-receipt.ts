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

export interface PublicationReceipt {
  id: string;
  seedHash: string;
  agent: string;
  dna: string;
  storage: string;
  head: string;
  authoredAt: string;
  declaredAt?: string;
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
