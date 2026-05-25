/**
 * Library A default story — elohim-feedback-mechanism-gateway
 *
 * Proves the element works as a blank-slate primitive: no brand tokens bound,
 * CSS system colors as defaults, override surface honest.
 *
 * <elohim-feedback-mechanism-gateway> orchestrates the feedback surface.
 * Level 0: context-menu slot only
 * Level 1: reactions slot (and badge)
 * Level 2: reactions + graduated-feedback
 * Level 3–7: psephos (full deliberation)
 * Shows sensemaking/controversy badges when accumulation thresholds are met.
 */

import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';

import 'elohim-core/register';
import type { MechanismSelection, AccumulationStatus } from 'elohim-core';

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

const SELECTION_L0: MechanismSelection = { level: 0, renderTarget: 'angular' };
const SELECTION_L1: MechanismSelection = { level: 1, renderTarget: 'angular' };
const SELECTION_L2: MechanismSelection = { level: 2, renderTarget: 'angular' };
const SELECTION_PSEPHOS: MechanismSelection = { level: 3, renderTarget: 'psephos' };

const ACCUMULATION_SENSEMAKING: AccumulationStatus = {
  readyForSensemaking: true,
  controversyDetected: false,
  settled: false,
};

const ACCUMULATION_CONTROVERSY: AccumulationStatus = {
  readyForSensemaking: false,
  controversyDetected: true,
  settled: false,
};

// ---------------------------------------------------------------------------
// Meta
// ---------------------------------------------------------------------------

const meta: Meta = {
  title: 'Default/Core/elohim-feedback-mechanism-gateway',
  parameters: {
    docs: {
      description: {
        component: `
\`<elohim-feedback-mechanism-gateway>\` — orchestrates the governance feedback surface.

Routes to named slots based on mechanism level and render target:
- Level 0 (angular): \`context-menu\` slot
- Level 1+ (angular): \`reactions\` slot, \`graduated-feedback\` slot at level 2
- Level 3–7 (psephos): \`psephos\` slot

Shows sensemaking/controversy badges when accumulation thresholds are reached.

**Override surface:** \`--elohim-gateway-*\`.
**Parts:** \`container\`, \`loading\`, \`sensemaking-link\`, \`controversy-badge\`.
        `.trim(),
      },
    },
  },
};

export default meta;
type Story = StoryObj;

// ---------------------------------------------------------------------------
// Content states
// ---------------------------------------------------------------------------

export const Loading: Story = {
  name: 'Loading',
  render: () => html`
    <div style="padding: 1rem;">
      <elohim-feedback-mechanism-gateway loading></elohim-feedback-mechanism-gateway>
    </div>
  `,
  parameters: {
    docs: {
      description: { story: 'Loading state — no selection resolved yet.' },
    },
  },
};

export const Level0ContextMenu: Story = {
  name: 'Level0 (context-menu slot)',
  render: () => html`
    <div style="padding: 1rem;">
      <elohim-feedback-mechanism-gateway
        entity-type="content"
        entity-id="test-content-1"
        .selection=${SELECTION_L0}
      >
        <div slot="context-menu" style="padding: 0.5rem; border: 1px dashed currentColor; font-size: 0.875rem;">
          [context-menu slot content — Angular component here]
        </div>
      </elohim-feedback-mechanism-gateway>
    </div>
  `,
};

export const Level1Reactions: Story = {
  name: 'Level1 (reactions slot)',
  render: () => html`
    <div style="padding: 1rem;">
      <elohim-feedback-mechanism-gateway
        entity-type="content"
        entity-id="test-content-2"
        .selection=${SELECTION_L1}
      >
        <div slot="reactions" style="padding: 0.5rem; border: 1px dashed currentColor; font-size: 0.875rem;">
          [reactions slot — elohim-reaction-bar here]
        </div>
      </elohim-feedback-mechanism-gateway>
    </div>
  `,
};

export const Level2ReactionsAndGraduated: Story = {
  name: 'Level2 (reactions + graduated)',
  render: () => html`
    <div style="padding: 1rem; max-inline-size: 500px;">
      <elohim-feedback-mechanism-gateway
        entity-type="content"
        entity-id="test-content-3"
        .selection=${SELECTION_L2}
      >
        <div slot="reactions" style="padding: 0.5rem; border: 1px dashed currentColor; font-size: 0.875rem;">
          [reactions slot]
        </div>
        <div slot="graduated-feedback" style="padding: 0.5rem; border: 1px dashed currentColor; font-size: 0.875rem;">
          [graduated-feedback slot]
        </div>
      </elohim-feedback-mechanism-gateway>
    </div>
  `,
};

export const PsephosLevel: Story = {
  name: 'PsephosLevel (renderTarget=psephos)',
  render: () => html`
    <div style="padding: 1rem; max-inline-size: 500px;">
      <elohim-feedback-mechanism-gateway
        entity-type="content"
        entity-id="test-content-4"
        .selection=${SELECTION_PSEPHOS}
      >
        <div slot="psephos" style="padding: 0.5rem; border: 1px dashed currentColor; font-size: 0.875rem;">
          [psephos slot — deliberation surface here]
        </div>
      </elohim-feedback-mechanism-gateway>
    </div>
  `,
};

export const SensemakingBadge: Story = {
  name: 'SensemakingBadge',
  render: () => html`
    <div style="padding: 1rem;">
      <p style="font-size: 0.875rem; opacity: 0.8; margin-block-end: 0.5rem;">
        Sensemaking threshold reached — badge appears.
      </p>
      <elohim-feedback-mechanism-gateway
        entity-type="content"
        entity-id="test-content-5"
        .selection=${SELECTION_L1}
        .accumulationStatus=${ACCUMULATION_SENSEMAKING}
      >
        <div slot="reactions" style="padding: 0.5rem; font-size: 0.875rem;">
          [reactions slot]
        </div>
      </elohim-feedback-mechanism-gateway>
    </div>
  `,
};

export const ControversyBadge: Story = {
  name: 'ControversyBadge',
  render: () => html`
    <div style="padding: 1rem;">
      <p style="font-size: 0.875rem; opacity: 0.8; margin-block-end: 0.5rem;">
        Controversy detected — badge appears.
      </p>
      <elohim-feedback-mechanism-gateway
        entity-type="content"
        entity-id="test-content-6"
        .selection=${SELECTION_L1}
        .accumulationStatus=${ACCUMULATION_CONTROVERSY}
      >
        <div slot="reactions" style="padding: 0.5rem; font-size: 0.875rem;">
          [reactions slot]
        </div>
      </elohim-feedback-mechanism-gateway>
    </div>
  `,
};

// ---------------------------------------------------------------------------
// Theme canaries
// ---------------------------------------------------------------------------

export const Dark: Story = {
  name: 'Dark',
  decorators: [
    story => html`
      <div style="background: #1a1a1a; color: #e8e8e8; color-scheme: dark; padding: 1.5rem;">
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-feedback-mechanism-gateway
      .selection=${SELECTION_L1}
      .accumulationStatus=${ACCUMULATION_SENSEMAKING}
    >
      <div slot="reactions" style="font-size: 0.875rem;">[reactions slot]</div>
    </elohim-feedback-mechanism-gateway>
  `,
};

// ---------------------------------------------------------------------------
// Blank-slate and override-surface proofs
// ---------------------------------------------------------------------------

export const Unstyled: Story = {
  name: 'Unstyled (blank-slate proof)',
  decorators: [story => html`<div style="all: initial; display: block; padding: 1rem;">${story()}</div>`],
  render: () => html`<elohim-feedback-mechanism-gateway loading></elohim-feedback-mechanism-gateway>`,
  parameters: {
    docs: {
      description: {
        story: 'Wrapped in `style="all: initial;"`. Proves the loading state renders with zero external tokens.',
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
          --elohim-gateway-loading-color: #60a5fa;
          --elohim-gateway-sensemaking-bg: #1e3a5f;
          --elohim-gateway-sensemaking-fg: #bfdbfe;
          background: #0a1628;
          color: #bfdbfe;
          font-family: ui-monospace, monospace;
          padding: 1.5rem;
        "
      >
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-feedback-mechanism-gateway
      .selection=${SELECTION_L1}
      .accumulationStatus=${ACCUMULATION_SENSEMAKING}
    >
      <div slot="reactions" style="font-size: 0.875rem;">[reactions slot]</div>
    </elohim-feedback-mechanism-gateway>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Deliberately non-Elohim ocean-blue theme. Proves the override surface is honest.',
      },
    },
  },
};
