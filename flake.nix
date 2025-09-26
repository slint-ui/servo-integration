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
          ];
          LD_LIBRARY_PATH = lib.makeLibraryPath buildInputs;
        };
    };
  };
}
