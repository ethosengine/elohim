# Nix package for elohim-node
#
# Build with: nix-build -E 'with import <nixpkgs> {}; callPackage ./package.nix {}'

{ lib
, rustPlatform
, pkg-config
, openssl
, protobuf
, sqlite
}:

rustPlatform.buildRustPackage rec {
  pname = "elohim-node";
  version = "0.1.0";

  src = ./..;

  cargoLock = {
    lockFile = ../Cargo.lock;
  };

  nativeBuildInputs = [
    pkg-config
    protobuf
  ];

  buildInputs = [
    openssl
    sqlite
  ];

  # Skip tests for now (they require network)
  doCheck = false;

  meta = with lib; {
    description = "Elohim Protocol infrastructure runtime for always-on nodes";
    homepage = "https://github.com/ethosengine/elohim";
    license = licenses.asl20;
    maintainers = [ ];
    platforms = platforms.linux ++ platforms.darwin;
  };
}
