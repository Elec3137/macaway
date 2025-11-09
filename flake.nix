{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
  };

  outputs =
    { self, nixpkgs, ... }:
    let
      pkgs = nixpkgs.legacyPackages."x86_64-linux";
    in
    with pkgs;
    {
      packages."x86_64-linux".default = rustPlatform.buildRustPackage {
        pname = "macro-recorder";
        version = self.shortRev or self.dirtyShortRev;
        src = ./.;
        cargoLock = {
          lockFile = ./Cargo.lock;
        };
        nativeBuildInputs = [ pkg-config ];
        buildInputs = [ systemd xorg.libX11 xorg.libXtst libinput slurp ];
      };

      devShells."x86_64-linux".default = mkShell {
        nativeBuildInputs = [ pkg-config ];
        buildInputs = [ systemd xorg.libX11 xorg.libXtst libinput slurp ];
      };
    };
}
