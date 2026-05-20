# hREA DNA — workdir

This directory holds the **external** hREA DNA bundle. hREA is published
by the Holochain hREA team (Lynn Foster, Bob Haugen, et al.); we consume
it as a versioned binary, not by building it ourselves.

## Wave 3 design context

Wave 3 of the cross-wave guidance adds hREA as a projection target for
VF-shaped writes. The valueflows bridge (at `bridges/valueflows/`)
translates VF-GraphQL queries and mutations and projects them into hREA
entries via per-Human cells provisioned during the VFBinding handshake.

See:
- `genesis/docs/superpowers/specs/2026-05-20-wave3-valueflows-hrea-interop-design.md`
- `https://github.com/h-REA/hREA/releases`

## Fetching the bundle

The `hrea.dna` binary is **not in git** (it's a built artifact from
upstream releases). To populate this directory:

```bash
# From repo root. Replace VERSION with the version pinned in happ.yaml's
# `# version_pin: ...` comment above the hrea role.
VERSION=$(grep "^  # version_pin:" elohim/holochain/dna/elohim/workdir/happ.yaml \
            | awk '{print $3}' | tr -d '"')
if [ -z "$VERSION" ] || [ "$VERSION" = "0.0.0" ]; then
    echo "ERROR: no real version pinned in happ.yaml yet (still 0.0.0 placeholder)"
    exit 1
fi
mkdir -p elohim/holochain/dna/hrea/workdir
curl -L -o elohim/holochain/dna/hrea/workdir/hrea.dna \
    "https://github.com/h-REA/hREA/releases/download/${VERSION}/hrea.dna"
curl -L -o elohim/holochain/dna/hrea/workdir/hrea.dna.sha256 \
    "https://github.com/h-REA/hREA/releases/download/${VERSION}/hrea.dna.sha256"
cd elohim/holochain/dna/hrea/workdir && sha256sum --check hrea.dna.sha256
```

Note: sha256 file availability depends on hREA upstream releases. If the
pinned version does not publish a `.sha256` file, the script will fail at
the `curl` step — which is the correct failure mode. Silently accepting an
unverified binary is not acceptable.

If the upstream URL changes, update both this README and the happ.yaml
`version_pin` comment.

## Currently pinned version

See `dna.path` in `elohim/holochain/dna/elohim/workdir/happ.yaml`.
