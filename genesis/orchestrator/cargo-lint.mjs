#!/usr/bin/env node
// Let Cargo finish the graph, then deny workspace warnings. gate-tap owns events.
import { spawn, spawnSync } from 'node:child_process';
import { createInterface } from 'node:readline';
import { constants } from 'node:os';
import { pathToFileURL } from 'node:url';

export function argumentsFor(args) {
  const separator = args.indexOf('--');
  const cargo = separator < 0 ? args : args.slice(0, separator);
  const lint = separator < 0 ? [] : args.slice(separator + 1);
  const kept = [];
  for (let i = 0; i < lint.length; i++) {
    if ((lint[i] === '-D' || lint[i] === '--deny') && lint[i + 1] === 'warnings') i++;
    else if (!['-Dwarnings', '--deny=warnings'].includes(lint[i])) kept.push(lint[i]);
  }
  const metadata = ['metadata', '--no-deps', '--format-version', '1'];
  for (let i = 0; i < cargo.length; i++) {
    if (
      ['--locked', '--offline', '--frozen'].includes(cargo[i]) ||
      cargo[i].startsWith('--manifest-path=')
    )
      metadata.push(cargo[i]);
    else if (cargo[i] === '--manifest-path') metadata.push(cargo[i], cargo[++i]);
  }
  return {
    metadata,
    clippy: [
      'clippy',
      '--keep-going',
      '--message-format=json-diagnostic-rendered-ansi',
      ...cargo,
      '--',
      ...kept,
    ],
  };
}

// ANSI may precede the severity. Preserve cargo's rendering apart from the denied label.
export function denyWarning(rendered) {
  const denied = rendered.replace(/^((?:\x1b\[[0-9;]*m)*)warning/, '$1error');
  const plain = denied.replace(/\x1b\[[0-9;]*m/g, '');
  const note = / = note: `-D [^`]+`/.test(plain)
    ? ''
    : '  = note: `-D warnings` implied by `-D warnings`\n';
  // The recognizer bounds excerpts and closes at a blank line. Put the synthetic
  // note immediately after the heading so even long suggestions arrive as lint.
  const [heading, ...body] = denied.trimEnd().split('\n');
  return `${heading}\n${note}${body.length ? `${body.join('\n')}\n` : ''}\n`;
}

export async function main(args) {
  const commands = argumentsFor(args);
  const metadata = spawnSync('cargo', commands.metadata, {
    encoding: 'utf8',
    maxBuffer: 16 * 1024 * 1024,
    stdio: ['inherit', 'pipe', 'inherit'],
  });
  if (metadata.error) {
    console.error(`cargo-lint: ${metadata.error.message}`);
    return 1;
  }
  if (metadata.status !== 0) return metadata.status ?? 1;
  const info = JSON.parse(metadata.stdout);
  const members = new Set(info.workspace_members);
  const names = new Map(info.packages.map((p) => [p.id, p.name]));
  const crates = new Set();
  const failedTargets = new Set();
  const failedUnits = new Set();
  let findings = 0;
  let warnings = 0;
  const child = spawn('cargo', commands.clippy, {
    stdio: ['inherit', 'pipe', 'pipe'],
  });
  // Cargo distinguishes lib from lib-test here; compiler-message.target does not.
  child.stderr.on('data', (chunk) => process.stderr.write(chunk));
  createInterface({ input: child.stderr, crlfDelay: Infinity }).on('line', (line) => {
    const plain = line.replace(/\x1b\[[0-9;]*m/g, '');
    const failed = /^error: could not compile (.+? \([^)]+\))/.exec(plain);
    if (failed) failedUnits.add(failed[1]);
  });
  const forwards = ['SIGINT', 'SIGTERM'].map((signal) => {
    const forward = () => child.kill(signal);
    process.on(signal, forward);
    return [signal, forward];
  });
  const finished = new Promise((resolve) => {
    child.on('error', (error) => {
      console.error(`cargo-lint: ${error.message}`);
      resolve(1);
    });
    child.on('close', (code, signal) => resolve(code ?? 128 + (constants.signals[signal] || 1)));
  });
  for await (const line of createInterface({
    input: child.stdout,
    crlfDelay: Infinity,
  })) {
    let event;
    try {
      event = JSON.parse(line);
    } catch {
      process.stdout.write(`${line}\n`);
      continue;
    }
    if (event.reason !== 'compiler-message') continue;
    const { message, package_id: id, target } = event;
    if (/^`?.+?`? (?:\([^)]*\) )?generated \d+ warnings?/.test(message.message)) continue;
    if (message.level === 'error')
      failedTargets.add(JSON.stringify([id, target?.name, target?.kind, target?.src_path]));
    const memberFinding = members.has(id) && ['warning', 'error'].includes(message.level);
    if (memberFinding) {
      findings++;
      crates.add(names.get(id) || id);
    }
    if (memberFinding && message.level === 'warning') {
      warnings++;
      process.stderr.write(denyWarning(message.rendered || `warning: ${message.message}\n`));
    } else if (message.rendered) process.stderr.write(message.rendered);
  }
  const code = await finished;
  for (const [signal, forward] of forwards) process.off(signal, forward);
  console.error(
    findings || code
      ? `cargo-lint: ${findings} finding(s) in ${[...crates].sort().join(', ') || '(none)'}; ${failedUnits.size || failedTargets.size} target(s) did not compile`
      : 'cargo-lint: clean',
  );
  return code || (warnings ? 1 : 0);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  process.exitCode = await main(process.argv.slice(2));
}
