import { CapabilityAwareElement } from 'elohim-core';
import { css, html, LitElement, nothing, type PropertyValues } from 'lit';
import { property, state } from 'lit/decorators.js';
import { keyed } from 'lit/directives/keyed.js';

/** What happened at one step. */
export type WitnessAct = 'checked' | 'signed' | 'recorded' | 'seen';

/**
 * Who did it, from the person's point of view. `this-device` is the node on
 * the machine the person is using right now.
 */
export type WitnessRelation =
  | 'you'
  | 'this-device'
  | 'your-device'
  | 'your-doorway'
  | 'vouches-for-you'
  | 'others';

export type WitnessStepState = 'waiting' | 'working' | 'done' | 'failed';

/** One step of the trail, exactly as the host observed it. */
export interface WitnessStep {
  id: string;
  act: WitnessAct;
  relation: WitnessRelation;
  /** The party's name as the person knows it. Never read for `others`. */
  label?: string;
  /** How many others took part. Only read for `others`. */
  count?: number;
  state: WitnessStepState;
  /** Plain reason, shown when the step failed. */
  note?: string;
}

export type WitnessTrailMode = 'live' | 'settled';
export type WitnessTrailLayout = 'rotate' | 'list';

export interface WitnessTrailSettledDetail {
  stepIds: string[];
}

/**
 * Who a sentence is about. `name` is already bidi-isolated by the element and
 * is never supplied for `others`; `count` is only supplied for `others`.
 */
export interface WitnessWho {
  name?: string;
  count?: number;
}

export type WitnessSentence = (who: WitnessWho) => string;
export type WitnessSentenceTable = Record<WitnessRelation, Record<WitnessAct, WitnessSentence>>;

/**
 * Every visible sentence the trail speaks. Hosts may replace any subset through
 * the `strings` property; the English defaults below fill whatever is missing.
 */
export interface WitnessTrailStrings {
  /** Language of these strings (BCP 47). */
  lang: string;
  /**
   * Writing direction of these strings. Every sentence is rendered in it, so an
   * English sentence stays left-to-right on a right-to-left page (and a name
   * at its start cannot flip it), while a Hebrew table reads right-to-left.
   */
  dir: 'ltr' | 'rtl';
  heading: string;
  /** What a party did (a `done`, `waiting` or `failed` step). */
  sentences: WitnessSentenceTable;
  /** What a party is doing right now (a `working` step). */
  workingSentences: WitnessSentenceTable;
  /** A party named inside a longer sentence (summary, failure). */
  party: Record<WitnessRelation, WitnessSentence>;
  stateWord: Record<WitnessStepState, string>;
  /**
   * Where a failed step stopped. Shown as the failed step's line (without the
   * note, which follows it as the host wrote it) and announced with the note.
   */
  failed: (party: string, note?: string) => string;
  doorwayAlone: string;
  thisDeviceAlone: string;
  joinParties: (parties: string[]) => string;
  summary: (parties: string) => string;
  summaryFailed: string;
  summaryNothing: string;
  showStep: (position: number, total: number) => string;
  stepsLabel: string;
  announce: (lines: string[], allDone: boolean) => string;
}

const EN_NUMBER = new Intl.NumberFormat('en');
const EN_LIST = new Intl.ListFormat('en', { style: 'long', type: 'conjunction' });

const hasCount = (who: WitnessWho): who is { count: number } =>
  typeof who.count === 'number' && who.count > 0;

/** "1 other on the network has" / "3 others on the network have". */
const net = (count: number, one: string, many: string): string =>
  count === 1
    ? `${EN_NUMBER.format(count)} other on the network ${one}`
    : `${EN_NUMBER.format(count)} others on the network ${many}`;

const nOthers = (count: number): string =>
  `${EN_NUMBER.format(count)} ${count === 1 ? 'other' : 'others'}`;

const thisDevice = (name: string | undefined, rest: string): string =>
  name ? `This device, ${name}, ${rest}` : `This device ${rest}`;

const someone = (name?: string): string => name ?? 'Someone who knows you';

const CHECKED_IT = 'checked it';
const THIS_DEVICE: WitnessRelation = 'this-device';

export const WITNESS_TRAIL_STRINGS_EN: WitnessTrailStrings = {
  lang: 'en',
  dir: 'ltr',
  heading: 'Who is vouching for this',
  sentences: {
    you: {
      checked: () => 'You confirmed it',
      signed: () => 'You signed this',
      recorded: () => 'You kept a record of it',
      seen: () => 'You have seen it',
    },
    'this-device': {
      checked: ({ name }) => thisDevice(name, CHECKED_IT),
      signed: ({ name }) => thisDevice(name, 'signed it'),
      recorded: ({ name }) => thisDevice(name, 'kept a record of it'),
      seen: ({ name }) => thisDevice(name, 'has seen it'),
    },
    'your-doorway': {
      checked: ({ name }) =>
        name ? `Your doorway ${name} checked your password` : 'Your doorway checked your password',
      signed: ({ name }) =>
        name ? `${name} signed this as you` : 'Your doorway signed this as you',
      recorded: ({ name }) =>
        name
          ? `${name} recorded it where anyone can check it`
          : 'Your doorway recorded it where anyone can check it',
      seen: ({ name }) => (name ? `${name} has seen it` : 'Your doorway has seen it'),
    },
    'your-device': {
      checked: ({ name }) =>
        name ? `Your device ${name} checked its key` : 'Your device checked its key',
      signed: ({ name }) => (name ? `${name} signed it too` : 'Your device signed it too'),
      recorded: ({ name }) =>
        name ? `${name} kept a record of it` : 'Your device kept a record of it',
      seen: ({ name }) => (name ? `${name} has seen it` : 'Your device has seen it'),
    },
    'vouches-for-you': {
      checked: ({ name }) => `${someone(name)} checked that it is really you`,
      signed: ({ name }) => `${someone(name)} vouches for you`,
      recorded: ({ name }) => `${someone(name)} kept a record of it`,
      seen: ({ name }) => `${someone(name)} has seen it`,
    },
    others: {
      checked: who =>
        hasCount(who)
          ? net(who.count, CHECKED_IT, CHECKED_IT)
          : 'Others on the network can check it',
      signed: who =>
        hasCount(who)
          ? net(who.count, 'signed it too', 'signed it too')
          : 'Others on the network signed it too',
      recorded: who =>
        hasCount(who)
          ? net(who.count, 'recorded it', 'recorded it')
          : 'Recorded where anyone can check it',
      seen: who =>
        hasCount(who)
          ? net(who.count, 'has seen it', 'have seen it')
          : 'Others on the network can check it',
    },
  },
  workingSentences: {
    you: {
      checked: () => 'You are confirming it…',
      signed: () => 'You are signing this…',
      recorded: () => 'You are keeping a record of it…',
      seen: () => 'You are looking at it…',
    },
    'this-device': {
      checked: ({ name }) => thisDevice(name, 'is checking it…'),
      signed: ({ name }) => thisDevice(name, 'is signing…'),
      recorded: ({ name }) => thisDevice(name, 'is keeping a record…'),
      seen: ({ name }) => thisDevice(name, 'is looking at it…'),
    },
    'your-doorway': {
      checked: ({ name }) =>
        name
          ? `Your doorway ${name} is checking your password…`
          : 'Your doorway is checking your password…',
      signed: ({ name }) =>
        name ? `${name} is signing this as you…` : 'Your doorway is signing this as you…',
      recorded: ({ name }) =>
        name
          ? `${name} is recording it where anyone can check it…`
          : 'Your doorway is recording it where anyone can check it…',
      seen: ({ name }) => (name ? `${name} is looking at it…` : 'Your doorway is looking at it…'),
    },
    'your-device': {
      checked: ({ name }) =>
        name ? `Your device ${name} is checking its key…` : 'Your device is checking its key…',
      signed: ({ name }) =>
        name ? `${name} is signing it too…` : 'Your device is signing it too…',
      recorded: ({ name }) =>
        name ? `${name} is keeping a record of it…` : 'Your device is keeping a record of it…',
      seen: ({ name }) => (name ? `${name} is looking at it…` : 'Your device is looking at it…'),
    },
    'vouches-for-you': {
      checked: ({ name }) => `${someone(name)} is checking that it is really you…`,
      signed: ({ name }) => `${someone(name)} is vouching for you…`,
      recorded: ({ name }) => `${someone(name)} is keeping a record of it…`,
      seen: ({ name }) => `${someone(name)} is looking at it…`,
    },
    others: {
      checked: who =>
        hasCount(who)
          ? net(who.count, 'is checking it…', 'are checking it…')
          : 'Others on the network are checking it…',
      signed: who =>
        hasCount(who)
          ? net(who.count, 'is signing it too…', 'are signing it too…')
          : 'Others on the network are signing it too…',
      recorded: who =>
        hasCount(who)
          ? net(who.count, 'is recording it…', 'are recording it…')
          : 'Recording it where anyone can check it…',
      seen: who =>
        hasCount(who)
          ? net(who.count, 'is looking at it…', 'are looking at it…')
          : 'Others on the network are looking at it…',
    },
  },
  party: {
    you: () => 'you',
    'this-device': ({ name }) => (name ? `this device (${name})` : 'this device'),
    'your-doorway': ({ name }) => (name ? `your doorway ${name}` : 'your doorway'),
    'your-device': ({ name }) => (name ? `your device ${name}` : 'your device'),
    'vouches-for-you': ({ name }) => name ?? 'someone who knows you',
    others: who => (hasCount(who) ? nOthers(who.count) : 'others on the network'),
  },
  stateWord: {
    waiting: 'not yet',
    working: 'happening now',
    done: 'done',
    failed: 'did not go through',
  },
  failed: (party, note) =>
    note ? `This stopped at ${party}: ${note}` : `This stopped at ${party}`,
  doorwayAlone: 'This sign-in rests on your doorway alone.',
  thisDeviceAlone: 'Right now, this device is the only one vouching for this sign-in.',
  joinParties: parties => EN_LIST.format(parties),
  summary: parties => `Secured by ${parties}`,
  summaryFailed: 'This did not finish',
  summaryNothing: 'Nothing has vouched for this yet',
  showStep: (position, total) => `Show step ${position} of ${total}`,
  stepsLabel: 'Every step',
  announce: (lines, allDone) => `${lines.join('. ')}.${allDone ? ' All done.' : ''}`,
};

const KNOWN_ACTS = new Set<string>(['checked', 'signed', 'recorded', 'seen']);
const KNOWN_RELATIONS = new Set<string>([
  'you',
  THIS_DEVICE,
  'your-device',
  'your-doorway',
  'vouches-for-you',
  'others',
]);
const MIN_ROTATE_MS = 1200;

/**
 * Wrap a person-supplied name in Unicode first-strong isolates so it cannot
 * reorder the sentence around it when the two run in different directions.
 */
const isolate = (value: string): string => `⁨${value}⁩`;

const inRotation = (s: WitnessStep): boolean => s.state === 'working' || s.state === 'done';

/**
 * <elohim-imagodei-witness-trail> — while a sign-in or device approval is
 * resolving, shows the person who is actually vouching for it, step by step;
 * afterwards the same steps stay available as a short "how this was secured"
 * summary.
 *
 * It shows only what it is given. It never invents, pads or reorders steps and
 * never holds anything back: when every step is `done` it fires `settled` in
 * the same update. What happens next is the host's decision. The one timer
 * that delays *display* (`revealAfterMs`) only keeps a fast sign-in from
 * flashing the trail; it never delays completion.
 *
 * Other people's participation is shown as a count only. For `relation:
 * 'others'` the `label` is never read; names appear only for the person's own
 * circle (`you`, `this-device`, `your-device`, `your-doorway`,
 * `vouches-for-you`).
 *
 * Ordering: the one ordering the element imposes is that `this-device` steps
 * (the node on the machine the person is using right now) come first — in
 * both layouts and in the settled summary — whatever position they were given
 * in. Every other step keeps the order it was given.
 *
 * Honest answers when nobody else is involved: when every step is
 * `your-doorway` the trail adds that this sign-in rests on the doorway alone;
 * when every step is `this-device` it adds that this device is, right now,
 * the only one vouching.
 *
 * Layouts (live mode): `rotate` shows one line at a time in a fixed two-line
 * slot, cycling through steps that are `working` or `done` from the top. A
 * step that newly arrives mid-rotation is shown next, once, and the cycle then
 * continues from the top. One mark per step underneath jumps to that step.
 * Rotation pauses on hover and focus. The slot is hidden from assistive tech,
 * which reads the full ordered list instead. `list` shows the whole sequence.
 * Rotation falls back to `list` under `prefers-reduced-motion: reduce`,
 * `update: slow`, a Capability Profile lock pinning stimulus to `still`, or
 * when a step has failed (a failure is said plainly, in full).
 *
 * Settled mode: one summary line in a native disclosure that opens to the
 * full sequence.
 *
 * Stimulus: `gentle`. A rotating line and a 180ms fade-in when a line or mark
 * changes. Justification: the rotation is the point of the live layout (one
 * spot that names who is vouching, rather than a spinner); it changes at most
 * once per 1.2s, never flashes, pauses for hover/focus, and is removed
 * entirely (list fallback, no fades) for reduced-motion, e-paper and a
 * still-locked profile. Nothing depends on an animation finishing.
 *
 * @element elohim-imagodei-witness-trail
 *
 * @prop {WitnessStep[]} steps - The observed steps, in order (property only)
 * @prop {WitnessTrailMode} mode - live | settled (default live)
 * @prop {WitnessTrailLayout} layout - rotate | list (default rotate; live mode only)
 * @prop {number} revealAfterMs - Live mode shows nothing until this long has passed with a step not done (default 400)
 * @prop {number} rotateMs - Rotation interval in ms (default 1800, minimum 1200)
 * @prop {string} heading - Replace the heading (live mode)
 * @prop {boolean} open - Settled mode: the disclosure is open
 * @prop {Partial<WitnessTrailStrings>} strings - Replace any visible sentence (property only)
 *
 * @fires {CustomEvent<{stepIds: string[]}>} settled - Every given step is done (live mode); fired once per completed set, in the same update
 *
 * @cssprop --elohim-witness-trail-gap - Gap between the trail's sections (default: 0.5rem)
 * @cssprop --elohim-witness-trail-step-gap - Gap between steps in the list (default: 0.375rem)
 * @cssprop --elohim-witness-trail-line-height - Line height; the rotate slot reserves two lines of it (default: 1.4)
 * @cssprop --elohim-witness-trail-muted-opacity - Opacity of waiting steps and secondary text (default: 0.72)
 * @cssprop --elohim-witness-trail-current-weight - Font weight of the step happening now (default: bold)
 * @cssprop --elohim-witness-trail-mark-size - State mark diameter (default: 0.8em)
 * @cssprop --elohim-witness-trail-mark-color - State mark color (default: currentColor)
 * @cssprop --elohim-witness-trail-failed-color - Failed mark and note color (default: currentColor)
 * @cssprop --elohim-witness-trail-note-border - Failure note inline-start rule (FULL shorthand; default: 3px solid 40% currentColor mix)
 * @cssprop --elohim-witness-trail-slot-bg - Rotate slot background (default: transparent)
 * @cssprop --elohim-witness-trail-slot-padding - Rotate slot inline padding (default: 0)
 * @cssprop --elohim-witness-trail-radius - Border radius for the slot and pips (default: 6px)
 * @cssprop --elohim-witness-trail-focus-ring - Focus outline (FULL shorthand; default: 2px solid currentColor)
 * @cssprop --elohim-witness-trail-ease-duration - Fade-in when a line or mark changes (default: 180ms; never applied under reduced motion)
 *
 * @csspart trail - The outer container (carries data-mode, data-layout)
 * @csspart heading - The heading line (live mode)
 * @csspart steps - The ordered list of steps
 * @csspart step - One step (carries data-state, data-relation)
 * @csspart mark - A state mark (shape and fill carry the state)
 * @csspart sentence - The sentence of a step
 * @csspart note - The plain failure sentence of a failed step
 * @csspart alone - The "rests on your doorway alone" sentence
 * @csspart rotator - The rotate layout wrapper (slot + pips)
 * @csspart slot - The fixed-height rotate slot
 * @csspart line - The line currently shown in the slot (carries data-state)
 * @csspart pips - The row of step marks under the slot
 * @csspart pip - One step mark button (carries data-state, aria-current when shown)
 * @csspart disclosure - Settled mode details element
 * @csspart summary - Settled mode summary line
 *
 * @capabilityMaxLens standard
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en, es, he
 * @capabilityMaxStimulus gentle
 * @capabilityTextuality textual
 * @capabilityRequiredStandings any
 * @capabilityContentCertainty observed
 * @capabilityStates empty:designed, loading:designed, error:designed, stale:n/a, contested:n/a, offline:n/a
 */
export class ElohimImagodeiWitnessTrail extends CapabilityAwareElement(LitElement) {
  static override readonly styles = css`
    :host {
      display: block;
      font: inherit;
      min-inline-size: 0;
      line-height: var(--elohim-witness-trail-line-height, 1.4);
    }

    [part='trail'] {
      display: grid;
      gap: var(--elohim-witness-trail-gap, 0.5rem);
      min-inline-size: 0;
    }

    p {
      margin: 0;
    }

    [part='heading'] {
      font-weight: bold;
      overflow-wrap: anywhere;
    }

    [part='steps'] {
      display: grid;
      gap: var(--elohim-witness-trail-step-gap, 0.375rem);
      margin: 0;
      padding: 0;
      list-style: none;
      min-inline-size: 0;
    }

    [part='step'] {
      display: grid;
      grid-template-columns: auto 1fr;
      column-gap: 0.5rem;
      align-items: baseline;
      min-inline-size: 0;
    }

    [part='step'][data-state='waiting'] [part='sentence'],
    [part='line'][data-state='waiting'] [part='sentence'] {
      opacity: var(--elohim-witness-trail-muted-opacity, 0.72);
    }

    [part='step'][data-state='working'] [part='sentence'],
    [part='line'][data-state='working'] [part='sentence'] {
      font-weight: var(--elohim-witness-trail-current-weight, bold);
    }

    [part='sentence'] {
      overflow-wrap: anywhere;
    }

    /* Sentences carry the strings table's own direction; their placement follows the
       page (grid start), so an English line sits on the right of a right-to-left page. */
    [part='heading'],
    [part='step'] [part='sentence'],
    [part='note'],
    [part='alone'] {
      justify-self: start;
      max-inline-size: 100%;
    }

    [part='note'] {
      grid-column: 2;
      padding-inline-start: 0.5rem;
      border-inline-start: var(
        --elohim-witness-trail-note-border,
        3px solid color-mix(in srgb, currentColor 40%, transparent)
      );
      color: var(--elohim-witness-trail-failed-color, currentColor);
      overflow-wrap: anywhere;
    }

    [part='alone'] {
      opacity: var(--elohim-witness-trail-muted-opacity, 0.72);
      overflow-wrap: anywhere;
    }

    /* State marks: shape and fill carry the state, never colour alone.
       waiting = hollow circle, working = half-filled circle,
       done = filled circle, failed = hollow square with a bar. */
    [part='mark'] {
      display: inline-block;
      box-sizing: border-box;
      inline-size: var(--elohim-witness-trail-mark-size, 0.8em);
      block-size: var(--elohim-witness-trail-mark-size, 0.8em);
      border: 2px solid var(--elohim-witness-trail-mark-color, currentColor);
      border-radius: 50%;
      background: transparent;
      flex: none;
    }

    [part='mark'][data-state='working'] {
      background: linear-gradient(
        to right,
        var(--elohim-witness-trail-mark-color, currentColor) 50%,
        transparent 50%
      );
    }

    :host(:dir(rtl)) [part='mark'][data-state='working'] {
      background: linear-gradient(
        to left,
        var(--elohim-witness-trail-mark-color, currentColor) 50%,
        transparent 50%
      );
    }

    [part='mark'][data-state='done'] {
      background: var(--elohim-witness-trail-mark-color, currentColor);
    }

    [part='mark'][data-state='failed'] {
      border-color: var(--elohim-witness-trail-failed-color, currentColor);
      border-radius: 1px;
      background: linear-gradient(
          var(--elohim-witness-trail-failed-color, currentColor),
          var(--elohim-witness-trail-failed-color, currentColor)
        )
        center / 100% 2px no-repeat;
    }

    [part='rotator'] {
      display: grid;
      gap: 0.375rem;
      min-inline-size: 0;
    }

    [part='slot'] {
      box-sizing: content-box;
      block-size: calc(2em * var(--elohim-witness-trail-line-height, 1.4));
      overflow: hidden;
      padding-inline: var(--elohim-witness-trail-slot-padding, 0);
      background: var(--elohim-witness-trail-slot-bg, transparent);
      border-radius: var(--elohim-witness-trail-radius, 6px);
      min-inline-size: 0;
    }

    [part='line'] {
      display: flex;
      gap: 0.5rem;
      align-items: baseline;
      min-inline-size: 0;
    }

    /* Never more than the two rows the slot reserves, however long a name. */
    [part='line'] [part='sentence'] {
      display: -webkit-box;
      overflow: hidden;
      -webkit-box-orient: vertical;
      -webkit-line-clamp: 2;
      line-clamp: 2;
    }

    [part='pips'] {
      display: flex;
      flex-wrap: wrap;
      gap: 0.125rem;
      margin: 0;
      padding: 0;
    }

    [part='pip'] {
      display: inline-grid;
      place-items: center;
      min-inline-size: 1.5rem;
      min-block-size: 1.5rem;
      padding: 0;
      border: 0;
      border-radius: var(--elohim-witness-trail-radius, 6px);
      background: transparent;
      color: inherit;
      font: inherit;
      cursor: pointer;
    }

    [part='pip'] [part='mark'] {
      --elohim-witness-trail-mark-size: 0.625rem;
    }

    [part='pip'][aria-current='step'] [part='mark'] {
      outline: 2px solid var(--elohim-witness-trail-mark-color, currentColor);
      outline-offset: 2px;
    }

    [part='pip']:focus-visible,
    summary:focus-visible {
      outline: var(--elohim-witness-trail-focus-ring, 2px solid currentColor);
      outline-offset: 2px;
    }

    [part='disclosure'] {
      display: grid;
      gap: var(--elohim-witness-trail-gap, 0.5rem);
      min-inline-size: 0;
    }

    [part='summary'] {
      cursor: pointer;
      overflow-wrap: anywhere;
      min-block-size: 1.5rem;
    }

    [part='disclosure'][open] [part='summary'] {
      margin-block-end: var(--elohim-witness-trail-gap, 0.5rem);
    }

    .visually-hidden {
      position: absolute;
      inline-size: 1px;
      block-size: 1px;
      overflow: hidden;
      clip-path: inset(50%);
      white-space: nowrap;
    }

    @media (prefers-reduced-motion: no-preference) and (update: fast) {
      [part='line'],
      [part='step'] [part='mark'] {
        animation: elohim-witness-trail-ease-in var(--elohim-witness-trail-ease-duration, 180ms)
          ease-out;
      }
    }

    @keyframes elohim-witness-trail-ease-in {
      from {
        opacity: 0;
      }

      to {
        opacity: 1;
      }
    }

    @media (pointer: coarse) {
      [part='pip'],
      [part='summary'] {
        min-inline-size: 2.75rem;
        min-block-size: 2.75rem;
      }
    }

    @media (forced-colors: active) {
      [part='mark'] {
        forced-color-adjust: none;
        border-color: CanvasText;
        background: Canvas;
      }

      [part='mark'][data-state='working'] {
        background: linear-gradient(to right, CanvasText 50%, Canvas 50%);
      }

      :host(:dir(rtl)) [part='mark'][data-state='working'] {
        background: linear-gradient(to left, CanvasText 50%, Canvas 50%);
      }

      [part='mark'][data-state='done'] {
        background: CanvasText;
      }

      [part='mark'][data-state='failed'] {
        background: linear-gradient(CanvasText, CanvasText) center / 100% 2px no-repeat Canvas;
      }

      [part='pip'] {
        background: ButtonFace;
        color: ButtonText;
      }

      [part='pip'][aria-current='step'] [part='mark'] {
        outline-color: Highlight;
      }

      [part='note'] {
        border-inline-start-color: CanvasText;
        color: CanvasText;
      }

      [part='pip']:focus-visible,
      summary:focus-visible {
        outline: 2px solid Highlight;
      }
    }
  `;

  /** The observed steps, in order. */
  @property({ attribute: false }) steps: WitnessStep[] = [];

  /** `live` while resolving; `settled` for the after-the-fact summary. */
  @property({ reflect: true }) mode: WitnessTrailMode = 'live';

  /** Live presentation: one rotating line, or the whole list. */
  @property({ reflect: true }) layout: WitnessTrailLayout = 'rotate';

  /** Live mode shows nothing until this long has passed with a step not done. */
  @property({ attribute: 'reveal-after-ms', type: Number }) revealAfterMs = 400;

  /** Rotation interval in milliseconds (minimum 1200). */
  @property({ attribute: 'rotate-ms', type: Number }) rotateMs = 1800;

  /** Replace the heading. */
  @property() heading?: string;

  /** Settled mode: whether the disclosure is open. */
  @property({ type: Boolean, reflect: true }) open = false;

  /** Replace any visible sentence; unspecified keys use the English defaults. */
  @property({ attribute: false }) strings: Partial<WitnessTrailStrings> = {};

  @state() private _revealed = false;
  @state() private _announcement = '';
  @state() private _currentId?: string;
  @state() private _hovered = false;
  @state() private _focused = false;
  @state() private _motionTick = 0;

  private _revealTimer?: ReturnType<typeof setTimeout>;
  private _rotateTimer?: ReturnType<typeof setInterval>;
  private _rotateInterval?: number;
  private _settledFor?: string;
  private _seen = new Map<string, WitnessStepState>();
  /** Steps that arrived mid-rotation and are shown next, once each. */
  private _queue: string[] = [];
  /** After the queue drains, the cycle resumes from the top. */
  private _fromTop = false;
  private _announced = new Map<string, WitnessStepState>();
  private _motionQueries: MediaQueryList[] = [];

  private get _s(): WitnessTrailStrings {
    return { ...WITNESS_TRAIL_STRINGS_EN, ...this.strings };
  }

  /**
   * Steps the element understands: `this-device` first, every other step in
   * the order given. Nothing is added, dropped (beyond unknown shapes) or
   * otherwise reordered.
   */
  private get _steps(): WitnessStep[] {
    const known = this.steps.filter(s => KNOWN_ACTS.has(s.act) && KNOWN_RELATIONS.has(s.relation));
    return [
      ...known.filter(s => s.relation === THIS_DEVICE),
      ...known.filter(s => s.relation !== THIS_DEVICE),
    ];
  }

  private get _allDone(): boolean {
    const steps = this._steps;
    return steps.length > 0 && steps.every(s => s.state === 'done');
  }

  /** True when `rotate` is actually in effect (live, motion allowed, nothing failed). */
  private get _rotating(): boolean {
    return (
      this.mode === 'live' &&
      this.layout === 'rotate' &&
      this._motionAllowed() &&
      !this._steps.some(s => s.state === 'failed')
    );
  }

  private _motionAllowed(): boolean {
    // `_motionTick` is reactive state bumped by media-query changes; reading it
    // here ties this decision to the render that the change requested.
    if (this._motionTick < 0) return false;
    const mm = globalThis.matchMedia;
    if (typeof mm === 'function') {
      if (mm('(prefers-reduced-motion: reduce)').matches) return false;
      if (mm('(update: slow)').matches) return false;
    }
    const lock = this.profile.lock;
    return lock.pinnedStimulus !== 'still' && lock.maxStimulus !== 'still';
  }

  // ── lifecycle ────────────────────────────────────────────────────────────

  override connectedCallback(): void {
    super.connectedCallback();
    const mm = globalThis.matchMedia;
    if (typeof mm === 'function') {
      this._motionQueries = ['(prefers-reduced-motion: reduce)', '(update: slow)'].map(q => mm(q));
      for (const q of this._motionQueries) q.addEventListener?.('change', this._onMotionChange);
    }
    if (this.hasUpdated) {
      this._syncReveal();
      this._syncRotation();
    }
  }

  override disconnectedCallback(): void {
    super.disconnectedCallback();
    for (const q of this._motionQueries) q.removeEventListener?.('change', this._onMotionChange);
    this._motionQueries = [];
    this._clearRevealTimer();
    this._stopRotation();
  }

  private readonly _onMotionChange = (): void => {
    this._motionTick++;
  };

  protected override willUpdate(changed: PropertyValues<this>): void {
    if (changed.has('steps')) {
      const steps = this._steps;
      const prev = this._seen;
      // A new flow (no id in common with the last one) starts hidden and unannounced.
      if (prev.size > 0 && !steps.some(s => prev.has(s.id))) {
        this._revealed = false;
        this._announced = new Map();
        this._currentId = undefined;
        this._queue = [];
        this._fromTop = false;
      }
      this._queueArrivals(steps, prev);
      this._seen = new Map(steps.map(s => [s.id, s.state]));
      if (!this._allDone) this._settledFor = undefined;
    }
    if (changed.has('steps') || changed.has('mode') || changed.has('revealAfterMs')) {
      this._syncReveal();
    }
    const steps = this._steps;
    if (this._currentId !== undefined && !steps.some(x => x.id === this._currentId)) {
      this._currentId = undefined;
    }
    if (this.mode === 'live' && this._revealed) this._collectAnnouncement(steps);
  }

  protected override updated(changed: PropertyValues<this>): void {
    super.updated(changed);
    this._syncRotation();
    if (this.mode === 'live' && this._allDone) {
      const ids = this._steps.map(s => s.id);
      const key = ids.join('\u0000');
      if (this._settledFor !== key) {
        this._settledFor = key;
        this.dispatchEvent(
          new CustomEvent<WitnessTrailSettledDetail>('settled', {
            detail: { stepIds: ids },
            bubbles: true,
            composed: true,
          })
        );
      }
    }
  }

  // ── reveal ───────────────────────────────────────────────────────────────

  private _syncReveal(): void {
    if (this.mode !== 'live' || this._revealed) {
      this._clearRevealTimer();
      return;
    }
    const steps = this._steps;
    const pending = steps.length > 0 && steps.some(s => s.state !== 'done');
    if (!pending) {
      this._clearRevealTimer();
      return;
    }
    if (Number.isNaN(this.revealAfterMs) || this.revealAfterMs <= 0) {
      this._revealed = true;
      return;
    }
    this._revealTimer ??= setTimeout(() => {
      this._revealTimer = undefined;
      if (this._steps.some(s => s.state !== 'done')) this._revealed = true;
    }, this.revealAfterMs);
  }

  private _clearRevealTimer(): void {
    if (this._revealTimer !== undefined) clearTimeout(this._revealTimer);
    this._revealTimer = undefined;
  }

  // ── rotation ─────────────────────────────────────────────────────────────

  /**
   * A step that newly enters the rotation (absent, or waiting before) while
   * something was already rotating is queued to be shown next, once.
   */
  private _queueArrivals(steps: WitnessStep[], prev: Map<string, WitnessStepState>): void {
    const wasRotating = [...prev.values()].some(v => v === 'working' || v === 'done');
    if (!wasRotating) return;
    for (const step of steps) {
      const before = prev.get(step.id);
      const arrived = inRotation(step) && before !== 'working' && before !== 'done';
      if (arrived && !this._queue.includes(step.id)) this._queue.push(step.id);
    }
  }

  private get _interval(): number {
    const ms = Number(this.rotateMs);
    return Number.isFinite(ms) ? Math.max(MIN_ROTATE_MS, ms) : 1800;
  }

  private _syncRotation(): void {
    const run =
      this._rotating &&
      this._revealed &&
      !this._hovered &&
      !this._focused &&
      this._steps.filter(inRotation).length > 1;
    if (!run) {
      this._stopRotation();
      return;
    }
    if (this._rotateTimer !== undefined && this._rotateInterval === this._interval) return;
    this._stopRotation();
    this._rotateInterval = this._interval;
    this._rotateTimer = setInterval(() => this._advance(), this._rotateInterval);
  }

  private _stopRotation(): void {
    if (this._rotateTimer !== undefined) clearInterval(this._rotateTimer);
    this._rotateTimer = undefined;
    this._rotateInterval = undefined;
  }

  /**
   * Queued arrivals first (once each), then the top of the cycle, otherwise the
   * next working/done step after the one shown, wrapping.
   */
  private _advance(): void {
    const steps = this._steps;
    const rotation = steps.filter(inRotation);
    if (rotation.length === 0) return;
    const shown = this._shownStep(steps)?.id;
    while (this._queue.length > 0) {
      const id = this._queue.shift();
      if (id !== shown && rotation.some(x => x.id === id)) {
        this._currentId = id;
        this._fromTop = true;
        return;
      }
    }
    if (this._fromTop) {
      this._fromTop = false;
      const top = rotation[0]!;
      if (top.id !== shown) {
        this._currentId = top.id;
        return;
      }
    }
    const at = steps.findIndex(x => x.id === shown);
    for (let k = 1; k <= steps.length; k++) {
      const step = steps[(at + k) % steps.length];
      if (step && inRotation(step)) {
        this._currentId = step.id;
        return;
      }
    }
  }

  /** The step in the slot: the chosen one, else the first under way, else the first given. */
  // eslint-disable-next-line sonarjs/function-return-type -- a step or none: undefined is the honest answer for an empty trail
  private _shownStep(steps: WitnessStep[]): WitnessStep | undefined {
    const chosen = steps.find(x => x.id === this._currentId);
    return chosen ?? steps.find(inRotation) ?? steps[0];
  }

  // ── sentences ────────────────────────────────────────────────────────────

  /** `others` never contributes a name: the label is not read. */
  private _who(step: WitnessStep): WitnessWho {
    if (step.relation === 'others') {
      return typeof step.count === 'number' && step.count > 0 ? { count: step.count } : {};
    }
    return step.label ? { name: isolate(step.label) } : {};
  }

  private _sentence(step: WitnessStep): string {
    // A failed step never claims what it did not do: it says where it stopped.
    if (step.state === 'failed') return this._failedSentence(step, false);
    const table = step.state === 'working' ? this._s.workingSentences : this._s.sentences;
    return table[step.relation][step.act](this._who(step));
  }

  private _failedSentence(step: WitnessStep, withNote = true): string {
    const party = this._s.party[step.relation](this._who(step));
    return this._s.failed(party, withNote ? step.note : undefined);
  }

  private _summary(steps: WitnessStep[]): string {
    const s = this._s;
    if (steps.some(x => x.state === 'failed')) return s.summaryFailed;
    const done = steps.filter(x => x.state === 'done');
    const seen = new Set<string>();
    const parties: string[] = [];
    for (const step of done) {
      if (step.relation === 'others') {
        // All of the network counts as one party: the largest count given, never a name.
        if (seen.has('others')) continue;
        seen.add('others');
        const counts = done
          .filter(x => x.relation === 'others' && typeof x.count === 'number' && x.count > 0)
          .map(x => x.count!);
        parties.push(s.party.others(counts.length > 0 ? { count: Math.max(...counts) } : {}));
        continue;
      }
      const key = `${step.relation}:${step.label ?? ''}`;
      if (seen.has(key)) continue;
      seen.add(key);
      parties.push(s.party[step.relation](this._who(step)));
    }
    return parties.length > 0 ? s.summary(s.joinParties(parties)) : s.summaryNothing;
  }

  /** The honest sentence when nobody else is involved, if any. */
  // eslint-disable-next-line sonarjs/function-return-type -- a sentence or none: undefined means nobody-else-involved does not apply
  private _aloneSentence(steps: WitnessStep[]): string | undefined {
    if (steps.length === 0) return undefined;
    if (steps.every(x => x.relation === 'your-doorway')) return this._s.doorwayAlone;
    if (steps.every(x => x.relation === THIS_DEVICE)) return this._s.thisDeviceAlone;
    return undefined;
  }

  private _renderAlone(steps: WitnessStep[]) {
    const sentence = this._aloneSentence(steps);
    return sentence
      ? html`
          <p part="alone" lang=${this._s.lang} dir=${this._s.dir}>${sentence}</p>
        `
      : nothing;
  }

  private _collectAnnouncement(steps: WitnessStep[]): void {
    const lines: string[] = [];
    for (const step of steps) {
      const before = this._announced.get(step.id);
      if (before === step.state) continue;
      if (step.state === 'done') lines.push(this._sentence(step));
      else if (step.state === 'failed') lines.push(this._failedSentence(step));
    }
    this._announced = new Map(steps.map(s => [s.id, s.state]));
    if (lines.length > 0) this._announcement = this._s.announce(lines, this._allDone);
  }

  // ── render ───────────────────────────────────────────────────────────────

  override render() {
    const steps = this._steps;
    const live = this.mode !== 'settled';
    const layout = live && this._rotating ? 'rotate' : 'list';
    const shown = !live || this._revealed;
    const modeName: WitnessTrailMode = live ? 'live' : 'settled';
    const body = live ? this._renderLive(steps, layout) : this._renderSettled(steps);
    return html`
      ${shown
        ? html`
            <div
              part="trail"
              role="group"
              aria-labelledby="witness-trail-label"
              data-mode=${modeName}
              data-layout=${layout}
            >
              ${body}
            </div>
          `
        : nothing}
      ${live
        ? html`
            <div class="visually-hidden" aria-live="polite" aria-atomic="true">
              ${this._announcement}
            </div>
          `
        : nothing}
    `;
  }

  private _renderLive(steps: WitnessStep[], layout: WitnessTrailLayout) {
    return html`
      <p part="heading" id="witness-trail-label" lang=${this._s.lang} dir=${this._s.dir}>
        ${this.heading ?? this._s.heading}
      </p>
      ${layout === 'rotate' ? this._renderRotator(steps) : this._renderList(steps, false)}
      ${this._renderAlone(steps)}
    `;
  }

  private _renderSettled(steps: WitnessStep[]) {
    return html`
      <details part="disclosure" ?open=${this.open} @toggle=${this._onToggle}>
        <summary part="summary" id="witness-trail-label">
          <span lang=${this._s.lang} dir=${this._s.dir}>${this._summary(steps)}</span>
        </summary>
        ${this._renderList(steps, false)} ${this._renderAlone(steps)}
      </details>
    `;
  }

  private _mark(stepState: WitnessStepState) {
    return html`
      <span part="mark" data-state=${stepState} aria-hidden="true"></span>
    `;
  }

  private _renderList(steps: WitnessStep[], hidden: boolean) {
    const s = this._s;
    return html`
      <ol part="steps" class=${hidden ? 'visually-hidden' : ''} aria-label=${s.stepsLabel}>
        ${steps.map(
          step => html`
            <li part="step" data-state=${step.state} data-relation=${step.relation}>
              ${keyed(step.state, this._mark(step.state))}
              <span part="sentence" lang=${this._s.lang} dir=${this._s.dir}>
                ${this._sentence(step)}
              </span>
              <span class="visually-hidden">(${s.stateWord[step.state]})</span>
              ${step.state === 'failed' && step.note
                ? html`
                    <p part="note" dir="auto">${step.note}</p>
                  `
                : nothing}
            </li>
          `
        )}
      </ol>
    `;
  }

  private _renderRotator(steps: WitnessStep[]) {
    const s = this._s;
    const current = this._shownStep(steps);
    return html`
      <div
        part="rotator"
        @pointerenter=${this._onPointerEnter}
        @pointerleave=${this._onPointerLeave}
        @focusin=${this._onFocusIn}
        @focusout=${this._onFocusOut}
      >
        <div part="slot" aria-hidden="true">
          ${current
            ? keyed(
                `${current.id}:${current.state}`,
                html`
                  <p part="line" data-state=${current.state}>
                    ${this._mark(current.state)}
                    <span part="sentence" lang=${this._s.lang} dir=${this._s.dir}>
                      ${this._sentence(current)}
                    </span>
                  </p>
                `
              )
            : nothing}
        </div>
        ${steps.length > 1
          ? html`
              <div part="pips">
                ${steps.map(
                  (step, i) => html`
                    <button
                      type="button"
                      part="pip"
                      data-state=${step.state}
                      aria-label=${s.showStep(i + 1, steps.length)}
                      aria-current=${step === current ? 'step' : nothing}
                      @click=${() => this._jump(step.id)}
                    >
                      ${this._mark(step.state)}
                    </button>
                  `
                )}
              </div>
            `
          : nothing}
      </div>
      ${this._renderList(steps, true)}
    `;
  }

  // ── intents ──────────────────────────────────────────────────────────────

  private _jump(id: string): void {
    this._currentId = id;
    this._queue = [];
    this._fromTop = false;
    this._stopRotation();
  }

  private readonly _onPointerEnter = (): void => {
    this._hovered = true;
  };

  private readonly _onPointerLeave = (): void => {
    this._hovered = false;
  };

  private readonly _onFocusIn = (): void => {
    this._focused = true;
  };

  private readonly _onFocusOut = (e: FocusEvent): void => {
    const next = e.relatedTarget as Node | null;
    const rotator = e.currentTarget as HTMLElement;
    if (next && rotator.contains(next)) return;
    this._focused = false;
  };

  private readonly _onToggle = (e: Event): void => {
    this.open = (e.currentTarget as HTMLDetailsElement).open;
  };
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-imagodei-witness-trail': ElohimImagodeiWitnessTrail;
  }
}
