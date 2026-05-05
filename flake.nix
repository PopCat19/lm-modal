{
  description = "lm-modal - Wayland-native LLM overlay";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, ... }:
    {
      # Home Manager module is system-agnostic
      # Package must be passed via the 'package' option
      homeManagerModules.default = import ./modules/home-manager.nix;
    } //
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        rustToolchain = pkgs.rust-bin.stable.latest.default;
      in
      {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "lm-modal";
          version = "0.1.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;

          buildInputs = with pkgs; [
            wayland
            libxkbcommon
            libGL
          ];

          nativeBuildInputs = with pkgs; [
            pkg-config
            makeWrapper
          ];

          postInstall = ''
            wrapProgram $out/bin/lm-modal \
              --prefix LD_LIBRARY_PATH : ${pkgs.lib.makeLibraryPath [ pkgs.wayland pkgs.libxkbcommon pkgs.libGL ]}
          '';
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain
            rust-analyzer
            pkg-config
            wayland
          ];

          RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
        };
      }
    );
}