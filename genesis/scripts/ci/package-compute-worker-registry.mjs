// Registry reads for the commissioning packager. No local image store or mounts.
import assert from "node:assert/strict";
import { createHash } from "node:crypto";

export const WORKER_REPOSITORY =
  "harbor.ethosengine.com/ethosengine/elohim-compute-worker";
export const JSON_LIMIT = 1024 * 1024;
const digestPattern = /^sha256:[a-f0-9]{64}$/;
const manifestTypes = new Set([
  "application/vnd.oci.image.manifest.v1+json",
  "application/vnd.docker.distribution.manifest.v2+json",
]);
const configTypes = new Set([
  "application/vnd.oci.image.config.v1+json",
  "application/vnd.docker.container.image.v1+json",
]);
export const digestBytes = (bytes) =>
  "sha256:" + createHash("sha256").update(bytes).digest("hex");
function descriptor(value, maxBytes = Number.MAX_SAFE_INTEGER) {
  assert(
    value &&
      typeof value.digest === "string" &&
      digestPattern.test(value.digest),
  );
  assert(Number.isSafeInteger(value.size) && value.size > 0);
  assert(value.size <= maxBytes);
  return value;
}
function verifiedBytes(bytes, expected, maxBytes) {
  assert(Buffer.isBuffer(bytes) && bytes.length <= maxBytes);
  assert.equal(bytes.length, expected.size);
  assert.equal(digestBytes(bytes), expected.digest);
  return bytes;
}
export function exporterDigest(bytes) {
  assert(Buffer.isBuffer(bytes) && bytes.length <= JSON_LIMIT);
  const metadata = JSON.parse(bytes);
  assert(metadata && !Array.isArray(metadata));
  assert(typeof metadata["containerimage.digest"] === "string");
  assert(digestPattern.test(metadata["containerimage.digest"]));
  return metadata["containerimage.digest"];
}
export function registryBlob(read, value, maxBytes = JSON_LIMIT) {
  descriptor(value, maxBytes);
  return verifiedBytes(
    read([
      "blob",
      "fetch",
      "--output",
      "-",
      `${WORKER_REPOSITORY}@${value.digest}`,
    ]),
    value,
    maxBytes,
  );
}
export function registryImage(read, digest, expectedLabels) {
  assert(typeof digest === "string" && digestPattern.test(digest));
  const image = `${WORKER_REPOSITORY}@${digest}`;
  // Read the immutable reference, never resolve the mutable build tag again.
  const bytes = read(["manifest", "fetch", image]);
  assert(Buffer.isBuffer(bytes) && bytes.length <= JSON_LIMIT);
  assert.equal(digestBytes(bytes), digest);
  const manifest = JSON.parse(bytes);
  assert.equal(manifest.schemaVersion, 2);
  assert(manifestTypes.has(manifest.mediaType));
  assert(configTypes.has(manifest.config?.mediaType));
  assert(Array.isArray(manifest.layers) && manifest.layers.length <= 256);
  for (const layer of manifest.layers) descriptor(layer);
  const config = JSON.parse(registryBlob(read, manifest.config));
  assert.equal(config.os, "linux");
  assert.equal(config.architecture, "amd64");
  for (const [key, value] of Object.entries(expectedLabels))
    assert.equal(config.config?.Labels?.[key], value);
  // Config.Env and arbitrary labels stay in private memory, never in artifacts.
  return {
    image,
    layers: manifest.layers,
    evidence: {
      image,
      manifestBytes: bytes.length,
      configDigest: manifest.config.digest,
      configBytes: manifest.config.size,
      labels: { ...expectedLabels },
      platform: "linux/amd64",
    },
  };
}
