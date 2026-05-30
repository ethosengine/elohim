/**
 * `look` — lightweight auth-aware "render & see" primitive.
 *
 * Renders a URL (optionally as a logged-in fixture human), screenshots it,
 * and writes a structured console/network/DOM capture the agent reads.
 * Reuses PlaywrightDevice so observability matches the cucumber suite exactly.
 */

export interface LookOptions {
  url: string;
  as?: string;
  doorway?: string;
  waitTestid?: string;
  out?: string;
  viewport?: { width: number; height: number };
}

const USAGE =
  'Usage: look <url> [--as <FixtureHuman>] [--doorway <id|url>] ' +
  '[--wait-testid <id>] [--out <slug>] [--viewport <WxH>]';

export function parseArgs(argv: string[]): LookOptions {
  const args = [...argv];
  const url = args.shift();
  if (!url || url.startsWith('--')) throw new Error(USAGE);

  const opts: LookOptions = { url };
  for (let i = 0; i < args.length; i++) {
    const flag = args[i];
    const val = args[i + 1];
    switch (flag) {
      case '--as':
        opts.as = val;
        i++;
        break;
      case '--doorway':
        opts.doorway = val;
        i++;
        break;
      case '--wait-testid':
        opts.waitTestid = val;
        i++;
        break;
      case '--out':
        opts.out = val;
        i++;
        break;
      case '--viewport': {
        const m = /^(\d+)x(\d+)$/.exec(val ?? '');
        if (!m) throw new Error(`--viewport expects WxH (e.g. 1280x800), got: ${val}`);
        opts.viewport = { width: Number(m[1]), height: Number(m[2]) };
        i++;
        break;
      }
      default:
        throw new Error(`Unknown flag: ${flag}`);
    }
  }
  return opts;
}
