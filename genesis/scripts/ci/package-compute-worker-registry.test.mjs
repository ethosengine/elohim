import test from "node:test";
import assert from "node:assert/strict";
import {
  WORKER_REPOSITORY,
  JSON_LIMIT,
  digestBytes,
  exporterDigest,
  registryImage,
  registryBlob,
} from "./package-compute-worker-registry.mjs";

const labels = { "org.opencontainers.image.revision": "a".repeat(40) };
function fixture(change = () => {}) {
  const config = {
    architecture: "amd64",
    os: "linux",
    config: {
      Labels: { ...labels, private: "PRIVATE_CONFIG" },
      Env: ["SECRET=PRIVATE_CONFIG"],
    },
  };
  const manifest = {
    schemaVersion: 2,
    mediaType: "application/vnd.oci.image.manifest.v1+json",
    layers: [],
    config: { mediaType: "application/vnd.oci.image.config.v1+json" },
  };
  change(config, manifest);
  const configBytes = Buffer.from(JSON.stringify(config) + "\n");
  manifest.config.digest = digestBytes(configBytes);
  manifest.config.size = configBytes.length;
  const bytes = Buffer.from(JSON.stringify(manifest) + "\n");
  const digest = digestBytes(bytes);
  const calls = [];
  const read = (args) => {
    calls.push(args);
    assert.equal(
      args.at(-1),
      `${WORKER_REPOSITORY}@${args[0] === "manifest" ? digest : manifest.config.digest}`,
    );
    return args[0] === "manifest" ? bytes : configBytes;
  };
  return { configBytes, manifest, bytes, digest, read, calls };
}
test("registry identity binds exact untrimmed manifest/config bytes and exposes only selected labels", () => {
  const f = fixture();
  const result = registryImage(f.read, f.digest, labels);
  assert.equal(result.image, `${WORKER_REPOSITORY}@${f.digest}`);
  assert.equal(result.evidence.manifestBytes, f.bytes.length);
  assert.equal(result.evidence.configBytes, f.configBytes.length);
  assert.deepEqual(result.evidence.labels, labels);
  assert(!JSON.stringify(result).includes("PRIVATE_CONFIG"));
  assert.deepEqual(
    f.calls.map((a) => a.slice(0, 2)),
    [
      ["manifest", "fetch"],
      ["blob", "fetch"],
    ],
  );
});
test("refuses changed manifest or config bytes even when the parsed labels agree", () => {
  for (const target of ["manifest", "blob"]) {
    const f = fixture();
    assert.throws(() =>
      registryImage(
        (args) => {
          const bytes = f.read(args);
          return args[0] === target
            ? Buffer.concat([bytes, Buffer.from(" ")])
            : bytes;
        },
        f.digest,
        labels,
      ),
    );
  }
});
test("refuses non-image manifests, wrong labels/platform and invalid layer descriptors", () => {
  for (const change of [
    (_, m) => {
      m.mediaType = "application/vnd.oci.image.index.v1+json";
    },
    (_, m) => {
      m.schemaVersion = 1;
    },
    (_, m) => {
      m.config.mediaType = "application/octet-stream";
    },
    (c) => {
      c.architecture = "arm64";
    },
    (c) => {
      c.os = "windows";
    },
    (c) => {
      c.config.Labels[Object.keys(labels)[0]] = "b".repeat(40);
    },
    (_, m) => {
      m.layers = [{ digest: "sha256:" + "a".repeat(64), size: -1 }];
    },
  ]) {
    const f = fixture(change);
    assert.throws(() => registryImage(f.read, f.digest, labels));
  }
});
test("refuses body bounds and config descriptor sizes before reading a blob", () => {
  const f = fixture();
  assert.throws(() =>
    registryImage(() => Buffer.alloc(JSON_LIMIT + 1), f.digest, labels),
  );
  assert.throws(() =>
    registryBlob(() => assert.fail("must not read"), {
      digest: f.digest,
      size: JSON_LIMIT + 1,
    }),
  );
  assert.throws(() =>
    registryBlob(() => Buffer.from("a"), { digest: f.digest, size: 2 }),
  );
});
test("metadata requires an actual canonical digest and never accepts an absent success", () => {
  const digest = "sha256:" + "a".repeat(64);
  assert.equal(
    exporterDigest(
      Buffer.from(JSON.stringify({ "containerimage.digest": digest })),
    ),
    digest,
  );
  for (const value of [
    null,
    {},
    { "containerimage.digest": [digest] },
    { "containerimage.digest": "latest" },
    { "containerimage.digest": "sha256:" + "A".repeat(64) },
  ])
    assert.throws(() => exporterDigest(Buffer.from(JSON.stringify(value))));
  assert.throws(() => exporterDigest(Buffer.alloc(JSON_LIMIT + 1)));
});
