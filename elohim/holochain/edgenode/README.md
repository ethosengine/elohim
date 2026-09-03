# Elohim Edge Node

This directory is the Holochain conductor configuration reference for Elohim edge-node
maintainers. Use it when changing the conductor's discovery endpoints, WebSocket
interfaces, or persistence layout. The active Holochain 0.7 image is built from
`che-devworkspaces/containers/elohim-edgenode/` and promoted through
`elohim/conductor-image/README.md`; this directory is not the production image source.

## Runtime contract

- Deployed Holochain conductor: 0.7.0, using the `elohim-0.7` fork.
- Network transport: iroh only. Every conductor receives an explicit bootstrap URL and
  the relay assigned by that doorway operator.
- Browser client target: `@holochain/client ^0.21.0`. The client dependency moves in
  its own upgrade lane; do not treat an older browser package as a 0.7 compatibility
  test.
- Admin WebSocket: port 4444. App WebSocket: port 4445.
- Persistent conductor data: `/var/local/lib/holochain`, including the in-process
  Lair keystore below `ks/`.

The checked-in `Dockerfile`, `Dockerfile.zombie-fix`, and `docker-compose.yml`
still describe the predecessor image and are not a supported Holochain 0.7 quick
start. Do not combine those image definitions with this 0.7 configuration. They must
be upgraded together in a separately scoped lane before the Compose path is usable.

## Mental model

```text
Admin/App WebSocket client ──> Holochain 0.7 conductor ──> DHT
                                      │
                                      ├── Doorway bootstrap endpoint
                                      ├── same operator's iroh relay endpoint
                                      └── persistent data + in-process keystore
```

Bootstrap discovers peers. The relay provides operator-owned connectivity when peers
cannot connect directly. They are separate endpoints but belong to the same doorway
operator; never substitute the public iroh default.

## First successful local run

Use the repository's local mesh instead of the legacy Compose file. This walkthrough
is specifically for the checked-in Eclipse Che/dev-container environment: Linux
x86_64 GNU with the `/projects/.cargo-target-pool` layout. Prerequisites are `just`,
`curl`, `sha256sum`, the repository's Rust toolchain, and network access to GitHub
Releases plus the configured Cargo registry.

The local smoke deliberately uses Holochain's stock 0.7.0 release pair. The mesh
launcher supports that pair for config-shape and local-flow checks; it is not proof of
the deployed `elohim-0.7` fork. Fork-parity verification uses the source-derived
image and fleet procedure in `elohim/conductor-image/README.md`.

From the repository root, build the two native services into the pool slots the mesh
launcher reads:

```bash
(
  cd doorway/doorway-service
  RUSTFLAGS="" \
    CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/doorway__doorway-service/dev \
    cargo build --bin doorway
)
(
  cd elohim/elohim-storage
  RUSTFLAGS='--cfg getrandom_backend="custom"' \
    CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
    cargo build --features "p2p p2p-iroh" --bin elohim-storage
)
```

Download the matching upstream Holochain 0.7 tools to an isolated directory and verify
both versions before they can create a sandbox:

```bash
HC07_BIN=/absolute/path/to/hc-0.7
mkdir -p "$HC07_BIN"
curl -fL \
  https://github.com/holochain/holochain/releases/download/holochain-0.7.0/holochain-x86_64-unknown-linux-gnu \
  -o "$HC07_BIN/holochain"
curl -fL \
  https://github.com/holochain/holochain/releases/download/holochain-0.7.0/hc-x86_64-unknown-linux-gnu \
  -o "$HC07_BIN/hc"
printf '%s  %s\n' \
  ffa40a0c6fab5ce062c4af76328dfe2de143256ddf791a504d72bca698a9ba20 "$HC07_BIN/holochain" \
  f1eca56b97bc2261324e00e0e86a274f7dc8363f73264e697fd0c0216b2aac23 "$HC07_BIN/hc" \
  | sha256sum --check -
chmod +x "$HC07_BIN/holochain" "$HC07_BIN/hc"
test "$("$HC07_BIN/holochain" --version)" = "holochain 0.7.0"
test "$("$HC07_BIN/hc" --version)" = "hc 0.7.0"
HOLOCHAIN_BIN="$HC07_BIN" just mesh start
just mesh status
```

The SHA-256 values above are the digests published with the official
`holochain-0.7.0` GitHub release. Success means `just mesh status` lists every
configured conductor and storage peer as running. The launcher also refuses a
mismatched `holochain`/`hc` pair. When finished:

```bash
just mesh stop
```

For a deployed image promotion, follow the build, pin, and fleet-verification procedure
in `elohim/conductor-image/README.md`; cluster operations are operator-owned.

## Configuration

`conductor-config.yaml` is the standalone reference surface. Kubernetes manifests
embed the same network shape. Each operator supplies its own paired endpoints:

```yaml
network:
  bootstrap_url: "https://doorway.<operator-domain>/bootstrap"
  relay_url: "https://relay.<operator-domain>"
```

The repository's concrete Elohim-hosted example uses
`https://doorway.elohim.host/bootstrap` with
`https://relay.elohim.host`. Alpha, staging, and other operators use their own
doorway/relay pair. A missing mapping is a render failure; there is no public-relay
fallback.

The `advanced.k2Gossip` values in the checked-in config preserve slow-WAN behavior.
Do not replace or remove them without new runtime evidence.

### Interface safety

Port 4444 is the administrative interface and must not be exposed to untrusted
networks. Keep `allowed_origins` limited to the clients that administer this
conductor. Port 4445 is the app interface used for authenticated zome calls.

## Persistence

The conductor stores its databases and key material under
`/var/local/lib/holochain`. Back up or migrate that data as one identity-bearing
unit; deleting it rekeys the agent and is not a routine upgrade step.

The local mesh uses its own isolated data root under `/tmp/elohim-local-mesh` by
default. It does not mutate deployed conductor state.

## Next action

After the local mesh is healthy, run:

```bash
just mesh prologue
```

That stages the household fixtures and seed chain used by the repository's mesh
scenarios. Then run the focused scenario for the behavior you changed with
`just test mesh [scope]`.
