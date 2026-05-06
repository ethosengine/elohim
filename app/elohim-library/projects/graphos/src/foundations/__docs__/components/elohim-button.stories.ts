import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';

import 'elohim-core/register';

interface ElohimButtonArgs {
  variant: 'primary' | 'secondary' | 'ghost';
  disabled: boolean;
  label: string;
}

const meta: Meta<ElohimButtonArgs> = {
  title: 'Foundations/Components/elohim-button',
  parameters: {
    docs: {
      description: {
        component:
          'Substrate primitive for action affordances. Token-driven; respects light/dark theme via the global tokens cascade.',
      },
    },
  },
  argTypes: {
    variant: {
      control: 'select',
      options: ['primary', 'secondary', 'ghost'],
    },
    disabled: { control: 'boolean' },
    label: { control: 'text' },
  },
  args: {
    variant: 'primary',
    disabled: false,
    label: 'Click me',
  },
  render: args => html`
    <elohim-button variant=${args.variant} ?disabled=${args.disabled}>${args.label}</elohim-button>
  `,
};

export default meta;
type Story = StoryObj<ElohimButtonArgs>;

export const Primary: Story = { args: { variant: 'primary', label: 'Primary' } };
export const Secondary: Story = { args: { variant: 'secondary', label: 'Secondary' } };
export const Ghost: Story = { args: { variant: 'ghost', label: 'Ghost' } };
export const Disabled: Story = { args: { variant: 'primary', disabled: true, label: 'Disabled' } };

export const AllVariants: Story = {
  render: () => html`
    <div style="display: flex; gap: 1rem; align-items: center; padding: 1rem;">
      <elohim-button variant="primary">Primary</elohim-button>
      <elohim-button variant="secondary">Secondary</elohim-button>
      <elohim-button variant="ghost">Ghost</elohim-button>
      <elohim-button variant="primary" disabled>Disabled</elohim-button>
    </div>
  `,
};
