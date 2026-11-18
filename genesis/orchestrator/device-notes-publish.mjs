#!/usr/bin/env node
// device-notes-publish — this device's gate build notes go to its OWN destination on the forge
// (ruling R4 of the native-throughput sprint; backlog row 3 of steward-device-arrival-open-items).
//
// brit writes build notes locally under `refs/notes/brit/build/<step>`. Until R4 every device
// pushed them to the same refs on origin, so the second device to push was rejected on every ref
// the first had moved: the two devices' signed records diverge and no one may merge over them.
// Each device now publishes to
//
//     refs/notes/brit/devices/<device>/build/<step>
//
// where <device> is the device's DID with brit's ref segment encoding (`encodeRefSegment`, the
// encoding brit already applies to step names: `did:key:z6Mk…` → `did%3Akey%3Az6Mk…`). The
// encoding is reversible, so a reader recovers the exact DID from the ref name; it is DID-method
// agnostic; and it is the one decoder brit, ark and delivery-series already apply to the step
// segment. Local refs stay where brit writes and reads them. The legacy shared
// `refs/notes/brit/build/*` refs on the forge are frozen: this script never names them as a
// destination.
//
// A device's mirror only fast-forwards. Before anything is pushed every destination the forge
// already holds is compared with the local ref: equal is up to date, an ancestor of the local ref
// is a fast-forward, anything else refuses the whole publish with a named cure. Nothing is forced,
// merged or deleted; a ref the forge holds that this device no longer has is kept. The push is
// `--atomic`, so a refusal on the forge side moves nothing either.
//
//   node genesis/orchestrator/device-notes-publish.mjs [--dry-run] [--remote origin]
//        [--device did:…] [--root DIR] [--no-atomic]
//   just dev notes            # dry run
//   just dev notes publish    # publish
//
// --dry-run reads only local state (no forge contact) and prints the refspecs it would push.
// The device DID comes from --device, else DEVICE_NOTES_DID, else `epr actor current --device`.
// The push program is git (the root CLAUDE.md landing exception: brit 0.1.12 closed send-pack
// after a green hook); DEVICE_NOTES_PUSH=brit selects brit's git-compatible `push`.
//
// Exit: 0 published, up to date, or dry run; 1 an error (no device, git failed); 2 refused.

import { spawnSync } from 'node:child_process';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { resolveBin } from './gate-evidence.mjs';
import { encodeRefSegment } from './scripts/brit-attest.mjs';

export const LOCAL_PREFIX = 'refs/notes/brit/build/';
export const LEGACY_PREFIX = LOCAL_PREFIX;
export const DEVICES_PREFIX = 'refs/notes/brit/devices/';
// DID core syntax (did:<method>:<method-specific-id>), without the path, query and fragment parts
// a device DID never carries. `/` would split the ref into extra components, so it is refused.
const DID = /^did:[a-z0-9]+:[A-Za-z0-9._:%-]+$/;

/** The forge prefix one device publishes under: `refs/notes/brit/devices/<encoded did>/build/`. */
export function devicePrefix(did) {
  if (!DID.test(String(did || ''))) throw new Error(`not a device DID: ${JSON.stringify(did)}`);
  return `${DEVICES_PREFIX}${encodeRefSegment(did)}/build/`;
}

/** The forge ref a local build-notes ref publishes to. */
export function destinationOf(localRef, did) {
  if (!localRef.startsWith(LOCAL_PREFIX)) throw new Error(`not a build-notes ref: ${localRef}`);
  return devicePrefix(did) + localRef.slice(LOCAL_PREFIX.length);
}

/** The pattern refspec equivalent to a full publish, for a reader doing it by hand. */
export function patternRefspec(did) {
  return `${LOCAL_PREFIX}*:${devicePrefix(did)}*`;
}

function git(root, args, { env = process.env, spawn = spawnSync } = {}) {
  const r = spawn('git', args, { cwd: root, env, encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 });
  return { status: r.status ?? 1, stdout: r.stdout || '', stderr: r.stderr || (r.error ? String(r.error.message) : '') };
}

function mustGit(root, args, opts) {
  const r = git(root, args, opts);
  if (r.status !== 0) throw new Error(`git ${args.join(' ')} failed (exit ${r.status}): ${r.stderr.trim()}`);
  return r.stdout;
}

/** This device's DID: --device, then DEVICE_NOTES_DID, then `epr actor current --device --json`. */
export function deviceDid({ root, explicit, env = process.env, spawn = spawnSync }) {
  if (explicit) return { did: explicit, via: '--device' };
  if ((env.DEVICE_NOTES_DID || '').trim()) return { did: env.DEVICE_NOTES_DID.trim(), via: 'DEVICE_NOTES_DID' };
  const epr = resolveBin('epr', ['EPR_BIN'], env);
  if (!epr) return { did: null, via: 'epr', why: 'no epr binary on PATH or EPR_BIN' };
  const r = spawn(epr, ['actor', 'current', '--device', '--json', '--root', root], { cwd: root, env, encoding: 'utf8' });
  if (r.status !== 0) return { did: null, via: 'epr', why: `epr actor current --device exited ${r.status}: ${(r.stderr || '').trim()}` };
  try {
    const did = JSON.parse(r.stdout)?.standing?.device;
    return typeof did === 'string' && did ? { did, via: 'epr' } : { did: null, via: 'epr', why: 'this checkout has no device standing' };
  } catch (e) {
    return { did: null, via: 'epr', why: `epr actor current --device printed no JSON: ${e.message}` };
  }
}

/** Local build-notes refs: [{ ref, sha }]. */
export function localNotes(root, opts) {
  return mustGit(root, ['for-each-ref', '--format=%(refname) %(objectname)', LOCAL_PREFIX], opts)
    .split('\n').filter(Boolean).map(line => { const [ref, sha] = line.split(' '); return { ref, sha }; });
}

/** The device's mirror on the forge: Map(destination ref → sha). */
export function forgeMirror(root, remote, did, opts) {
  const out = mustGit(root, ['ls-remote', '--refs', remote, `${devicePrefix(did)}*`], opts);
  const held = new Map();
  for (const line of out.split('\n').filter(Boolean)) {
    const [sha, ref] = line.split('\t');
    held.set(ref, sha);
  }
  return held;
}

/**
 * Classify each local ref against what the forge holds at its destination. Pure apart from the
 * object and ancestry reads, which go through `has(sha)` and `isAncestor(a, b)`.
 */
export function plan({ locals, held, did, remote, has, isAncestor }) {
  const actions = [];
  const refusals = [];
  for (const { ref, sha } of locals) {
    const dest = destinationOf(ref, did);
    if (!dest.startsWith(DEVICES_PREFIX)) throw new Error(`refusing a destination outside ${DEVICES_PREFIX}: ${dest}`);
    const forge = held.get(dest);
    if (!forge) { actions.push({ kind: 'new', ref, dest, sha }); continue; }
    if (forge === sha) { actions.push({ kind: 'up-to-date', ref, dest, sha }); continue; }
    if (!has(forge)) {
      refusals.push({
        kind: 'unknown-forge-tip', ref, dest, sha, forge,
        cure: `the forge's tip is not in this checkout; fetch it beside the notes to compare, `
          + `\`git fetch ${remote} ${dest}:refs/brit-mirror/${dest.slice(DEVICES_PREFIX.length)}\`, and run again`,
      });
      continue;
    }
    if (isAncestor(forge, sha)) { actions.push({ kind: 'fast-forward', ref, dest, sha, forge }); continue; }
    if (isAncestor(sha, forge)) {
      refusals.push({
        kind: 'rewound', ref, dest, sha, forge,
        cure: `the local ref is behind what this device already published; restore it with `
          + `\`git fetch ${remote} ${dest}:${ref}\` (a fast-forward of the local ref) — never force the mirror back`,
      });
      continue;
    }
    refusals.push({
      kind: 'diverged', ref, dest, sha, forge,
      cure: `the local ref and this device's published mirror have diverged; both hold signed records, `
        + `which are never merged or forced (ruling R4). Compare with \`git log --oneline ${forge}...${sha}\` `
        + `and record the finding against backlog row 3 before publishing this step`,
    });
  }
  return { actions, refusals };
}

const short = sha => String(sha || '').slice(0, 12);

/** Run one publish. Returns the exit code; `log` receives every line. */
export function publish({ root, remote = 'origin', did: explicit, dryRun = false, atomic = true,
  env = process.env, spawn = spawnSync, log = line => console.log(line) }) {
  const opts = { env, spawn };
  const who = deviceDid({ root, explicit, env, spawn });
  if (!who.did) {
    log(`device-notes: REFUSED no device identity (${who.why}) — cure: run \`just status device\` and bind this `
      + `workspace (\`epr actor device enroll | authorize | bind\`), or pass --device <did>`);
    return 1;
  }
  let prefix;
  try { prefix = devicePrefix(who.did); } catch (e) { log(`device-notes: REFUSED ${e.message}`); return 1; }
  const check = git(root, ['check-ref-format', `${prefix}x`], opts);
  if (check.status !== 0) { log(`device-notes: REFUSED ${prefix} is not a valid ref name`); return 1; }

  const locals = localNotes(root, opts);
  log(`device-notes: device ${who.did} (via ${who.via}) → ${prefix}*`);
  log(`device-notes: legacy ${LEGACY_PREFIX}* on the forge is frozen; this publish never writes it`);
  if (locals.length === 0) { log('device-notes: no local build notes; nothing to publish'); return 0; }

  if (dryRun) {
    for (const { ref } of locals) log(`device-notes: refspec ${ref}:${destinationOf(ref, who.did)}`);
    log(`device-notes: dry run, ${locals.length} refspec(s) (pattern ${patternRefspec(who.did)}); `
      + `the forge was not read and nothing was pushed`);
    return 0;
  }

  const held = forgeMirror(root, remote, who.did, opts);
  const has = sha => git(root, ['cat-file', '-e', `${sha}^{commit}`], opts).status === 0;
  const isAncestor = (a, b) => git(root, ['merge-base', '--is-ancestor', a, b], opts).status === 0;
  const { actions, refusals } = plan({ locals, held, did: who.did, remote, has, isAncestor });
  const localDests = new Set(locals.map(({ ref }) => destinationOf(ref, who.did)));
  for (const dest of held.keys()) if (!localDests.has(dest)) log(`device-notes: kept ${dest} (forge only; never deleted)`);

  if (refusals.length > 0) {
    for (const r of refusals) {
      log(`device-notes: REFUSED ${r.kind} ${r.dest}: forge ${short(r.forge)}, local ${short(r.sha)} — cure: ${r.cure}`);
    }
    log(`device-notes: REFUSED ${refusals.length} of ${locals.length} ref(s); nothing pushed (a mirror only fast-forwards)`);
    return 2;
  }
  const moving = actions.filter(a => a.kind !== 'up-to-date');
  for (const a of actions) {
    if (a.kind === 'up-to-date') continue;
    log(`device-notes: ${a.kind} ${a.dest} ${a.forge ? `${short(a.forge)}..` : ''}${short(a.sha)}`);
  }
  if (moving.length === 0) { log(`device-notes: up to date, ${actions.length} ref(s)`); return 0; }

  // Refspecs carry no `+`: the push program refuses a non-fast-forward on its own too.
  const program = (env.DEVICE_NOTES_PUSH || 'git').trim() === 'brit' ? 'brit' : 'git';
  const args = ['push', ...(atomic ? ['--atomic'] : []), remote, ...moving.map(a => `${a.ref}:${a.dest}`)];
  const r = spawn(program, args, { cwd: root, env, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] });
  for (const line of `${r.stdout || ''}${r.stderr || ''}`.split('\n').filter(Boolean)) log(`device-notes: ${program}: ${line}`);
  if (r.status !== 0) {
    log(`device-notes: REFUSED ${program} push exited ${r.status ?? 'without a status'}; the forge refused or was unreachable `
      + `— cure: read the lines above; a non-fast-forward means another writer moved this device's mirror (run again to classify it)`);
    return 2;
  }
  log(`device-notes: published ${moving.length} ref(s), ${actions.length - moving.length} already up to date`);
  return 0;
}

function parseArgs(argv) {
  const out = { dryRun: false, atomic: true, remote: 'origin', did: null, root: process.cwd() };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === '--dry-run') out.dryRun = true;
    else if (a === '--no-atomic') out.atomic = false;
    else if (a === '--remote') out.remote = argv[++i];
    else if (a === '--device') out.did = argv[++i];
    else if (a === '--root') out.root = resolve(argv[++i]);
    else throw new Error(`unknown argument: ${a}`);
  }
  if (!out.remote) throw new Error('--remote needs a value');
  return out;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  let args;
  try { args = parseArgs(process.argv.slice(2)); } catch (e) {
    console.error(`device-notes: ${e.message}`);
    process.exit(1);
  }
  try {
    process.exit(publish(args));
  } catch (e) {
    console.error(`device-notes: ${e.message}`);
    process.exit(1);
  }
}
