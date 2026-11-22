// The rendered supervisor's process tests (E11): they spawn the rendered bash supervisor and read
// /proc/<pid>/stat of its children, run the host's flock, grep and sleep, and bound termination by
// wall clock, so they run in orchestrator-tree, not in the named half whose green is reused.
import test from "node:test";
import { spawn } from "node:child_process";
import { once } from "node:events";
import { setTimeout as sleep } from "node:timers/promises";
import assert from "node:assert/strict";
import { readFile, rm, writeFile } from "node:fs/promises";
import { join } from "node:path";
import YAML from "yaml";
import { renderWorker } from "./scripts/render-compute-worker.mjs";
import { writeFittingFixtureRepo } from "./compute-worker-fixture.mjs";

const source = await readFile(
  new URL("./manifests/humans/adam-firstman.yaml", import.meta.url),
  "utf8",
);

for (const signal of ["SIGTERM", "SIGINT"]) {
  test(
    `rendered supervisor retries failures, releases the lock and cleans children on ${signal}`,
    { timeout: 8000 },
    async (t) => {
      const repoRoot = await writeFittingFixtureRepo();
      const worker = YAML.parseAllDocuments(
        await renderWorker(
          source,
          {
            enabled: true,
            image: `registry/worker@sha256:${"a".repeat(64)}`,
            performer: "adam-key",
            slice: { cpuMillis: 500, memoryBytes: 512 * 1024 * 1024 },
          },
          { repoRoot },
        ),
      )
        .map((doc) => doc.toJS())
        .find((doc) => doc?.kind === "StatefulSet")
        .spec.template.spec.containers.find(
          (container) => container.name === "compute-worker",
        );
      const stub = join(repoRoot, "worker-start.sh");
      await writeFile(
        stub,
        `#!/usr/bin/env bash
state="$COMPUTE_STATE_ROOT"
exec 9>"$state/worker.lock"
flock -n 9 || exit 98
echo "$$" >> "$state/attempts"
bash -c 'trap "" TERM INT; echo "$$" >> "$COMPUTE_STATE_ROOT/children"; exec sleep 60' &
child=$!
until [[ -f "$state/children" ]] && grep -qx "$child" "$state/children"; do sleep 0.01; done
if [[ -f "$state/retried" ]]; then
  wait "$child"
else
  touch "$state/retried"
  exit 1
fi
`,
      );
      const supervisor = spawn(
        worker.command[0],
        [
          worker.command[1],
          worker.command[2].replaceAll(
            "/opt/compute/worker-start.sh",
            '"$COMPUTE_TEST_WORKER"',
          ),
        ],
        {
          env: {
            ...process.env,
            COMPUTE_TEST_WORKER: stub,
            COMPUTE_STATE_ROOT: repoRoot,
            COMPUTE_SUPERVISOR_BACKOFF_START: "1",
            COMPUTE_SUPERVISOR_BACKOFF_CAP: "2",
            COMPUTE_SUPERVISOR_STOP_GRACE: "0.1",
          },
          stdio: ["ignore", "ignore", "pipe"],
        },
      );
      const closed = once(supervisor, "close");
      let stderr = "";
      supervisor.stderr.on("data", (chunk) => {
        stderr += chunk;
      });
      const pids = async (name) => {
        try {
          return (await readFile(join(repoRoot, name), "utf8"))
            .trim()
            .split("\n")
            .map(Number);
        } catch (error) {
          if (error.code === "ENOENT") return [];
          throw error;
        }
      };
      const running = async (pid) => {
        try {
          const stat = await readFile(`/proc/${pid}/stat`, "utf8");
          // Outside a PID namespace, the host's PID 1 owns orphan reaping.
          return !["Z", "X"].includes(
            stat.slice(stat.lastIndexOf(")") + 2).split(" ")[0],
          );
        } catch (error) {
          // A process that exits between open and read answers ESRCH.
          if (["ENOENT", "ESRCH"].includes(error.code)) return false;
          throw error;
        }
      };
      t.after(async () => {
        supervisor.kill("SIGKILL");
        for (const pid of await pids("attempts")) {
          try {
            process.kill(-pid, "SIGKILL");
          } catch (error) {
            if (error.code !== "ESRCH") throw error;
          }
        }
        await closed;
        await rm(repoRoot, { recursive: true, force: true });
      });
      const deadline = Date.now() + 5000;
      while ((await pids("children")).length < 2 && Date.now() < deadline)
        await sleep(20);
      const attempts = await pids("attempts");
      const children = await pids("children");
      assert.equal(attempts.length, 2, stderr);
      assert.equal(children.length, 2, stderr);
      assert.equal(
        await running(children[0]),
        false,
        "failed attempt's child was killed before retry",
      );
      const status = await readFile(
        join(repoRoot, "supervisor-status.txt"),
        "utf8",
      );
      assert.match(
        status,
        /^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ exit=1 attempt=1\n$/,
      );
      assert.ok(stderr.includes(status));
      const stoppedAt = Date.now();
      supervisor.kill(signal);
      const [code, exitSignal] = await closed;
      assert.equal(code, 0, stderr);
      assert.equal(exitSignal, null);
      assert.ok(
        Date.now() - stoppedAt < 1500,
        "termination must not wait for backoff or child exit",
      );
      // The supervisor's last act on an attempt is a SIGKILL to its process group, and it waits
      // only for the entrypoint, so a killed descendant can still be read as running for a moment
      // after the supervisor has exited. What is owed is that none survives: each gets a bounded
      // moment to be gone. Read at once, this was one red in a second device's gate on 2026-11-22
      // (backlog steward-device-arrival-open-items row 78, which also names what the supervisor
      // itself does not yet confirm).
      const gone = async (pid) => {
        const until = Date.now() + 1000;
        while (await running(pid)) {
          if (Date.now() > until) return false;
          await sleep(10);
        }
        return true;
      };
      for (const pid of [...attempts, ...children])
        assert.equal(await gone(pid), true, `surviving child ${pid}`);
      const lock = spawn("flock", [
        "-n",
        join(repoRoot, "worker.lock"),
        "true",
      ]);
      assert.deepEqual(await once(lock, "close"), [0, null]);
    },
  );
}
