import { HttpInterceptorFn } from '@angular/common/http';

import { environment } from '../../../environments/environment';

const DOORWAY_PATH_PREFIXES = ['/api/', '/db/', '/blob/', '/apps/', '/health'];

function isAbsolute(url: string): boolean {
  return url.startsWith('http://') || url.startsWith('https://');
}

function matchesDoorwayPath(url: string): boolean {
  return DOORWAY_PATH_PREFIXES.some(p => url === p.replace(/\/$/, '') || url.startsWith(p));
}

function isTauri(): boolean {
  return typeof globalThis !== 'undefined' && '__TAURI__' in globalThis;
}

function isCheOrigin(origin: string): boolean {
  return origin.includes('.devspaces.') || origin.includes('.code.ethosengine.com');
}

function isLocalDevOrigin(origin: string): boolean {
  return origin.startsWith('http://localhost:') || origin.startsWith('http://127.0.0.1:');
}

function safeOrigin(url: string): string | null {
  try {
    return new URL(url).origin;
  } catch {
    return null;
  }
}

function resolveBaseUrl(): string {
  if (isTauri()) {
    return environment.client?.storageUrl ?? environment.holochain?.storageUrl ?? '';
  }

  const origin = globalThis.location?.origin ?? '';
  if (!origin) return '';
  if (isCheOrigin(origin) || isLocalDevOrigin(origin)) return '';

  const doorwayUrl = environment.client?.doorwayUrl ?? '';
  if (!doorwayUrl) return '';

  if (origin === safeOrigin(doorwayUrl)) return '';

  return doorwayUrl;
}

export const apiBaseUrlInterceptor: HttpInterceptorFn = (req, next) => {
  if (isAbsolute(req.url) || !matchesDoorwayPath(req.url)) {
    return next(req);
  }

  const base = resolveBaseUrl();
  if (!base) return next(req);

  return next(req.clone({ url: `${base.replace(/\/$/, '')}${req.url}` }));
};
