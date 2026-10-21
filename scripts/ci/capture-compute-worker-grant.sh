#!/usr/bin/env bash
# CI-only, read-only activation evidence. Run after the provider conductor rollout.
# Local runtime: bash + coreutils + kubectl. JSON validation runs in the worker's
# pinned Node runtime; neither the receipt nor its environment is copied wholesale.
# Usage: capture-compute-worker-grant.sh <statefulset> <namespace> [output-root]
# Public inputs: COMPUTE_WORKER_ENABLED, COMPUTE_WORKER_GRANT_ENABLED,
# COMPUTE_WORKER_PROVIDER, COMPUTE_WORKER_RECIPIENT, COMPUTE_WORKER_IMAGE,
# COMPUTE_WORKER_CPU_MILLIS, COMPUTE_WORKER_MEMORY_BYTES (declared slice).
# COMPUTE_WORKER_STARTUP_SECS defaults to the private mesh's 900-second startup
# budget; overrides may shorten it, never exceed it. Disabled workers spend none.
set -euo pipefail
umask 077
workload="${1:?statefulset required}"
namespace="${2:?namespace required}"
output="${3:-compute-worker-evidence}"
[[ "$workload" =~ ^[a-z0-9][a-z0-9-]*$ && "$namespace" =~ ^[a-z0-9][a-z0-9-]*$ ]] || exit 65
directory="$output/$namespace--$workload"
mkdir -p "$directory"
rm -f "$directory/activation.json" "$directory/status.txt"
if [[ "${COMPUTE_WORKER_ENABLED:-false}" != true ]]; then
  echo 'compute worker receipt: SKIPPED (worker disabled)'
  exit 0
fi
refuse() {
  printf 'REFUSED compute worker receipt: %s\n' "$1" | tee "$directory/status.txt" >&2
  exit 1
}
provider="${COMPUTE_WORKER_PROVIDER:-}"
recipient="${COMPUTE_WORKER_RECIPIENT:-}"
image="${COMPUTE_WORKER_IMAGE:-}"
cpu="${COMPUTE_WORKER_CPU_MILLIS:-}"
memory="${COMPUTE_WORKER_MEMORY_BYTES:-}"
budget="${COMPUTE_WORKER_STARTUP_SECS:-900}"
[[ "${COMPUTE_WORKER_GRANT_ENABLED:-false}" == true ]] || refuse 'enabled worker has no enabled runner grant declaration'
[[ "$provider" =~ ^uhCAk[A-Za-z0-9_-]{48}$ && "$recipient" =~ ^uhCAk[A-Za-z0-9_-]{48}$ && "$provider" != "$recipient" ]] || refuse 'approved provider/recipient binding is invalid'
[[ "$image" =~ ^[a-zA-Z0-9./:_-]+@sha256:[a-f0-9]{64}$ ]] || refuse 'worker image must be digest pinned'
[[ "$cpu" =~ ^[1-9][0-9]{0,15}$ && "$memory" =~ ^[1-9][0-9]{0,15}$ ]] || refuse 'declared worker slice must have finite positive CPU and memory bounds'
[[ "$budget" =~ ^[1-9][0-9]{0,2}$ && "$budget" -le 900 ]] || refuse 'startup budget must be 1..900 seconds'
pod="$workload-0"
pending="$(mktemp "$directory/.activation.XXXXXX")"
trap 'rm -f "$pending"' EXIT
deadline=$((SECONDS + budget))
last='worker not ready'
remaining() { left=$((deadline - SECONDS)); ((left > 0)); }
pod_state() {
  remaining || return 1
  timeout --signal=KILL "${left}s" kubectl --request-timeout="${left}s" \
    get pod "$pod" -n "$namespace" -o 'jsonpath={.metadata.uid}{"|"}{.metadata.deletionTimestamp}{"|"}{.status.phase}{"|"}{.spec.containers[?(@.name=="compute-worker")].image}{"|"}{.status.containerStatuses[?(@.name=="compute-worker")].ready}{"|"}{.status.containerStatuses[?(@.name=="compute-worker")].containerID}{"|"}{.status.containerStatuses[?(@.name=="compute-worker")].restartCount}' 2>/dev/null
}
while remaining; do
  state="$(pod_state)" || state=''
  IFS='|' read -r uid deleting phase actual_image ready container_id restarts <<< "$state"
  if [[ "$uid" =~ ^[a-f0-9-]+$ && -z "$deleting" && "$phase" == Running && "$ready" == true && "$actual_image" == "$image" && "$container_id" =~ ^[a-z0-9-]+://[a-zA-Z0-9._-]+$ && "$restarts" =~ ^[0-9]+$ ]]; then
    remaining || break
    # PID 1 becomes worker.mjs only AFTER the current bootstrap writes its grant.
    # This prevents a retained receipt from greening a still-starting container.
    if timeout --signal=KILL "${left}s" kubectl --request-timeout="${left}s" \
      exec -i "$pod" -n "$namespace" -c compute-worker -- /usr/local/bin/node - \
      "$provider" "$recipient" "$image" "$namespace" "$pod" "$uid" "$container_id" "$restarts" "$cpu" "$memory" \
      > "$pending" 2>/dev/null <<'NODE'
const fs = require('node:fs');
const pathModule = require('node:path');
const [provider, recipient, image, namespace, pod, podUid, containerId, restartCount, declaredCpu, declaredMemory] = process.argv.slice(2);
const invalid = () => { process.exitCode = 65; };
// Observation mirrors rakia-executor::runtime::capacity: resolve the visible
// cgroup-v2 path (mount fallback for namespaced paths), then take finite minima
// through every visible ancestor. This is not a placement/admission decision.
function observeCapacity() {
  if (![declaredCpu, declaredMemory].every(value => Number.isSafeInteger(Number(value)))) throw new Error('invalid declared slice');
  const max = (1n << 64n) - 1n;
  const number = value => {
    if (!/^[0-9]+$/.test(value) || BigInt(value) > max) throw new Error('invalid cgroup number');
    return BigInt(value);
  };
  const read = file => { try { return fs.readFileSync(file, 'utf8').trim(); } catch { return null; } };
  const relative = read('/proc/self/cgroup')?.split('\n').find(line => line.startsWith('0::'))?.slice(3);
  if (relative === undefined) throw new Error('cgroup v2 required');
  const mount = '/sys/fs/cgroup';
  let directory = mount + '/' + relative.replace(/^\/+/, '');
  try { if (relative.split('/').includes('..') || !fs.statSync(directory).isDirectory()) directory = mount; }
  catch { directory = mount; }
  directory = directory.replace(/\/+$/, '');
  let cpu = max, memory = max;
  const hierarchy = [];
  for (;;) {
    const raw = { path: directory, cpuMax: read(directory + '/cpu.max'),
      memoryMax: read(directory + '/memory.max'), memoryCurrent: read(directory + '/memory.current'),
      memoryPeak: read(directory + '/memory.peak') };
    hierarchy.push(raw);
    const words = raw.cpuMax?.split(/\s+/);
    if (words?.length === 2 && words[0] !== 'max') {
      const quota = number(words[0]), period = number(words[1]);
      if (period === 0n) throw new Error('invalid cgroup period');
      const scaled = quota * 1000n > max ? max : quota * 1000n;
      const millis = (scaled + period - 1n) / period;
      if (millis < cpu) cpu = millis;
    }
    if (raw.memoryMax !== null && raw.memoryMax !== 'max') {
      const bytes = number(raw.memoryMax);
      if (bytes < memory) memory = bytes;
    }
    for (const value of [raw.memoryCurrent, raw.memoryPeak]) if (value !== null) number(value);
    if (directory === mount) break;
    directory = pathModule.dirname(directory);
  }
  if (cpu === max || memory === max || cpu === 0n || memory === 0n ||
      cpu > BigInt(declaredCpu) || memory > BigInt(declaredMemory)) throw new Error('cgroup slice not enforced');
  return { version: 2, effectiveCpuMillis: Number(cpu), effectiveMemoryBytes: Number(memory),
    declaredCpuMillis: Number(declaredCpu), declaredMemoryBytes: Number(declaredMemory), hierarchy,
    measurement: 'activation snapshot; memory.peak is cgroup lifetime peak, not stage peak' };
}
try {
  const command = fs.readFileSync('/proc/1/cmdline', 'utf8').split('\0');
  if (command[1] !== '/opt/compute/worker.mjs') process.exit(75);
  const declaration = JSON.parse(process.env.COMPUTE_RUNNER_GRANT_JSON || 'null');
  if (declaration?.enabled !== true || declaration.provider !== provider ||
      declaration.recipient !== recipient || declaration.stewardOfRecord !== 'human:matthew' ||
      declaration.fixtureProvider !== 'adam' || process.env.COMPUTE_PERFORMER !== provider ||
      process.env.COMPUTE_RUNTIME_IMAGE !== image.split('@')[1]) process.exit(65);
  const path = '/var/lib/compute/runner-grant.json';
  const stat = fs.lstatSync(path);
  if (!stat.isFile() || stat.size > 16384) process.exit(65);
  const receipt = JSON.parse(fs.readFileSync(path, 'utf8'));
  if (receipt.declarationProvenance !== 'operator-approved-config' ||
      receipt.stewardOfRecord !== 'human:matthew' || receipt.fixtureProvider !== 'adam' ||
      receipt.provider !== provider || receipt.recipient !== recipient || receipt.state !== 'active' ||
      !/^uhCEk[A-Za-z0-9_-]{48}$/.test(receipt.grantCid || '') ||
      !/^uhCkk[A-Za-z0-9_-]{48}$/.test(receipt.grantActionHash || '') ||
      !/^uhCkk[A-Za-z0-9_-]{48}$/.test(receipt.consentActionHash || '')) process.exit(65);
  const capacity = observeCapacity();
  // Whitelist public evidence; never spread receipt/config/env into the artifact.
  console.log(JSON.stringify({
    observedAt: new Date().toISOString(),
    declarationProvenance: receipt.declarationProvenance,
    stewardOfRecord: receipt.stewardOfRecord, fixtureProvider: receipt.fixtureProvider,
    provider: receipt.provider, recipient: receipt.recipient, state: receipt.state,
    grantCid: receipt.grantCid, grantActionHash: receipt.grantActionHash,
    consentActionHash: receipt.consentActionHash,
    worker: { state: 'running', command: '/opt/compute/worker.mjs', image,
      namespace, pod, podUid, containerId, restartCount: Number(restartCount) },
    capacity,
  }, null, 2));
} catch (error) {
  if (error.code === 'ENOENT') process.exitCode = 75;
  else invalid();
}
NODE
    then
      after="$(pod_state)" || after=''
      if [[ "$after" == "$state" && -s "$pending" ]]; then
        mv "$pending" "$directory/activation.json"
        printf 'compute worker receipt: READY (%s/%s); public grant artifact %s/activation.json\n' "$namespace" "$pod" "$directory" | tee "$directory/status.txt"
        exit 0
      fi
      last='worker incarnation changed during capture'
    else
      status=$?
      [[ "$status" != 65 ]] || refuse 'public receipt, running worker, or enforced capacity differs from approved grant/image binding and slice'
      last='worker bootstrap or grant receipt unavailable'
    fi
  else
    last='worker missing, not Ready, terminating, or image differs from declaration'
  fi
  remaining || break
  sleep "$((left < 5 ? left : 5))"
done
refuse "startup budget ${budget}s exhausted: $last"
