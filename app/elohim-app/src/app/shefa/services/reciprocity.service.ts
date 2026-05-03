import { HttpClient } from '@angular/common/http';
import { Injectable, inject, signal } from '@angular/core';
import { firstValueFrom } from 'rxjs';

import type { ReciprocityView } from '@app/generated/reciprocity-view';

@Injectable({ providedIn: 'root' })
export class ReciprocityService {
  private readonly http = inject(HttpClient);

  readonly reciprocity = signal<ReciprocityView | null>(null);
  readonly loading = signal(false);

  async getMyReciprocity(): Promise<ReciprocityView> {
    this.loading.set(true);
    try {
      const view = await firstValueFrom(
        this.http.get<ReciprocityView>('/api/v1/reciprocity'),
      );
      this.reciprocity.set(view);
      return view;
    } finally {
      this.loading.set(false);
    }
  }

  startPolling(intervalMs = 30_000): () => void {
    const id = setInterval(() => {
      void this.getMyReciprocity();
    }, intervalMs);
    return () => clearInterval(id);
  }
}
