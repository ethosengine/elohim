import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { dirname } from 'node:path';

import { renderTransportMatrix, type TransportMatrixInput } from './lib/transport-matrix.js';

import type { SprintReport } from './lib/aggregate.js';

function parseArgs(argv: string[]): { reports: string[]; out: string; generatedAt: string } {
  const reports: string[] = [];
  let out = 'reports/transport-matrix.md';
  let generatedAt = new Date().toISOString();
  for (let i = 0; i < argv.length; i += 1) {
    if (argv[i] === '--report') reports.push(argv[++i]);
    else if (argv[i] === '--out') out = argv[++i];
    else if (argv[i] === '--generated-at') generatedAt = argv[++i];
    else throw new Error(`unknown argument: ${argv[i]}`);
  }
  if (reports.length === 0) throw new Error('at least one --report path is required');
  return { reports, out, generatedAt };
}

function main(): void {
  const args = parseArgs(process.argv.slice(2));
  const inputs: TransportMatrixInput[] = args.reports.map(path => ({
    path,
    report: JSON.parse(readFileSync(path, 'utf8')) as SprintReport,
  }));
  mkdirSync(dirname(args.out), { recursive: true });
  writeFileSync(args.out, renderTransportMatrix(inputs, args.out, args.generatedAt));
  console.log(`Transport matrix written: ${args.out}`);
}

main();
