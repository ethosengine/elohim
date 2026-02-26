/**
 * Elohim Config Component — Backend settings panel.
 *
 * Simple UI for selecting inference backend, entering BYOK API key,
 * testing connectivity, and viewing session cost.
 * Accessible from debug bar or settings.
 */

import { CommonModule } from '@angular/common';
import {
  Component,
  ChangeDetectionStrategy,
  signal,
  inject,
  computed,
  DestroyRef,
} from '@angular/core';
import { takeUntilDestroyed } from '@angular/core/rxjs-interop';

import { NativeBackend } from '../../services/backends/native-backend';
import { ElohimBackendCatalog } from '../../services/elohim-backend';
import { ElohimPresenceService } from '../../services/elohim-presence.service';

import type { ElohimComputationCost } from '../../models/elohim-agent.model';
import type { ElohimBackendType } from '../../services/elohim-backend';

@Component({
  selector: 'app-elohim-config',
  standalone: true,
  imports: [CommonModule],
  template: `
    <div class="elohim-config" data-testid="elohim-config">
      <h3 class="config-title">Elohim Backend</h3>

      <!-- Backend Selector -->
      <div class="config-section" data-testid="backend-selector">
        <label class="config-label">Inference Backend</label>
        <div class="backend-options">
          @for (opt of backendOptions; track opt.type) {
            <button
              class="backend-option"
              [class.selected]="selectedBackend() === opt.type"
              [attr.data-testid]="'backend-' + opt.type"
              [attr.aria-label]="'Select ' + opt.name + ' backend'"
              (click)="selectBackend(opt.type)"
            >
              <span class="option-name">{{ opt.name }}</span>
              <span class="option-desc">{{ opt.description }}</span>
            </button>
          }
        </div>
      </div>

      <!-- API Key (BYOK) — shown for native backend -->
      @if (selectedBackend() === 'native') {
        <div class="config-section" data-testid="api-key-section">
          <label class="config-label">API Key (BYOK)</label>
          <div class="key-input-row">
            <input
              type="password"
              class="config-input"
              aria-label="BYOK API Key"
              [value]="apiKeyMask()"
              placeholder="sk-ant-..."
              data-testid="api-key-input"
              (input)="onApiKeyInput($event)"
            />
            @if (hasApiKey()) {
              <button class="btn-clear" data-testid="clear-api-key" (click)="clearApiKey()">
                Clear
              </button>
            }
          </div>
          <span class="config-hint">
            Sent securely to your elohim-agent for testing. Not used for direct API calls.
          </span>
        </div>
      }

      <!-- Test Button -->
      <div class="config-section">
        <button
          class="btn-test"
          [disabled]="testing()"
          data-testid="test-backend"
          (click)="testBackend()"
        >
          {{ testing() ? 'Testing...' : 'Test Connection' }}
        </button>
        @if (testResult()) {
          <span
            class="test-result"
            [class.success]="testResult()!.success"
            [class.error]="!testResult()!.success"
            data-testid="test-result"
          >
            {{ testResult()!.message }}
          </span>
        }
      </div>

      <!-- Session Cost -->
      @if (sessionCost(); as cost) {
        <div class="config-section" data-testid="session-cost">
          <label class="config-label">Session Cost</label>
          <div class="cost-grid">
            <span class="cost-label">Tokens</span>
            <span class="cost-value">{{ cost.tokensProcessed | number }}</span>
            <span class="cost-label">Time</span>
            <span class="cost-value">{{ cost.timeMs | number }}ms</span>
            <span class="cost-label">Checks</span>
            <span class="cost-value">{{ cost.constitutionalChecks }}</span>
          </div>
        </div>
      }
    </div>
  `,
  styles: [
    `
      .elohim-config {
        padding: 1.5rem;
        max-width: 480px;
      }

      .config-title {
        font-size: 1.1rem;
        font-weight: 600;
        margin: 0 0 1.25rem;
        color: #e0e0e0;
      }

      .config-section {
        margin-bottom: 1.25rem;
      }

      .config-label {
        display: block;
        font-size: 0.8rem;
        font-weight: 500;
        color: #999;
        text-transform: uppercase;
        letter-spacing: 0.05em;
        margin-bottom: 0.5rem;
      }

      .backend-options {
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
      }

      .backend-option {
        display: flex;
        flex-direction: column;
        align-items: flex-start;
        padding: 0.75rem 1rem;
        background: rgba(255, 255, 255, 0.05);
        border: 1px solid rgba(255, 255, 255, 0.1);
        border-radius: 8px;
        cursor: pointer;
        color: #ccc;
        text-align: left;
        transition:
          border-color 0.2s,
          background 0.2s;
      }

      .backend-option:hover {
        background: rgba(255, 255, 255, 0.08);
      }

      .backend-option.selected {
        border-color: #64b5f6;
        background: rgba(100, 181, 246, 0.1);
      }

      .option-name {
        font-weight: 600;
        font-size: 0.9rem;
      }

      .option-desc {
        font-size: 0.75rem;
        color: #888;
        margin-top: 0.125rem;
      }

      .key-input-row {
        display: flex;
        gap: 0.5rem;
      }

      .config-input {
        flex: 1;
        padding: 0.5rem 0.75rem;
        background: rgba(255, 255, 255, 0.05);
        border: 1px solid rgba(255, 255, 255, 0.15);
        border-radius: 6px;
        color: #e0e0e0;
        font-family: monospace;
        font-size: 0.85rem;
      }

      .config-input::placeholder {
        color: #666;
      }

      .config-hint {
        display: block;
        font-size: 0.7rem;
        color: #777;
        margin-top: 0.375rem;
      }

      .btn-clear {
        padding: 0.5rem 0.75rem;
        background: rgba(244, 67, 54, 0.15);
        border: 1px solid rgba(244, 67, 54, 0.3);
        border-radius: 6px;
        color: #ef9a9a;
        cursor: pointer;
        font-size: 0.8rem;
      }

      .btn-test {
        padding: 0.5rem 1rem;
        background: rgba(100, 181, 246, 0.15);
        border: 1px solid rgba(100, 181, 246, 0.3);
        border-radius: 6px;
        color: #90caf9;
        cursor: pointer;
        font-size: 0.85rem;
        transition: background 0.2s;
      }

      .btn-test:hover:not(:disabled) {
        background: rgba(100, 181, 246, 0.25);
      }

      .btn-test:disabled {
        opacity: 0.5;
        cursor: not-allowed;
      }

      .test-result {
        display: inline-block;
        margin-left: 0.75rem;
        font-size: 0.8rem;
      }

      .test-result.success {
        color: #81c784;
      }

      .test-result.error {
        color: #ef9a9a;
      }

      .cost-grid {
        display: grid;
        grid-template-columns: auto 1fr;
        gap: 0.25rem 0.75rem;
        font-size: 0.8rem;
      }

      .cost-label {
        color: #888;
      }

      .cost-value {
        color: #ccc;
        font-family: monospace;
      }
    `,
  ],
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ElohimConfigComponent {
  private readonly catalog = inject(ElohimBackendCatalog);
  private readonly presenceService = inject(ElohimPresenceService);
  private readonly destroyRef = inject(DestroyRef);

  readonly backendOptions: { type: ElohimBackendType; name: string; description: string }[] = [
    { type: 'mock', name: 'Simulation (Mock)', description: 'Simulated responses for development' },
    {
      type: 'native',
      name: 'Native (Rust Agent)',
      description: 'Doorway proxy to elohim-agent (supports BYOK)',
    },
  ];

  readonly selectedBackend = signal<ElohimBackendType>(
    (this.catalog.getPreferredId() as ElohimBackendType) ?? 'mock'
  );
  readonly apiKey = signal<string | null>(null);
  readonly hasApiKey = computed(() => !!this.apiKey());
  readonly apiKeyMask = computed(() => (this.hasApiKey() ? '••••••••••••' : ''));
  readonly testing = signal(false);
  readonly testResult = signal<{ success: boolean; message: string } | null>(null);
  readonly sessionCost = signal<ElohimComputationCost | null>(null);

  constructor() {
    this.presenceService.cost$
      .pipe(takeUntilDestroyed(this.destroyRef))
      .subscribe(cost => this.sessionCost.set(cost));
  }

  selectBackend(type: ElohimBackendType): void {
    this.selectedBackend.set(type);
    this.testResult.set(null);

    // Lazily register NativeBackend when first selected
    if (type === 'native' && !this.catalog.get('native')) {
      this.catalog.register(new NativeBackend());
    }

    this.catalog.setPreferred(type);
  }

  onApiKeyInput(event: Event): void {
    const value = (event.target as HTMLInputElement).value;
    if (value && !value.startsWith('•')) {
      this.apiKey.set(value);
      // Forward BYOK key to the NativeBackend instance
      const native = this.catalog.get('native') as NativeBackend | undefined;
      if (native) {
        native.byokApiKey = value;
      }
    }
  }

  clearApiKey(): void {
    this.apiKey.set(null);
    const native = this.catalog.get('native') as NativeBackend | undefined;
    if (native) {
      native.byokApiKey = null;
    }
    this.testResult.set(null);
  }

  async testBackend(): Promise<void> {
    this.testing.set(true);
    this.testResult.set(null);

    try {
      const backend = await this.catalog.selectBackend();
      const available = await backend.isAvailable();

      if (available) {
        this.testResult.set({ success: true, message: `${backend.name} is available` });
      } else {
        this.testResult.set({ success: false, message: `${backend.name} is not available` });
      }
    } catch {
      this.testResult.set({ success: false, message: 'Connection test failed' });
    } finally {
      this.testing.set(false);
    }
  }
}
