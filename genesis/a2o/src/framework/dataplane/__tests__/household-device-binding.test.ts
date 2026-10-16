/* eslint-disable @typescript-eslint/no-floating-promises -- node:test owns test promises. */
import { strict as assert } from 'node:assert';
import { mkdirSync, mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, describe, it } from 'node:test';

import { householdDeviceBinding, missingDeviceBindingMessage } from '../carried-election.js';

const saved = { b: process.env.A2O_DEVICE_BINDING, m: process.env.MESH_DIR, cwd: process.cwd() };
afterEach(() => {
  for (const [k, v] of [
    ['A2O_DEVICE_BINDING', saved.b],
    ['MESH_DIR', saved.m],
  ] as const) {
    if (v === undefined) delete process.env[k];
    else process.env[k] = v;
  }
  process.chdir(saved.cwd);
});

function bindingFile(dir: string, binding: string, nested = true): string {
  const parent = nested ? join(dir, 'device-ceremony') : dir;
  mkdirSync(parent, { recursive: true });
  const file = join(parent, 'binding.json');
  writeFileSync(file, JSON.stringify({ binding }));
  return file;
}

describe('householdDeviceBinding', () => {
  it('A2O_DEVICE_BINDING wins over MESH_DIR', () => {
    const a = mkdtempSync(join(tmpdir(), 'bind-a-'));
    const m = mkdtempSync(join(tmpdir(), 'bind-m-'));
    process.env.A2O_DEVICE_BINDING = bindingFile(a, 'from-env', false);
    process.env.MESH_DIR = m;
    bindingFile(m, 'from-mesh');
    assert.equal(householdDeviceBinding(), 'from-env');
  });

  it('uses MESH_DIR when the env var is absent', () => {
    const m = mkdtempSync(join(tmpdir(), 'bind-m-'));
    delete process.env.A2O_DEVICE_BINDING;
    process.env.MESH_DIR = m;
    bindingFile(m, 'from-mesh');
    assert.equal(householdDeviceBinding(), 'from-mesh');
  });

  it('is undefined when nothing resolves', () => {
    const empty = mkdtempSync(join(tmpdir(), 'bind-empty-'));
    mkdirSync(join(empty, 'cwd'));
    process.chdir(join(empty, 'cwd'));
    delete process.env.A2O_DEVICE_BINDING;
    delete process.env.MESH_DIR;
    assert.equal(householdDeviceBinding(), undefined);
  });

  it('names the cure and the paths tried', () => {
    process.env.MESH_DIR = '/nowhere/household';
    delete process.env.A2O_DEVICE_BINDING;
    const message = missingDeviceBindingMessage();
    assert.match(message, /household device binding not found/);
    assert.match(message, /\/nowhere\/household\/device-ceremony\/binding\.json/);
    assert.match(message, /set A2O_DEVICE_BINDING or run the lane with MESH_DIR/);
  });

  it('a missing override stops resolution and the message names only it', () => {
    const m = mkdtempSync(join(tmpdir(), 'bind-m-'));
    bindingFile(m, 'from-mesh');
    process.env.MESH_DIR = m;
    process.env.A2O_DEVICE_BINDING = join(m, 'absent.json');
    assert.equal(householdDeviceBinding(), undefined);
    const message = missingDeviceBindingMessage();
    assert.match(message, /absent\.json/);
    assert.match(message, /fix or unset A2O_DEVICE_BINDING/);
    assert.doesNotMatch(message, /device-ceremony|run the lane with MESH_DIR/);
  });
});
