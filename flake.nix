{
  description = "Terminal screensaver with EarthBound battle backgrounds, cellular automata, and more";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
  };

  outputs = { self, nixpkgs }:
  let
    system = "x86_64-linux";
    pkgs = nixpkgs.legacyPackages.${system};
  in {
    packages.${system} = rec {
      whoa = pkgs.rustPlatform.buildRustPackage {
        pname = "whoa";
        version = "0.1.0";
        src = ./.;
        cargoLock.lockFile = ./Cargo.lock;
      };
      default = whoa;
    };
  };
}
