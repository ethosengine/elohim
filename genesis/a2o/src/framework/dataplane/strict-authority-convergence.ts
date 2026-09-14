export interface AuthorityConvergenceResult {
  converged: boolean;
  lastMismatches: Record<string, string>;
}

/** Run one expensive acceptance probe only when it starts and finishes inside the same deadline. */
export async function completeOnceWithinDeadline<T>(
  deadlineAt: number,
  operation: (remainingMs: number) => Promise<T>,
  now: () => number = Date.now
): Promise<T> {
  const remainingMs = deadlineAt - now();
  if (remainingMs <= 0)
    throw new Error('shared authority deadline expired before acceptance probe');
  const result = await operation(remainingMs);
  if (now() > deadlineAt)
    throw new Error('acceptance probe completed after shared authority deadline');
  return result;
}

/** Poll every named observer against one immutable author receipt and one shared deadline. */
export async function waitForStrictAuthorityConvergence(
  observers: string[],
  deadlineAt: number,
  probe: (observer: string, remainingMs: number) => Promise<string | undefined>,
  options: {
    intervalMs?: number;
    now?: () => number;
    sleep?: (milliseconds: number) => Promise<void>;
  } = {}
): Promise<AuthorityConvergenceResult> {
  const intervalMs = options.intervalMs ?? 1000;
  const now = options.now ?? Date.now;
  const sleep =
    options.sleep ?? (async milliseconds => await new Promise(r => setTimeout(r, milliseconds)));
  let lastMismatches: Record<string, string> = {};

  while (now() < deadlineAt) {
    const remainingMs = deadlineAt - now();
    const observations = await Promise.all(
      observers.map(async observer => [observer, await probe(observer, remainingMs)] as const)
    );
    lastMismatches = Object.fromEntries(
      observations.filter((entry): entry is readonly [string, string] => entry[1] !== undefined)
    );
    if (Object.keys(lastMismatches).length === 0) {
      return { converged: true, lastMismatches };
    }
    const remainingAfterProbe = deadlineAt - now();
    if (remainingAfterProbe <= 0) break;
    await sleep(Math.min(intervalMs, remainingAfterProbe));
  }
  return { converged: false, lastMismatches };
}
