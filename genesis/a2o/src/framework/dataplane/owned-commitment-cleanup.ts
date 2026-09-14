export interface CleanupResponse {
  ok: boolean;
  status: number;
  text: string;
}

export interface CommitmentCleanupReadback {
  id?: string;
  state?: string;
  finished?: boolean;
}

const PROJECTION_DEFERRED = 'REA projection changed during authority read; deferred';
const CLEANUP_BACKOFFS_MS = [100, 200, 400] as const;

function isExactProjectionDeferral(response: CleanupResponse): boolean {
  if (response.status !== 400) return false;
  try {
    const body = JSON.parse(response.text) as { error?: unknown };
    return body.error === PROJECTION_DEFERRED;
  } catch {
    return false;
  }
}

function isExpectedCancellation(
  current: CommitmentCleanupReadback | undefined,
  id: string
): boolean {
  return current?.id === id && current.state === 'cancelled' && current.finished === true;
}

/** Cancel one run-owned commitment without mistaking a deferred projection for failure. */
export async function cancelOwnedCommitmentWithReadback(
  id: string,
  cancel: () => Promise<CleanupResponse>,
  read: () => Promise<CommitmentCleanupReadback | undefined>,
  sleep: (milliseconds: number) => Promise<void> = async milliseconds => {
    await new Promise(resolve => setTimeout(resolve, milliseconds));
  }
): Promise<CleanupResponse> {
  for (const delay of CLEANUP_BACKOFFS_MS) {
    const response = await cancel();
    if (response.status === 404) return response;
    if (!response.ok && !isExactProjectionDeferral(response)) return response;

    const current = await read();
    if (isExpectedCancellation(current, id)) {
      return { ok: true, status: 200, text: JSON.stringify(current) };
    }
    await sleep(delay);
  }
  const response = await cancel();
  if (response.status === 404) return response;
  if (!response.ok && !isExactProjectionDeferral(response)) return response;
  const current = await read();
  return isExpectedCancellation(current, id)
    ? { ok: true, status: 200, text: JSON.stringify(current) }
    : {
        ok: false,
        status: response.status,
        text: `owned cancellation was not visible after bounded retries: ${response.text}`,
      };
}
