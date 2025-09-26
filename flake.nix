{
  inputs.nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";

  outputs = {
    self,
    nixpkgs,
  }: let
    system = "x86_64-linux";
    pkgs = import nixpkgs {inherit system;};
  in {
    devShells.${system} = {
      default = with pkgs;
        mkShell rec {
          nativeBuildInputs = [
            python3
            uv
            pkg-config
            clang
          ];
          buildInputs = [
            fontconfig
            libclang
            stdenv.cc.cc
            glslang
            wayland
            libxkbcommon
            # Not strictly required, but helps with
            # https://github.com/NixOS/nixpkgs/issues/370494
            rust-jemalloc-sys
            # Merge the qt packages together to make a lighter version of qt6.full
            (symlinkJoin {
              name = "qt packages";
              paths = [
                qt6.qtbase
                # Required for 'QT_QPA_PLATFORM=wayland' to work
                qt6.qtwayland
              ];
            })
            openssl
            pkg-config
            udev
            libGL
            seatd
            libgbm
            libinput
            freetype
            nodejs
            pnpm
            xorg.libX11
            xorg.libXcursor
            xorg.libXi
            xorg.libXrandr # To use the x11 feature
            vulkan-loader
          ];
          LD_LIBRARY_PATH = lib.makeLibraryPath buildInputs;
        };
    };
  };
}
