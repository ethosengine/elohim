import { Injectable, inject, signal } from '@angular/core';

import { firstValueFrom } from 'rxjs';

import {
  EprNavContextService,
  type EprNavContextView,
  type EprNavRef,
} from './epr-nav-context.service';
import { SessionNavStackService } from './session-nav-stack.service';

/**
 * Composes SessionNavStack (visitor history) and EprNavContext (substrate
 * adjacency) into unified back/forward affordances for ProtocolOmniComponent.
 *
 * Composition rule:
 *   - back():   session-previous if present, else EPR prev, else null.
 *   - forward(): EPR next (browsers don't track forward beyond their own
 *                history; substrate-derived signal is the meaningful one).
 *
 * Cold anonymous hit → session is empty → EPR drives both.
 * Walked-from-protocol → session has previous → session-back + EPR-forward.
 */
@Injectable({ providedIn: 'root' })
export class ProtocolNavigationService {
  private readonly eprSvc = inject(EprNavContextService);
  private readonly session = inject(SessionNavStackService);

  private readonly _ctx = signal<EprNavContextView | null>(null);
  readonly context = this._ctx.asReadonly();

  /** Resolve EPR context for the given CID and record the visit. */
  async activate(cid: string, url: string): Promise<void> {
    const ctx = await firstValueFrom(this.eprSvc.fetch(cid));
    this._ctx.set(ctx);
    this.session.record({ url, cid });
  }

  back(): EprNavRef | null {
    const prior = this.session.previous();
    if (prior) {
      return { cid: prior.cid, label: prior.label };
    }
    return this._ctx()?.prev ?? null;
  }

  forward(): EprNavRef | null {
    return this._ctx()?.next ?? null;
  }
}
