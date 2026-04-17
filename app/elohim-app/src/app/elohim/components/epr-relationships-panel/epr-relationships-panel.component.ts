import { CommonModule } from '@angular/common';
import { ChangeDetectionStrategy, Component, Input } from '@angular/core';

import { EprRelationshipCardComponent } from '../epr-relationship-card/epr-relationship-card.component';

import type { EprRelationship } from '../../models/epr-head.model';

// ── Group model ───────────────────────────────────────────────────────────────

interface RelationshipGroup {
  type: string;
  label: string;
  items: EprRelationship[];
}

// ── Ordering & labels ─────────────────────────────────────────────────────────

const KNOWN_ORDER = ['PREREQUISITE', 'TEACHES', 'CONTAINS', 'REFERENCES'];

const GROUP_LABELS: Record<string, string> = {
  PREREQUISITE: 'Prerequisites',
  TEACHES: 'This teaches',
  CONTAINS: 'Contains',
  REFERENCES: 'References',
};

function groupLabel(type: string): string {
  return GROUP_LABELS[type] ?? type.charAt(0).toUpperCase() + type.slice(1).toLowerCase();
}

function buildGroups(relationships: EprRelationship[]): RelationshipGroup[] {
  // Collect items per type, preserving encounter order for unknowns
  const map = new Map<string, EprRelationship[]>();
  for (const rel of relationships) {
    const bucket = map.get(rel.type);
    if (bucket) {
      bucket.push(rel);
    } else {
      map.set(rel.type, [rel]);
    }
  }

  // Known types in fixed order, then unknowns in encounter order
  const knownPresent = KNOWN_ORDER.filter(t => map.has(t));
  const unknownPresent = [...map.keys()].filter(t => !KNOWN_ORDER.includes(t));
  const orderedTypes = [...knownPresent, ...unknownPresent];

  return orderedTypes.map(type => ({
    type,
    label: groupLabel(type),
    items: map.get(type)!,
  }));
}

// ── Component ─────────────────────────────────────────────────────────────────

@Component({
  selector: 'app-epr-relationships-panel',
  standalone: true,
  imports: [CommonModule, EprRelationshipCardComponent],
  changeDetection: ChangeDetectionStrategy.OnPush,
  styles: [
    `
      .epr-rel-panel {
        margin-top: 24px;
      }

      .epr-rel-group {
        margin-bottom: 20px;
      }

      .epr-rel-group__heading {
        font-size: 0.72rem;
        font-weight: 700;
        text-transform: uppercase;
        letter-spacing: 0.06em;
        color: var(--epr-panel-heading-color, #6b7280);
        margin: 0 0 8px;
      }

      .epr-rel-group__grid {
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
        gap: 10px;
      }
    `,
  ],
  template: `
    @if (groups.length > 0) {
      <section class="epr-rel-panel" data-testid="epr-relationships-panel">
        @for (group of groups; track trackGroup(group)) {
          <div class="epr-rel-group" data-testid="epr-rel-group" [attr.data-type]="group.type">
            <h3 class="epr-rel-group__heading">{{ group.label }}</h3>
            <div class="epr-rel-group__grid">
              @for (item of group.items; track trackItem(item)) {
                <app-epr-relationship-card [relationship]="item" />
              }
            </div>
          </div>
        }
      </section>
    }
  `,
})
export class EprRelationshipsPanelComponent {
  groups: RelationshipGroup[] = [];

  @Input()
  set relationships(value: EprRelationship[]) {
    this.groups = value.length > 0 ? buildGroups(value) : [];
  }

  trackGroup(group: RelationshipGroup): string {
    return group.type;
  }

  trackItem(item: EprRelationship): string {
    return `${item.type}:${item.target}`;
  }
}
