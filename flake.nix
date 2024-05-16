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
        buildInputs = with pkgs; [ openssl cacert curl ];
        nativeBuildInputs = with pkgs; [ pkg-config cargo rustc rustfmt rustPackages.clippy ];
        rustPackage = naersk-lib.buildPackage {
          inherit buildInputs nativeBuildInputs;
          # China registry mirror
          cratesDownloadUrl = "https://crates-io.proxy.ustclug.org";
          src = ./.;
        };
        dockerImage = pkgs.dockerTools.buildLayeredImage {
          name = "nextdotid/relation_server";
          tag = "latest";
          created = "now";
          contents = [ rustPackage pkgs.curl pkgs.cacert ];
          config = {
            Workingdir = "/app";
            Cmd = [ "standalone" ];
            ExposedPorts = {
              "8000/tcp" = {};
            };
            Volumes = {
              "/app/config" = {};
            };
            Test = [ "CMD" "curl" "-s" "http://127.0.0.1:8000/" ];
            Interval = 30000000000; # 30s
            Timeout =  10000000000; # 10s
            Retries = 3;
            Labels = {
              "maintainer" = "Nyk Ma <nykma@mask.io>";
              "org.opencontainers.image.description" = "RelationService: relation aggregation.";
              "org.opencontainers.image.source" = "https://github.com/NextDotID/relation_server";
              "org.opencontainers.image.title" = "relation_server";
              "org.opencontainers.image.url" = "https://github.com/NextDotID/relation_server";
            };
          };
        };
      in {
        packages = {
          main = rustPackage;
          docker = dockerImage;
        };
        defaultPackage = rustPackage;
        devShell = with pkgs; mkShell {
          buildInputs = buildInputs ++
                        nativeBuildInputs ++
                        (with pkgs; [ act rust-analyzer pre-commit ]);
          RUST_SRC_PATH = rustPlatform.rustLibSrc;
        };
      });
}
