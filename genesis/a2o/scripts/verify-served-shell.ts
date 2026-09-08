/** CI adapter: keep the browser's capture and screenshot even when boot fails. */
import { mkdir, writeFile } from 'node:fs/promises';
import { join } from 'node:path';

import { browserShellFailures, captureBrowserShell } from './browser-shell.js';

const [url, slug, head, output] = process.argv.slice(2);
if (!url || !slug || !head || !output)
  throw new Error('Expected URL, slug, head and artifact directory');
await mkdir(output, { recursive: true });
const visit = await captureBrowserShell(
  url,
  Number(process.env.BOOT_TIMEOUT_MS ?? 30_000),
  join(output, 'shot.png')
);
await writeFile(join(output, 'capture.json'), JSON.stringify(visit, null, 2));
const failures = browserShellFailures(visit);
if (failures.length) {
  console.error(`[${slug}] ${url} @ ${head}: ${failures.join('; ')}`);
  process.exitCode = 1;
} else {
  console.log(`[${slug}] ${url} @ ${head}: browser rendered the application without errors`);
}
