import { CommonModule } from '@angular/common';
import {
  Component,
  OnInit,
  computed,
  inject,
  signal,
  ChangeDetectionStrategy,
} from '@angular/core';
import { HttpClient, HttpErrorResponse } from '@angular/common/http';
import { firstValueFrom } from 'rxjs';
import type { StabilityStatusView } from '../../generated/stability-status-view';
import { DoorwayAdminService } from '../../doorway/services/doorway-admin.service';
import { DebugContextService } from '../../elohim/services/debug-context.service';
import { BlockState } from '../debug.types';

/** Minimal read-subset of storage's ProjectorStatusView / P2PStatusInfo (canonical
 *  types: ts-rs ProjectorStatusView + P2PStatusInfo in @elohim/storage-client). */
interface ProjectorStatusReadModel {
  lag: Array<{ lagSeconds: number | null }>;
}
interface P2pStatusReadModel {
  projectionReconcile?: { caughtUp: boolean; divergentAnchor: number } | null;
}

type ProjectorBlock = StabilityStatusView['projector'];

interface StabilityBlocks {
  autoPreset: BlockState<unknown>;
  admission: BlockState<unknown>;
  upstreams: BlockState<unknown>;
  projector: BlockState<ProjectorBlock>;
  peers: BlockState<StabilityStatusView['peers']>;
  render: BlockState<StabilityStatusView['render']>;
  warmup: BlockState<StabilityStatusView['warmup']>;
  conductor: BlockState<StabilityStatusView['conductor']>;
}

const NA_NODE = 'doorway-role — not applicable on this single node';
const PENDING = 'pending wire-up (sibling follow-on)';

@Component({
  selector: 'app-stability-lens',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './stability-lens.component.html',
  changeDetection: ChangeDetectionStrategy.Eager,
  styleUrl: './stability-lens.component.scss',
})
export class StabilityLensComponent implements OnInit {
  private readonly admin = inject(DoorwayAdminService);
  private readonly http = inject(HttpClient);
  private readonly ctx = inject(DebugContextService);

  readonly error = signal<string | null>(null);
  readonly blocks = signal<StabilityBlocks>(this.loadingBlocks());
  readonly contextNote = computed(() =>
    this.ctx.mode() === 'doorway'
      ? 'Composed by the doorway edge (full self-healing view).'
      : 'Composed on-device from storage status endpoints (node-local).'
  );

  /**
   * Render order + labels. The key is typed `keyof StabilityBlocks` (not a bare
   * string) so the template's `blocks()[row.key]` index passes strictTemplates
   * (an inline `*ngFor` array would widen the key to `string` → AOT error).
   */
  readonly rows: { key: keyof StabilityBlocks; label: string }[] = [
    { key: 'projector', label: 'Projector (lag / caught-up / divergent-anchor)' },
    { key: 'peers', label: 'Peers (signal-peer health)' },
    { key: 'render', label: 'Render (SSR trace)' },
    { key: 'warmup', label: 'Warmup (projection warm-stream)' },
    { key: 'conductor', label: 'Conductor (worker pool)' },
    { key: 'admission', label: 'Admission (inbound semaphore)' },
    { key: 'upstreams', label: 'Upstreams (circuit breakers)' },
    { key: 'autoPreset', label: 'Auto preset (resource policy)' },
  ];

  ngOnInit(): void {
    const work =
      this.ctx.mode() === 'doorway'
        ? firstValueFrom(this.admin.getSelfHealing()).then(v => this.fromDoorway(v))
        : this.fromStorage();
    work.then(
      b => {
        this.blocks.set(b);
      },
      (e: unknown) => {
        this.error.set(this.describe(e));
        this.blocks.set(this.errorBlocks());
      }
    );
  }

  private fromDoorway(v: StabilityStatusView): StabilityBlocks {
    return {
      autoPreset:
        v.autoPreset == null
          ? { state: 'pending', note: PENDING }
          : { state: 'real', value: v.autoPreset },
      admission:
        v.admission == null
          ? { state: 'pending', note: PENDING }
          : { state: 'real', value: v.admission },
      upstreams: !v.upstreams?.length
        ? { state: 'pending', note: PENDING }
        : { state: 'real', value: v.upstreams },
      projector: { state: 'real', value: v.projector },
      peers: { state: 'real', value: v.peers },
      render: { state: 'real', value: v.render },
      warmup: { state: 'real', value: v.warmup },
      conductor: { state: 'real', value: v.conductor },
    };
  }

  private fromStorage(): Promise<StabilityBlocks> {
    const base = this.ctx.storageBaseUrl();
    return Promise.all([
      firstValueFrom(this.http.get<ProjectorStatusReadModel>(`${base}/api/v1/status/projector`)),
      firstValueFrom(this.http.get<P2pStatusReadModel>(`${base}/p2p/status`)),
    ]).then(([proj, p2p]) => {
      const lags = (proj.lag ?? []).map(l => l.lagSeconds).filter((n): n is number => n != null);
      const projector: ProjectorBlock = {
        lagSeconds: lags.length ? Math.max(...lags) : null,
        caughtUp: p2p.projectionReconcile?.caughtUp ?? null,
        divergentAnchor: p2p.projectionReconcile?.divergentAnchor ?? null,
      };
      return {
        autoPreset: { state: 'pending', note: PENDING },
        admission: { state: 'pending', note: PENDING },
        upstreams: { state: 'pending', note: PENDING },
        projector: { state: 'real', value: projector },
        peers: { state: 'na', note: NA_NODE },
        render: { state: 'na', note: NA_NODE },
        warmup: { state: 'na', note: NA_NODE },
        conductor: { state: 'na', note: NA_NODE },
      } as StabilityBlocks;
    });
  }

  private describe(e: unknown): string {
    if (e instanceof HttpErrorResponse) {
      if (e.status === 503) return 'Node catching up (503) — retry shortly.';
      if (e.status === 404) return 'Endpoint not present in this context (404).';
      return `Request failed (${e.status}).`;
    }
    return String(e);
  }

  private base(state: BlockState<unknown>['state']): StabilityBlocks {
    const b: BlockState<never> = { state };
    return {
      autoPreset: b,
      admission: b,
      upstreams: b,
      projector: b,
      peers: b,
      render: b,
      warmup: b,
      conductor: b,
    } as unknown as StabilityBlocks;
  }
  private loadingBlocks(): StabilityBlocks {
    return this.base('loading');
  }
  private errorBlocks(): StabilityBlocks {
    return this.base('error');
  }
}
