#!/usr/bin/env bash
# Build-context floor shared by the local and CI Rust-style wrappers.

rust_style_build_environment() {
  local style_repo_root="$1"
  local style_checker_root="$style_repo_root/genesis/orchestrator/rust-style"
  local style_config_root="$style_checker_root"
  local style_legacy_config
  local style_nested_config

  while true; do
    style_legacy_config="$style_config_root/.cargo/config"
    if [[ -e "$style_legacy_config" || -L "$style_legacy_config" ]]; then
      echo "Rust style UNMEASURED: legacy Cargo configuration is not an inventoried input: $style_legacy_config; remove it or migrate it to the reviewed config.toml." >&2
      return 2
    fi
    if [[ "$style_config_root" != "$style_repo_root" ]]; then
      style_nested_config="$style_config_root/.cargo/config.toml"
      if [[ -e "$style_nested_config" || -L "$style_nested_config" ]]; then
        echo "Rust style UNMEASURED: Cargo configuration below the repository root is not an inventoried input: $style_nested_config; only $style_repo_root/.cargo/config.toml is allowed." >&2
        return 2
      fi
    fi
    [[ "$style_config_root" == "$style_repo_root" ]] && break
    style_config_root="$(dirname "$style_config_root")"
  done

  if ! cd "$style_repo_root"; then
    echo "Rust style UNMEASURED: cannot enter repository root before invoking Cargo: $style_repo_root" >&2
    return 2
  fi

  export CARGO_BUILD_RUSTC=rustc
  export CARGO_BUILD_RUSTC_WRAPPER=
  export CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER=
  export RUSTC_WRAPPER=
  export RUSTC_WORKSPACE_WRAPPER=
  export CARGO_REGISTRY_GLOBAL_CREDENTIAL_PROVIDERS=cargo:token
  # Repository precedent: genesis/agentic/compute/Dockerfile.source declares this
  # non-secret sentinel for an endpoint that serves reads anonymously while
  # advertising auth-required metadata.
  if [[ ! -v CARGO_REGISTRIES_ELOHIM_TOKEN ]]; then
    export CARGO_REGISTRIES_ELOHIM_TOKEN=anonymous
  fi
}
