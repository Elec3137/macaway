{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      nixpkgs,
      crane,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs { inherit system; };
        craneLib = crane.mkLib pkgs;

        commonArgs = {
          src = craneLib.cleanCargoSource ./.;

          nativeBuildInputs = with pkgs; [
            makeBinaryWrapper
            pkg-config

            xorg.libX11
            xorg.libXtst

            systemd
            libinput
          ];

          buildInputs = with pkgs; [
            slurp

            xorg.libX11
            xorg.libXtst

            libinput
          ];
        };

        LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath commonArgs.buildInputs;

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        pname = (fromTOML (builtins.readFile ./Cargo.toml)).package.name;
        crate = craneLib.buildPackage (
          commonArgs
          // {
            inherit cargoArtifacts;

            postFixup = ''
              wrapProgram $out/bin/${pname} \
                --prefix PATH : ${pkgs.lib.makeBinPath [ pkgs.slurp ]} \
                --prefix LD_LIBRARY_PATH : ${LD_LIBRARY_PATH}
            '';
          }
        );
      in
      {
        packages.default = crate;

        checks = {
          crate-clippy = craneLib.cargoClippy (
            commonArgs
            // {
              inherit cargoArtifacts;

              cargoClippyExtraArgs = "-- --deny warnings";
            }
          );
        };

        devShells.default = craneLib.devShell {
          inherit LD_LIBRARY_PATH;

          inputsFrom = [ crate ];
          packages = [ pkgs.rust-analyzer ];
        };
      }
    );

}
