# Runtime resource manifests

These files serve fleet maintainers changing the orchestrator's Kubernetes resource budgets.
Each file is an ark `RuntimeManifest`: its root envelope declares limits, its archetype selects
the scheduler request floor, and `deployments.json` pins its canonical DAG-CBOR CID. The
orchestrator consumes the `k8s-bridge` library through its CLI. These declarations do not
enforce quotas or change ark's effective enforcement tier.

From the repository root, with Rust/Cargo and `just` installed, verify the checked-in fleet:

```bash
source genesis/agentic/bin/pool-lib.sh
export CARGO_TARGET_DIR="$(slot_path "$(detect_family "$PWD")" k8s-bridge dev)"
export CARGO_BUILD_JOBS=2 RUSTFLAGS=""
cargo run -q --manifest-path bridges/k8s/Cargo.toml --bin k8s-bridge -- verify --deployments genesis/orchestrator/data/deployments.json --manifests-dir genesis/orchestrator/manifests/runtime
echo EXIT=$?
```

Success prints seven `Fresh` verdicts and `EXIT=0`. To inspect one envelope, use the same
environment and replace the arguments after `--` with `render
genesis/orchestrator/manifests/runtime/adam.manifest.json --human adam`. JSON goes to stdout;
the rendered manifest CID goes to stderr.

V1 declares `headroom_bytes: 0`: no extra reservation is inferred. The conductor process shape
was copied from the live Matthew household manifest on 2026-09-05. It is provenance for the
declaration, not a new Kubernetes executable pin.

The five archetype manifests use the canonical limit budgets in
`genesis/data/devices/archetype-resource-budgets.json`. Human exceptions supersede their archetype
CID: Adam (8000m CPU), Jessica (1Gi memory request and 4Gi limit), James (8Gi memory and 2000m
CPU limits), and Gertrude/Susan/Eve (3000m CPU limits). Their existing justifications remain in
`deployments.json`. Jessica's optional `envelope.requests.memory_bytes` pins her request
exception; absent request dimensions use the archetype floor. Request exceptions below that
floor or above a declared limit are refused.

Rendering emits Mi/m strings. Verification compares equivalent quantities (8Gi equals 8192Mi),
preserving the fleet's existing spellings and all dollar-prefixed comments. The eight split
fields use floor arithmetic, 5/8 memory and 1/2 CPU for conductor, remainder for storage, checked
byte-for-byte against the existing bash splitter. Missing limits pass through as empty strings;
a manifest without any envelope emits `{}`.

When changing resources, edit the manifest, run `render`, update the deployment's CID and
resource values deliberately, then run `verify` and `just gate k8s-bridge`. Use a superseding
human manifest for a justified exception. Do not regenerate deployments.json wholesale.
A semantic manifest change invalidates its pin; whitespace alone does not change a DAG-CBOR CID.
Since 2026-09-05 all active humans are pinned: missing pins, CID mismatch, or resource drift refuse
with exit 65. Suspended humans are skipped. The pre-push leg runs this verification when its
inputs change. `observe` is reserved for Station 3b and exits 64 without observing or writing.
