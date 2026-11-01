function marker(source, _argc, format) {
  return Object.freeze({ re: new RegExp(source), format });
}

const ARG_COUNTS = Object.freeze({
  eprRefused: 1,
  storeRefused: 2,
  intent: 5,
  noChangeId: 1,
  degraded: 1,
  skipped: 2,
  governance: 2,
  moved: 1,
  claimed: 0,
  held: 1,
  overrideWait: 2,
  daemonDown: 0,
  interrupted: 1,
  dismissed: 2,
  gateBanner: 0,
});

export const MARKERS = Object.freeze({
  eprRefused: marker(
    '^\\[pre-push\\] REFUSED: epr is required before any gate — run: epr device arrive, or set EPR_BIN=',
    1,
    value =>
      `[pre-push] REFUSED: epr is required before any gate — run: epr device arrive, or set EPR_BIN=${value}`,
  ),
  storeRefused: marker(
    "^\\[pre-push\\] REFUSED: the device's flow store .+ is unreachable — run: ",
    2,
    (store, cure) =>
      `[pre-push] REFUSED: the device's flow store ${store} is unreachable — run: ${cure}`,
  ),
  intent: marker(
    '^\\[pre-push\\] Phase V: intent (\\S+) raised for (I[0-9a-f]{40}) on base ([0-9a-f]{12}) \\(chain (\\d+): (.*)\\)$',
    5,
    (intent, change, base, chain, detail) =>
      `[pre-push] Phase V: intent ${intent} raised for ${change} on base ${base} (chain ${chain}: ${detail})`,
  ),
  noChangeId: marker(
    '^\\[pre-push\\] Phase V: no Change-Id on ([0-9a-f]{12}) — no chain position; to get one: git commit --amend$',
    1,
    tip =>
      `[pre-push] Phase V: no Change-Id on ${tip} — no chain position; to get one: git commit --amend`,
  ),
  degraded: marker(
    '^\\[pre-push\\] Phase V: queue unreadable \\((.+)\\) — gating against the dev base$',
    1,
    why => `[pre-push] Phase V: queue unreadable (${why}) — gating against the dev base`,
  ),
  skipped: marker(
    '^\\[pre-push\\] Phase V: skipped intent (\\S+) — (dormant|governance-class|cap|bytes not on this device|advisory (?:withdraw|landed|dismiss) by \\S+|withdrawn|landed)',
    2,
    (intent, why) => `[pre-push] Phase V: skipped intent ${intent} — ${why}`,
  ),
  governance: marker(
    '^\\[pre-push\\] change class: governance — judged by the merge-base recipe at ([0-9a-f]{40}) \\(GATE_EVIDENCE=(\\w+)\\)$',
    2,
    (base, evidence) =>
      `[pre-push] change class: governance — judged by the merge-base recipe at ${base} (GATE_EVIDENCE=${evidence})`,
  ),
  moved: marker(
    '^\\[pre-push\\] REFUSED: candidate moved to ([0-9a-f]{40}); push again$',
    1,
    tip => `[pre-push] REFUSED: candidate moved to ${tip}; push again`,
  ),
  claimed: marker(
    '^\\[pre-push\\] Phase L: dev-integration claimed',
    0,
    () => '[pre-push] Phase L: dev-integration claimed',
  ),
  held: marker(
    '^\\[pre-push\\] Phase L: dev-integration held (\\d+)s$',
    1,
    seconds => `[pre-push] Phase L: dev-integration held ${seconds}s`,
  ),
  overrideWait: marker(
    '^\\[pre-push\\] Phase L: override on the record; effective at (\\S+) — waiting (\\d+)s$',
    2,
    (at, seconds) =>
      `[pre-push] Phase L: override on the record; effective at ${at} — waiting ${seconds}s`,
  ),
  daemonDown: marker(
    "^\\[pre-push\\] Phase L: this device's peer daemon is down — run: epr peer serve$",
    0,
    () => "[pre-push] Phase L: this device's peer daemon is down — run: epr peer serve",
  ),
  interrupted: marker(
    '^\\[pre-push\\] interrupted: intent (\\S+) withdrawn; resume: git push origin HEAD:dev$',
    1,
    intent =>
      `[pre-push] interrupted: intent ${intent} withdrawn; resume: git push origin HEAD:dev`,
  ),
  dismissed: marker(
    '^\\[pre-push\\] red at the predicted base, green on dev — dismissed chain (.+) \\(capsule (\\S+)\\)$',
    2,
    (chain, capsule) =>
      `[pre-push] red at the predicted base, green on dev — dismissed chain ${chain} (capsule ${capsule})`,
  ),
  gateBanner: marker('PRE-PUSH GATE: ALL CLEAR', 0, () => 'PRE-PUSH GATE: ALL CLEAR'),
});

function usage() {
  process.stderr.write('usage: gate-landing-markers.mjs print <key> [arg…] | regex <key>\n');
}

if (process.argv[1]?.endsWith('gate-landing-markers.mjs')) {
  const [, , command, key, ...args] = process.argv;
  const selected = MARKERS[key];
  if (
    !selected ||
    !['print', 'regex'].includes(command) ||
    (command === 'print' && args.length !== ARG_COUNTS[key]) ||
    (command === 'regex' && args.length !== 0)
  ) {
    usage();
    process.exitCode = 2;
  } else if (command === 'regex') {
    process.stdout.write(`${selected.re.source}\n`);
  } else {
    process.stdout.write(`${selected.format(...args)}\n`);
  }
}
