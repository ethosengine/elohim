import { strict as assert } from 'node:assert';

export interface OwnedCommitmentCreateInput {
  id: string;
  action: string;
  provider: string;
  receiver: string;
  inScopeOf: string | string[];
  metadataJson: string;
  metadata: unknown;
}

interface OwnedCommitmentView {
  id?: string;
  action?: string;
  provider?: string;
  receiver?: string;
  inScopeOf?: string[];
  metadata?: unknown;
  dhtAnchorHash?: string;
}

interface HttpResult {
  ok: boolean;
  status: number;
  text: string;
}

const PROJECTION_DEFERRED = 'REA projection changed during authority read; deferred';
const READ_BACKOFFS_MS = [100, 200, 400] as const;

function exactDeferred(result: HttpResult): boolean {
  if (result.status !== 400) return false;
  try {
    return (JSON.parse(result.text) as { error?: unknown }).error === PROJECTION_DEFERRED;
  } catch {
    return false;
  }
}

function validateOwnedCommitment(
  actual: OwnedCommitmentView,
  expected: OwnedCommitmentCreateInput
): OwnedCommitmentView {
  const metadataFromJson = JSON.parse(expected.metadataJson) as unknown;
  assert.deepEqual(
    metadataFromJson,
    expected.metadata,
    'create metadata and metadataJson diverged'
  );
  assert.deepEqual(
    {
      id: actual.id,
      action: actual.action,
      provider: actual.provider,
      receiver: actual.receiver,
      inScopeOf: actual.inScopeOf,
      metadata: actual.metadata,
    },
    {
      id: expected.id,
      action: expected.action,
      provider: expected.provider,
      receiver: expected.receiver,
      inScopeOf: Array.isArray(expected.inScopeOf) ? expected.inScopeOf : [expected.inScopeOf],
      metadata: expected.metadata,
    },
    `commitment ${expected.id} does not match the complete requested terms`
  );
  assert.ok(actual.dhtAnchorHash, `commitment ${expected.id} has no notarized action hash`);
  return actual;
}

/** Recognize an already-authored create without ever repeating the POST. */
export async function resolveOwnedCommitmentCreate(
  result: HttpResult,
  expected: OwnedCommitmentCreateInput,
  read: () => Promise<HttpResult>,
  deadlineAt: number,
  sleep: (milliseconds: number) => Promise<void> = async milliseconds => {
    await new Promise(resolve => setTimeout(resolve, milliseconds));
  },
  now: () => number = Date.now
): Promise<OwnedCommitmentView> {
  if (result.ok)
    return validateOwnedCommitment(JSON.parse(result.text) as OwnedCommitmentView, expected);
  const readbackAllowed =
    exactDeferred(result) || result.text.includes('UNIQUE constraint failed: rea_commitments.id');
  assert.ok(readbackAllowed, `commitment create failed: ${result.status} ${result.text}`);

  let last: HttpResult | undefined;
  for (const backoff of [...READ_BACKOFFS_MS, 0]) {
    if (now() >= deadlineAt) break;
    last = await read();
    if (last.ok)
      return validateOwnedCommitment(JSON.parse(last.text) as OwnedCommitmentView, expected);
    if (backoff > 0) await sleep(Math.min(backoff, Math.max(1, deadlineAt - now())));
  }
  assert.fail(
    `commitment ${expected.id} was not readable before the setup deadline: ` +
      `${last?.status ?? 'not-read'} ${last?.text ?? ''}`
  );
}
