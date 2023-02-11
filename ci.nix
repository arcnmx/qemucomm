{ config, pkgs, lib, ... }: with pkgs; with lib; let
  inherit (import ./. { inherit pkgs; }) packages;
  qemucomm = packages.qemucomm.override {
    buildType = "debug";
  };
in {
  config = {
    name = "qemucomm";
    ci.gh-actions.enable = true;
    cache.cachix = {
      ci.signingKey = "";
      arc.enable = true;
    };
    channels = {
      nixpkgs = "22.11";
    };
    tasks = {
      build.inputs = singleton qemucomm;
    };
  };
}
