import { CommonModule } from '@angular/common';
import { Component, OnInit, inject } from '@angular/core';
import { RouterModule } from '@angular/router';

import { EconomicEventView } from '@app/elohim/adapters/storage-types.adapter';
import { AgentService } from '@app/elohim/services/agent.service';
import { EventService } from '@app/shefa/services/event.service';

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
  imports: [CommonModule, RouterModule],
  templateUrl: './attention-flow.component.html',
  styleUrls: ['./attention-flow.component.css'],
})
export class AttentionFlowComponent implements OnInit {
  events: EconomicEventView[] = [];
  uniqueContentCount = 0;
  isLoading = true;

  private readonly eventService = inject(EventService);
  private readonly agentService = inject(AgentService);

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

  getEventLabel(event: EconomicEventView): string {
    const type = event.lamadEventType;
    if (type === 'content-view') return 'Viewed';
    if (type === 'content-complete') return 'Completed';
    if (type === 'assessment-complete') return 'Assessment passed';
    if (type === 'quiz-submit') return 'Quiz submitted';
    return type ?? 'Event';
  }
}
