import { CommonModule } from '@angular/common';
import { Component, OnInit, inject } from '@angular/core';

import { eprToUniversalHref } from '@elohim/service';
import type { EconomicEventView } from '@elohim/storage-client/generated';
import { LAMAD_AGENT, type ILamadAgent } from '../../interfaces/agent.interface';
import { EventService } from '@elohim/rea-runtime';

/**
 * AttentionFlowComponent — Learner's personal attention history.
 *
 * Like YouTube watch history: shows what you engaged with recently,
 * recorded as protocol economic events (REA flows), not extracted
 * to third parties. Embedded in the learner dashboard at /lamad/me.
 */
@Component({
  selector: 'app-attention-flow',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './attention-flow.component.html',
  styleUrls: ['./attention-flow.component.css'],
})
export class AttentionFlowComponent implements OnInit {
  events: EconomicEventView[] = [];
  uniqueContentCount = 0;
  isLoading = true;

  private readonly eventService = inject(EventService);
  private readonly agentService = inject(LAMAD_AGENT);

  ngOnInit(): void {
    const agentId = this.agentService.getCurrentAgentId();

    this.eventService.getRecentEvents(agentId, 100).subscribe({
      next: events => {
        this.events = events;
        const uniqueIds = new Set(events.map(e => e.contentId).filter(Boolean));
        this.uniqueContentCount = uniqueIds.size;
        this.isLoading = false;
      },
      error: () => {
        this.isLoading = false;
      },
    });
  }

  getEventIcon(event: EconomicEventView): string {
    const type = event.lamadEventType;
    if (type === 'content-view') return '\u{1F441}';
    if (type === 'content-complete') return '\u{2705}';
    if (type === 'assessment-complete') return '\u{1F3AF}';
    if (type === 'quiz-submit') return '\u{1F4DD}';
    return '\u{25CF}';
  }

  /** Mint the universal EPR address for an event's content (never a literal). */
  eprHref(contentId: string): string {
    return eprToUniversalHref({ id: contentId, tier: 'head' });
  }

  getEventLabel(event: EconomicEventView): string {
    const type = event.lamadEventType;
    if (type === 'content-view') return 'Viewed';
    if (type === 'content-complete') return 'Completed';
    if (type === 'assessment-complete') return 'Assessment passed';
    if (type === 'quiz-submit') return 'Quiz submitted';
    return type ?? 'Event';
  }
}
