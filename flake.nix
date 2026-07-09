{
  description = "Pyth Lazer development and build environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      rust-overlay,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        rustPlatform = pkgs.makeRustPlatform {
          cargo = rustToolchain;
          rustc = rustToolchain;
        };

        commonNativeBuildInputs = with pkgs; [
          bun
          pkg-config
          protobuf
          rustToolchain
        ];

        commonBuildInputs =
          with pkgs;
          [
            openssl
          ]
          ++ lib.optionals stdenv.isDarwin [
            apple-sdk_15
          ];
      in
      {
        formatter = pkgs.nixfmt;

        packages.default = rustPlatform.buildRustPackage {
          pname = "pyth-lazer";
          version = "0.1.0";

          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;

          nativeBuildInputs = commonNativeBuildInputs;
          buildInputs = commonBuildInputs;

          doCheck = false;
        };

        devShells.default = pkgs.mkShell {
          packages =
            commonNativeBuildInputs
            ++ commonBuildInputs
            ++ (with pkgs; [
              nodejs_24
            ]);

          PROTOC = "${pkgs.protobuf}/bin/protoc";
        };
      }
    );
}
