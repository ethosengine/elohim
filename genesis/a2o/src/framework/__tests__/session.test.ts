import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';

import { resolve, resolveFallback, type Registry, type SessionIdentity } from '../session.js';

const cases = JSON.parse(
  readFileSync(new URL('../../../../agentic/fixtures/session-cases.json', import.meta.url), 'utf8')
) as {
  name: string;
  env: Record<string, string>;
  startPid: number;
  registry: Registry;
  processes: [number, number, string, number, number, number, Record<string, string> | null][];
  expected: SessionIdentity;
}[];
for (const fixture of cases) {
  await test(`session parity: ${fixture.name}`, () => {
    const root = mkdtempSync(join(tmpdir(), 'session-'));
    try {
      for (const [pid, parent, comm, leader, tty, start, environ] of fixture.processes) {
        const dir = join(root, String(pid));
        mkdirSync(dir);
        if (environ !== null)
          writeFileSync(
            join(dir, 'environ'),
            Object.entries(environ)
              .map(([k, v]) => `${k}=${v}\0`)
              .join('')
          );
        const fields = Array<string>(20).fill('0');
        fields[0] = 'S';
        for (const [index, value] of [
          [1, parent],
          [3, leader],
          [4, tty],
          [19, start],
        ])
          fields[index] = String(value);
        writeFileSync(join(dir, 'stat'), `${pid} (${comm}) ${fields.join(' ')}`);
      }
      assert.deepEqual(
        resolveFallback({
          env: fixture.env,
          procRoot: root,
          startPid: fixture.startPid,
          registry: fixture.registry,
        }),
        fixture.expected
      );
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });
}
await test('session absent native binary falls back', () => {
  assert.equal(
    resolve({ env: { EPR_BIN: '/no/such/epr', ELOHIM_SESSION_ID: 'local' } }).session,
    'local'
  );
});
