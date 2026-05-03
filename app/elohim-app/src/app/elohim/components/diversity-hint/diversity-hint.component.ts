import { CommonModule } from '@angular/common';
import { ChangeDetectionStrategy, Component, Input } from '@angular/core';

import type { DistributionSummary } from '@app/generated/distribution-summary';

type DiversityHint = DistributionSummary['diversityHint'];

const ARCHETYPE_HUMAN: Record<string, string> = {
  desktop: 'Laptop',
  node: 'Home server',
  mobile: 'Phone',
  steward: 'Steward',
};

@Component({
  selector: 'app-diversity-hint',
  standalone: true,
  imports: [CommonModule],
  template: `
    @switch (hint.kind) {
      @case ('region_metro') {
        <span data-testid="diversity-hint-region">
          {{ asArray(hint.value).length }}
          {{ asArray(hint.value).length === 1 ? 'region' : 'regions' }}
          ({{ asArray(hint.value).join(' · ') }})
        </span>
      }
      @case ('household_archetypes') {
        <span data-testid="diversity-hint-households">
          {{ archetypeLabels(asArray(hint.value)).join(' · ') }}
        </span>
      }
      @case ('collective_member_count') {
        <span data-testid="diversity-hint-collective">
          hosted by {{ hint.value }} collective members
        </span>
      }
      @case ('none') {
        <span></span>
      }
    }
  `,
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class DiversityHintComponent {
  @Input({ required: true }) hint!: DiversityHint;

  asArray(v: unknown): string[] {
    return Array.isArray(v) ? (v as string[]) : [];
  }

  archetypeLabels(values: string[]): string[] {
    return values.map(a => ARCHETYPE_HUMAN[a] ?? a);
  }
}
