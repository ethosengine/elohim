#!/usr/bin/env node
// Batch Prettier worker for the commit-time formatting ratchet (_lib/epr_format.py).
//
// One process per gate invocation. Each stdin line is {"items":[{id, file, ignorePaths,
// resolveFrom, config?}]}; each stdout line is {"results":[{id, state, version, configFile, detail,
// syntax}]}. `file` is a path inside a disposable snapshot tree (the index or HEAD bytes with
// their committed config closure), so resolveConfig/getFileInfo see the COMMITTED configuration,
// never the worktree's. Prettier itself is loaded from the owning package (`resolveFrom`), once
// per installed copy: no npx, no download, no global fallback. This worker never writes a file.
import { createRequire } from 'node:module';
import { readFile } from 'node:fs/promises';
import { realpathSync } from 'node:fs';
import { join } from 'node:path';
import { createInterface } from 'node:readline';

const installs = new Map(); // resolveFrom -> { prettier, version, path } | { error }

function load(resolveFrom) {
  if (installs.has(resolveFrom)) return installs.get(resolveFrom);
  let entry;
  try {
    const req = createRequire(join(resolveFrom, '__epr_format__.js'));
    const path = realpathSync(req.resolve('prettier'));
    const prettier = req(path);
    entry = { prettier, version: prettier.version, path };
  } catch (error) {
    entry = { error: String(error?.message ?? error).split('\n')[0] };
  }
  installs.set(resolveFrom, entry);
  return entry;
}

async function checkOne(item) {
  const lib = load(item.resolveFrom);
  if (lib.error) return { id: item.id, state: 'missing', detail: lib.error };
  const { prettier, version } = lib;
  try {
    // An explicit --config also governs parser inference: discovery here could otherwise load
    // a nearby config (and its plugins) that the package's CLI command would never read.
    const explicitConfig = item.config
      ? ((await prettier.resolveConfig(item.file, { config: item.config, editorconfig: true })) ?? {})
      : undefined;
    const info = await prettier.getFileInfo(item.file, {
      ignorePath: item.ignorePaths,
      ...(item.config ? { resolveConfig: false, plugins: explicitConfig.plugins } : {}),
    });
    if (info.ignored) return { id: item.id, state: 'ignored', version };
    if (!info.inferredParser) {
      return { id: item.id, state: 'outside', version, detail: 'Prettier infers no parser' };
    }
    // `config` is the package's `--config <file>` (the CLI then skips discovery but still merges
    // .editorconfig); without it Prettier discovers the nearest config, as the CLI does.
    const config =
      explicitConfig ?? (await prettier.resolveConfig(item.file, { editorconfig: true })) ?? {};
    const configFile = item.config ?? (await prettier.resolveConfigFile(item.file));
    const source = await readFile(item.file, 'utf8');
    const ok = await prettier.check(source, { ...config, filepath: item.file });
    return { id: item.id, state: ok ? 'formatted' : 'unformatted', version, configFile };
  } catch (error) {
    const syntax = error?.name === 'SyntaxError' || Boolean(error?.loc);
    return {
      id: item.id,
      state: 'error',
      version,
      syntax,
      detail: String(error?.message ?? error).split('\n')[0].slice(0, 300),
    };
  }
}

const rl = createInterface({ input: process.stdin, crlfDelay: Infinity });
let chain = Promise.resolve();
rl.on('line', (line) => {
  chain = chain.then(async () => {
    let request;
    try {
      request = JSON.parse(line);
    } catch (error) {
      process.stdout.write(`${JSON.stringify({ results: [], error: String(error) })}\n`);
      return;
    }
    const results = [];
    for (const item of request.items ?? []) results.push(await checkOne(item));
    process.stdout.write(`${JSON.stringify({ results })}\n`);
  });
});
