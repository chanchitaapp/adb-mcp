{
  description = "ADB MCP Server - Rust implementation";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    systems.url = "github:nix-systems/default";
  };

  outputs = {
    self,
    nixpkgs,
    systems,
  }: let
    inherit (nixpkgs) lib;
    eachSystem = lib.genAttrs (import systems);
  in {
    devShells = eachSystem (system: let
      pkgs = import nixpkgs {inherit system;};
    in
      pkgs.mkShell {
        buildInputs = with pkgs; [
          rustup
          pkg-config
          android-tools
        ];
      });

    packages = eachSystem (system: let
      pkgs = import nixpkgs {inherit system;};
    in {
      default = pkgs.rustPlatform.buildRustPackage {
        pname = "adb-mcp";
        version = "0.1.0";
        src = ./.;
        cargoLock.lockFile = ./Cargo.lock;

        buildInputs = with pkgs; [pkg-config];

        CARGO_PROFILE_RELEASE_OPT_LEVEL = 3;
        CARGO_PROFILE_RELEASE_LTO = "true";
        CARGO_PROFILE_RELEASE_STRIP = "true";

        meta = with pkgs.lib; {
          description = "MCP server for ADB interactions";
          homepage = "https://github.com/chanchitaapp/adb-mcp";
          license = licenses.mit;
          mainProgram = "adb-mcp";
        };
      };
    });
  };
}
