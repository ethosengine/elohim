# iroh-relay — packaging for our own peer-connectivity relay server

**Scope:** what this directory is and how to bump the pinned relay version.
For how the relay is deployed/operated and why (this is "Elohim Protocol",
the project that owns this repo), see the design doc linked at the bottom.

This directory holds no Rust source — it is only a Docker build context
(`Dockerfile`) that `cargo install`s the unmodified, published `iroh-relay`
crate from crates.io (the relay component of the third-party `iroh` p2p
networking stack) and packages it as a container image. Keep it unmodified
upstream ("stock" = no local patches, no fork) — this isn't enforced by
tooling, only caught at code review — because if a behavior change ever
needs more than a version bump, that's an upstream contribution to the
`iroh-relay` crate, not an edit in this directory.

The relay helps two peers reach each other when they can't connect directly
(NAT/firewall); it forwards their traffic (which stays end-to-end encrypted)
rather than joining or reading it. We run our own relay instance per premise
(each self-hosted deployment site — there is more than one) rather than
depend on someone else's, so peer connectivity never depends on
infrastructure outside this project's control. This image is consumed by
"doorway" (this project's gateway service — see `doorway/`), deployed
alongside a relay pod per premise (see the design doc for the deployment
manifests and how doorway is wired to its relay).

**A version bump is fleet-wide, not per-premise:** every premise's manifests
pull the same shared Harbor tag, so a bump reaches all of them on their next
deploy, not just the one you're thinking about. Keep that in mind reading
the bump procedure below.

## Building it

**Local smoke test** — confirms the pinned version still compiles, without
needing to know or type what that version currently is:

```
docker build -t iroh-relay-test iroh-relay    # run from the repo root
```

This reads the version from the Dockerfile's own `ARG IROH_RELAY_VERSION=…`
default, so there is nothing to fill in. Requires a local Docker (or
compatible OCI builder) daemon and network egress to crates.io — this is an
ordinary `docker build`, no special credentials needed, but if your
environment has neither (the CI path below deliberately uses a different,
CI-only tool), skip straight to pushing and let CI verify. Expect several
minutes (it compiles the crate from source, it doesn't download a binary);
`docker build` exiting `0` is the pass signal. A version that doesn't exist
on crates.io fails fast, during the `cargo install` step, with a "not
found"/no matching package error — it will not hang.

To test a *different* version before committing to it, override the
default: `docker build --build-arg IROH_RELAY_VERSION=0.96.0 -t
iroh-relay-test iroh-relay`.

**CI build/push** (the path that actually ships the image): the two scripts
below are CI-runtime-specific (they drive `nerdctl`/`buildkitd` sockets only
available on the Jenkins build agent) — don't expect them to run outside CI.

- **Any push, any branch** touching `iroh-relay/**` triggers a rebuild of
  the "elohim-edge" pipeline (`elohim/holochain/Jenkinsfile`), whose Build
  stage runs `scripts/ci/build-iroh-relay.sh` then, later in the same
  pipeline, `scripts/ci/push-iroh-relay.sh` — which already pushes
  `harbor.ethosengine.com/ethosengine/iroh-relay:<version>-<commit-hash>`
  to Harbor (our container registry). This only catches a nonexistent or
  non-compiling version, visibly, before it ever reaches `dev` — it does
  **not** catch the wire-protocol mismatch described below; nothing
  automated does.
- **Landing on `dev`** (this repo's integration branch — landed via a local
  fast-forward merge, not a PR; if you can't perform that merge yourself,
  hand the bump to someone who can) additionally publishes the
  `:<version>-dev-latest` alias (e.g. `0.95.1-dev-latest`) in the *same*
  push that does the merge — no separate/further push is needed. That
  alias is the one tag the deploy manifests actually pull.

If you don't have Harbor/Jenkins access, the local smoke test above is the
only verification available to you — treat a successful build there plus a
landed `dev` merge as done; the pipeline's own status (visible to whoever
has Jenkins access) is the authority on whether the `dev-latest` alias
actually landed.

## Bumping the pinned version

**Who does this, and when:** any contributor, whenever upstream `iroh-relay`
ships a release you want (a security fix, a bugfix) — there's no schedule.
This crate's version is not free-floating: the relay only works with a
**conductor** (the Holochain node runtime this project runs — a separate
component, tracked in its own `elohim/holochain-conductor` git submodule,
with its own release cadence) whose embedded iroh client speaks the same
wire protocol. Nothing automated checks this — not the local smoke test,
not CI (which only catches a nonexistent/non-compiling version), not code
review unless the reviewer knows to look. A mismatch's symptom is silent at
the image-build level and shows up later as peers failing to connect
through the relay — there is no post-deploy probe for this specific check
today; verifying live relay connectivity after a deploy is out of scope for
this README (see the design doc). This needs a local Rust toolchain
(separate from, and in addition to, the Docker smoke test) to fetch crate
source for comparison — if you don't have one, or don't have the
`elohim/holochain-conductor` submodule initialized (`git submodule update
--init elohim/holochain-conductor`), hand the bump to someone who does; the
Docker-only smoke test alone cannot perform this check. Checklist, in order:

The test is **constant equality, not version-number equality** — the relay
crate and the conductor's internal republish of it are allowed to carry
different version numbers; what must be identical is the two wire constants
below. This is also what "whenever upstream ships a release you want"
means: you are free to pick any candidate release, as long as it passes
step 3.

1. **Find the conductor's pinned client version:** in
   `elohim/holochain-conductor/Cargo.lock` (that submodule, not this repo's
   top-level lockfile), trace the dependency edge `holochain → kitsune2 →
   kitsune2_transport_iroh → iroh-holochain → iroh-relay-holochain` — the
   resolved `iroh-relay-holochain` version is the client the relay must be
   checked against (it's a republish of `iroh-relay`, so today it happens
   to share `iroh-holochain`'s `0.95.1`, but trace the full edge — don't
   assume the two stay numerically equal on a future bump). This is also
   the "conductor" half of the "Current pairing" line below — record the
   `iroh-relay-holochain` version you found here, not a Holochain release
   number.
2. **Fetch both crates' source:** on your host (not inside Docker — the
   smoke test's `cargo install` runs in a throwaway container and never
   touches your host's cargo cache), run `cargo install iroh-relay
   --version <candidate> --locked --features server --root
   /tmp/iroh-relay-check` and `cargo install iroh-relay-holochain --version
   <version from step 1> --locked --features server --root
   /tmp/iroh-relay-holochain-check`. Each is a full from-source compile
   (several minutes, not seconds); when both finish, your host's
   `~/.cargo/registry/src/index.crates.io-*/` has both
   `iroh-relay-<candidate>/` and `iroh-relay-holochain-<version>/`
   directories.
3. **Compare the wire-protocol constants:** diff the two crates'
   `src/http.rs` — both should define the same `RELAY_PROTOCOL_VERSION` and
   `RELAY_PATH` constants. Identical strings = correct pairing; anything
   else, stop and read the design doc's version-contract section for the
   full evidence chain and what to do about a divergence. Do this for every
   bump, not just "big" ones — a patch release can change these constants
   too, and do it again whenever the *conductor* submodule updates, since
   that can change the step-1 version just as much as an `iroh-relay` bump
   can.
4. **If they match:** edit the `ARG IROH_RELAY_VERSION=x.y.z` line in
   `iroh-relay/Dockerfile`, run the local Docker smoke test (see "Building it"
   above — this is a separate, later check: it confirms the crate still
   compiles, it does not repeat the protocol comparison you just did),
   update this section's "Current pairing" line below to match, then
   commit. No other file in this repo needs to change for the bump itself.
5. **If a mismatch reaches `dev-latest` anyway:** revert the `ARG` line and
   land that revert on `dev` the same way. To recover a premise faster than
   waiting for the alias to update: find the pre-mismatch commit in `git
   log -- iroh-relay/Dockerfile`, take its short hash, and edit that
   premise's manifest under `genesis/orchestrator/manifests/doorway/` to
   pull `:<version>-<that-commit-hash>` directly instead of
   `:<version>-dev-latest` — this is an operator action on the manifest,
   not something this directory's files can do alone.

**Current pairing** (update this line at step 4, every bump): relay
`0.95.1` ↔ conductor `0.6.3`, matched on `RELAY_PROTOCOL_VERSION =
"iroh-relay-v1"` and `RELAY_PATH = "/relay"`.

Full rationale (why running our own relay matters), the version contract and
compatibility evidence, and deployment details:
`genesis/docs/content/elohim-protocol/architecture/2026-08-05-wave2-relay-sovereignty-design.md`
(path from repo root).
