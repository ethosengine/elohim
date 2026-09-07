import { describe, it, expect } from 'vitest';
import { resolveDnaCreateContentReach } from '../reach-resolver.js';

// genesis/data/timeline/backlog/prologue-seed-step-relationship-type-invalid.md
// "Root cause found" section: seed-production.ts never sent `reach` on the
// create_content payload it sends directly to the content_store zome, and
// `CreateContentInput.reach` (elohim/sdk/domains/lamad/types/src/lib.rs:39)
// has no serde default — so the call returns `WasmError Deserialize`, not a
// catchable validation error. This resolver is the one place a direct-WASM
// create_content caller in the seeder turns a row + a corpus declaration into
// a value that CANNOT be absent, and never falls back to a hardcoded literal.
describe('resolveDnaCreateContentReach', () => {
  it('honors the row-authored reach', () => {
    expect(resolveDnaCreateContentReach('community', 'commons', 'content x')).toBe('community');
  });

  it('falls back to the corpus-declared reach when the row carries none', () => {
    expect(resolveDnaCreateContentReach(undefined, 'commons', 'content x')).toBe('commons');
  });

  it('falls back to the corpus-declared reach when the row reach is an empty string', () => {
    expect(resolveDnaCreateContentReach('', 'commons', 'content x')).toBe('commons');
  });

  it('falls back to the corpus-declared reach when the row reach is not a string', () => {
    expect(resolveDnaCreateContentReach(42, 'commons', 'content x')).toBe('commons');
  });

  it('HARD-FAILS naming the content when neither the row nor the corpus declares a reach', () => {
    expect(() => resolveDnaCreateContentReach(undefined, undefined, 'content x')).toThrow(
      /content x/,
    );
    expect(() => resolveDnaCreateContentReach(undefined, undefined, 'content x')).toThrow(
      /reach/i,
    );
  });

  it('HARD-FAILS on a non-canonical authored reach (no silent coalesce)', () => {
    expect(() => resolveDnaCreateContentReach('invited', 'commons', 'content x')).toThrow(
      /non-canonical reach/i,
    );
  });

  it('HARD-FAILS on a non-canonical corpus-declared reach', () => {
    expect(() => resolveDnaCreateContentReach(undefined, 'invited', 'content x')).toThrow(
      /non-canonical reach/i,
    );
  });
});
