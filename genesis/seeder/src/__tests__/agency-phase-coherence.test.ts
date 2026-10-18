/**
 * Who may sign in, and where they live, must be declared once and agree
 * (ruled 2026-10-14).
 *
 * The household once carried 20 agents on matthew's conductor because two
 * homes disagreed and a default filled the gap: fixture humans with no
 * declared agencyPhase registered as `hosted`, and a doorway installed cells
 * for `device` humans who already had their own conductor. These checks read
 * the two homes, genesis/data/humans (agencyPhase) and
 * genesis/orchestrator/data/deployments.json (pods), and fail on any human the
 * two describe differently.
 *
 *   - a human with a pod declares the phase of someone with their own peer
 *     runtime (device, node or doorway), never hosted and never nothing;
 *   - a hosted human has no pod: they live on a doorway operator's conductor;
 *   - only a doorway operator (agencyPhase doorway or node) is flagged
 *     hostsAgents, and at least one operator is.
 */
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

const root = resolve(__dirname, '../../../..');

interface Human {
  displayName: string;
  agencyPhase?: string;
}
interface Pod {
  name: string;
  hostsAgents?: boolean;
}

const humans: Human[] = JSON.parse(
  readFileSync(resolve(root, 'genesis/data/humans/humans.json'), 'utf-8')
).humans;
const pods: Pod[] = JSON.parse(
  readFileSync(resolve(root, 'genesis/orchestrator/data/deployments.json'), 'utf-8')
).humans;

const phaseOf = new Map(humans.map(h => [h.displayName.toLowerCase(), h.agencyPhase]));
const podNames = new Set(pods.map(p => p.name.toLowerCase()));

describe('agency phase and pods agree', () => {
  it('every human with a pod declares device, node or doorway', () => {
    const wrong = pods
      .filter(p => !['device', 'node', 'doorway'].includes(phaseOf.get(p.name) ?? ''))
      .map(p => `${p.name}: agencyPhase=${phaseOf.get(p.name) ?? '(none)'}`);
    expect(wrong).toEqual([]);
  });

  it('no hosted human has a pod', () => {
    const wrong = humans
      .filter(h => h.agencyPhase === 'hosted' && podNames.has(h.displayName.toLowerCase()))
      .map(h => h.displayName);
    expect(wrong).toEqual([]);
  });

  it('only doorway operators host agents, and some operator does', () => {
    const hosts = pods.filter(p => p.hostsAgents);
    const wrong = hosts
      .filter(p => !['doorway', 'node'].includes(phaseOf.get(p.name) ?? ''))
      .map(p => `${p.name}: agencyPhase=${phaseOf.get(p.name) ?? '(none)'}`);
    expect(wrong).toEqual([]);
    expect(hosts.length).toBeGreaterThan(0);
  });
});
