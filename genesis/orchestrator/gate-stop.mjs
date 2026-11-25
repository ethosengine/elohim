// Ends one gate's process tree when its live run carries a stop-now request (`gate-live.mjs stop
// --now`). The tap (gate-tap.mjs) starts each gate command as the leader of its own session and
// process group, so the recipe, `just`, `gate-steps.sh`, cargo, rustc, test binaries and node test
// runners are all members of that one group, and this module signals the group: TERM first, KILL
// after a bounded grace.
//
// Never a pid that is not this gate's. A process belongs to the gate when its session id is the
// leader's pid N, and N is still the gate's: Linux keeps a pid number allocated while any process
// holds it as a session or group id, so N can be reused only once every process of the gate's
// session is gone. Custody is therefore a chain: at each scan some process already verified as the
// gate's (pid AND start time, so a reused pid never passes) must still be alive in session N.
// The leader, verified at spawn while the tap holds it unreaped, is the first link. Processes that
// left the session (a nested setsid) are signalled one by one, only while they are still
// descendants of a verified member and their start time matches the scan's.
//
// Nothing here decides a verdict or records evidence: the caller reports a stopped gate as
// stopped, and gate-runner records nothing for it.

import { readdirSync, readFileSync } from 'node:fs';
import { setTimeout as delay } from 'node:timers/promises';

export const STOP_GRACE_MS = 10_000;
const KILL_WAIT_MS = 2_000;
const POLL_MS = 100;

/** The TERM-to-KILL grace: GATE_STOP_GRACE_MS when it is a whole number of ms up to a minute. */
export function stopGraceMs(env = process.env) {
  const raw = String(env.GATE_STOP_GRACE_MS ?? '').trim();
  if (!/^\d+$/.test(raw)) return STOP_GRACE_MS;
  return Math.min(Number(raw), 60_000);
}

/**
 * One process's identity from /proc/<pid>/stat: its parent, group, session, state and start
 * time (clock ticks since boot, field 22). Null when it is gone or unreadable. The command name
 * sits in parentheses and may itself hold spaces or parentheses, so fields are read after the last ')'.
 */
export function readStat(pid, { proc = '/proc', readFile = readFileSync } = {}) {
  try {
    const text = String(readFile(`${proc}/${pid}/stat`, 'utf8'));
    const fields = text.slice(text.lastIndexOf(')') + 2).trim().split(/\s+/);
    const stat = {
      pid: Number(pid), state: fields[0], ppid: Number(fields[1]), pgid: Number(fields[2]),
      sid: Number(fields[3]), start: fields[19],
    };
    if (![stat.pid, stat.ppid, stat.pgid, stat.sid].every(Number.isInteger) || !/^\d+$/.test(stat.start ?? '')) return null;
    return stat;
  } catch {
    return null;
  }
}

/** Every readable process on the host, by pid. */
export function snapshot({ proc = '/proc', listDir = readdirSync, readFile = readFileSync } = {}) {
  const table = new Map();
  let entries = [];
  try { entries = listDir(proc); } catch { return table; }
  for (const entry of entries) {
    if (!/^\d+$/.test(entry)) continue;
    const stat = readStat(entry, { proc, readFile });
    if (stat) table.set(stat.pid, stat);
  }
  return table;
}

const identity = (stat) => `${stat.pid}:${stat.start}`;

/**
 * The gate's processes in one scan. `leaderPid` is N, the session and group the tap created;
 * `known` holds `pid:start` of every process already verified as the gate's, and is extended with
 * this scan's members. Without custody (no verified process still alive in session N) nothing is
 * the gate's, whatever now carries the number N.
 */
export function gateTree(leaderPid, known, table) {
  const inSession = [...table.values()].filter((stat) => stat.sid === leaderPid);
  const custody = inSession.some((stat) => known.has(identity(stat)));
  if (!custody) return { custody: false, members: [] };
  const members = new Map(inSession.map((stat) => [stat.pid, stat]));
  // Descendants that left the session (their own setsid) stay the gate's while their parent is.
  for (let grew = true; grew;) {
    grew = false;
    for (const stat of table.values()) {
      if (!members.has(stat.pid) && members.has(stat.ppid)) { members.set(stat.pid, stat); grew = true; }
    }
  }
  for (const stat of members.values()) known.add(identity(stat));
  return { custody: true, members: [...members.values()] };
}

const living = (members) => members.filter((stat) => stat.state !== 'Z' && stat.state !== 'X');

/**
 * Sends `signal` to the gate: the group N once, then each member outside group N whose start time
 * still matches. Returns the pids signalled one by one. Only called with custody held.
 */
export function signalTree(leaderPid, members, signal, { kill = process.kill, stat = readStat } = {}) {
  const sent = [];
  try { kill(-leaderPid, signal); } catch { /* the group is already empty */ }
  for (const member of members) {
    if (member.pgid === leaderPid) continue;
    const now = stat(member.pid);
    if (!now || now.start !== member.start) continue;
    try { kill(member.pid, signal); sent.push(member.pid); } catch { /* gone meanwhile */ }
  }
  return sent;
}

/**
 * Ends the gate whose leader is `leader` ({ pid, start }, read at spawn): TERM to its tree, a wait
 * of up to `graceMs` for every member to go, then KILL to whatever is left and a short wait.
 * Resolves to { custody, signal, survivors }: `signal` is the last one sent (null when nothing of
 * the gate was alive), `survivors` the pids still alive after the KILL wait.
 */
export async function endTree(leader, {
  graceMs = STOP_GRACE_MS, kill = process.kill, scan = () => snapshot(), stat = readStat,
  sleep = delay, pollMs = POLL_MS, clock = () => Date.now(),
} = {}) {
  if (!leader || !Number.isInteger(leader.pid) || !leader.start) return { custody: false, signal: null, survivors: [] };
  const known = new Set([identity(leader)]);
  let tree = gateTree(leader.pid, known, scan());
  if (!tree.custody || living(tree.members).length === 0) return { custody: tree.custody, signal: null, survivors: [] };

  signalTree(leader.pid, tree.members, 'SIGTERM', { kill, stat });
  const termDeadline = clock() + graceMs;
  while (clock() < termDeadline) {
    await sleep(pollMs);
    tree = gateTree(leader.pid, known, scan());
    if (!tree.custody || living(tree.members).length === 0) return { custody: true, signal: 'SIGTERM', survivors: [] };
  }

  signalTree(leader.pid, tree.members, 'SIGKILL', { kill, stat });
  const killDeadline = clock() + KILL_WAIT_MS;
  while (clock() < killDeadline) {
    await sleep(pollMs);
    tree = gateTree(leader.pid, known, scan());
    if (!tree.custody || living(tree.members).length === 0) return { custody: true, signal: 'SIGKILL', survivors: [] };
  }
  return { custody: true, signal: 'SIGKILL', survivors: living(tree.members).map((member) => member.pid) };
}
