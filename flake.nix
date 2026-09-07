{
  description = "kestrel bevy dev shell";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";
    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "clippy" "rustfmt" "rust-analyzer" ];
        };

        vulkan-sdk = pkgs.symlinkJoin {
          name = "vulkan-sdk";
          paths = with pkgs; [ vulkan-headers vulkan-loader vulkan-validation-layers ];
        };

        runtimeLibs = with pkgs; [
          libGL
          libclang
          libxkbcommon
          wayland
          xorg.libX11
          xorg.libXcursor
          xorg.libXi
          xorg.libXrandr
          vulkan-loader
        ];

        runtimeLibString = pkgs.lib.makeLibraryPath runtimeLibs;
      in
      {
        devShells.default = pkgs.mkShell {
          packages =
            runtimeLibs
            ++ [ rustToolchain ]
            ++ (with pkgs; [
              pkg-config
              antimicrox
              glibc.dev
              sdl-jstest
              udev
              libudev-zero
              alsa-lib
              vulkan-tools
            ]);

          shellHook = ''
            export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:/run/opengl-driver/lib:${runtimeLibString}"
            export DLSS_SDK="/home/me/Documents/Programming/DLSS-310.6.0"
            export VULKAN_SDK="${vulkan-sdk}"
            export LIBCLANG_PATH="${pkgs.llvmPackages.libclang.lib}/lib"
            export BINDGEN_EXTRA_CLANG_ARGS="$NIX_CFLAGS_COMPILE -isystem ${pkgs.glibc.dev}/include"
            export RUST_SRC_PATH="${rustToolchain}/lib/rustlib/src/rust/library"
            if [ -f /run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json ]; then
              export VK_ICD_FILENAMES=/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json
            elif [ -f /run/opengl-driver/share/vulkan/icd.d/nvidia.json ]; then
              export VK_ICD_FILENAMES=/run/opengl-driver/share/vulkan/icd.d/nvidia.json
            fi
            echo "== bevy dev shell =="
          '';
        };
      }
    );
}
