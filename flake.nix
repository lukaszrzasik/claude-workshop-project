{
  description = "claude-workshop-project — a simple Rust binary + library";

  inputs = {
    nixpkgs.url     = "github:NixOS/nixpkgs/nixos-24.11";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in
      {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname   = "claude-workshop-project";
          version = "0.1.0";
          src     = ./.;
          cargoLock.lockFile = ./Cargo.lock;
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustc
            cargo
            clippy
            rustfmt
          ];
          shellHook = ''
            export PS1="\[\e[1;34m\][nix]\[\e[0m\] $PS1"
          '';
        };
      }
    );
}
