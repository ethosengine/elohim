import { CommonModule } from '@angular/common';
import { Component, inject } from '@angular/core';
import { DebugContextService } from '../../elohim/services/debug-context.service';
import { environment } from '../../../environments/environment';

/** Answers "what context am I in?" — the cheapest, always-available lens. */
@Component({
  selector: 'app-connection-lens',
  standalone: true,
  imports: [CommonModule],
  template: `
    <dl class="debug-kv">
      <dt>Connection mode</dt>
      <dd>{{ ctx.mode() }}</dd>
      <dt>Tauri (native)</dt>
      <dd>{{ ctx.isTauri() ? 'yes' : 'no' }}</dd>
      <dt>Direct-to-storage</dt>
      <dd>{{ ctx.isDirectStorage() ? 'yes' : 'no' }}</dd>
      <dt>Storage base URL</dt>
      <dd>{{ ctx.storageBaseUrl() || '(same-origin)' }}</dd>
      <dt>Doorway URL</dt>
      <dd>{{ doorwayUrl || '(same-origin)' }}</dd>
      <dt>Environment</dt>
      <dd>{{ ctx.environmentName }}</dd>
      <dt>Production build</dt>
      <dd>{{ production ? 'yes' : 'no' }}</dd>
      <dt>Git hash</dt>
      <dd>{{ gitHash }}</dd>
    </dl>
  `,
  styles: [
    `
      .debug-kv {
        display: grid;
        grid-template-columns: max-content 1fr;
        gap: 0.25rem 1rem;
      }
      dt {
        font-weight: 600;
        opacity: 0.8;
      }
      dd {
        margin: 0;
        font-family: monospace;
      }
    `,
  ],
})
export class ConnectionLensComponent {
  readonly ctx = inject(DebugContextService);
  readonly doorwayUrl = environment.doorwayUrl ?? '';
  readonly production = environment.production;
  readonly gitHash = environment.gitHash;
}
