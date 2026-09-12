/**
 * Start-tick-guarded /proc control for an OWNED mesh doorway process,
 * resolved through hc-mesh.sh's own recorded-pid ledger rather than `pgrep`.
 *
 * Same technique as doorway-sibling-reader.steps.ts:77's `signalOwnedDoorway`
 * and dataplane/apex-transition.steps.ts's `signalApexDoorway`: resolve the
 * process via `live_recorded_pid <role> <name>` (hc-mesh.sh's own ledger,
 * validated against the persisted start-tick), record its start ticks and
 * executable inode at capture time, and refuse to signal a pid whose
 * ticks/exe no longer match at signal time — a recycled pid must never be
 * SIGSTOP'd or SIGCONT'd as if it were still the doorway.
 *
 * Extracted here rather than copied a third time. Neither doorway-sibling-
 * reader.steps.ts nor apex-transition.steps.ts import this module — they
 * keep their own inline copies (out of scope to touch); this module exists
 * so the NEXT caller (federation-failover.steps.ts) has one home instead of
 * a fourth copy.
 */

import { strict as assert } from 'node:assert';
import { execFile } from 'node:child_process';
import { readFile, readlink } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';

const run = promisify(execFile);

/** genesis/a2o/src/framework/fixtures/ -> repo root -> app/elohim-app/scripts/hc-mesh.sh */
const meshScript = fileURLToPath(
  new URL('../../../../../app/elohim-app/scripts/hc-mesh.sh', import.meta.url)
);

export interface OwnedProcessHandle {
  pid: number;
  ticks: string;
  executable: string;
}

/** The kernel's start-tick field (field 22 of /proc/<pid>/stat) for `pid`. */
export async function processStartTicks(pid: number): Promise<string> {
  const stat = await readFile(`/proc/${pid}/stat`, 'utf8');
  return stat.slice(stat.lastIndexOf(')') + 2).split(' ')[19];
}

/**
 * Resolve an owned mesh process's OS handle via hc-mesh.sh's `live_recorded_pid
 * <role> <name>` (e.g. `doorway a`, `doorway b`) — never `pgrep`/`ps` string
 * matching, which can silently hand back a recycled pid.
 */
export async function resolveOwnedMeshProcess(
  role: string,
  name: string,
  label: string
): Promise<OwnedProcessHandle> {
  const { stdout } = await run('bash', [
    '-c',
    'source "$1"; live_recorded_pid "$2" "$3"',
    label,
    meshScript,
    role,
    name,
  ]);
  const pid = Number(stdout.trim());
  assert.ok(
    Number.isSafeInteger(pid) && pid > 1 && pid !== process.pid,
    `safe owned ${label} PID required (got "${stdout.trim()}")`
  );
  return {
    pid,
    ticks: await processStartTicks(pid),
    executable: await readlink(`/proc/${pid}/exe`),
  };
}

/** Refuse a recycled pid: the ticks and executable must still match capture time. */
export async function assertStillOwnedProcess(
  handle: OwnedProcessHandle,
  label: string
): Promise<void> {
  assert.equal(await processStartTicks(handle.pid), handle.ticks, `refuse a recycled ${label} PID`);
  assert.equal(
    await readlink(`/proc/${handle.pid}/exe`),
    handle.executable,
    `${label} executable changed`
  );
}

/** SIGSTOP or SIGCONT an owned process, guarded by `assertStillOwnedProcess`. */
export async function signalOwnedMeshProcess(
  handle: OwnedProcessHandle,
  signal: 'SIGSTOP' | 'SIGCONT',
  label: string
): Promise<void> {
  await assertStillOwnedProcess(handle, label);
  process.kill(handle.pid, signal);
}
