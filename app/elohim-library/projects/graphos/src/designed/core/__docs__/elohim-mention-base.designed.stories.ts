/**
 * Library B designed story — elohim-mention-base
 *
 * Binds the Elohim brand tokens to `<elohim-mention-base>` via story-decorator
 * overrides. The primitive itself is NEVER modified — binding happens above via
 * `--elohim-mention-*` CSS custom properties mapped to `--el-*` brand tokens.
 *
 * Sources of truth (per elohim-library/CLAUDE.md):
 *   1. Types — elohim-mention-base takes `epr`, `label`, and `pillar` attributes.
 *      No ts-rs view dependency (metadata is display-only at this layer).
 *   2. Manifest vocabulary — `pillar` values match the protocol pillar names:
 *      lamad, imagodei, qahal, shefa, elohim.
 *   3. Brand tokens — `--el-*` palette from design spec §14 declared inline.
 *
 * Binding canvas:
 *   --elohim-mention-bg           → --el-cream (light) / --el-night-alt (dark)
 *   --elohim-mention-fg           → --el-stone (light) / --el-starlight (dark)
 *   --elohim-mention-border       → --el-stone at 25% (light) / --el-starlight at 20% (dark)
 *   --elohim-mention-fallback-dot → --el-amber (warm indicator that resolution is pending)
 *   --elohim-mention-radius       → 999px (pill shape — per brand spec)
 *
 * Protocol vocabulary:
 *   EPR refs use the protocol's pillar namespaces: epr:lamad-*, epr:imagodei-*,
 *   epr:qahal-*, epr:shefa-*. Household members are named (Matthew, Jessica,
 *   James) not "User 1". Concepts are named (fair-exchange, harvest-bounty,
 *   covenant-of-care) not "Test Content".
 */

import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';

import 'elohim-core/register';

// ---------------------------------------------------------------------------
// Brand token declaration
// ---------------------------------------------------------------------------

const EL_TOKENS = `
  --el-green-deep:  #2D5F3B;
  --el-green-light: #7FB069;
  --el-amber:       #D4A03E;
  --el-clay:        #B8664F;
  --el-cream:       #F5F0E8;
  --el-stone:       #6B6157;
  --el-sky:         #7BAFCB;
  --el-plum:        #6E4B6B;
  --el-starlight:   #E8E4D9;
  --el-night:       #0F1A12;
  --el-night-alt:   #1A1A2E;
  --el-font-display: 'Fraunces', Georgia, serif;
  --el-font-body:    'Source Serif 4', Georgia, serif;
  --el-font-ui:      'DM Sans', system-ui, sans-serif;
  --el-font-mono:    'JetBrains Mono', monospace;
  --el-space-xs:  8px;
  --el-space-sm:  16px;
  --el-space-md:  24px;
  --el-space-lg:  32px;
  --el-space-xl:  48px;
  --el-radius-sm: 4px;
  --el-radius-md: 8px;
  --el-radius-lg: 16px;
  --el-shadow-soft:   0 2px 8px rgba(107, 97, 87, 0.08);
  --el-shadow-medium: 0 4px 16px rgba(107, 97, 87, 0.12);
`;

/** Light mode — warm earth chip on Linen */
const MENTION_TOKENS_LIGHT = `
  --elohim-mention-bg:           color-mix(in oklch, var(--el-stone) 8%, var(--el-cream));
  --elohim-mention-fg:           var(--el-stone);
  --elohim-mention-border:       color-mix(in oklch, var(--el-stone) 25%, transparent);
  --elohim-mention-fallback-dot: var(--el-amber);
  --elohim-mention-radius:       999px;
`;

/** Dark mode — constellation chip on Indigo Night */
const MENTION_TOKENS_DARK = `
  --elohim-mention-bg:           color-mix(in oklch, var(--el-starlight) 8%, var(--el-night-alt));
  --elohim-mention-fg:           var(--el-starlight);
  --elohim-mention-border:       color-mix(in oklch, var(--el-starlight) 20%, transparent);
  --elohim-mention-fallback-dot: var(--el-amber);
  --elohim-mention-radius:       999px;
`;

// ---------------------------------------------------------------------------
// Decorator factories
// ---------------------------------------------------------------------------

function lightDecorator(story: () => unknown) {
  return html`
    <div
      style="
        ${EL_TOKENS}
        ${MENTION_TOKENS_LIGHT}
        font-family: var(--el-font-ui);
        background: var(--el-cream);
        padding: var(--el-space-lg);
      "
    >
      ${story()}
    </div>
  `;
}

function darkDecorator(story: () => unknown) {
  return html`
    <div
      style="
        ${EL_TOKENS}
        ${MENTION_TOKENS_DARK}
        font-family: var(--el-font-ui);
        background: var(--el-night-alt);
        padding: var(--el-space-lg);
      "
    >
      ${story()}
    </div>
  `;
}

// ---------------------------------------------------------------------------
// Meta
// ---------------------------------------------------------------------------

const meta: Meta = {
  title: 'Designed/Core/elohim-mention-base',
  parameters: {
    docs: {
      description: {
        component: `
\`<elohim-mention-base>\` — Elohim brand-bound EPR reference chip.

The mention chip is how the protocol names things in running prose and in
structured views. When a lamad concept attests to a teacher's contribution,
when a shefa commitment names a household, when a qahal proposal references
a prior covenant — the mention chip is the inline handle.

The fallback dot (Harvest Gold) signals "this reference has not yet resolved
to a pillar-specific mention element." It is not an error — it is an honest
acknowledgment that the protocol is still working.

**Token binding:**
| Property | Light | Dark |
|---|---|---|
| \`--elohim-mention-bg\` | stone 8% on cream | starlight 8% on night-alt |
| \`--elohim-mention-fg\` | stone | starlight |
| \`--elohim-mention-border\` | stone 25% | starlight 20% |
| \`--elohim-mention-fallback-dot\` | amber | amber |
| \`--elohim-mention-radius\` | 999px (pill) | 999px (pill) |

Library B — graphos-designer. Primitive untouched.
        `.trim(),
      },
    },
  },
};

export default meta;
type Story = StoryObj;

// ---------------------------------------------------------------------------
// Protocol-realistic mention variants
// ---------------------------------------------------------------------------

export const LamadTeacher: Story = {
  name: 'LamadTeacher',
  decorators: [lightDecorator],
  render: () => html`
    <p
      style="
        font-family: var(--el-font-body);
        font-size: 1rem;
        line-height: 1.75;
        color: var(--el-stone);
        max-inline-size: 560px;
      "
    >
      The concept "fair-exchange" was attested by
      <elohim-mention-base
        epr="epr:imagodei-jessica-dowell"
        label="Jessica (teacher)"
        pillar="imagodei"
      ></elohim-mention-base>
      and confirmed by the lamad council on the first of the harvest month.
    </p>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'A lamad concept attesting to a teacher — Jessica (teacher) named inline in the ' +
          'attestation prose. The chip is rendered in the imagodei pillar context. ' +
          'The fallback dot is Harvest Gold — warm, not alarming.',
      },
    },
  },
};

export const ShefaHousehold: Story = {
  name: 'ShefaHousehold',
  decorators: [lightDecorator],
  render: () => html`
    <p
      style="
        font-family: var(--el-font-body);
        font-size: 1rem;
        line-height: 1.75;
        color: var(--el-stone);
        max-inline-size: 560px;
      "
    >
      The harvest-bounty provision was committed by
      <elohim-mention-base
        epr="epr:shefa-household-aleph"
        label="Aleph household"
        pillar="shefa"
      ></elohim-mention-base>
      and received by
      <elohim-mention-base
        epr="epr:shefa-household-bethlehem"
        label="Bethlehem household"
        pillar="shefa"
      ></elohim-mention-base>
      on the third day.
    </p>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Two household mention chips inline in a shefa provision narrative. ' +
          'Demonstrates multiple chips at body line-height — the pill shape ' +
          'sits cleanly in Source Serif 4 prose without visual turbulence.',
      },
    },
  },
};

export const QahalElder: Story = {
  name: 'QahalElder',
  decorators: [lightDecorator],
  render: () => html`
    <p
      style="
        font-family: var(--el-font-body);
        font-size: 1rem;
        line-height: 1.75;
        color: var(--el-stone);
        max-inline-size: 560px;
      "
    >
      The covenant-of-care proposal was reviewed by
      <elohim-mention-base
        epr="epr:imagodei-matthew-dowell"
        label="Matthew (elder)"
        pillar="imagodei"
      ></elohim-mention-base>
      before the qahal gathering.
    </p>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'A qahal elder named in a governance narrative. Note the vocabulary: ' +
          '"elder" not "admin", "covenant-of-care proposal" not "policy document", ' +
          '"qahal gathering" not "group meeting."',
      },
    },
  },
};

export const RecoveryWitness: Story = {
  name: 'RecoveryWitness',
  decorators: [lightDecorator],
  render: () => html`
    <p
      style="
        font-family: var(--el-font-body);
        font-size: 1rem;
        line-height: 1.75;
        color: var(--el-stone);
        max-inline-size: 560px;
      "
    >
      Account recovery was witnessed by
      <elohim-mention-base
        epr="epr:imagodei-james-dowell"
        label="James (recovery witness)"
        pillar="imagodei"
      ></elohim-mention-base>
      — one of three intimate circle members who hold a Shamir shard.
    </p>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'A recovery-witness mention in an imagodei identity narrative. ' +
          'The vocabulary is honest: Shamir shard, intimate circle, recovery — ' +
          'not "backup code", "trusted contact", "reset."',
      },
    },
  },
};

export const UnresolvedEprOnly: Story = {
  name: 'UnresolvedEprOnly',
  decorators: [lightDecorator],
  render: () => html`
    <p
      style="
        font-family: var(--el-font-body);
        font-size: 1rem;
        line-height: 1.75;
        color: var(--el-stone);
        max-inline-size: 560px;
      "
    >
      This provision references
      <elohim-mention-base
        epr="epr:shefa-commitment-sha256-4a7b9c2d1e8f3a5b6c0d9e2f1a4b7c8d9e0f1a2b"
      ></elohim-mention-base>
      — the pillar bundle has not yet loaded so only the EPR identifier is shown.
    </p>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'No label provided — the EPR identifier is the chip text. The Harvest Gold ' +
          'fallback dot communicates "resolution is pending, not failed." ' +
          'The EPR uses a sha256 prefix — no fake-looking short IDs.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Light / Dark named theme stories
// ---------------------------------------------------------------------------

export const Light: Story = {
  name: 'Light',
  decorators: [lightDecorator],
  render: () => html`
    <div style="display: flex; flex-wrap: wrap; gap: var(--el-space-xs); align-items: center;">
      <elohim-mention-base epr="epr:imagodei-jessica-dowell" label="Jessica (teacher)" pillar="imagodei"></elohim-mention-base>
      <elohim-mention-base epr="epr:shefa-household-aleph" label="Aleph household" pillar="shefa"></elohim-mention-base>
      <elohim-mention-base epr="epr:qahal-covenant-of-care" label="covenant-of-care" pillar="qahal"></elohim-mention-base>
      <elohim-mention-base epr="epr:lamad-concept-fair-exchange" label="fair-exchange" pillar="lamad"></elohim-mention-base>
      <elohim-mention-base epr="epr:shefa-commitment-sha256-4a7b9c2d1e8f3a5b6c0d9e2f1a4b7c8d9e0f1a2b"></elohim-mention-base>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Light theme palette — five chip variants: imagodei member, shefa household, qahal concept, ' +
          'lamad concept, unresolved EPR-only. Warm earth Linen background, Hearthstone text, ' +
          'Harvest Gold fallback dot.',
      },
    },
  },
};

export const Dark: Story = {
  name: 'Dark',
  decorators: [darkDecorator],
  render: () => html`
    <div style="display: flex; flex-wrap: wrap; gap: var(--el-space-xs); align-items: center;">
      <elohim-mention-base epr="epr:imagodei-jessica-dowell" label="Jessica (teacher)" pillar="imagodei"></elohim-mention-base>
      <elohim-mention-base epr="epr:shefa-household-aleph" label="Aleph household" pillar="shefa"></elohim-mention-base>
      <elohim-mention-base epr="epr:qahal-covenant-of-care" label="covenant-of-care" pillar="qahal"></elohim-mention-base>
      <elohim-mention-base epr="epr:lamad-concept-fair-exchange" label="fair-exchange" pillar="lamad"></elohim-mention-base>
      <elohim-mention-base epr="epr:shefa-commitment-sha256-4a7b9c2d1e8f3a5b6c0d9e2f1a4b7c8d9e0f1a2b"></elohim-mention-base>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Constellation dark theme — Indigo Night background, Starlight chip text, ' +
          'Harvest Gold fallback dot. The chips read as named stars in the household ' +
          'constellation — small, bright, precisely placed.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// RTL canary — Hebrew
// ---------------------------------------------------------------------------

export const Hebrew: Story = {
  name: 'Hebrew',
  decorators: [
    (story: () => unknown) => html`
      <div
        dir="rtl"
        lang="he"
        style="
          ${EL_TOKENS}
          ${MENTION_TOKENS_LIGHT}
          font-family: var(--el-font-ui);
          background: var(--el-cream);
          padding: var(--el-space-lg);
        "
      >
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <p
      style="
        font-family: var(--el-font-body);
        font-size: 1rem;
        line-height: 1.75;
        color: var(--el-stone);
        max-inline-size: 560px;
      "
    >
      ההצהרה על מושג "חוזה-הטיפול" אושרה על ידי
      <elohim-mention-base
        epr="epr:imagodei-jessica-dowell"
        label="ג׳סיקה (מורה)"
        pillar="imagodei"
      ></elohim-mention-base>
      לפני כינוס הקהל.
    </p>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'RTL canary — he-IL locale inside a dir="rtl" container. The fallback dot and label ' +
          'mirror correctly: dot trails on the inline-start side (visual left in RTL). ' +
          'The Hebrew label uses the same Source Serif 4 body prose as the LTR stories.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Inline in prose — multiple chips flowing in text
// ---------------------------------------------------------------------------

export const InlineInProse: Story = {
  name: 'InlineInProse',
  decorators: [lightDecorator],
  render: () => html`
    <p
      style="
        font-family: var(--el-font-body);
        font-size: 1rem;
        line-height: 2;
        color: var(--el-stone);
        max-inline-size: 640px;
      "
    >
      The Aleph household's stewardship of the commons is documented in the
      <elohim-mention-base epr="epr:shefa-commitment-sha256-4a7b9c2d1e8f3a5b6c0d9e2f1a4b7c8d9e0f1a2b" label="harvest-bounty commitment" pillar="shefa"></elohim-mention-base>.
      The lamad pathway
      <elohim-mention-base epr="epr:lamad-concept-fair-exchange" label="fair-exchange" pillar="lamad"></elohim-mention-base>
      was the prerequisite before
      <elohim-mention-base epr="epr:imagodei-matthew-dowell" label="Matthew (steward)" pillar="imagodei"></elohim-mention-base>
      could accept the qahal's
      <elohim-mention-base epr="epr:qahal-covenant-of-care" label="covenant-of-care" pillar="qahal"></elohim-mention-base>.
    </p>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Four mention chips in a single paragraph of protocol narrative — shefa, lamad, ' +
          'imagodei, and qahal pillars all represented. The pill shape sits naturally at ' +
          'line-height: 2 in Source Serif 4. This is the primary use context: ' +
          'rich attestation prose that names the actors and concepts precisely.',
      },
    },
  },
};
