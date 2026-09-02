/**
 * One-shot, HTTP-only receipt for the T3 workspace device-peer ladder
 * (genesis/docs/superpowers/specs/2026-08-30-workspace-stewarded-device-peer-design.md
 * "The ladder (four stations, each with its own counter)"), plus the release-
 * adoption "following" fact `hc-start.sh` now passes through on the join-alpha
 * path.
 *
 * Read-only: every line is answered from a GET fetch against an already
 * running peer. This script NEVER starts a process — no conductor, no
 * storage, no doorway, no child_process, no /proc reads — it only observes
 * state a running `just dev conductor alpha` (hc-start.sh join-alpha)
 * already produced.
 *
 * - **station 1 (joined)** reads the peer's own `/p2p/status` for a remote
 *   peer count, and corroborates with the fleet doorway's `/health` — "joined
 *   to alpha" is only meaningful if alpha itself is reachable from here.
 *   "elapsed" is this receipt run's own wall-clock, not the storage
 *   process's uptime: elohim-storage exposes no process-start field over
 *   HTTP, and reading `/proc/<pid>/stat` would need a PID lookup this
 *   script is deliberately not doing.
 * - **following** reads `/admin/adoption` for the release channel(s)
 *   `CONDUCTOR_RELEASE_CHANNELS` passed through as `ELOHIM_RELEASE_CHANNELS`
 *   on the storage launch. Not following is not a failure, so it SKIPs.
 * - **station 2 (pulled)** and **station 3 (recognised)** are structurally
 *   unmeasurable from this peer today — `sovereign-peer-network-read-no-
 *   authorities.md` names the pull-leg gap (every live alpha agent-info
 *   advertises `storageArc: null`), and `mishpat::bind_identity` (the
 *   identity-head spec's open graduation trigger) is unshipped — so they
 *   always print SKIP with the honest reason, never a synthesized PASS.
 *
 * Run from genesis/a2o:
 *   pnpm exec tsx scripts/device-peer-receipt.ts [--storage <url>] [--doorway <url>]
 */

const DEFAULT_STORAGE = 'http://127.0.0.1:8090';
const DEFAULT_DOORWAY = 'https://doorway-alpha.elohim.host';
const REQUEST_TIMEOUT_MS = 5_000;

const USAGE = `Usage: device-peer-receipt.ts [options]

Options:
  --storage <url>   the workspace device peer's own elohim-storage HTTP base
                     (default: ${DEFAULT_STORAGE})
  --doorway <url>   the fleet doorway used as the "alpha is reachable" leg
                     (default: ${DEFAULT_DOORWAY})
  -h, --help        show this help

Read-only. Never starts a conductor, storage, doorway, or mesh process —
point it at an already-running \`just dev conductor alpha\`.`;

class UsageError extends Error {}

type Verdict = 'PASS' | 'FAIL' | 'SKIP';

interface StationResult {
  label: string;
  verdict: Verdict;
  detail: string;
}

interface Options {
  storage: string;
  doorway: string;
  help: boolean;
}

function requiredValue(args: string[], index: number, flag: string): string {
  const value = args[index + 1];
  if (!value || value.startsWith('--')) throw new UsageError(`${flag} requires a value`);
  return value;
}

function parseHttpUrl(rawUrl: string, flag: string): string {
  let url: URL;
  try {
    url = new URL(rawUrl);
  } catch {
    throw new UsageError(`${flag} is not a valid URL: ${rawUrl}`);
  }
  if (url.protocol !== 'http:' && url.protocol !== 'https:') {
    throw new UsageError(`${flag} expects an HTTP(S) URL, got: ${rawUrl}`);
  }
  return url.toString().replace(/\/$/, '');
}

function parseArgs(argv: string[]): Options {
  let storage = DEFAULT_STORAGE;
  let doorway = DEFAULT_DOORWAY;
  let help = false;
  for (let index = 0; index < argv.length; index++) {
    const arg = argv[index];
    switch (arg) {
      case '--storage':
        storage = parseHttpUrl(requiredValue(argv, index, arg), arg);
        index++;
        break;
      case '--doorway':
        doorway = parseHttpUrl(requiredValue(argv, index, arg), arg);
        index++;
        break;
      case '-h':
      case '--help':
        help = true;
        break;
      default:
        throw new UsageError(`unknown option: ${arg}`);
    }
  }
  return { storage, doorway, help };
}

/** Node's fetch wraps the real connect error in `.cause`; surface both. */
function formatFetchError(error: unknown): string {
  if (error instanceof Error) {
    const cause = (error as Error & { cause?: unknown }).cause;
    if (cause instanceof Error) return `${error.message}: ${cause.message}`;
    return error.message;
  }
  return String(error);
}

async function fetchJson(url: string): Promise<Record<string, unknown>> {
  const response = await fetch(url, {
    headers: { accept: 'application/json' },
    signal: AbortSignal.timeout(REQUEST_TIMEOUT_MS),
  });
  if (!response.ok) throw new Error(`${url} answered HTTP ${response.status}`);
  const value: unknown = await response.json();
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    throw new Error(`${url} did not return a JSON object`);
  }
  return value as Record<string, unknown>;
}

async function probeReachable(url: string): Promise<{ ok: boolean; detail: string }> {
  try {
    const response = await fetch(url, { signal: AbortSignal.timeout(REQUEST_TIMEOUT_MS) });
    return { ok: response.ok, detail: `HTTP ${response.status}` };
  } catch (error) {
    return { ok: false, detail: formatFetchError(error) };
  }
}

/** connectedPeers (libp2p-backed status) or irohPeers.length (pure-iroh). */
function remotePeerCount(status: Record<string, unknown>): number | undefined {
  if (typeof status['connectedPeers'] === 'number') return status['connectedPeers'];
  if (Array.isArray(status['irohPeers'])) return status['irohPeers'].length;
  return undefined;
}

async function checkJoined(options: Options, runStartMs: number): Promise<StationResult> {
  const label = 'station 1 (joined)';
  let status: Record<string, unknown>;
  try {
    status = await fetchJson(`${options.storage}/p2p/status`);
  } catch (error) {
    return {
      label,
      verdict: 'FAIL',
      detail: `${options.storage}/p2p/status unreachable: ${formatFetchError(error)}`,
    };
  }
  const peers = remotePeerCount(status);
  const fleet = await probeReachable(`${options.doorway}/health`);
  const elapsedSecs = ((Date.now() - runStartMs) / 1_000).toFixed(2);
  if (peers === undefined) {
    return {
      label,
      verdict: 'FAIL',
      detail: `${options.storage}/p2p/status has neither connectedPeers nor irohPeers — cannot tell if this peer joined anything`,
    };
  }
  const fleetText = fleet.ok ? `reachable (${fleet.detail})` : `unreachable (${fleet.detail})`;
  if (peers < 1) {
    return {
      label,
      verdict: 'FAIL',
      detail: `remotePeers=${peers} elapsed=${elapsedSecs}s fleet(${options.doorway})=${fleetText}`,
    };
  }
  if (!fleet.ok) {
    return {
      label,
      verdict: 'FAIL',
      detail: `remotePeers=${peers} elapsed=${elapsedSecs}s but fleet(${options.doorway})=${fleetText}`,
    };
  }
  return {
    label,
    verdict: 'PASS',
    detail: `remotePeers=${peers} elapsed=${elapsedSecs}s fleet(${options.doorway})=${fleetText}`,
  };
}

interface FollowedChannel {
  channelId?: unknown;
  mode?: unknown;
}

function followedChannels(report: Record<string, unknown>): FollowedChannel[] {
  const value = report['channels'];
  return Array.isArray(value) ? (value as FollowedChannel[]) : [];
}

async function checkFollowing(options: Options): Promise<StationResult> {
  const label = 'following';
  let report: Record<string, unknown>;
  try {
    report = await fetchJson(`${options.storage}/admin/adoption`);
  } catch (error) {
    return {
      label,
      verdict: 'FAIL',
      detail: `${options.storage}/admin/adoption unreachable: ${formatFetchError(error)}`,
    };
  }
  const channels = followedChannels(report);
  if (channels.length === 0) {
    return {
      label,
      verdict: 'SKIP',
      detail:
        'not following — set CONDUCTOR_RELEASE_CHANNELS=<channel>=observe on ' +
        '`just dev conductor alpha` to ride a release channel',
    };
  }
  const names = channels
    .map(channel => `${String(channel.channelId ?? '?')}=${String(channel.mode ?? '?')}`)
    .join(', ');
  return { label, verdict: 'PASS', detail: names };
}

function checkPulled(): StationResult {
  return {
    label: 'station 2 (pulled)',
    verdict: 'SKIP',
    detail:
      'every live alpha agent-info advertises storageArc:null — a fleet peer has no ' +
      'authority to pull a workspace-authored id from yet ' +
      '(backlog: sovereign-peer-network-read-no-authorities.md); this peer alone cannot ' +
      'measure whether the fleet fetched it',
  };
}

function checkRecognised(): StationResult {
  return {
    label: 'station 3 (recognised)',
    verdict: 'SKIP',
    detail:
      "mishpat::bind_identity is unshipped (the identity-head spec's open graduation " +
      "trigger) — W cannot yet be bound as a controller of matthew's identity head, so " +
      'no fleet peer can resolve signer_is_known_agent(W)',
  };
}

function printResult(result: StationResult): void {
  console.log(`${result.verdict.padEnd(4)} ${result.label}: ${result.detail}`);
}

async function run(options: Options): Promise<number> {
  const runStartMs = Date.now();
  const results: StationResult[] = [];
  results.push(await checkJoined(options, runStartMs));
  results.push(await checkFollowing(options));
  results.push(checkPulled());
  results.push(checkRecognised());
  for (const result of results) printResult(result);
  const failed = results.filter(result => result.verdict === 'FAIL');
  if (failed.length > 0) {
    console.error(`RECEIPT INCOMPLETE: ${failed.length} station(s) FAILed`);
    return 1;
  }
  return 0;
}

try {
  const options = parseArgs(process.argv.slice(2));
  if (options.help) {
    console.log(USAGE);
    process.exitCode = 0;
  } else {
    process.exitCode = await run(options);
  }
} catch (error) {
  if (error instanceof UsageError) {
    console.error(`${error.message}\n\n${USAGE}`);
    process.exitCode = 64;
  } else {
    console.error(error instanceof Error ? error.stack : String(error));
    process.exitCode = 1;
  }
}
