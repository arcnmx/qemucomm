{
  description = "QAPI QMP and QGA (Guest Agent) CLI tool";
  inputs = {
    flakelib.url = "github:flakelib/fl";
    nixpkgs = { };
    rust = {
      url = "github:arcnmx/nixexprs-rust";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
  outputs = { flakelib, self, nixpkgs, rust, ... }@inputs: let
    nixlib = nixpkgs.lib;
    impure = builtins ? currentSystem;
    inherit (nixlib)
      filter optional
      hasSuffix
    ;
  in flakelib {
    inherit (rust.lib) systems;
    inherit inputs;
    config = {
      name = "qemucomm";
    };
    packages = {
      qemucomm = {
        __functor = _: import ./derivation.nix;
        fl'config.args = {
          crate.fallback = self.lib.crate;
        };
      };
      default = { qemucomm }: qemucomm;
    };
    checks = {
      test = { rustPlatform, source, qemucomm }: rustPlatform.buildRustPackage {
        pname = self.lib.crate.package.name;
        inherit (self.lib.crate) cargoLock version;
        inherit (qemucomm) buildInputs nativeBuildInputs;
        src = source;
        buildType = "debug";
        meta.name = "cargo test";
      };
    };
    devShells = {
      plain = {
        mkShell, writeShellScriptBin, hostPlatform, lib
      , enableRust ? true, cargo
      , rustTools ? [ ]
      , qemucomm
      }: mkShell {
        RUST_LOG = "qemucomm=debug";
        QEMUCOMM_QMP_SOCKET_PATH = "/run/vfio/hourai/qmp";
        QEMUCOMM_QGA_SOCKET_PATH = "/run/vfio/hourai/qga";
        allowBroken = true;
        inherit rustTools;
        inherit (qemucomm) buildInputs;
        nativeBuildInputs = qemucomm.nativeBuildInputs ++ optional enableRust cargo ++ [
          (writeShellScriptBin "generate" ''nix run .#generate "$@"'')
        ];
      };
      stable = { rust'stable, outputs'devShells'plain }: outputs'devShells'plain.override {
        inherit (rust'stable) mkShell;
        enableRust = false;
      };
      dev = { rust'unstable, outputs'devShells'plain }: outputs'devShells'plain.override {
        inherit (rust'unstable) mkShell;
        enableRust = false;
        rustTools = [ "rust-analyzer" ];
      };
      default = { outputs'devShells }: outputs'devShells.plain;
    };
    legacyPackages = { callPackageSet }: callPackageSet {
      source = { rust'builders }: rust'builders.wrapSource self.lib.crate.src;

      generate = { rust'builders, outputHashes }: rust'builders.generateFiles {
        paths = {
          "lock.nix" = outputHashes;
        };
      };
      outputHashes = { rust'builders }: rust'builders.cargoOutputHashes {
        inherit (self.lib) crate;
      };
    } { };
    lib = {
      crate = rust.lib.importCargo {
        path = ./Cargo.toml;
        inherit (import ./lock.nix) outputHashes;
      };
      inherit (self.lib.crate.package) version;
    };
  };
}
