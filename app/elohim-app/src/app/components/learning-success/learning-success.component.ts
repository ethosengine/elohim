import { Component, ChangeDetectionStrategy } from '@angular/core';

import { EprRelationshipCardComponent } from '@app/elohim/components/epr-relationship-card/epr-relationship-card.component';

import type { EprRelationship } from '@app/elohim/models/epr-head.model';

/** A single "door" into the platform: a one-line hint plus a live reference card. */
interface WayIn {
  hint: string;
  rel: EprRelationship;
  /** Optional stable data-testid for the a2o landing-discovery contract. */
  testid?: string;
}

@Component({
  selector: 'app-learning-success',
  imports: [EprRelationshipCardComponent],
  templateUrl: './learning-success.component.html',
  changeDetection: ChangeDetectionStrategy.Eager,
  styleUrl: './learning-success.component.css',
})
export class LearningSuccessComponent {
  /** The prominent first door — the onboarding path that opens with the manifesto. */
  readonly startHere: EprRelationship = { type: 'START', target: 'elohim-protocol' };

  /** Cross-bundle mount links into the lamad SPA — plain hrefs, not minted content links. */
  readonly libraryHref = '/lamad'; // route-literal-ok: cross-bundle mount link, not a minted content link
  readonly exploreHref = '/lamad/explore'; // route-literal-ok: cross-bundle mount link, not a minted content link

  /** Alternate doors for visitors who want to self-route. */
  readonly otherWays: WayIn[] = [
    {
      hint: 'Not sure where to start? Two minutes.',
      rel: { type: 'QUIZ', target: 'quiz-who-are-you' },
      testid: 'landing-wayin-quiz',
    },
    {
      hint: 'Try the ideas — an explorable game about trust.',
      rel: { type: 'PLAY', target: 'evolution-of-trust' },
      testid: 'landing-wayin-trust',
    },
    {
      hint: 'Ready to go deeper into who you are.',
      rel: { type: 'PATH', target: 'know-thyself-path' },
    },
  ];
}
