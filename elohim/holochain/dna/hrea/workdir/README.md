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
# From repo root. Replace VERSION with the version pinned in happ.yaml.
VERSION=$(grep -A 2 "name: hrea" elohim/holochain/dna/elohim/workdir/happ.yaml \
            | grep "version_pin" | awk '{print $2}' | tr -d '"')
mkdir -p elohim/holochain/dna/hrea/workdir
curl -L -o elohim/holochain/dna/hrea/workdir/hrea.dna \
    "https://github.com/h-REA/hREA/releases/download/${VERSION}/hrea.dna"
```

If the upstream URL changes, update both this README and the happ.yaml
`version_pin` comment.

## Currently pinned version

See `dna.path` in `elohim/holochain/dna/elohim/workdir/happ.yaml`.
