// CI transport uses the pod's existing projected authority, never a new credential.
import {
  accessSync,
  constants,
  mkdtempSync,
  writeFileSync,
  rmSync,
} from "node:fs";
import { join, delimiter } from "node:path";
import { tmpdir } from "node:os";

export async function withClusterTransport(
  run,
  {
    env = process.env,
    serviceAccountDirectory = "/var/run/secrets/kubernetes.io/serviceaccount",
    temporaryDirectory = tmpdir(),
  } = {},
) {
  // An explicitly selected context remains the caller's authority.
  if (env.KUBECONFIG) {
    const files = env.KUBECONFIG.split(delimiter);
    if (files.some((file) => !file.trim()))
      throw new Error("invalid explicit cluster configuration");
    for (const file of files) accessSync(file, constants.R_OK);
    return await run({ ...env });
  }
  const tokenFile = join(serviceAccountDirectory, "token");
  const certificateAuthority = join(serviceAccountDirectory, "ca.crt");
  // Do not read, copy, print, or place bearer bytes in argv/environment.
  for (const file of [tokenFile, certificateAuthority])
    accessSync(file, constants.R_OK);
  const directory = mkdtempSync(join(temporaryDirectory, "runner-kube-"));
  try {
    const config = join(directory, "config.json");
    writeFileSync(
      config,
      JSON.stringify({
        apiVersion: "v1",
        kind: "Config",
        clusters: [
          {
            name: "in-cluster",
            cluster: {
              server: "https://kubernetes.default.svc",
              "certificate-authority": certificateAuthority,
            },
          },
        ],
        users: [{ name: "projected-service-account", user: { tokenFile } }],
        contexts: [
          {
            name: "runner",
            context: {
              cluster: "in-cluster",
              user: "projected-service-account",
            },
          },
        ],
        "current-context": "runner",
      }),
      { mode: 0o600 },
    );
    return await run({ ...env, KUBECONFIG: config });
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
}
