import { Component } from '@angular/core';

import { EprRelationshipCardComponent } from '@app/elohim/components/epr-relationship-card/epr-relationship-card.component';

import type { EprRelationship } from '@app/elohim/models/epr-head.model';

@Component({
  selector: 'app-path-forward',
  imports: [EprRelationshipCardComponent],
  templateUrl: './path-forward.component.html',
  styleUrl: './path-forward.component.css',
})
export class PathForwardComponent {
  /**
   * The three audience guides. The prose lives in the EPR nodes now — each card
   * self-resolves its title, description, reach glyph, and steward count, and links
   * cross-bundle correctly.
   */
  readonly audiences: EprRelationship[] = [
    { type: 'GUIDANCE', target: 'concept-path-forward-policymakers' },
    { type: 'GUIDANCE', target: 'concept-path-forward-developers' },
    { type: 'GUIDANCE', target: 'concept-path-forward-communities' },
  ];
}
