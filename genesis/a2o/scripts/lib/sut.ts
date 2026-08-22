/**
 * `sut` — the system-under-test half of the derivation key (Law II, guidestar
 * 2026-08-22 §3).
 *
 * WHY THIS EXISTS AT ALL. Clippy works as a memoized measure because its
 * subject IS the code: an edit always moves the key. An a2o scenario is not
 * the code — its subject is a feature file and the code under test lives
 * somewhere else entirely. Every other env candidate (fixture manifest, DNA
 * hash, image tag, peer count) is a DEPLOYED-ARTIFACT identity, and none of
 * them moves when the operator edits an uncommitted source file. Under that
 * key an edit to elohim-storage moves neither subject nor env, and the run
 * reports permitted for a binary nobody ran. `sut` is the term that closes it.
 *
 * RESOLUTION ORDER per component, honest about what it can reach:
 *   1. `A2O_SUT_<NAME>`     — an identity the lane DECLARES (image digest,
 *                             artifact id). The fleet's only real answer.
 *   2. `A2O_SUT_BIN_<NAME>` — a built binary to hash BY BYTES. Opt-in by path
 *                             because hashing a multi-hundred-MB debug binary
 *                             on every local run spends the operator's evening,
 *                             which is the one budget this design protects.
 *   3. `git rev-parse HEAD:<path>` tree hash of the source that produces it,
 *                             PLUS a dirty digest over `git diff HEAD` and the
 *                             content hashes of untracked files when the tree
 *                             is not clean. An uncommitted edit MUST move this.
 *   4. nothing → the component's NAME goes in `unknown` and it is left out of
 *      the hash. Never silently hash a shorter tuple.
 *
 * On (3) vs (1): a source edit moves the tree hash before the binary is
 * rebuilt, so the key can be fresher than the artifact. That direction is a
 * miss where a hit was possible — cheap. The opposite (a stale hit for an
 * unbuilt edit) is the laundering failure Law I exists to abolish. When the two
 * disagree, prefer the false miss.
 *
 * Every function here takes an injected `SutProbe`, so the whole computation is
 * unit-testable without a git tree, a mesh, or a build.
 */

import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

/** sha256 truncated to 16 hex chars — the shape the schema's `sut` pattern enforces. */
export function sha256Short(input: string | Buffer): string {
  return createHash('sha256').update(input).digest('hex').slice(0, 16);
}

export interface SutProbe {
  /** stdout of a git command run at the repo root, or null when git cannot answer. */
  git(args: string[], stdin?: string): string | null;
  /** sha256Short over a file's bytes, or null when unreadable. */
  hashFile(path: string): string | null;
  env: Readonly<Record<string, string | undefined>>;
}

export interface SutComponent {
  /** Name in `sutParts` / `unknown` (prefixed `sut.` in `unknown`). */
  name: string;
  /** Repo-relative source path that produces the artifact, when one exists. */
  path?: string;
  /** Env vars carrying a ready-made identity (image digest, DNA hash, …). */
  declaredBy?: string[];
  /** Env var naming a file whose BYTES are the identity (the fixture manifest). */
  fileBy?: string[];
}

/**
 * The components of an a2o run's system under test.
 *
 * `conductor` has no source tree here on purpose: elohim/holochain-conductor is
 * an `update = none` submodule that CI compiles at exactly the committed SHA,
 * so the gitlink IS the built conductor's identity (see CLAUDE.md, "The custom
 * conductor reaches the fleet via the submodule pointer"). `git rev-parse
 * HEAD:<submodule>` returns that SHA, and `git status --porcelain` still
 * reports a moved pointer as dirty.
 */
export const DEFAULT_SUT_COMPONENTS: readonly SutComponent[] = [
  { name: 'storage', path: 'elohim/elohim-storage' },
  { name: 'doorway', path: 'doorway/doorway-service' },
  { name: 'conductor', path: 'elohim/holochain-conductor' },
  { name: 'dna', path: 'elohim/holochain/dna' },
  { name: 'dnaHash', declaredBy: ['E2E_DNA_HASH', 'ELOHIM_DNA_HASH'] },
  { name: 'fixtures', fileBy: ['E2E_HOUSEHOLD_FIXTURE_PATH'] },
];

export interface SutIdentity {
  sut: string;
  sutParts: Record<string, string>;
  /** Names of components that could not be obtained, prefixed `sut.`. */
  unknown: string[];
}

function envVarName(prefix: string, name: string): string {
  return `${prefix}${name.replace(/([a-z0-9])([A-Z])/g, '$1_$2').toUpperCase()}`;
}

/**
 * A digest over the UNCOMMITTED state of `path`, or null when the tree is clean
 * (or when git cannot say).
 *
 * The porcelain line alone is not enough: it names which files are dirty but
 * not what they now contain, so a second edit to the same file would not move
 * it. The digest therefore covers the full `git diff HEAD` text plus the
 * content hashes of untracked files. Ignored paths (every `target/`) are
 * excluded by `--exclude-standard`, so this stays cheap.
 */
export function dirtyDigest(probe: SutProbe, path: string): string | null {
  const porcelain = probe.git(['status', '--porcelain', '--', path]);
  if (porcelain === null || porcelain.trim() === '') return null;

  const diff = probe.git(['diff', 'HEAD', '--', path]) ?? '';
  const untracked = probe.git(['ls-files', '--others', '--exclude-standard', '--', path]) ?? '';
  const untrackedHashes = untracked.trim()
    ? (probe.git(['hash-object', '--stdin-paths'], untracked) ?? '')
    : '';

  return sha256Short([porcelain, diff, untracked, untrackedHashes].join('\n'));
}

/** One component's identity, or null when nothing could be obtained. */
export function resolveComponent(probe: SutProbe, component: SutComponent): string | null {
  for (const key of [envVarName('A2O_SUT_', component.name), ...(component.declaredBy ?? [])]) {
    const declared = probe.env[key]?.trim();
    if (declared) return `declared:${declared}`;
  }

  for (const key of [envVarName('A2O_SUT_BIN_', component.name), ...(component.fileBy ?? [])]) {
    const filePath = probe.env[key]?.trim();
    if (!filePath) continue;
    const digest = probe.hashFile(filePath);
    // A named-but-unreadable artifact is NOT a clean absence: fall through so
    // the component lands in `unknown` rather than quietly borrowing the tree.
    if (digest) return `file:sha256:${digest}`;
    return null;
  }

  if (component.path) {
    const tree = probe.git(['rev-parse', `HEAD:${component.path}`])?.trim();
    if (tree) {
      const dirty = dirtyDigest(probe, component.path);
      return dirty ? `tree:${tree}+dirty:${dirty}` : `tree:${tree}`;
    }
  }

  return null;
}

/**
 * Deterministic hash over the obtained parts: sorted `name=value` lines.
 *
 * An EMPTY parts map still yields a hash — of the empty tuple. That is not a
 * silent shorter tuple: every missing component is named in `unknown` beside
 * it, so the key reads "nothing about the SUT was determinable here" rather
 * than pretending to a specific system.
 */
export function hashSutParts(parts: Record<string, string>): string {
  const canonical = Object.keys(parts)
    .sort((a, b) => a.localeCompare(b))
    .map(key => `${key}=${parts[key]}`)
    .join('\n');
  return `sha256:${sha256Short(canonical)}`;
}

export function computeSut(
  probe: SutProbe,
  components: readonly SutComponent[] = DEFAULT_SUT_COMPONENTS
): SutIdentity {
  const sutParts: Record<string, string> = {};
  const unknown: string[] = [];

  for (const component of components) {
    const value = resolveComponent(probe, component);
    if (value) sutParts[component.name] = value;
    else unknown.push(`sut.${component.name}`);
  }

  return { sut: hashSutParts(sutParts), sutParts, unknown };
}

/** The real probe: git at `repoRoot`, sha256 over file bytes, process env. */
export function createSutProbe(
  repoRoot: string,
  env: Readonly<Record<string, string | undefined>> = process.env
): SutProbe {
  return {
    git(args: string[], stdin?: string): string | null {
      try {
        // eslint-disable-next-line sonarjs/no-os-command-from-path -- git is a standard system tool
        return execFileSync('git', args, {
          cwd: repoRoot,
          encoding: 'utf8',
          input: stdin,
          maxBuffer: 64 * 1024 * 1024,
          stdio: ['pipe', 'pipe', 'ignore'],
        });
      } catch {
        return null;
      }
    },
    hashFile(path: string): string | null {
      try {
        return sha256Short(readFileSync(path));
      } catch {
        return null;
      }
    },
    env,
  };
}
