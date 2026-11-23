// A landing intent permits a bounded wait, never a gate pass or a reservation.
/**
 * Synchronous contract to match the gate loop. All effects are supplied by deps:
 * exec(args, options) returns a spawnSync result; now() is Unix seconds;
 * monotonic() is monotonic seconds; sleep(seconds); log(line);
 * verdict(project, closure) returns admitted sibling verdict rows.
 * hardCapS and pollS are seconds. Absolute bounds are pinned to the entry clocks.
 */
export function landerWait({ intent, project, closure, freshnessS, mode = 'shadow',
  ownStopAt, hardCapS = 1800, pollS = 3, deps }) {
  if (mode === 'off') return { waitedS: 0, outcome: 'off', reason: 'carry wait is off' };
  if (!intent) return { waitedS: 0, outcome: 'ran-here-no-carry', reason: 'no landing intent' };
  const { exec, now, monotonic, sleep, log, verdict } = deps;
  const start = monotonic();
  const wall = now();
  const cap = Number.isFinite(hardCapS) && hardCapS >= 0 ? hardCapS : 1800;
  const own = ownStopAt == null ? Infinity : Number(ownStopAt);
  let deadline = Math.min(start + cap, Number.isFinite(own) ? start + own - wall : Infinity);
  let bound = 'own-bound-passed';
  let carryUntil = Infinity;
  let by;
  const finish = (outcome, reason, shadow = false) => {
    const waitedS = shadow ? 0 : Math.max(0, monotonic() - start);
    const line = String(reason).replace(/[\r\n]+/g, ' ');
    log(`${shadow ? 'shadow: ' : ''}${line}; waited ${waitedS}s`);
    return { waitedS, outcome, ...(by ? { by } : {}), reason: line };
  };
  if (!closure) return finish('decision-unavailable', 'carry decision unavailable: no closure');
  const ask = () => {
    try {
      // Freshness is the policy's (berth.channel.freshness_s); the native read takes it from
      // there unless a caller names one, so no second copy of the number lives here.
      const r = exec(['flow', 'intend', '--carry-decision', intent, '--project', project,
        '--closure', closure, '--role', 'lander',
        ...(freshnessS == null ? [] : ['--freshness-s', String(freshnessS)]), '--json'],
      { encoding: 'utf8', timeout: 5000 });
      if (r.error) throw r.error;
      if (r.status !== 0) throw new Error(`exit ${r.status}${r.signal ? ` (${r.signal})` : ''}`);
      const d = JSON.parse(r.stdout);
      if (!d || typeof d.reason !== 'string' || !d.reason.trim()
        || !['run-here', 'wait'].includes(d.decision)
        || (d.decision === 'wait' && (!Number.isFinite(d.until) || typeof d.by !== 'string' || !d.by.trim()))) {
        throw new Error('invalid carry decision');
      }
      return d;
    } catch (error) {
      return { unavailable: true, reason: `carry decision unavailable: ${error.message}` };
    }
  };
  let decision = ask();
  if (decision.unavailable) return finish('decision-unavailable', decision.reason, mode !== 'on');
  if (decision.decision === 'run-here') return finish('ran-here-no-carry', decision.reason, mode !== 'on');
  by = decision.by;
  if (mode !== 'on') return finish('ran-here-no-carry',
    `would have waited for ${by} until ${decision.until}: ${decision.reason}`, true);
  for (;;) {
    // A replay can shorten the carry bound, never move it later. Wall-clock jumps cannot extend it.
    carryUntil = Math.min(carryUntil, decision.until);
    const carryEnd = start + carryUntil - wall;
    if (carryEnd < deadline) { deadline = carryEnd; bound = 'carry-lapsed'; }
    const wallNow = now();
    if (Number.isFinite(own) && wallNow >= own) return finish('own-bound-passed', 'own wait bound passed');
    if (wallNow >= carryUntil) return finish('carry-lapsed', 'carry bound passed');
    if (monotonic() >= deadline) {
      return finish(bound, bound === 'carry-lapsed' ? 'carry bound passed' : 'own wait bound passed');
    }
    let rows;
    try { rows = verdict(project, closure); }
    catch (error) { return finish('decision-unavailable', `verdict lookup unavailable: ${error.message}`); }
    if (rows?.some(row => row.success === true || row.success === false)) {
      return finish('verdict-arrived', 'sibling verdict visible; reuse still decides');
    }
    const remaining = deadline - monotonic();
    if (remaining <= 0) continue;
    sleep(Math.min(Number.isFinite(pollS) && pollS > 0 ? pollS : 3, remaining));
    if (monotonic() >= deadline) continue;
    decision = ask();
    if (decision.unavailable) return finish('decision-unavailable', decision.reason);
    if (decision.decision === 'run-here') return finish('carry-lapsed', decision.reason);
    by = decision.by;
  }
}
