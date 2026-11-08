---
id: artifact-store-epr-build-shem
status: verified
---

# epr build on shem (non-current ingest)

Source: worktree /projects/elohim/.worktrees/as-integrate, commit 303af86f38daac62006f454d0503cdb6d094d8cb (contains abd66bdb1), tree clean.

Build (from elohim/eprfs; `.family` = as-integrate so the target is its own family):

    CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/as-integrate/elohim__eprfs/dev RUSTUP_TOOLCHAIN=1.96.1 RUSTFLAGS="" CARGO_BUILD_JOBS=4
    cargo build -p elohim-epr-cli --bin epr               -> EXIT=0 (7m36s)
    cargo build -p elohim-device-runtime --bin bin-launch -> EXIT=0 (ingest requires the registry's second bin, bin-launch)

Binary: /projects/.cargo-target-pool/family/as-integrate/elohim__eprfs/dev/debug/epr

source-state: closureCid = bafkr4if4nz33odlwpnixns3e3e7ivxkjsp65eekrljrkuuatg7c7nzfb6i (bafkr4i prefix: byte profile, OK)

Gate test: `EPR_BIN=<binary> node --test --test-name-pattern "dirty external dependency" genesis/orchestrator/gate-runner.test.mjs` -> pass 1, fail 0.

Ingest (does not move current; no pin, no land, no launcher):

    epr bin ingest --tool epr <debug>/epr --repo <worktree> --source-commit 303af86f38daac62006f454d0503cdb6d094d8cb --tree-clean true --gate-run local-build --json

- release CID: bafyreif5zxb6vv2b6dbu42jicmvxw4hv7nkr3vjtlzdokapsl4mlnrq6zu (ring dev-context, version 0.3.3)
- stored: /projects/.claude-config/tools/bin-store/epr/bafyreif5zxb6vv2b6dbu42jicmvxw4hv7nkr3vjtlzdokapsl4mlnrq6zu/epr
- epr blob cid: bafkr4ihsh7qaj5ocla5e332o4yoeifaglt2qa3tbdli2emq5xd45gynn4y
- store current unchanged: bafyreifvzagc7qjzk2rlqeamukcsq4doemioeb3aamigrbnut3fifqgzta

EPR_BIN to use: /projects/.claude-config/tools/bin-store/epr/bafyreif5zxb6vv2b6dbu42jicmvxw4hv7nkr3vjtlzdokapsl4mlnrq6zu/epr
