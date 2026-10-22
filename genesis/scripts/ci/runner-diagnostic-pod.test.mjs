import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import YAML from "yaml";

const pipeline = readFileSync(
  new URL("../../Jenkinsfile", import.meta.url),
  "utf8",
);
const pod = YAML.parse(
  readFileSync(
    new URL("./runner-diagnostic-pod.yaml", import.meta.url),
    "utf8",
  ),
);
function helper(name) {
  const start = pipeline.indexOf(`def ${name}(`);
  assert(start >= 0);
  const end = pipeline.indexOf("\ndef ", start + 1);
  return pipeline.slice(start, end < 0 ? undefined : end);
}
test("only the scoped diagnostic pod uses existing deployer authority", () => {
  const main = pipeline.slice(pipeline.indexOf("\npipeline {"));
  assert.match(main, /serviceAccount: ee-jenkins/);
  assert.equal(pod.spec.serviceAccountName, "jenkins-deployer");
  assert.deepEqual(
    pod.spec.containers.map((c) => c.name),
    ["builder"],
  );
  assert.equal(
    pod.spec.containers[0].image,
    "harbor.ethosengine.com/ethosengine/ci-builder:latest",
  );
  assert.equal(pod.spec.volumes, undefined);
  const scoped = helper("withAdamDiagnosticPod");
  assert.match(
    scoped,
    /podTemplate\(cloud: 'kubernetes', inheritFrom: '', namespace: 'jenkins', yaml: podYaml\)/,
  );
  assert.match(scoped, /node\(POD_LABEL\)/);
  assert.match(
    scoped,
    /withEnv\(\['ADMIN_KEY=', 'RUNNER_JWT=', 'KUBECONFIG='\]\)/,
  );
  assert(!/stash|unstash|withCredentials/.test(scoped));
});
test("child checkout is compared to parent HEAD before any privileged work", () => {
  const scoped = helper("withAdamDiagnosticPod");
  const parent = scoped.indexOf(
    "def expectedHead = sh(script: 'git rev-parse HEAD'",
  );
  const node = scoped.indexOf("node(POD_LABEL)");
  const safe = scoped.indexOf("safe.directory");
  const checkout = scoped.indexOf("checkout scm");
  const comparison = scoped.indexOf("if (actualHead != expectedHead)");
  assert(
    parent >= 0 &&
      parent < node &&
      node < safe &&
      safe < checkout &&
      checkout < comparison,
  );
  assert(comparison < scoped.indexOf("body()"));
  assert.match(scoped, /if \(actualHead != expectedHead\) \{ error\(/);
});
test("authority proof installs existing dependencies and binds its credential only in child", () => {
  const authority = helper("runAdamAuthorityProof");
  const child = authority.indexOf("withAdamDiagnosticPod {");
  const deps = authority.indexOf("runner-prepare-authority.sh");
  const credential = authority.indexOf("withCredentials(");
  assert(child >= 0 && child < deps && deps < credential);
  assert.match(authority, /RUN_ADAM_AUTHORITY_PROBE=true/);
  assert.match(authority, /timeout\(time: 15, unit: 'MINUTES'\)/);
  assert.match(
    authority,
    /finally \{\s*archiveArtifacts\(artifacts: 'adam-authority-activation.json,adam-authority-status.txt,rollout-evidence\/\*\*'/,
  );
  assert.match(
    authority,
    /artifacts: 'adam-authority-probe.json,adam-authority-activation.json', allowEmptyArchive: false/,
  );
  const call = pipeline.slice(
    pipeline.indexOf("def authorityMessage ="),
    pipeline.indexOf("// The CI runner's standing"),
  );
  assert.match(
    call,
    /contains\('\[build:genesis\]'\).*contains\('\[adam-authority:probe\]'\)/,
  );
  assert.match(call, /runAdamAuthorityProof\(\)/);
});
test("submission remains on parent and poststage capture follows credential closure", () => {
  const e2e = pipeline.slice(
    pipeline.indexOf("if (peerStageFeature) {"),
    pipeline.indexOf("// Always run API-level tests"),
  );
  assert.match(e2e, /runner-stage\.mjs/);
  assert(!e2e.includes("withAdamDiagnosticPod"));
  // Close timeout, withEnv and withCredentials before entering the capture helper.
  assert.match(
    e2e,
    /runner-stage\.mjs'"\s*\}\s*\}\s*\}\s*captureAdamAfterPeerStage\(peerStageConfig\)/,
  );
  const capture = helper("captureAdamAfterPeerStage");
  assert.match(capture, /withAdamDiagnosticPod \{/);
  assert(
    !/withCredentials|install-substrate|runner-prepare|pnpm/.test(capture),
  );
  assert.match(
    capture,
    /artifacts: 'compute-worker-after-stage\/\*\*\/activation.json', allowEmptyArchive: false/,
  );
  assert.match(
    capture,
    /finally \{\s*archiveArtifacts\(artifacts: 'compute-worker-after-stage\/\*\*\/status.txt,rollout-evidence\/\*\*'/,
  );
});

// Run the real preparation shell against isolated fake package managers. No network.
import {
  mkdtempSync,
  mkdirSync,
  writeFileSync,
  symlinkSync,
  rmSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";
for (const mode of ["present", "missing", "install-failed"]) {
  test(`fresh authority pod preparation: jq ${mode}`, (t) => {
    const root = mkdtempSync(join(tmpdir(), "runner-prepare-"));
    t.after(() => rmSync(root, { recursive: true, force: true }));
    const bin = join(root, "bin"),
      scripts = join(root, "scripts");
    mkdirSync(bin);
    mkdirSync(scripts);
    symlinkSync("/bin/bash", join(bin, "bash"));
    symlinkSync("/usr/bin/dirname", join(bin, "dirname"));
    const prep = readFileSync(
      new URL("./runner-prepare-authority.sh", import.meta.url),
      "utf8",
    );
    writeFileSync(join(scripts, "prepare.sh"), prep);
    writeFileSync(
      join(scripts, "install-substrate-runner.sh"),
      'printf "substrate\\n" >> "$TRACE"\n',
    );
    const jq = '#!/bin/bash\nprintf "jq\\n" >> "$TRACE"\n';
    if (mode === "present") writeFileSync(join(bin, "jq"), jq, { mode: 0o755 });
    const apt = `#!${process.execPath}\nconst fs=require('node:fs'); fs.appendFileSync(process.env.TRACE,process.argv.slice(2).join(' ')+'\\n'); if(process.env.MODE==='install-failed')process.exit(1); if(process.argv[2]==='install')fs.writeFileSync(process.env.FAKE_BIN+'/jq',${JSON.stringify(jq)},{mode:0o755});\n`;
    writeFileSync(join(bin, "apt-get"), apt, { mode: 0o755 });
    writeFileSync(join(bin, "apk"), "#!/bin/bash\nexit 1\n", { mode: 0o755 });
    const trace = join(root, "trace");
    writeFileSync(trace, "");
    const result = spawnSync("/bin/bash", [join(scripts, "prepare.sh")], {
      encoding: "utf8",
      env: { PATH: bin, TRACE: trace, FAKE_BIN: bin, MODE: mode },
      timeout: 10000,
    });
    const calls = readFileSync(trace, "utf8").trim().split("\n");
    if (mode === "install-failed") {
      assert.notEqual(result.status, 0);
      assert(!calls.includes("substrate"));
    } else {
      assert.equal(result.status, 0, result.stderr);
      assert.deepEqual(
        calls,
        mode === "present"
          ? ["jq", "substrate"]
          : ["update -qq", "install -y -qq jq", "jq", "substrate"],
      );
    }
  });
}
