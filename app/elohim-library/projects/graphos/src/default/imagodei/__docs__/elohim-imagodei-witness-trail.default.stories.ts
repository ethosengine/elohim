/**
 * Library A default story — elohim-imagodei-witness-trail
 *
 * Proves the element works as a blank-slate primitive: no brand tokens bound,
 * CSS system colors as defaults, override surface honest.
 *
 * While a sign-in or a device approval resolves, the trail shows who is
 * actually vouching for it, step by step; afterwards it stays available as a
 * short "how this was secured" summary. Companion of
 * <elohim-imagodei-device-consent-card> (same portal flow).
 *
 * No ts-rs view exists yet for the steps of a sign-in; the fixture type is the
 * element's own `WitnessStep` (what the host observed, in order). Mock cast:
 * doorway `alpha`, the person's device `workspace`, a person who vouches for
 * them, `Jessica`.
 */

import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';

import 'elohim-imagodei/register';
import type { ElohimImagodeiWitnessTrail, WitnessStep } from 'elohim-imagodei';

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

const passwordOnly: WitnessStep[] = [
  { id: 'pw', act: 'checked', relation: 'your-doorway', label: 'alpha', state: 'working' },
];

const deviceInProgress: WitnessStep[] = [
  { id: 'door-sign', act: 'signed', relation: 'your-doorway', label: 'alpha', state: 'done' },
  { id: 'dev-sign', act: 'signed', relation: 'your-device', label: 'workspace', state: 'working' },
  { id: 'record', act: 'recorded', relation: 'others', state: 'waiting' },
];

const deviceComplete: WitnessStep[] = [
  { id: 'door-sign', act: 'signed', relation: 'your-doorway', label: 'alpha', state: 'done' },
  { id: 'dev-sign', act: 'signed', relation: 'your-device', label: 'workspace', state: 'done' },
  { id: 'record', act: 'recorded', relation: 'others', state: 'done' },
  { id: 'seen', act: 'seen', relation: 'others', count: 3, state: 'done' },
];

/** Mid-flight, three under way or done, one still to come. */
const rotatingDevice: WitnessStep[] = [
  { id: 'pw', act: 'checked', relation: 'your-doorway', label: 'alpha', state: 'done' },
  { id: 'door-sign', act: 'signed', relation: 'your-doorway', label: 'alpha', state: 'done' },
  { id: 'dev-sign', act: 'signed', relation: 'your-device', label: 'workspace', state: 'working' },
  { id: 'seen', act: 'seen', relation: 'others', count: 3, state: 'waiting' },
];

const withPeople: WitnessStep[] = [
  { id: 'pw', act: 'checked', relation: 'your-doorway', label: 'alpha', state: 'done' },
  { id: 'jessica', act: 'signed', relation: 'vouches-for-you', label: 'Jessica', state: 'done' },
  { id: 'dev-sign', act: 'signed', relation: 'your-device', label: 'workspace', state: 'working' },
  { id: 'seen', act: 'seen', relation: 'others', count: 3, state: 'waiting' },
];

/** `this-device` given LAST: the element shows it first. */
const thisDeviceLast: WitnessStep[] = [
  { id: 'door-sign', act: 'signed', relation: 'your-doorway', label: 'alpha', state: 'done' },
  { id: 'seen', act: 'seen', relation: 'others', count: 3, state: 'done' },
  { id: 'here', act: 'signed', relation: 'this-device', label: 'workspace', state: 'done' },
];

const frame = (story: () => unknown) => html`
  <div style="max-inline-size: 440px; padding: 1rem;">${story()}</div>
`;

async function trailIn(canvas: HTMLElement): Promise<ElohimImagodeiWitnessTrail> {
  await customElements.whenDefined('elohim-imagodei-witness-trail');
  const el = canvas.querySelector('elohim-imagodei-witness-trail')!;
  await el.updateComplete;
  return el;
}

/** Jump the rotation to one step and hold it there (as a hovering pointer would). */
async function pin(canvas: HTMLElement, index: number): Promise<void> {
  const el = await trailIn(canvas);
  const pips = el.shadowRoot!.querySelectorAll<HTMLButtonElement>('[part="pip"]');
  pips[index]?.click();
  el.shadowRoot!.querySelector('[part="rotator"]')?.dispatchEvent(new PointerEvent('pointerenter'));
  await el.updateComplete;
}

// ---------------------------------------------------------------------------
// Meta
// ---------------------------------------------------------------------------

const meta: Meta = {
  title: 'Default/Imagodei/elohim-imagodei-witness-trail',
  parameters: {
    docs: {
      description: {
        component: `
\`<elohim-imagodei-witness-trail>\` — while a sign-in or device approval resolves, shows who is
actually vouching for it; afterwards, a short "how this was secured" summary.

It shows only the \`steps\` it is given. The one ordering it imposes: \`this-device\` comes first.
\`others\` is a count, never a name. When every step is \`done\` it fires \`settled\`
\`{ stepIds }\` in the same update. A failed step is said plainly and never settles.

Live mode waits \`reveal-after-ms\` (400) before showing anything, so an instant sign-in never
flashes it. \`layout="rotate"\` (default) shows one line at a time in a fixed two-line slot,
with one mark per step; \`layout="list"\` shows the whole sequence. Rotation falls back to the
list under reduced motion, e-paper, a still-locked profile, or a failure.
\`mode="settled"\` is one summary line that opens to the full sequence.

**Override surface:** \`--elohim-witness-trail-*\` CSS custom properties with neutral defaults.
        `.trim(),
      },
    },
  },
};

export default meta;
type Story = StoryObj;

// ---------------------------------------------------------------------------
// Blank-slate + override-surface proofs
// ---------------------------------------------------------------------------

export const Unstyled: Story = {
  name: 'Unstyled (blank-slate proof)',
  decorators: [
    story => html`
      <div style="all: initial; display: block; padding: 1rem;">${story()}</div>
    `,
  ],
  render: () => html`
    <elohim-imagodei-witness-trail
      .steps=${deviceInProgress}
      layout="list"
      reveal-after-ms="0"
    ></elohim-imagodei-witness-trail>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Wrapped in `style="all: initial;"`. Heading, marks and sentences render with system defaults and zero tokens.',
      },
    },
  },
};

export const CustomTheme: Story = {
  name: 'CustomTheme (override-surface proof)',
  decorators: [
    story => html`
      <div
        style="
          font-family: 'Gill Sans', 'Gill Sans MT', Calibri, sans-serif;
          color: #10243a;
          background: #e6eef5;
          --elohim-witness-trail-gap: 0.75rem;
          --elohim-witness-trail-step-gap: 0.625rem;
          --elohim-witness-trail-line-height: 1.5;
          --elohim-witness-trail-muted-opacity: 0.85;
          --elohim-witness-trail-current-weight: 600;
          --elohim-witness-trail-mark-size: 1em;
          --elohim-witness-trail-mark-color: #2a6f97;
          --elohim-witness-trail-failed-color: #9a3412;
          --elohim-witness-trail-note-border: 4px double #9a3412;
          --elohim-witness-trail-slot-bg: #ffffff;
          --elohim-witness-trail-slot-padding: 0.75rem;
          --elohim-witness-trail-radius: 0;
          --elohim-witness-trail-focus-ring: 3px dashed #c2410c;
          padding: 1.5rem;
          max-inline-size: 440px;
          display: grid;
          gap: 1.5rem;
        "
      >
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-imagodei-witness-trail
      .steps=${rotatingDevice}
      reveal-after-ms="0"
    ></elohim-imagodei-witness-trail>
    <elohim-imagodei-witness-trail
      .steps=${[
        deviceInProgress[0],
        { ...deviceInProgress[1], state: 'failed', note: 'it did not answer in time' },
      ]}
      layout="list"
      reveal-after-ms="0"
    ></elohim-imagodei-witness-trail>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Override-surface proof: a square-cornered steel-blue palette with a white slot, a rust failure rule and a dashed orange focus ring — deliberately non-Elohim. Every visual value comes through `--elohim-witness-trail-*`.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// List layout — live
// ---------------------------------------------------------------------------

export const PasswordSignInDoorwayOnly: Story = {
  name: 'PasswordSignInDoorwayOnly',
  decorators: [frame],
  render: () => html`
    <elohim-imagodei-witness-trail
      .steps=${passwordOnly}
      layout="list"
      reveal-after-ms="0"
    ></elohim-imagodei-witness-trail>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'A plain password sign-in: one step, the doorway checking the password, plus one sentence that this sign-in rests on the doorway alone. No decorative witnesses.',
      },
    },
  },
};

export const AloneThisDevice: Story = {
  name: 'AloneThisDevice',
  decorators: [frame],
  render: () => html`
    <elohim-imagodei-witness-trail
      .steps=${[
        {
          id: 'here',
          act: 'signed',
          relation: 'this-device',
          label: 'workspace',
          state: 'working',
        },
      ]}
      reveal-after-ms="0"
    ></elohim-imagodei-witness-trail>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'The node on this machine is the only witness: one still line (nothing to rotate) and a plain sentence that, right now, this device is the only one vouching.',
      },
    },
  },
};

export const DeviceApprovalInProgress: Story = {
  name: 'DeviceApprovalInProgress',
  decorators: [frame],
  render: () => html`
    <elohim-imagodei-witness-trail
      .steps=${deviceInProgress}
      layout="list"
      reveal-after-ms="0"
    ></elohim-imagodei-witness-trail>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'The doorway has signed, the device is signing now (current), the record is still to come (quiet).',
      },
    },
  },
};

export const DeviceApprovalComplete: Story = {
  name: 'DeviceApprovalComplete',
  decorators: [frame],
  render: () => html`
    <elohim-imagodei-witness-trail
      .steps=${deviceInProgress}
      layout="list"
      reveal-after-ms="0"
    ></elohim-imagodei-witness-trail>
  `,
  play: async ({ canvasElement }) => {
    const el = await trailIn(canvasElement);
    el.steps = deviceComplete;
    await el.updateComplete;
  },
  parameters: {
    docs: {
      description: {
        story:
          'Shown while in progress, then every step lands: the done steps stay visible and `settled` fires in the same update.',
      },
    },
  },
};

export const WithPeopleWhoVouch: Story = {
  name: 'WithPeopleWhoVouch',
  decorators: [frame],
  render: () => html`
    <elohim-imagodei-witness-trail
      .steps=${withPeople}
      layout="list"
      reveal-after-ms="0"
    ></elohim-imagodei-witness-trail>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Someone in the person’s own circle vouches for them, by name.',
      },
    },
  },
};

export const OthersCountOnly: Story = {
  name: 'OthersCountOnly',
  decorators: [frame],
  render: () => html`
    <elohim-imagodei-witness-trail
      .steps=${[
        deviceComplete[0],
        // The label is deliberately set: it must never reach the page.
        {
          id: 'seen',
          act: 'seen',
          relation: 'others',
          label: 'Somebody Else',
          count: 3,
          state: 'working',
        },
      ]}
      layout="list"
      reveal-after-ms="0"
    ></elohim-imagodei-witness-trail>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Other people on the network are a count, never a name — the fixture passes a label and the element ignores it.',
      },
    },
  },
};

export const OthersNoCount: Story = {
  name: 'OthersNoCount',
  decorators: [frame],
  render: () => html`
    <elohim-imagodei-witness-trail
      .steps=${[
        deviceComplete[0],
        { id: 'seen', act: 'seen', relation: 'others', state: 'done' },
        { id: 'record', act: 'recorded', relation: 'others', state: 'waiting' },
      ]}
      layout="list"
      reveal-after-ms="0"
    ></elohim-imagodei-witness-trail>
  `,
};

export const FailedStep: Story = {
  name: 'FailedStep',
  decorators: [frame],
  render: () => html`
    <elohim-imagodei-witness-trail
      .steps=${[
        deviceInProgress[0],
        {
          ...deviceInProgress[1],
          state: 'failed',
          note: 'it did not answer in time',
        },
        deviceInProgress[2],
      ]}
      layout="list"
      reveal-after-ms="0"
    ></elohim-imagodei-witness-trail>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Said plainly, with the host’s note. A failed trail never fires `settled`.',
      },
    },
  },
};

export const InstantResolveShowsNothing: Story = {
  name: 'InstantResolveShowsNothing',
  decorators: [
    story => html`
      <div style="max-inline-size: 440px; padding: 1rem; display: grid; gap: 0.5rem;">
        <p style="margin: 0;">The trail sits inside the dashed box:</p>
        <div style="outline: 1px dashed GrayText; min-block-size: 1rem;">${story()}</div>
      </div>
    `,
  ],
  render: () => html`
    <elohim-imagodei-witness-trail .steps=${deviceComplete}></elohim-imagodei-witness-trail>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Every step was already done before `reveal-after-ms` (default 400) passed: nothing is shown, nothing flashes.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Rotate layout — live
// ---------------------------------------------------------------------------

export const RotatingDeviceApproval: Story = {
  name: 'RotatingDeviceApproval',
  decorators: [frame],
  render: () => html`
    <elohim-imagodei-witness-trail
      .steps=${rotatingDevice}
      reveal-after-ms="0"
    ></elohim-imagodei-witness-trail>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'One line at a time, cycling through the steps under way or done every 1.8s; marks below show how many there are and which is current. Hover or focus pauses.',
      },
    },
  },
};

export const RotatingDeviceApprovalAtFirst: Story = {
  name: 'RotatingDeviceApproval (held at step 1)',
  decorators: [frame],
  render: RotatingDeviceApproval.render,
  play: async ({ canvasElement }) => pin(canvasElement, 0),
};

export const RotatingDeviceApprovalAtThird: Story = {
  name: 'RotatingDeviceApproval (held at step 3)',
  decorators: [frame],
  render: RotatingDeviceApproval.render,
  play: async ({ canvasElement }) => pin(canvasElement, 2),
};

export const RotatingWithPeopleWhoVouch: Story = {
  name: 'RotatingWithPeopleWhoVouch',
  decorators: [frame],
  render: () => html`
    <elohim-imagodei-witness-trail
      .steps=${withPeople}
      reveal-after-ms="0"
    ></elohim-imagodei-witness-trail>
  `,
  play: async ({ canvasElement }) => pin(canvasElement, 1),
};

export const RotatingThisDeviceFirst: Story = {
  name: 'RotatingThisDeviceFirst',
  decorators: [frame],
  render: () => html`
    <elohim-imagodei-witness-trail
      .steps=${[
        ...thisDeviceLast,
        { id: 'later', act: 'recorded', relation: 'others', state: 'waiting' },
      ]}
      reveal-after-ms="0"
    ></elohim-imagodei-witness-trail>
  `,
  parameters: {
    docs: {
      description: {
        story:
          '`this-device` is given LAST in the array and shown first — the one ordering the element imposes. The other steps keep their given order.',
      },
    },
  },
};

export const RotatingSingleStep: Story = {
  name: 'RotatingSingleStep',
  decorators: [frame],
  render: () => html`
    <elohim-imagodei-witness-trail
      .steps=${passwordOnly}
      reveal-after-ms="0"
    ></elohim-imagodei-witness-trail>
  `,
  parameters: {
    docs: {
      description: {
        story: 'One step: nothing to rotate, no marks. The line stays still.',
      },
    },
  },
};

export const RotatingReducedMotion: Story = {
  name: 'RotatingReducedMotion',
  decorators: [frame],
  render: () => html`
    <elohim-imagodei-witness-trail
      .steps=${rotatingDevice}
      reveal-after-ms="0"
    ></elohim-imagodei-witness-trail>
  `,
  play: async ({ canvasElement }) => {
    const el = await trailIn(canvasElement);
    // Storybook cannot set the OS preference; a profile locked to stillness takes the
    // same path as prefers-reduced-motion: no rotation, the whole list.
    el.profile = { ...el.profile, lock: { kind: 'steward', pinnedStimulus: 'still' } };
    await el.updateComplete;
  },
  parameters: {
    docs: {
      description: {
        story:
          '`layout="rotate"` under reduced motion (shown here through a still-locked profile, the same code path): the rotation is replaced by the full list.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Settled mode
// ---------------------------------------------------------------------------

export const SettledCollapsed: Story = {
  name: 'SettledCollapsed',
  decorators: [frame],
  render: () => html`
    <elohim-imagodei-witness-trail
      .steps=${deviceComplete}
      mode="settled"
    ></elohim-imagodei-witness-trail>
  `,
  parameters: {
    docs: {
      description: {
        story: 'After the fact: one summary line, closed. The person can open it.',
      },
    },
  },
};

export const SettledOpen: Story = {
  name: 'SettledOpen',
  decorators: [frame],
  render: () => html`
    <elohim-imagodei-witness-trail
      .steps=${[
        ...withPeople.slice(0, 3).map(s => ({ ...s, state: 'done' as const })),
        deviceComplete[3],
      ]}
      mode="settled"
      open
    ></elohim-imagodei-witness-trail>
  `,
};

// ---------------------------------------------------------------------------
// Theme + direction canaries
// ---------------------------------------------------------------------------

export const Dark: Story = {
  name: 'Dark',
  decorators: [
    story => html`
      <div
        style="background: Canvas; color: CanvasText; color-scheme: dark; padding: 1.5rem; max-inline-size: 440px; display: grid; gap: 1.5rem;"
      >
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-imagodei-witness-trail
      .steps=${deviceInProgress}
      layout="list"
      reveal-after-ms="0"
    ></elohim-imagodei-witness-trail>
    <elohim-imagodei-witness-trail
      .steps=${rotatingDevice}
      reveal-after-ms="0"
    ></elohim-imagodei-witness-trail>
    <elohim-imagodei-witness-trail
      .steps=${deviceComplete}
      mode="settled"
      open
    ></elohim-imagodei-witness-trail>
  `,
};

export const RTLCanary: Story = {
  name: 'RTLCanary',
  decorators: [
    story => html`
      <div
        dir="rtl"
        lang="he"
        style="padding: 1rem; max-inline-size: 440px; display: grid; gap: 1.5rem;"
      >
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-imagodei-witness-trail
      .steps=${[
        { ...deviceInProgress[0] },
        { ...deviceInProgress[1], label: 'עבודה' },
        deviceInProgress[2],
      ]}
      layout="list"
      reveal-after-ms="0"
    ></elohim-imagodei-witness-trail>
    <elohim-imagodei-witness-trail
      .steps=${rotatingDevice}
      reveal-after-ms="0"
    ></elohim-imagodei-witness-trail>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'RTL container: layout mirrors through logical properties; the half-filled “happening now” mark fills from the inline start; a Hebrew device name is isolated inside the English sentence.',
      },
    },
  },
};
