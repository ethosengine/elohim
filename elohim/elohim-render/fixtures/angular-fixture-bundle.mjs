// Mimics Angular's main.server.mjs surface for tests:
// exports a default `bootstrap()` and a `renderApplication(bootstrap, {url})`.
// In real Angular 19.2, renderApplication comes from @angular/platform-server
// and bootstrap is the default export of main.server.ts.
export default function bootstrap() {
  return Promise.resolve(
    `<!doctype html><html><head><title>Fixture</title></head>` +
      `<body><app-root>fixture rendered</app-root></body></html>`
  );
}

export async function renderApplication(_bootstrap, opts) {
  return `<!doctype html><html><head><title>${opts.url}</title></head>` +
    `<body><app-root ngh="0">fixture rendered ${opts.url}</app-root></body></html>`;
}
