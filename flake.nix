# SPDX-FileCopyrightText: 2021 Serokell <https://serokell.io/>
#
# SPDX-License-Identifier: CC0-1.0
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
  };

  outputs =
    { nixpkgs, flake-parts, ... }@inputs:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
        "x86_64-darwin"
      ];
      perSystem =
        {
          config,
          self',
          inputs',
          pkgs,
          system,
          lib,
          ...
        }:
        {
          devShells.default =
            with pkgs;
            mkShell.override { stdenv = pkgs.clangStdenv; } {
              RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
              RUST_BACKTRACE = 1;

              buildInputs =
                [
                ]
                ++ lib.optionals pkgs.stdenv.isLinux [
                  alsa-lib
                  libudev-zero
                  libx11
                  libxcursor
                  libxi
                  libxrandr
                  libxkbcommon
                  libglvnd
                  mesa
                  vulkan-loader
                  vulkan-tools
                  wayland
                  blender
                  fontconfig
                ];
              nativeBuildInputs = [
                pkg-config
                nixfmt
                nixd
                rustc
                cargo
                rust-analyzer
                clippy
                openssl
                rustfmt
                # wasm32-unknown-unknown links through lld (scripts/build_web.sh)
                lld
                binaryen
              ];
              LD_LIBRARY_PATH = lib.makeLibraryPath [
                alsa-lib
                libudev-zero
                libx11
                libxcursor
                libxi
                libxrandr
                libxkbcommon
                libglvnd
                mesa
                vulkan-loader
                wayland
              ];
            };
        };
    };
}
