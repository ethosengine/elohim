import { Component, ChangeDetectionStrategy } from '@angular/core';

import { EprRelationshipCardComponent } from '@app/elohim/components/epr-relationship-card/epr-relationship-card.component';

import type { EprRelationship } from '@app/elohim/models/epr-head.model';

@Component({
  selector: 'app-vision',
  imports: [EprRelationshipCardComponent],
  templateUrl: './vision.component.html',
  changeDetection: ChangeDetectionStrategy.Eager,
  styleUrl: './vision.component.css',
})
export class VisionComponent {
  /** The six epic storylines, rendered as live self-resolving reference cards. */
  readonly epics: EprRelationship[] = [
    { type: 'EPIC', target: 'value-scanner-epic' },
    { type: 'EPIC', target: 'autonomous-entity-epic' },
    { type: 'EPIC', target: 'governance-epic' },
    { type: 'EPIC', target: 'social-medium-epic' },
    { type: 'EPIC', target: 'economic-coordination-epic' },
    { type: 'EPIC', target: 'public-observer-epic' },
  ];
}
