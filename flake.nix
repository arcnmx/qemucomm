{
  description = "QAPI QMP and QGA (Guest Agent) CLI tool";
  inputs = {
    flakelib.url = "github:flakelib/fl";
    nixpkgs = { };
  };
  outputs = { flakelib, ... }@inputs: flakelib {
    inherit inputs;
    config = {
      name = "qemucomm";
    };
    packages.qemucomm = {
      __functor = _: import ./derivation.nix;
      fl'config.args = {
        _arg'qemucomm.fallback = inputs.self.outPath;
      };
    };
    defaultPackage = "qemucomm";
  };
}
