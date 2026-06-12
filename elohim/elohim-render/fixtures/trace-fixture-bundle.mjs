// Like angular-fixture-bundle.mjs, but exercises the `fetch` shim during render
// so the render trace records a data fetch. Used to prove the TracingFetcher is
// wired into the AngularRenderer worker loop (events flow into RenderOutput.trace).
export default function bootstrap() {
  return Promise.resolve(
    `<!doctype html><html><head><title>Trace fixture</title></head>` +
      `<body><app-root>traced</app-root></body></html>`
  );
}

export async function renderApplication(_bootstrap, opts) {
  // A single data fetch — the outcome (arrived / empty / stalled) is determined
  // by the DataFetcher the renderer was constructed with. Failures are expected
  // (stall/error paths reject) and must not abort the render.
  try {
    await fetch('/api/v1/concept' + opts.url);
  } catch (e) {
    /* expected on stall/error — the render still produces a shell */
  }
  return `<!doctype html><html><head><title>${opts.url}</title></head>` +
    `<body><app-root ngh="0">traced ${opts.url}</app-root></body></html>`;
}
