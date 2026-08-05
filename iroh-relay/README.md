# iroh-relay — packaging for our own peer-connectivity relay server

This directory holds no Rust source — it is only a Docker build context
(`Dockerfile`) that `cargo install`s the unmodified, published `iroh-relay`
crate from crates.io (the relay component of the third-party `iroh` p2p
networking stack) and packages it as a container image. The relay helps two
peers reach each other when they can't connect directly (NAT/firewall); it
forwards their traffic (which stays end-to-end encrypted) rather than joining
or reading it. We run our own relay instance per premise (each self-hosted
deployment site) rather than depend on someone else's, so peer connectivity
never depends on infrastructure outside this project's control.

This image is consumed by "doorway" (this project's gateway service — see
`doorway/`), which is deployed alongside a relay pod per premise. CI (the
edge pipeline: any push touching `iroh-relay/**` runs
`elohim/holochain/Jenkinsfile`'s Build stage, which calls
`scripts/ci/build-iroh-relay.sh` then `scripts/ci/push-iroh-relay.sh`) builds
this image and pushes it to `harbor.ethosengine.com/ethosengine/iroh-relay` —
there is no local run/build step for a developer to invoke here.

Keep this unmodified upstream ("stock" = no local patches, no fork). If a
behavior change needs more than a version bump, that's an upstream
contribution to the `iroh-relay` crate, not an edit in this directory.

**Bumping the pinned version:** edit the `ARG IROH_RELAY_VERSION=x.y.z` line
in `./Dockerfile`. No other file needs to change for the bump itself — there
is no compatibility constraint pinning this crate's version to any other
component's today (see the design doc below for why the current version was
chosen against the conductor's — the Holochain node runtime's — own embedded
iroh generation; that reasoning is the kind of constraint a future bump
should re-check). Commit the change; the edge pipeline above builds and
pushes it automatically. Verify by checking for a new
`harbor.ethosengine.com/ethosengine/iroh-relay:<version>-<branch>-latest`
tag (e.g. `0.95.1-dev-latest` on `dev`) in Harbor after the pipeline
completes.

Full rationale (why "sovereignty" over the relay matters), the version
contract, and deployment details:
`genesis/docs/content/elohim-protocol/architecture/2026-08-05-wave2-relay-sovereignty-design.md`
(path from repo root).
