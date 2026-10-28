import assert from "node:assert/strict";
import { execFile } from "node:child_process";

export function allowed(method, path) {
  const ref = "uhCkk[A-Za-z0-9_-]{48}";
  return (
    (method === "POST" && path === "/api/v1/compute/grants") ||
    (method === "GET" &&
      new RegExp(`^/api/v1/compute/tasks/${ref}$`).test(path)) ||
    (method === "POST" &&
      new RegExp(
        `^/api/v1/compute/tasks/${ref}/(accept|authorize-launch)$`,
      ).test(path)) ||
    (method === "GET" &&
      new RegExp(`^/api/v1/economic-events/compute-admission:${ref}$`).test(
        path,
      ))
  );
}
// Fixed program, never shell interpolation. Request JSON arrives only through stdin.
export const podProgram = `
const fs = require('node:fs');
const allowed = ${allowed.toString()};
(async () => {
  const raw = fs.readFileSync(0, 'utf8'); if (raw.length > 32768) throw Error();
  const {activation:a, profile, method, path, body} = JSON.parse(raw);
  if (!allowed(method,path)) throw Error();
  const running = pid => {
    try { return fs.readFileSync('/proc/' + pid + '/cmdline','utf8').split('\\0')[1] === '/opt/compute/worker.mjs'; }
    catch (error) { if (error.code === 'ENOENT' || error.code === 'ESRCH') return false; throw error; }
  };
  const candidates = running('1') ? ['1'] : fs.readFileSync('/proc/1/task/1/children','utf8').trim().split(/\\s+/).filter(pid => /^[1-9][0-9]*$/.test(pid));
  const workers = candidates.filter(running);
  if (workers.length !== 1) throw Error();
  const workerRoot = '/proc/' + workers[0];
  const identity = () => {
    const stat = fs.readFileSync(workerRoot + '/stat','utf8');
    return stat.slice(stat.lastIndexOf(')') + 2).split(' ')[19];
  };
  const started = identity();
  const d = JSON.parse(process.env.COMPUTE_RUNNER_GRANT_JSON || 'null');
  const r = JSON.parse(fs.readFileSync('/var/lib/compute/runner-grant.json','utf8'));
  if (d?.enabled !== true ||
      d.provider !== a.provider || d.recipient !== a.recipient ||
      d.fixtureProvider !== 'adam' || d.stewardOfRecord !== 'human:matthew' ||
      process.env.COMPUTE_PERFORMER !== a.provider ||
      process.env.COMPUTE_RUNTIME_IMAGE !== a.worker.image.split('@')[1] ||
      r.grantActionHash !== a.grantActionHash || r.grantCid !== a.grantCid ||
      r.provider !== a.provider || r.recipient !== a.recipient) throw Error();
  if(profile && fs.realpathSync(workerRoot + '/cwd') !== fs.realpathSync(profile.repoRoot)) throw Error();
  const base = new URL(process.env.COMPUTE_API_URL || 'http://127.0.0.1:8090');
  if (base.protocol !== 'http:' || !['localhost','127.0.0.1','[::1]'].includes(base.hostname) || base.username || base.password) throw Error();
  const token = process.env.ELOHIM_COMPUTE_LOCAL_TOKEN || ''; if(token.length < 32) throw Error();
  const read = async response => {
    let bytes=0, chunks=[]; for await (const chunk of response.body) {
      bytes+=chunk.length; if(bytes>262144) throw Error(); chunks.push(chunk);
    } return JSON.parse(Buffer.concat(chunks).toString());
  };
  const health = await fetch(new URL('/health',base),{redirect:'error',signal:AbortSignal.timeout(10000)});
  if(!health.ok || (await read(health)).dhtParticipation?.agentKeys?.lamad !== a.provider) throw Error();
  if(!running(workers[0]) || identity() !== started) throw Error();
  if(path === '/api/v1/compute/grants') {
    if(body.revoke === true) { if(body.grantCid === a.grantCid) throw Error(); }
    else if(body.scope !== 'measure-stage' || body.bounds?.epr_scope?.length !== 1 || body.bounds.epr_scope[0] !== '*') throw Error();
  }
  const response = await fetch(new URL(path,base),{method,redirect:'error',signal:AbortSignal.timeout(30000),
    headers:{'content-type':'application/json','x-elohim-verified-performer':a.provider,'x-elohim-compute-token':token},
    body:body === undefined ? undefined : JSON.stringify(body)});
  let value={}; try {value=await read(response);} catch {if(response.status!==404) throw Error();}
  const output={};
  for(const name of ['grantCid','grantActionHash','consentActionHash','state','provider','recipient','requester','requestActionHash','taskCid','reason'])
    if(typeof value[name]==='string') output[name]=value[name];
  for(const name of ['acceptance','completion','refusal']) if(value[name]) output[name]={exists:true};
  console.log(JSON.stringify({status:response.status,body:output}));
})().catch(()=>{console.error('provider authority transport refused');process.exitCode=1;});
`;

export function runKubectl(args, input) {
  return new Promise((resolve, reject) => {
    const child = execFile(
      "kubectl",
      args,
      { timeout: 60000, killSignal: "SIGKILL", maxBuffer: 1048576 },
      (error, stdout) =>
        error
          ? reject(new Error("provider transport command failed"))
          : resolve(stdout),
    );
    child.stdin.on("error", () => {});
    child.stdin.end(input);
  });
}

export function pinnedPod(pod, activation) {
  const w = activation.worker;
  const status = pod.status?.containerStatuses?.find(
    (c) => c.name === "compute-worker",
  );
  const spec = pod.spec?.containers?.find((c) => c.name === "compute-worker");
  assert(
    pod.metadata?.uid === w.podUid &&
      !pod.metadata.deletionTimestamp &&
      pod.status?.phase === "Running",
  );
  assert(
    status?.ready === true &&
      status.containerID === w.containerId &&
      status.restartCount === w.restartCount,
  );
  assert.equal(spec?.image, w.image);
  // Capture v1 pins the digest-qualified spec image and containerID, not imageID.
  // Pin observed imageID additionally within this transport's lifetime.
  assert(typeof status.imageID === "string" && status.imageID.length > 0);
  return status.imageID;
}

export function providerTransport(activation, execute = runKubectl, profile) {
  const w = activation.worker;
  assert(
    /^[a-z0-9][a-z0-9-]*$/.test(w.namespace) &&
      /^[a-z0-9][a-z0-9-]*$/.test(w.pod),
  );
  assert(w.state === "running" && w.command === "/opt/compute/worker.mjs");
  assert(/@sha256:[0-9a-f]{64}$/.test(w.image));
  let imageID;
  const inspect = async () => {
    const pod = JSON.parse(
      await execute([
        "--request-timeout=15s",
        "get",
        "pod",
        w.pod,
        "-n",
        w.namespace,
        "-o",
        "json",
      ]),
    );
    const observed = pinnedPod(pod, activation);
    imageID ??= observed;
    assert.equal(observed, imageID, "provider imageID changed");
  };
  return async (method, path, body) => {
    assert(allowed(method, path), "provider route outside probe allowlist");
    await inspect();
    const result = await execute(
      [
        "--request-timeout=50s",
        "exec",
        "-i",
        w.pod,
        "-n",
        w.namespace,
        "-c",
        "compute-worker",
        "--",
        "/usr/local/bin/node",
        "-e",
        podProgram,
      ],
      JSON.stringify({ activation, profile, method, path, body }),
    );
    await inspect();
    const parsed = JSON.parse(result);
    assert(
      Number.isInteger(parsed.status) &&
        parsed.body &&
        typeof parsed.body === "object",
    );
    return parsed;
  };
}
