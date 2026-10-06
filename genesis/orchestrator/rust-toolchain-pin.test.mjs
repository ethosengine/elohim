// Backlog arch-workspace-discipline row 31: the local gate and the epr/eprfs CI flake
// run ONE declared Rust. The gate pins RUSTUP_TOOLCHAIN in pool-policy.json's
// cargo_env_overrides."*"; elohim/epr/flake.nix reads elohim/epr/rust-toolchain.toml.
// A mismatch — or the flake floating back to `stable.latest` — fails here, so CI can
// never run a newer clippy than the gate without a commit that says so.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const read = rel => readFileSync(resolve(ROOT, rel), 'utf8');

test('the epr flake toolchain equals the local gate pin', () => {
  const gate = JSON.parse(read('genesis/agentic/pool-policy.json')).cargo_env_overrides['*'].RUSTUP_TOOLCHAIN;
  assert.match(gate, /^\d+\.\d+\.\d+$/, 'the gate pin is an exact version, not a channel name');
  const channel = read('elohim/epr/rust-toolchain.toml').match(/^\s*channel\s*=\s*"([^"]+)"/m)?.[1];
  assert.equal(channel, gate, 'elohim/epr/rust-toolchain.toml channel must equal pool-policy RUSTUP_TOOLCHAIN');
});

test('the flake reads the declared toolchain file and never floats', () => {
  const flake = read('elohim/epr/flake.nix').replace(/#.*$/gm, '');
  assert.match(flake, /fromRustupToolchainFile\s+\.\/rust-toolchain\.toml/);
  assert.doesNotMatch(flake, /stable\.latest|nightly\.latest|beta\.latest/);
});
