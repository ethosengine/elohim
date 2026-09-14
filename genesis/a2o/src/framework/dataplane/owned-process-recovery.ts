export interface OwnedProcessRecoveryChecks {
  identityMatches: () => Promise<boolean>;
  processRunning: () => Promise<boolean>;
  faultedPathHealthy: () => Promise<boolean>;
}

export interface OwnedProcessRecoveryClock {
  now: () => number;
  sleep: (milliseconds: number) => Promise<void>;
}

/** Verify an owned fault target after SIGCONT, including an already-resumed target. */
export async function awaitOwnedProcessRecovery(
  checks: OwnedProcessRecoveryChecks,
  boundMs: number,
  pollMs: number,
  clock: OwnedProcessRecoveryClock = {
    now: Date.now,
    sleep: async milliseconds => await new Promise(resolve => setTimeout(resolve, milliseconds)),
  }
): Promise<void> {
  const deadline = clock.now() + boundMs;
  for (;;) {
    if (!(await checks.identityMatches())) {
      throw new Error('owned fault target identity changed during recovery');
    }
    if ((await checks.processRunning()) && (await checks.faultedPathHealthy())) return;
    if (clock.now() >= deadline) {
      throw new Error('owned fault target process/path did not recover');
    }
    await clock.sleep(pollMs);
  }
}
