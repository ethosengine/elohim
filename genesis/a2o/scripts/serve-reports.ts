#!/usr/bin/env tsx
/**
 * serve-reports — operator-viewable eyes.
 *
 * Serves genesis/a2o/reports/ (look captures, screenshots, cucumber reports) as
 * static files on the Che `ui-playground` endpoint port (4201, already exposed
 * public in devfile.yaml). The agent reads shot.png via its multimodal Read;
 * the operator opens the Che route for the SAME artifacts — symmetric vision.
 *
 * Zero dependencies (node http/fs only). Usage: `pnpm reports:serve`
 * Port override: REPORTS_PORT. Mutually exclusive with anything else on 4201
 * (the ui-playground port is a shared dev slot).
 */

import { createReadStream } from 'node:fs';
import { readdir, stat } from 'node:fs/promises';
import { createServer } from 'node:http';
import { extname, join, normalize, resolve, sep } from 'node:path';

const ROOT = resolve(import.meta.dirname, '..', 'reports');
const PORT = Number(process.env['REPORTS_PORT'] ?? 4201);

const MIME: Record<string, string> = {
  '.png': 'image/png',
  '.jpg': 'image/jpeg',
  '.json': 'application/json; charset=utf-8',
  '.html': 'text/html; charset=utf-8',
  '.md': 'text/plain; charset=utf-8',
  '.txt': 'text/plain; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
};

function htmlIndex(urlPath: string, entries: { name: string; dir: boolean }[]): string {
  const rows = entries
    .sort((a, b) => Number(b.dir) - Number(a.dir) || a.name.localeCompare(b.name))
    .map((e) => {
      const href = `${urlPath.replace(/\/$/, '')}/${e.name}${e.dir ? '/' : ''}`;
      return `<li><a href="${href}">${e.name}${e.dir ? '/' : ''}</a></li>`;
    })
    .join('\n');
  return `<!doctype html><meta charset="utf-8"><title>a2o reports ${urlPath}</title>
<style>body{font:14px/1.6 system-ui;margin:2rem;max-width:60rem}img{max-width:100%}</style>
<h1>a2o reports — ${urlPath}</h1><ul>${urlPath !== '/' ? '<li><a href="../">../</a></li>' : ''}${rows}</ul>`;
}

const server = createServer((req, res) => {
  void (async () => {
    const urlPath = decodeURIComponent((req.url ?? '/').split('?')[0]);
    const fsPath = normalize(join(ROOT, urlPath));
    if (!fsPath.startsWith(ROOT + sep) && fsPath !== ROOT) {
      res.writeHead(403).end('forbidden');
      return;
    }
    try {
      const info = await stat(fsPath);
      if (info.isDirectory()) {
        const names = await readdir(fsPath, { withFileTypes: true });
        const entries = names.map((d) => ({ name: d.name, dir: d.isDirectory() }));
        res.writeHead(200, { 'content-type': 'text/html; charset=utf-8', 'cache-control': 'no-store' });
        res.end(htmlIndex(urlPath, entries));
        return;
      }
      res.writeHead(200, {
        'content-type': MIME[extname(fsPath).toLowerCase()] ?? 'application/octet-stream',
        'content-length': info.size,
        'cache-control': 'no-store',
      });
      createReadStream(fsPath).pipe(res);
    } catch {
      res.writeHead(404).end('not found');
    }
  })();
});

server.listen(PORT, '0.0.0.0', () => {
  console.log(`a2o reports server: http://0.0.0.0:${PORT}/  (root: ${ROOT})`);
  console.log('Operator: open the Che "ui-playground" endpoint route for this workspace.');
});
