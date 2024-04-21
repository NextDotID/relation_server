{
  inputs = {
    naersk.url = "github:nix-community/naersk/master";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, utils, naersk }:
    utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        naersk-lib = pkgs.callPackage naersk { };
        buildInputs = with pkgs; [ openssl pkg-config ];
        nativeBuildInputs = with pkgs; [ cargo rustc rustfmt rust-analyzer pre-commit rustPackages.clippy ];
      in
      {
        defaultPackage = naersk-lib.buildPackage {
          inherit buildInputs;
          src = ./.;
        };
        devShell = with pkgs; mkShell {
          buildInputs = buildInputs ++ nativeBuildInputs;
          RUST_SRC_PATH = rustPlatform.rustLibSrc;
        };
      });
}
