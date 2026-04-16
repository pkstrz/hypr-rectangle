{
  description = "Rectangle-like window management for Hyprland";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = [
            pkgs.cargo
            pkgs.rustc
            pkgs.rust-analyzer
            pkgs.clippy
            pkgs.rustfmt
          ];
        };

        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "hypr-rectangle";
          version = "0.1.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;

          meta = with pkgs.lib; {
            description = "Rectangle-like window management for Hyprland";
            homepage = "https://github.com/pkstrz/hypr-rectangle";
            license = licenses.mit;
            maintainers = [ ];
            platforms = platforms.linux;
          };
        };
      }
    );
}
