"""The pinned embedding fold procedure for the recall executor's semantic route.

Governed-discovery station 4, task 4.2. The model manifest
(.epr-meta/elohim/algorithms/embedding-models/all-minilm-l6-v2.json) pins THIS file's bytes as
its `procedure`, so any edit here is a new method: re-pin the manifest in the same change.

Contract (one request, one reply, nothing else):

  stdin   ONE JSON object: {"model_dir": <dir>, "pin": {"model_bytes": <cid>,
          "tokenizer_bytes": <cid>}, "texts": [<str>, ...]}
  stdout  {"dims": 384, "vectors": [[<float>, ...], ...]} -- one unit-length vector per text,
          in order; floats rounded to 6 significant digits.
  exit 0  success
  exit 2  a bad request, or `unavailable: <reason>` on stderr when a module cannot be imported
  exit 3  model.onnx or tokenizer.json in model_dir does not hash to the pin (one-line reason)

The pin is checked BEFORE any third-party module is imported or any model byte is interpreted.
Standard library + onnxruntime + tokenizers + numpy only. No network, no writes: the model is
read in place and the reply goes to stdout.

Method (sentence-transformers/all-MiniLM-L6-v2): batches of 32 texts, truncation at 256 tokens,
padding to the longest text in the batch, mean pooling of `last_hidden_state` over the attention
mask, L2 normalisation.
"""

import base64
import hashlib
import json
import os
import sys

BATCH = 32
MAX_TOKENS = 256
DIMS = 384
MODEL_FILE = "model.onnx"
TOKENIZER_FILE = "tokenizer.json"

EXIT_BAD_REQUEST = 2
EXIT_PIN_MISMATCH = 3


def refuse(code, reason):
    sys.stderr.write(reason.replace("\n", " ") + "\n")
    sys.exit(code)


def raw_cid(path):
    """The CIDv1 string of a file's bytes under the raw codec and sha256.

    This DECODES a known encoding for comparison with the pin; it does not mint identity (the
    pin was minted by the canonical codec, eprfs-core's BlobCid::compute_raw). Byte layout:

        0x01        CID version 1
        0x55        multicodec `raw`
        0x12 0x20   multihash sha2-256, 32-byte digest
        <32 bytes>  sha256 of the file

    rendered as multibase base32 lower-case: the prefix `b`, then RFC 4648 base32 of those 36
    bytes, lower-cased, without `=` padding.
    """
    digest = hashlib.sha256()
    with open(path, "rb") as handle:
        for block in iter(lambda: handle.read(1 << 20), b""):
            digest.update(block)
    binary = bytes([0x01, 0x55, 0x12, 0x20]) + digest.digest()
    return "b" + base64.b32encode(binary).decode("ascii").lower().rstrip("=")


def read_request():
    try:
        request = json.loads(sys.stdin.read())
    except ValueError as error:
        refuse(EXIT_BAD_REQUEST, "bad request: stdin is not one JSON object ({})".format(error))
    if not isinstance(request, dict):
        refuse(EXIT_BAD_REQUEST, "bad request: stdin is not one JSON object")
    model_dir = request.get("model_dir")
    pin = request.get("pin")
    texts = request.get("texts")
    if not isinstance(model_dir, str) or not model_dir:
        refuse(EXIT_BAD_REQUEST, "bad request: model_dir must be a non-empty string")
    if not isinstance(pin, dict) or not all(
        isinstance(pin.get(key), str) for key in ("model_bytes", "tokenizer_bytes")
    ):
        refuse(EXIT_BAD_REQUEST, "bad request: pin must name model_bytes and tokenizer_bytes")
    if not isinstance(texts, list) or not all(isinstance(text, str) for text in texts):
        refuse(EXIT_BAD_REQUEST, "bad request: texts must be a list of strings")
    return model_dir, pin, texts


def verify_pin(model_dir, pin):
    for name, key in ((MODEL_FILE, "model_bytes"), (TOKENIZER_FILE, "tokenizer_bytes")):
        path = os.path.join(model_dir, name)
        try:
            actual = raw_cid(path)
        except OSError as error:
            refuse(EXIT_PIN_MISMATCH, "{} unreadable in {}: {}".format(name, model_dir, error))
        if actual != pin[key]:
            refuse(
                EXIT_PIN_MISMATCH,
                "{} hashes to {}, the pin is {}".format(name, actual, pin[key]),
            )


def import_runtime():
    modules = []
    for name in ("numpy", "onnxruntime", "tokenizers"):
        try:
            modules.append(__import__(name))
        except ImportError:
            refuse(EXIT_BAD_REQUEST, "unavailable: python module '{}' is not importable".format(name))
    return modules


def embed(model_dir, texts):
    np, ort, tokenizers = import_runtime()
    tokenizer = tokenizers.Tokenizer.from_file(os.path.join(model_dir, TOKENIZER_FILE))
    tokenizer.enable_truncation(max_length=MAX_TOKENS)
    tokenizer.enable_padding(pad_id=0, pad_token="[PAD]")
    options = ort.SessionOptions()
    options.log_severity_level = 3
    session = ort.InferenceSession(
        os.path.join(model_dir, MODEL_FILE), options, providers=["CPUExecutionProvider"]
    )
    vectors = []
    for start in range(0, len(texts), BATCH):
        encoded = tokenizer.encode_batch(texts[start : start + BATCH])
        ids = np.array([e.ids for e in encoded], dtype=np.int64)
        mask = np.array([e.attention_mask for e in encoded], dtype=np.int64)
        hidden = session.run(
            ["last_hidden_state"],
            {"input_ids": ids, "attention_mask": mask, "token_type_ids": np.zeros_like(ids)},
        )[0]
        weights = mask[:, :, None].astype(np.float32)
        pooled = (hidden * weights).sum(axis=1) / np.clip(weights.sum(axis=1), 1e-9, None)
        norms = np.linalg.norm(pooled, axis=1, keepdims=True)
        normalised = pooled / np.clip(norms, 1e-12, None)
        for row in normalised:
            vectors.append([float("{:.6g}".format(value)) for value in row.tolist()])
    return vectors


def main():
    model_dir, pin, texts = read_request()
    verify_pin(model_dir, pin)
    vectors = embed(model_dir, texts) if texts else []
    sys.stdout.write(json.dumps({"dims": DIMS, "vectors": vectors}, separators=(",", ":")))
    sys.stdout.write("\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
