import { aTimeout, expect, fixture, html } from '@open-wc/testing';
import axe from 'axe-core';
import { DEFAULT_PROFILE } from 'elohim-core';
import {
  assertThemeContrast,
  clearMediaQueries,
  measureLuminanceChanges,
  renderInLocale,
  requiresLogicalProperties,
  setMediaQuery,
  themeFixture,
  type ThemeCell,
} from 'elohim-core/testing';

import './register.js';
import {
  ElohimImagodeiWitnessTrail as WitnessTrailClass,
  WITNESS_TRAIL_STRINGS_EN,
} from './elohim-imagodei-witness-trail.js';
import type {
  ElohimImagodeiWitnessTrail,
  WitnessStep,
  WitnessTrailSettledDetail,
} from './elohim-imagodei-witness-trail.js';

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

const PASSWORD_ONLY: WitnessStep[] = [
  { id: 'pw', act: 'checked', relation: 'your-doorway', label: 'alpha', state: 'working' },
];

const DEVICE_IN_PROGRESS: WitnessStep[] = [
  { id: 'door-sign', act: 'signed', relation: 'your-doorway', label: 'alpha', state: 'done' },
  { id: 'dev-sign', act: 'signed', relation: 'your-device', label: 'workspace', state: 'working' },
  { id: 'record', act: 'recorded', relation: 'others', state: 'waiting' },
];

const DEVICE_COMPLETE: WitnessStep[] = [
  { id: 'door-sign', act: 'signed', relation: 'your-doorway', label: 'alpha', state: 'done' },
  { id: 'dev-sign', act: 'signed', relation: 'your-device', label: 'workspace', state: 'done' },
  { id: 'seen', act: 'seen', relation: 'others', count: 3, state: 'done' },
];

/** Three steps under way or done, plus one still waiting: the rotation has three lines. */
const ROTATING: WitnessStep[] = [
  ...DEVICE_COMPLETE,
  { id: 'later', act: 'recorded', relation: 'others', state: 'waiting' },
];

const SECRET = 'Zedekiah Secretname';

const cssText = (): string =>
  (WitnessTrailClass as unknown as { styles: { cssText: string } }).styles.cssText;

const q = <T extends Element = HTMLElement>(el: Element, sel: string): T | null =>
  el.shadowRoot!.querySelector<T>(sel);

const qa = <T extends Element = HTMLElement>(el: Element, sel: string): T[] =>
  Array.from(el.shadowRoot!.querySelectorAll<T>(sel));

/** Drop the Unicode isolate marks the trail wraps around person-supplied names. */
const plain = (value: string): string => value.replace(/[⁦-⁩]/g, '').replace(/\s+/g, ' ').trim();

const text = (el: Element, sel = '[part="trail"]'): string => plain(q(el, sel)?.textContent ?? '');

/** Visible list sentences, in rendered order. */
const sentences = (el: Element): string[] =>
  qa(el, '[part="step"] [part="sentence"]').map(n => plain(n.textContent ?? ''));

const line = (el: Element): string => text(el, '[part="line"] [part="sentence"]');

/**
 * Render a trail. A live trail whose steps are all done is never shown when
 * it starts that way (rule: an instant sign-in does not flash), so — unless a
 * reveal threshold is being tested — such fixtures first render the same
 * steps still under way, then complete them: the real order of events.
 */
async function trail(
  steps: WitnessStep[],
  opts: {
    layout?: 'list' | 'rotate';
    mode?: 'live' | 'settled';
    reveal?: number;
    open?: boolean;
  } = {}
) {
  const mode = opts.mode ?? 'live';
  const prime =
    mode === 'live' &&
    opts.reveal === undefined &&
    steps.length > 0 &&
    steps.every(s => s.state === 'done');
  const first = prime
    ? steps.map((s, i) => (i === 0 ? { ...s, state: 'working' as const } : s))
    : steps;
  const el = await fixture<ElohimImagodeiWitnessTrail>(html`
    <elohim-imagodei-witness-trail
      .steps=${first}
      layout=${opts.layout ?? 'list'}
      mode=${mode}
      reveal-after-ms=${opts.reveal ?? 0}
      ?open=${opts.open ?? false}
    ></elohim-imagodei-witness-trail>
  `);
  if (prime) {
    el.steps = steps;
    await el.updateComplete;
  }
  return el;
}

/** Poll the rotate slot and return the first `n` distinct consecutive lines it shows. */
async function collectLines(el: ElohimImagodeiWitnessTrail, n: number, maxMs: number) {
  const seen: string[] = [line(el)];
  const end = performance.now() + maxMs;
  while (seen.length < n && performance.now() < end) {
    await aTimeout(50);
    const now = line(el);
    if (now !== seen.at(-1)) seen.push(now);
  }
  return seen;
}

// ---------------------------------------------------------------------------
// Hard rule 1 — only what it is given; no delay on completion
// ---------------------------------------------------------------------------

describe('<elohim-imagodei-witness-trail> — shows only what it is given', () => {
  it('renders exactly one line per given step, in the given order', async () => {
    const el = await trail(DEVICE_IN_PROGRESS);
    const steps = qa(el, '[part="step"]');
    expect(steps.length).to.equal(3);
    expect(steps.map(s => s.getAttribute('data-relation'))).to.deep.equal([
      'your-doorway',
      'your-device',
      'others',
    ]);
    expect(steps.map(s => s.getAttribute('data-state'))).to.deep.equal([
      'done',
      'working',
      'waiting',
    ]);
  });

  it('renders nothing (no trail, no padding) when given no steps', async () => {
    const el = await trail([]);
    expect(q(el, '[part="trail"]')).to.equal(null);
  });

  it('drops a step whose shape it does not know rather than guessing', async () => {
    const el = await trail([
      ...PASSWORD_ONLY,
      { id: 'x', act: 'blessed' as 'seen', relation: 'others', state: 'done' },
    ]);
    expect(qa(el, '[part="step"]').length).to.equal(1);
  });

  it('fires settled in the same update that makes every step done', async () => {
    const el = await trail(DEVICE_IN_PROGRESS);
    const fired: WitnessTrailSettledDetail[] = [];
    el.addEventListener('settled', e => fired.push((e as CustomEvent).detail));
    el.steps = DEVICE_COMPLETE;
    await el.updateComplete;
    expect(fired).to.deep.equal([{ stepIds: ['door-sign', 'dev-sign', 'seen'] }]);
  });

  it('settled bubbles and is composed', async () => {
    const wrapper = await fixture<HTMLDivElement>(html`
      <div>
        <elohim-imagodei-witness-trail
          .steps=${DEVICE_IN_PROGRESS}
          reveal-after-ms="0"
        ></elohim-imagodei-witness-trail>
      </div>
    `);
    const el = wrapper.firstElementChild as ElohimImagodeiWitnessTrail;
    let ev: CustomEvent | undefined;
    wrapper.addEventListener('settled', e => (ev = e as CustomEvent));
    el.steps = DEVICE_COMPLETE;
    await el.updateComplete;
    expect(ev?.bubbles).to.equal(true);
    expect(ev?.composed).to.equal(true);
  });

  it('fires settled once per completed set, and again for a new completion', async () => {
    const el = await trail(DEVICE_IN_PROGRESS);
    let count = 0;
    el.addEventListener('settled', () => count++);
    el.steps = DEVICE_COMPLETE;
    await el.updateComplete;
    el.steps = [...DEVICE_COMPLETE];
    await el.updateComplete;
    expect(count).to.equal(1);
    el.steps = DEVICE_IN_PROGRESS;
    await el.updateComplete;
    el.steps = DEVICE_COMPLETE;
    await el.updateComplete;
    expect(count).to.equal(2);
  });

  it('fires settled immediately even while the reveal threshold is still running', async () => {
    const el = await trail(DEVICE_IN_PROGRESS, { reveal: 5000 });
    let fired = false;
    el.addEventListener('settled', () => (fired = true));
    el.steps = DEVICE_COMPLETE;
    await el.updateComplete;
    expect(fired).to.equal(true);
  });

  it('does not fire settled for an empty trail, nor in settled mode', async () => {
    const empty = await trail([]);
    let fired = false;
    empty.addEventListener('settled', () => (fired = true));
    empty.steps = [];
    await empty.updateComplete;
    const summary = await trail(DEVICE_IN_PROGRESS, { mode: 'settled' });
    summary.addEventListener('settled', () => (fired = true));
    summary.steps = DEVICE_COMPLETE;
    await summary.updateComplete;
    expect(fired).to.equal(false);
  });
});

// ---------------------------------------------------------------------------
// Hard rule 2 — others are a count, never a name
// ---------------------------------------------------------------------------

describe('<elohim-imagodei-witness-trail> — others are never named', () => {
  const withSecret: WitnessStep[] = [
    { id: 'door', act: 'signed', relation: 'your-doorway', label: 'alpha', state: 'done' },
    { id: 'seen', act: 'seen', relation: 'others', label: SECRET, count: 3, state: 'done' },
    { id: 'chk', act: 'checked', relation: 'others', label: SECRET, state: 'working' },
  ];

  const variants: [string, () => Promise<ElohimImagodeiWitnessTrail>][] = [
    ['list', () => trail(withSecret, { layout: 'list' })],
    ['rotate', () => trail(withSecret, { layout: 'rotate' })],
    ['settled open', () => trail(withSecret, { mode: 'settled', open: true })],
    [
      'settled summary',
      () =>
        trail(
          withSecret.map(s => ({ ...s, state: 'done' as const })),
          { mode: 'settled', open: true }
        ),
    ],
    ['failed', () => trail([{ ...withSecret[1]!, state: 'failed', note: 'Offline' }])],
  ];

  for (const [name, make] of variants) {
    it(`${name}: a label given with others never reaches the DOM`, async () => {
      const el = await make();
      await el.updateComplete;
      const dom = el.shadowRoot!.innerHTML + el.outerHTML;
      expect(dom).to.not.include('Zedekiah');
      expect(dom).to.not.include('Secretname');
      for (const node of el.shadowRoot!.querySelectorAll('*')) {
        for (const attr of Array.from(node.attributes)) {
          expect(attr.value).to.not.include('Zedekiah');
        }
      }
      // Accessible names are built from text + aria-label: both already scanned above.
    });
  }

  it('renders the count when given', async () => {
    const el = await trail([withSecret[1]!]);
    expect(sentences(el)).to.deep.equal(['3 others on the network have seen it']);
  });

  it('renders no number when count is absent', async () => {
    const el = await trail([{ id: 's', act: 'seen', relation: 'others', state: 'done' }]);
    expect(sentences(el)).to.deep.equal(['Others on the network can check it']);
  });

  it('uses the singular for one other', async () => {
    const el = await trail([{ id: 's', act: 'seen', relation: 'others', count: 1, state: 'done' }]);
    expect(sentences(el)).to.deep.equal(['1 other on the network has seen it']);
  });

  it('names the person’s own circle', async () => {
    const el = await trail([
      { id: 'v', act: 'signed', relation: 'vouches-for-you', label: 'Jessica', state: 'done' },
      { id: 'd', act: 'signed', relation: 'your-device', label: 'workspace', state: 'done' },
    ]);
    expect(sentences(el)).to.deep.equal(['Jessica vouches for you', 'workspace signed it too']);
  });
});

// ---------------------------------------------------------------------------
// Hard rule 3 — honest when nobody else is involved
// ---------------------------------------------------------------------------

describe('<elohim-imagodei-witness-trail> — honest when nobody else is involved', () => {
  it('a plain password sign-in says exactly that, plus one sentence', async () => {
    const el = await trail(PASSWORD_ONLY);
    expect(qa(el, '[part="step"]').length).to.equal(1);
    expect(sentences(el)).to.deep.equal(['Your doorway alpha is checking your password…']);
    expect(text(el, '[part="alone"]')).to.equal('This sign-in rests on your doorway alone.');
    expect(qa(el, '[part="alone"]').length).to.equal(1);
  });

  it('the doorway-only sentence also shows in rotate layout, with no pips to rotate', async () => {
    const el = await trail(PASSWORD_ONLY, { layout: 'rotate' });
    expect(line(el)).to.equal('Your doorway alpha is checking your password…');
    expect(q(el, '[part="pips"]')).to.equal(null);
    expect(text(el, '[part="alone"]')).to.equal('This sign-in rests on your doorway alone.');
  });

  it('no alone sentence once anyone else is involved', async () => {
    const el = await trail(DEVICE_IN_PROGRESS);
    expect(q(el, '[part="alone"]')).to.equal(null);
  });
});

// ---------------------------------------------------------------------------
// this-device — the node on the machine in use, first, and its own alone case
// ---------------------------------------------------------------------------

describe('<elohim-imagodei-witness-trail> — this device', () => {
  const thisDeviceLast: WitnessStep[] = [
    { id: 'door', act: 'signed', relation: 'your-doorway', label: 'alpha', state: 'done' },
    { id: 'others', act: 'seen', relation: 'others', count: 3, state: 'done' },
    { id: 'dev', act: 'signed', relation: 'your-device', label: 'laptop', state: 'done' },
    { id: 'here', act: 'signed', relation: 'this-device', label: 'workspace', state: 'done' },
  ];

  it('sentences: done, working, and with its name', async () => {
    const el = await trail([
      { id: 'a', act: 'checked', relation: 'this-device', state: 'done' },
      { id: 'b', act: 'signed', relation: 'this-device', state: 'done' },
      { id: 'c', act: 'signed', relation: 'this-device', state: 'working' },
      { id: 'd', act: 'signed', relation: 'this-device', label: 'workspace', state: 'done' },
    ]);
    expect(sentences(el)).to.deep.equal([
      'This device checked it',
      'This device signed it',
      'This device is signing…',
      'This device, workspace, signed it',
    ]);
  });

  it('list: this-device is first wherever it was given; the rest keep their order', async () => {
    const el = await trail(thisDeviceLast);
    expect(qa(el, '[part="step"]').map(s => s.getAttribute('data-relation'))).to.deep.equal([
      'this-device',
      'your-doorway',
      'others',
      'your-device',
    ]);
  });

  it('rotate: this-device leads the slot, the pips and the assistive list', async () => {
    const el = await trail(thisDeviceLast, { layout: 'rotate' });
    expect(line(el)).to.equal('This device, workspace, signed it');
    expect(q(el, '[part="pip"]')!.getAttribute('aria-current')).to.equal('step');
    expect(qa(el, '[part="step"]')[0]!.getAttribute('data-relation')).to.equal('this-device');
  });

  it('settled summary: this-device is named first; others keep given order', async () => {
    const el = await trail(thisDeviceLast, { mode: 'settled' });
    expect(text(el, '[part="summary"]')).to.equal(
      'Secured by this device (workspace), your doorway alpha, 3 others, and your device laptop'
    );
  });

  it('settled event lists this-device first too', async () => {
    const el = await trail(thisDeviceLast.map(s => ({ ...s, state: 'waiting' as const })));
    let ids: string[] = [];
    el.addEventListener('settled', e => (ids = (e as CustomEvent).detail.stepIds));
    el.steps = thisDeviceLast;
    await el.updateComplete;
    expect(ids).to.deep.equal(['here', 'door', 'others', 'dev']);
  });

  it('alone: the sole this-device step is one still line plus its own sentence', async () => {
    const el = await trail(
      [{ id: 'here', act: 'signed', relation: 'this-device', state: 'working' }],
      { layout: 'rotate' }
    );
    expect(line(el)).to.equal('This device is signing…');
    expect(q(el, '[part="pips"]')).to.equal(null);
    expect(text(el, '[part="alone"]')).to.equal(
      'Right now, this device is the only one vouching for this sign-in.'
    );
  });

  it('alone sentence does not appear when anyone else is given', async () => {
    const el = await trail([
      { id: 'here', act: 'signed', relation: 'this-device', state: 'done' },
      { id: 'door', act: 'signed', relation: 'your-doorway', state: 'working' },
    ]);
    expect(q(el, '[part="alone"]')).to.equal(null);
  });
});

// ---------------------------------------------------------------------------
// Hard rule 4 — a failed step is said plainly and does not settle
// ---------------------------------------------------------------------------

describe('<elohim-imagodei-witness-trail> — failure', () => {
  const failed: WitnessStep[] = [
    { id: 'door', act: 'signed', relation: 'your-doorway', label: 'alpha', state: 'done' },
    {
      id: 'dev',
      act: 'signed',
      relation: 'your-device',
      label: 'workspace',
      state: 'failed',
      note: 'it did not answer in time',
    },
  ];

  it('says where it stopped, never what it did not do, with the note', async () => {
    const el = await trail(failed);
    expect(sentences(el)).to.deep.equal([
      'alpha signed this as you',
      'This stopped at your device workspace',
    ]);
    expect(text(el, '[part="note"]')).to.equal('it did not answer in time');
    expect(q(el, '[part="step"][data-state="failed"] [part="mark"]')).to.exist;
  });

  it('without a note, still says where it stopped', async () => {
    const el = await trail([{ ...failed[1]!, note: undefined }]);
    expect(sentences(el)).to.deep.equal(['This stopped at your device workspace']);
    expect(q(el, '[part="note"]')).to.equal(null);
  });

  it('does not fire settled', async () => {
    const el = await trail(DEVICE_IN_PROGRESS);
    let fired = false;
    el.addEventListener('settled', () => (fired = true));
    el.steps = failed;
    await el.updateComplete;
    await aTimeout(20);
    expect(fired).to.equal(false);
  });

  it('a failure shows the full list even in rotate layout', async () => {
    const el = await trail(failed, { layout: 'rotate' });
    expect(q(el, '[part="trail"]')!.getAttribute('data-layout')).to.equal('list');
    expect(q(el, '[part="slot"]')).to.equal(null);
    expect(q(el, '[part="note"]')).to.exist;
  });

  it('settled summary says it did not finish', async () => {
    const el = await trail(failed, { mode: 'settled' });
    expect(text(el, '[part="summary"]')).to.equal('This did not finish');
  });
});

// ---------------------------------------------------------------------------
// Hard rule 5 — motion is presentation only
// ---------------------------------------------------------------------------

describe('<elohim-imagodei-witness-trail> — motion is presentation only', () => {
  it('every animation sits inside a reduced-motion no-preference block', () => {
    const css = cssText();
    expect(css).to.not.contain('transition');
    const block = css.indexOf('@media (prefers-reduced-motion: no-preference) and (update: fast)');
    expect(block).to.be.greaterThan(-1);
    const uses = [...css.matchAll(/animation\s*:/g)].map(m => m.index!);
    expect(uses.length).to.be.greaterThan(0);
    for (const at of uses) {
      expect(at).to.be.greaterThan(block);
      // Before the next top-level block closes: the keyframes rule follows the media block.
      expect(at).to.be.lessThan(css.indexOf('@keyframes'));
    }
  });

  it('completion does not wait for any animation (settled fires before animationend)', async () => {
    const el = await trail(DEVICE_IN_PROGRESS, { layout: 'rotate' });
    let ended = false;
    el.shadowRoot!.addEventListener('animationend', () => (ended = true));
    let fired = false;
    el.addEventListener('settled', () => (fired = true));
    el.steps = DEVICE_COMPLETE;
    await el.updateComplete;
    expect(fired).to.equal(true);
    expect(ended).to.equal(false);
  });
});

// ---------------------------------------------------------------------------
// Sentence selection
// ---------------------------------------------------------------------------

describe('<elohim-imagodei-witness-trail> — sentences', () => {
  const cases: [Partial<WitnessStep>, string][] = [
    [
      { act: 'checked', relation: 'your-doorway', label: 'alpha' },
      'Your doorway alpha checked your password',
    ],
    [{ act: 'signed', relation: 'your-doorway', label: 'alpha' }, 'alpha signed this as you'],
    [{ act: 'signed', relation: 'your-doorway' }, 'Your doorway signed this as you'],
    [{ act: 'signed', relation: 'your-device', label: 'workspace' }, 'workspace signed it too'],
    [{ act: 'recorded', relation: 'others' }, 'Recorded where anyone can check it'],
    [{ act: 'seen', relation: 'others', count: 3 }, '3 others on the network have seen it'],
    [{ act: 'signed', relation: 'vouches-for-you', label: 'Jessica' }, 'Jessica vouches for you'],
    [{ act: 'signed', relation: 'vouches-for-you' }, 'Someone who knows you vouches for you'],
    [{ act: 'signed', relation: 'you' }, 'You signed this'],
    [
      { act: 'signed', relation: 'your-doorway', label: 'alpha', state: 'working' },
      'alpha is signing this as you…',
    ],
    [
      { act: 'seen', relation: 'others', count: 3, state: 'working' },
      '3 others on the network are looking at it…',
    ],
  ];
  for (const [partial, expected] of cases) {
    it(`${partial.relation}/${partial.act}/${partial.state ?? 'done'} → “${expected}”`, async () => {
      const el = await trail([{ id: 'x', state: 'done', ...partial } as WitnessStep]);
      expect(sentences(el)).to.deep.equal([expected]);
    });
  }

  it('uses no protocol vocabulary in any default sentence', async () => {
    const el = await trail([
      ...DEVICE_IN_PROGRESS,
      { id: 'v', act: 'checked', relation: 'vouches-for-you', label: 'Jessica', state: 'done' },
    ]);
    const all = text(el).toLowerCase();
    for (const word of ['dht', 'validation', 'attestation', 'cap grant', 'agent', 'hash']) {
      expect(all).to.not.include(word);
    }
  });

  it('a host may replace sentences through strings', async () => {
    const el = await trail(PASSWORD_ONLY);
    el.strings = { doorwayAlone: 'Solo la puerta.', heading: 'Quién responde' };
    await el.updateComplete;
    expect(text(el, '[part="alone"]')).to.equal('Solo la puerta.');
    expect(text(el, '[part="heading"]')).to.equal('Quién responde');
  });

  it('heading attribute overrides the heading', async () => {
    const el = await fixture<ElohimImagodeiWitnessTrail>(html`
      <elohim-imagodei-witness-trail
        .steps=${PASSWORD_ONLY}
        reveal-after-ms="0"
        heading="How your sign-in is secured"
      ></elohim-imagodei-witness-trail>
    `);
    expect(text(el, '[part="heading"]')).to.equal('How your sign-in is secured');
  });
});

// ---------------------------------------------------------------------------
// Reveal threshold (real short timers, as the sibling's expiry tests do)
// ---------------------------------------------------------------------------

describe('<elohim-imagodei-witness-trail> — reveal threshold', () => {
  it('shows nothing until revealAfterMs has passed with a step not done', async () => {
    const el = await trail(DEVICE_IN_PROGRESS, { reveal: 80 });
    expect(q(el, '[part="trail"]')).to.equal(null);
    await aTimeout(140);
    await el.updateComplete;
    expect(q(el, '[part="trail"]')).to.exist;
  });

  it('a sign-in that resolves before the threshold never flashes the trail', async () => {
    const el = await trail(DEVICE_IN_PROGRESS, { reveal: 80 });
    let fired = false;
    el.addEventListener('settled', () => (fired = true));
    await aTimeout(20);
    el.steps = DEVICE_COMPLETE;
    await el.updateComplete;
    expect(fired).to.equal(true);
    await aTimeout(140);
    await el.updateComplete;
    expect(q(el, '[part="trail"]')).to.equal(null);
  });

  it('an instantly resolved sign-in shows nothing at all', async () => {
    const el = await fixture<ElohimImagodeiWitnessTrail>(html`
      <elohim-imagodei-witness-trail .steps=${DEVICE_COMPLETE}></elohim-imagodei-witness-trail>
    `);
    await aTimeout(500);
    await el.updateComplete;
    expect(q(el, '[part="trail"]')).to.equal(null);
    expect(el.getBoundingClientRect().height).to.equal(0);
  });

  it('once shown, done steps stay visible after everything completes', async () => {
    const el = await trail(DEVICE_IN_PROGRESS, { reveal: 0 });
    el.steps = DEVICE_COMPLETE;
    await el.updateComplete;
    expect(qa(el, '[part="step"]').length).to.equal(3);
  });

  it('defaults to 400ms', async () => {
    const el = document.createElement('elohim-imagodei-witness-trail');
    expect(el.revealAfterMs).to.equal(400);
    expect(el.rotateMs).to.equal(1800);
    expect(el.layout).to.equal('rotate');
    expect(el.mode).to.equal('live');
  });
});

// ---------------------------------------------------------------------------
// Settled mode disclosure
// ---------------------------------------------------------------------------

describe('<elohim-imagodei-witness-trail> — settled mode', () => {
  it('one summary line, collapsed by default', async () => {
    const el = await trail(DEVICE_COMPLETE, { mode: 'settled' });
    const details = q<HTMLDetailsElement>(el, '[part="disclosure"]')!;
    expect(details.open).to.equal(false);
    expect(text(el, '[part="summary"]')).to.equal(
      'Secured by your doorway alpha, your device workspace, and 3 others'
    );
  });

  it('opens to the full sequence', async () => {
    const el = await trail(DEVICE_COMPLETE, { mode: 'settled', open: true });
    expect(q<HTMLDetailsElement>(el, '[part="disclosure"]')!.open).to.equal(true);
    expect(sentences(el)).to.deep.equal([
      'alpha signed this as you',
      'workspace signed it too',
      '3 others on the network have seen it',
    ]);
  });

  it('the person can open it; open reflects', async () => {
    const el = await trail(DEVICE_COMPLETE, { mode: 'settled' });
    q<HTMLElement>(el, '[part="summary"]')!.click();
    await aTimeout(20);
    await el.updateComplete;
    expect(el.open).to.equal(true);
    expect(el.hasAttribute('open')).to.equal(true);
  });

  it('is shown immediately (no reveal threshold) and has no live region', async () => {
    const el = await trail(DEVICE_COMPLETE, { mode: 'settled', reveal: 5000 });
    expect(q(el, '[part="summary"]')).to.exist;
    expect(q(el, '[aria-live]')).to.equal(null);
  });

  it('collapses one doorway that acted twice into one party', async () => {
    const el = await trail(
      [
        { id: 'a', act: 'checked', relation: 'your-doorway', label: 'alpha', state: 'done' },
        { id: 'b', act: 'signed', relation: 'your-doorway', label: 'alpha', state: 'done' },
      ],
      { mode: 'settled' }
    );
    expect(text(el, '[part="summary"]')).to.equal('Secured by your doorway alpha');
  });
});

// ---------------------------------------------------------------------------
// Announcements
// ---------------------------------------------------------------------------

describe('<elohim-imagodei-witness-trail> — announcements', () => {
  it('has exactly one polite live region', async () => {
    const el = await trail(DEVICE_IN_PROGRESS, { layout: 'rotate' });
    const regions = qa(el, '[aria-live]');
    expect(regions.length).to.equal(1);
    expect(regions[0]!.getAttribute('aria-live')).to.equal('polite');
    expect(q(el, '[part="slot"]')!.hasAttribute('aria-live')).to.equal(false);
  });

  it('coalesces steps that land together into one announcement', async () => {
    const el = await trail(DEVICE_IN_PROGRESS);
    el.steps = DEVICE_COMPLETE;
    await el.updateComplete;
    const said = plain(q(el, '[aria-live]')!.textContent!);
    expect(said).to.equal(
      'workspace signed it too. 3 others on the network have seen it. All done.'
    );
  });

  it('announces a failure plainly', async () => {
    const el = await trail(DEVICE_IN_PROGRESS);
    el.steps = [
      DEVICE_IN_PROGRESS[0]!,
      { ...DEVICE_IN_PROGRESS[1]!, state: 'failed', note: 'it did not answer' },
    ];
    await el.updateComplete;
    expect(plain(q(el, '[aria-live]')!.textContent!)).to.equal(
      'This stopped at your device workspace: it did not answer.'
    );
  });
});

// ---------------------------------------------------------------------------
// Rotate layout
// ---------------------------------------------------------------------------

describe('<elohim-imagodei-witness-trail> — rotate layout', function () {
  this.timeout(10_000);
  afterEach(() => clearMediaQueries());

  async function rotating(steps: WitnessStep[]) {
    const el = await fixture<ElohimImagodeiWitnessTrail>(html`
      <elohim-imagodei-witness-trail
        .steps=${steps}
        reveal-after-ms="0"
        rotate-ms="1200"
      ></elohim-imagodei-witness-trail>
    `);
    await el.updateComplete;
    return el;
  }

  it('one line in a slot hidden from assistive tech, with the full list for it', async () => {
    const el = await rotating(ROTATING);
    expect(q(el, '[part="trail"]')!.getAttribute('data-layout')).to.equal('rotate');
    expect(qa(el, '[part="line"]').length).to.equal(1);
    expect(q(el, '[part="slot"]')!.getAttribute('aria-hidden')).to.equal('true');
    const list = q(el, '[part="steps"]')!;
    expect(list.classList.contains('visually-hidden')).to.equal(true);
    expect(qa(el, '[part="step"]').length).to.equal(4);
  });

  it('one mark per given step; marks are buttons with a state and a position', async () => {
    const el = await rotating(DEVICE_IN_PROGRESS);
    const pips = qa<HTMLButtonElement>(el, '[part="pip"]');
    expect(pips.length).to.equal(3);
    expect(pips.map(p => p.getAttribute('data-state'))).to.deep.equal([
      'done',
      'working',
      'waiting',
    ]);
    expect(pips.map(p => p.getAttribute('aria-label'))).to.deep.equal([
      'Show step 1 of 3',
      'Show step 2 of 3',
      'Show step 3 of 3',
    ]);
  });

  it('cycles through working/done steps in order and skips waiting ones', async () => {
    const el = await rotating([
      ...DEVICE_IN_PROGRESS,
      { id: 'v', act: 'signed', relation: 'vouches-for-you', label: 'Jessica', state: 'done' },
    ]);
    const lines = await collectLines(el, 4, 4200);
    expect(lines).to.deep.equal([
      'alpha signed this as you',
      'workspace is signing it too…',
      'Jessica vouches for you',
      'alpha signed this as you',
    ]);
  });

  it('a step arriving mid-rotation is shown next, once, then the cycle resumes from the top', async () => {
    const el = await rotating([
      { id: 'a', act: 'signed', relation: 'your-doorway', label: 'alpha', state: 'done' },
      { id: 'b', act: 'signed', relation: 'your-device', label: 'workspace', state: 'done' },
      { id: 'c', act: 'seen', relation: 'others', count: 3, state: 'waiting' },
      { id: 'd', act: 'signed', relation: 'vouches-for-you', label: 'Jessica', state: 'done' },
    ]);
    expect(line(el)).to.equal('alpha signed this as you');
    el.steps = el.steps.map(s => (s.id === 'c' ? { ...s, state: 'done' as const } : s));
    await el.updateComplete;
    const lines = await collectLines(el, 4, 4200);
    expect(lines).to.deep.equal([
      'alpha signed this as you',
      '3 others on the network have seen it',
      'alpha signed this as you',
      'workspace signed it too',
    ]);
  });

  it('pauses while hovered and resumes after', async () => {
    const el = await rotating(ROTATING);
    const first = line(el);
    q(el, '[part="rotator"]')!.dispatchEvent(new PointerEvent('pointerenter'));
    await aTimeout(1500);
    expect(line(el)).to.equal(first);
    q(el, '[part="rotator"]')!.dispatchEvent(new PointerEvent('pointerleave'));
    const lines = await collectLines(el, 2, 1600);
    expect(lines.length).to.equal(2);
  });

  it('pauses while a mark has keyboard focus', async () => {
    const el = await rotating(ROTATING);
    const first = line(el);
    q<HTMLButtonElement>(el, '[part="pip"]')!.focus();
    await aTimeout(1500);
    expect(line(el)).to.equal(first);
  });

  it('a mark jumps to its step, waiting ones included', async () => {
    const el = await rotating(DEVICE_IN_PROGRESS);
    const pips = qa<HTMLButtonElement>(el, '[part="pip"]');
    pips[2]!.click();
    await el.updateComplete;
    expect(line(el)).to.equal('Recorded where anyone can check it');
    expect(q(el, '[part="line"]')!.getAttribute('data-state')).to.equal('waiting');
    expect(pips[2]!.getAttribute('aria-current')).to.equal('step');
    expect(pips[0]!.hasAttribute('aria-current')).to.equal(false);
  });

  it('a single step does not rotate', async () => {
    const el = await rotating(PASSWORD_ONLY);
    expect(q(el, '[part="pips"]')).to.equal(null);
    expect((el as unknown as { _rotateTimer?: unknown })._rotateTimer).to.equal(undefined);
  });

  it('only one step under way does not rotate either', async () => {
    const el = await rotating([
      PASSWORD_ONLY[0]!,
      { id: 'w', act: 'recorded', relation: 'others', state: 'waiting' },
    ]);
    expect((el as unknown as { _rotateTimer?: unknown })._rotateTimer).to.equal(undefined);
  });

  it('prefers-reduced-motion: no rotation, the list instead', async () => {
    setMediaQuery('prefers-reduced-motion', 'reduce');
    const el = await rotating(ROTATING);
    expect(q(el, '[part="trail"]')!.getAttribute('data-layout')).to.equal('list');
    expect(q(el, '[part="slot"]')).to.equal(null);
    expect(q(el, '[part="steps"]')!.classList.contains('visually-hidden')).to.equal(false);
    expect((el as unknown as { _rotateTimer?: unknown })._rotateTimer).to.equal(undefined);
  });

  it('update: slow (e-paper): the list instead', async () => {
    setMediaQuery('update', 'slow');
    const el = await rotating(ROTATING);
    expect(q(el, '[part="trail"]')!.getAttribute('data-layout')).to.equal('list');
  });

  it('a profile locked to still: the list instead', async () => {
    const el = await rotating(ROTATING);
    el.profile = { ...DEFAULT_PROFILE, lock: { kind: 'steward', pinnedStimulus: 'still' } };
    await el.updateComplete;
    expect(q(el, '[part="trail"]')!.getAttribute('data-layout')).to.equal('list');
    expect((el as unknown as { _rotateTimer?: unknown })._rotateTimer).to.equal(undefined);
  });

  it('settled fires the moment all are done, wherever the rotation is (even paused)', async () => {
    const el = await rotating(DEVICE_IN_PROGRESS);
    qa<HTMLButtonElement>(el, '[part="pip"]')[2]!.click();
    q(el, '[part="rotator"]')!.dispatchEvent(new PointerEvent('pointerenter'));
    await el.updateComplete;
    let fired = false;
    el.addEventListener('settled', () => (fired = true));
    el.steps = DEVICE_COMPLETE;
    await el.updateComplete;
    expect(fired).to.equal(true);
  });

  it('the slot does not change height as lines change, at 320px', async () => {
    const steps: WitnessStep[] = [
      { id: 'a', act: 'seen', relation: 'others', count: 3, state: 'done' },
      { id: 'b', act: 'checked', relation: 'your-doorway', label: 'alpha', state: 'done' },
      {
        id: 'c',
        act: 'recorded',
        relation: 'your-doorway',
        label: 'jessica-desk',
        state: 'working',
      },
      { id: 'd', act: 'signed', relation: 'you', state: 'done' },
      {
        // A pathological name: the line is clamped to the two rows the slot reserves.
        id: 'e',
        act: 'signed',
        relation: 'vouches-for-you',
        label: 'Jessica Abigail Dowell-Montgomery of the Riverside Household Circle',
        state: 'done',
      },
    ];
    const wrapper = await fixture<HTMLDivElement>(html`
      <div style="inline-size: 320px;">
        <elohim-imagodei-witness-trail
          .steps=${steps}
          reveal-after-ms="0"
        ></elohim-imagodei-witness-trail>
      </div>
    `);
    const el = wrapper.firstElementChild as ElohimImagodeiWitnessTrail;
    await el.updateComplete;
    const heights = new Set<number>();
    const rotatorHeights = new Set<number>();
    const lineHeights = new Set<number>();
    for (const pip of qa<HTMLButtonElement>(el, '[part="pip"]')) {
      pip.click();
      await el.updateComplete;
      heights.add(q(el, '[part="slot"]')!.getBoundingClientRect().height);
      rotatorHeights.add(q(el, '[part="rotator"]')!.getBoundingClientRect().height);
      lineHeights.add(Math.round(q(el, '[part="line"]')!.getBoundingClientRect().height));
    }
    expect(heights.size).to.equal(1);
    expect(rotatorHeights.size).to.equal(1);
    // The fixture really does include a one-row and a two-row line.
    expect(lineHeights.size).to.be.greaterThan(1);
    const [slot] = [...heights];
    // Lines are rounded to whole pixels above; allow that rounding.
    expect(slot! + 1).to.be.at.least(Math.max(...lineHeights));
  });

  it('stops its timers when removed', async () => {
    const el = await rotating(ROTATING);
    el.remove();
    expect((el as unknown as { _rotateTimer?: unknown })._rotateTimer).to.equal(undefined);
  });
});

// ---------------------------------------------------------------------------
// a11y precondition gate
// ---------------------------------------------------------------------------

describe('<elohim-imagodei-witness-trail> — a11y precondition gate', () => {
  const variants: [string, () => Promise<ElohimImagodeiWitnessTrail>][] = [
    ['list in progress', () => trail(DEVICE_IN_PROGRESS)],
    ['rotate in progress', () => trail(DEVICE_IN_PROGRESS, { layout: 'rotate' })],
    ['password only', () => trail(PASSWORD_ONLY)],
    ['settled collapsed', () => trail(DEVICE_COMPLETE, { mode: 'settled' })],
    ['settled open', () => trail(DEVICE_COMPLETE, { mode: 'settled', open: true })],
    [
      'failed',
      () =>
        trail([
          DEVICE_IN_PROGRESS[0]!,
          { ...DEVICE_IN_PROGRESS[1]!, state: 'failed', note: 'it did not answer' },
        ]),
    ],
  ];
  for (const [name, make] of variants) {
    it(`passes axe: ${name}`, async () => {
      const el = await make();
      const results = await axe.run(el);
      expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
    });
  }

  it('the steps are an ordered list', async () => {
    const el = await trail(DEVICE_IN_PROGRESS);
    expect(q(el, '[part="steps"]')!.tagName).to.equal('OL');
  });

  it('state is never colour-only: shape, fill and a state word', async () => {
    const el = await trail([
      { id: 'a', act: 'signed', relation: 'your-doorway', state: 'waiting' },
      { id: 'b', act: 'signed', relation: 'your-doorway', state: 'working' },
      { id: 'c', act: 'signed', relation: 'your-doorway', state: 'done' },
      { id: 'd', act: 'signed', relation: 'your-doorway', state: 'failed' },
    ]);
    const marks = qa(el, '[part="step"] [part="mark"]').map(m => getComputedStyle(m));
    const signature = marks.map(
      m => `${m.borderTopLeftRadius}|${m.backgroundImage}|${m.backgroundColor}`
    );
    expect(new Set(signature).size).to.equal(4);
    const words = qa(el, '[part="step"] .visually-hidden').map(n => plain(n.textContent!));
    expect(words).to.deep.equal(['(not yet)', '(happening now)', '(done)', '(did not go through)']);
  });

  it('marks and the summary are keyboard-reachable', async () => {
    const r = await trail(DEVICE_IN_PROGRESS, { layout: 'rotate' });
    for (const pip of qa<HTMLButtonElement>(r, '[part="pip"]')) {
      expect(pip.tabIndex).to.equal(0);
    }
    const s = await trail(DEVICE_COMPLETE, { mode: 'settled' });
    expect(q(s, '[part="summary"]')!.tagName).to.equal('SUMMARY');
  });
});

// ---------------------------------------------------------------------------
// theme-contrast precondition gate (system cells — blank-slate)
// ---------------------------------------------------------------------------

describe('<elohim-imagodei-witness-trail> — theme-contrast precondition gate', () => {
  const CELLS: ThemeCell[] = ['system-light', 'system-dark'];
  for (const cell of CELLS) {
    it(`list holds WCAG contrast in ${cell}`, async () => {
      const { el } = await themeFixture<ElohimImagodeiWitnessTrail>(
        html`
          <elohim-imagodei-witness-trail
            .steps=${DEVICE_IN_PROGRESS}
            layout="list"
            reveal-after-ms="0"
          ></elohim-imagodei-witness-trail>
        `,
        cell
      );
      await el.updateComplete;
      assertThemeContrast(el);
    });

    it(`rotate holds WCAG contrast in ${cell}`, async () => {
      const { el } = await themeFixture<ElohimImagodeiWitnessTrail>(
        html`
          <elohim-imagodei-witness-trail
            .steps=${DEVICE_IN_PROGRESS}
            reveal-after-ms="0"
          ></elohim-imagodei-witness-trail>
        `,
        cell
      );
      await el.updateComplete;
      assertThemeContrast(el);
    });
  }
});

// ---------------------------------------------------------------------------
// ua-prefs precondition gate
// ---------------------------------------------------------------------------

describe('<elohim-imagodei-witness-trail> — ua-prefs precondition gate', () => {
  afterEach(() => clearMediaQueries());

  it('CSS has a forced-colors override block', () => {
    expect(cssText()).to.contain('forced-colors');
  });

  it('CSS enlarges touch targets under coarse pointers', () => {
    expect(cssText()).to.match(/@media \(pointer: coarse\)/);
  });

  it('passes the photosensitive-flash analyzer while rotating', async () => {
    const el = await fixture<ElohimImagodeiWitnessTrail>(html`
      <elohim-imagodei-witness-trail
        .steps=${ROTATING}
        reveal-after-ms="0"
        rotate-ms="1200"
      ></elohim-imagodei-witness-trail>
    `);
    const result = await measureLuminanceChanges(el, { sampleMs: 1500, sampleHz: 30 });
    expect(result.exceedsThreshold).to.be.false;
  });

  it('clamps the rotation interval to at least 1200ms', async () => {
    const el = await fixture<ElohimImagodeiWitnessTrail>(html`
      <elohim-imagodei-witness-trail
        .steps=${ROTATING}
        reveal-after-ms="0"
        rotate-ms="100"
      ></elohim-imagodei-witness-trail>
    `);
    await el.updateComplete;
    expect((el as unknown as { _rotateInterval?: number })._rotateInterval).to.equal(1200);
  });
});

// ---------------------------------------------------------------------------
// i18n precondition gate
// ---------------------------------------------------------------------------

describe('<elohim-imagodei-witness-trail> — i18n precondition gate', () => {
  it('renders in RTL document direction (he-IL)', async () => {
    const el = await renderInLocale<ElohimImagodeiWitnessTrail>(
      'he-IL',
      html`
        <elohim-imagodei-witness-trail
          .steps=${DEVICE_IN_PROGRESS}
          reveal-after-ms="0"
        ></elohim-imagodei-witness-trail>
      `
    );
    expect(document.documentElement.getAttribute('dir')).to.equal('rtl');
    expect(q(el, '[part="trail"]')!.getBoundingClientRect().width).to.be.greaterThan(0);
  });

  it('isolates person-supplied names so they cannot reorder the sentence', async () => {
    const el = await trail([
      { id: 'd', act: 'signed', relation: 'your-device', label: 'עבודה', state: 'done' },
      { id: 'v', act: 'signed', relation: 'vouches-for-you', label: 'Jessica', state: 'done' },
    ]);
    const raw = qa(el, '[part="step"] [part="sentence"]').map(n => n.textContent!);
    expect(raw[0]).to.include('⁨עבודה⁩');
    expect(raw[1]).to.include('⁨Jessica⁩');
  });

  it('an English sentence keeps its own direction in an RTL page, whatever name it starts with', async () => {
    const wrapper = await fixture<HTMLDivElement>(html`
      <div dir="rtl" lang="he">
        <elohim-imagodei-witness-trail
          .steps=${[
            { id: 'a', act: 'signed', relation: 'your-doorway', label: 'alpha', state: 'done' },
            { id: 'b', act: 'signed', relation: 'your-device', label: 'עבודה', state: 'working' },
          ]}
          layout="list"
          reveal-after-ms="0"
        ></elohim-imagodei-witness-trail>
      </div>
    `);
    const el = wrapper.firstElementChild as ElohimImagodeiWitnessTrail;
    await el.updateComplete;
    for (const sentence of qa(el, '[part="sentence"]')) {
      expect(sentence.matches(':dir(ltr)'), sentence.textContent!).to.equal(true);
    }
    // The list itself still follows the page.
    expect(q(el, '[part="steps"]')!.matches(':dir(rtl)')).to.equal(true);
  });

  it('a translated (Hebrew) sentence takes the right-to-left direction', async () => {
    const el = await trail(PASSWORD_ONLY);
    el.strings = {
      lang: 'he',
      dir: 'rtl',
      workingSentences: {
        ...WITNESS_TRAIL_STRINGS_EN.workingSentences,
        'your-doorway': {
          ...WITNESS_TRAIL_STRINGS_EN.workingSentences['your-doorway'],
          checked: ({ name }) => `השער ${name ?? ''} בודק את הסיסמה שלך…`,
        },
      },
    };
    await el.updateComplete;
    expect(q(el, '[part="sentence"]')!.matches(':dir(rtl)')).to.equal(true);
    expect(q(el, '[part="sentence"]')!.getAttribute('lang')).to.equal('he');
  });

  it('uses no physical CSS properties (only logical or non-positional)', () => {
    const findings = requiresLogicalProperties(cssText());
    expect(findings, JSON.stringify(findings, null, 2)).to.have.lengthOf(0);
  });

  for (const layout of ['list', 'rotate'] as const) {
    it(`fits a 320px-wide container without horizontal overflow (${layout})`, async () => {
      const wrapper = await fixture<HTMLDivElement>(html`
        <div style="inline-size: 320px;">
          <elohim-imagodei-witness-trail
            .steps=${[
              ...DEVICE_IN_PROGRESS,
              {
                id: 'v',
                act: 'checked',
                relation: 'vouches-for-you',
                label: 'Jessica',
                state: 'waiting',
              },
            ]}
            layout=${layout}
            reveal-after-ms="0"
          ></elohim-imagodei-witness-trail>
        </div>
      `);
      const el = wrapper.firstElementChild as ElohimImagodeiWitnessTrail;
      await el.updateComplete;
      expect(q(el, '[part="trail"]')!.scrollWidth).to.be.at.most(320);
    });
  }
});
