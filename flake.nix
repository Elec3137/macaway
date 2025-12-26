{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
  };

  outputs =
    { self, nixpkgs, ... }:
    let
      pkgs = nixpkgs.legacyPackages."x86_64-linux";
      cargoToml = (builtins.fromTOML (builtins.readFile ./Cargo.toml));
    in
    with pkgs;
    {
      packages."x86_64-linux".default = rustPlatform.buildRustPackage rec {
        pname = cargoToml.package.name;
        version = cargoToml.package.version;

        src = ./.;

        cargoLock = {
          lockFile = ./Cargo.lock;
        };

        nativeBuildInputs = [
          makeBinaryWrapper
          pkg-config

          xorg.libX11
          xorg.libXtst

          systemd
          libinput
        ];

        buildInputs = [
          slurp

          xorg.libX11
          xorg.libXtst

          libinput
        ];

        postFixup = ''
          wrapProgram $out/bin/${pname} \
            --prefix PATH : ${lib.makeBinPath [ slurp ]} \
            --prefix LD_LIBRARY_PATH : ${lib.makeLibraryPath buildInputs}
        '';
      };

      devShells."x86_64-linux".default = mkShell {
        inputsFrom = [ self.packages."x86_64-linux".default ];
      };
    };
}
