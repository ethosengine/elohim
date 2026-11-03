---
id: rust-style-checker-distribution
status: Draft
cites: []
---

# Standalone Rust checker distribution

## Addressed source holding

The local Brit trial uses an explicit source-distribution mode, independent of
registry publication. `source-distribution.py --out <directory>` produces a
deterministic raw-CID-named tar.gz containing the unchanged checker source,
manifests and lockfile plus its first-party dependencies (`eprfs-meta`,
`eprfs-core`, `elohim-seam-contracts`). The dependencies retain their declared
licenses; `LICENSE-NOTICES.txt` and their manifests travel with the sources.

The only projected build input is `elohim/eprfs/Cargo.toml`: its member list is
reduced to core/meta, its inheritance tables remain verbatim, and unrelated
members, the iroh patch and workspace-only profiles are omitted. The archive's
`source-distribution.json` records that derivation, the original workspace CID,
and every held file CID. This is an addressed local source holding, not a Cargo
registry package or proof of publication. The third-party dependency versions and
checksums remain locked to crates.io; initial installation requires their cache
or network access. The installed checker can run offline.

Brit holds the immutable archive under `.epr-meta/rust-style/objects/` and its
locally adopted locator in `source.json`. Its Bash installer verifies the raw CID
and digest before accepting only regular, contained archive paths; it builds in
a fresh extraction outside any repository and installs to an explicit private
prefix. Build output lives in an artifact-specific subdirectory of the supplied
native Cargo pool slot, so another checkout's stale first-party fingerprints cannot
stand in for this source. It records source identity, toolchain and binary digest in `install.json`.
`install-rust-style.sh --verify <prefix>` checks held source and binary receipts
without invoking Cargo. There is no PATH or sibling-checkout fallback. A source
holding is inert until the receiving steward explicitly installs it; native policy
evaluation additionally verifies the complete discipline and compiled method.
This installer targets the Ubuntu CI/development lane: Bash, GNU tar/coreutils,
`jq`, `xxd`, Cargo and Rust 1.96.1 are explicit tools. The checker toolchain is
separate from Brit's own crate MSRVs; installing it changes no Brit compiler policy.

Regenerate the archive only after checker source, Cargo manifest and lockfile are
final. A source change creates a new address; never overwrite an existing object.
The native method descriptor uses the exact compiled checker inputs, so the
source distribution does not normalize the checker's own Cargo manifest.

## Registry distribution is not supported by this method

The checker remains `publish = false`. Its compiled method includes the exact
first-party dependency source inventory; an ordinary normalized Cargo package
does not hold that sibling source layout. Merely publishing its dependencies
would not make the checker package independently buildable. The earlier registry
preparation was not a release and is superseded by the source-distribution mode.

Future registry support would need an explicit packaging derivation that carries
the same verified method inputs inside the package and a source-free qualification.
Do not patch in local paths and call that registry qualification. The selected
source archive already carries the original declared layout and can be built by
a consumer with no Elohim checkout or registry credentials.

Installation supplies a checker, not authority to apply a policy. The consuming
space separately adopts an addressed rule and binds its scope, parameters and
baseline. Shared content, independent adoption and input-specific evidence remain
separate. The local gate continues to use its existing development build.
