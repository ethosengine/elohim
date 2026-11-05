---
id: rust-style-checker-distribution
status: Draft
cites: []
---

# Standalone Rust checker distribution

## Addressed source holding

The local Brit trial uses an explicit source-distribution mode, independent of
checker registry publication. `source-distribution.py --out <directory>` produces
a deterministic raw-CID-named tar.gz containing the unchanged checker source,
its path-only scanner crate, manifest, lockfile and checked-in Cargo registry
configuration. The scanner has no EPRFS or seam-contract dependency. The archive does not carry
source from the Elohim tree the checker polices. `eprfs-core =0.3.3`,
`eprfs-meta =0.3.4` and transitive `elohim-seam-contracts` are immutable published
packages from the `elohim` registry, pinned by checksums in `Cargo.lock`.

The archive's `source-distribution.json` records that registry closure and every
held file CID. This is an addressed local source holding, not a Cargo registry
package or proof of publication. Initial installation requires the pinned registry
and crates.io dependencies to be cached or fetchable. The installed checker can
then run offline.

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

## Registry dependencies do not publish the checker

The checker remains `publish = false`. Publishing its dependencies makes the
source archive independently buildable without any sibling checkout, but does not
publish the checker or grant policy standing. The compiled method inventories the
checker manifest and lockfile, not mutable copies of registry dependency sources.

Do not patch the dependencies back to local paths. A consumer builds the selected
source archive with the registry endpoint declared by `.cargo/config.toml`. Cargo
must be given `CARGO_REGISTRY_GLOBAL_CREDENTIAL_PROVIDERS=cargo:token` because the
endpoint advertises `auth-required` even for anonymously served reads. The local
and CI wrappers preserve an explicitly supplied `CARGO_REGISTRIES_ELOHIM_TOKEN`,
or set its non-secret `anonymous` sentinel only when the variable is absent. That
sentinel is repository precedent in `genesis/agentic/compute/Dockerfile.source`;
publication credentials remain out of band and are never printed.

The wrappers also pin Cargo's compiler and rustc-wrapper environment overrides,
enter the repository root before invoking Cargo, refuse a legacy `.cargo/config`
at the repository root or below it, and refuse `.cargo/config.toml` below the
repository root through the checker directory. This is a contained defense for the
compiled checker. It does not isolate user/global Cargo configuration such as
`CARGO_HOME/config.toml` (normally `~/.cargo/config.toml`), or Cargo configuration
in directories above the repository root. Those limitations, together with other
unaccounted effective Cargo settings, remain out of scope and are tracked by
[workspace-discipline backlog row 84](../../data/timeline/backlog/arch-workspace-discipline-backlog.md#row-84);
no claim of full hostile-environment isolation is made here.

Installation supplies a checker, not authority to apply a policy. The consuming
space separately adopts an addressed rule and binds its scope, parameters and
baseline. Shared content, independent adoption and input-specific evidence remain
separate. The local gate continues to use its existing development build.
