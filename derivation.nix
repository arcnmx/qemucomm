let
  self = import ./. { pkgs = null; system = null; };
in {
  rustPlatform
, buildType ? "release"
, lib
, cargoLock ? crate.cargoLock
, source ? crate.src
, crate ? self.lib.crate
}: with lib; rustPlatform.buildRustPackage rec {
  pname = crate.name;
  inherit (crate) version;

  src = source;
  inherit cargoLock buildType;

  cargoBuildFlags = [ "--bins" ];
  doCheck = buildType != "release";

  meta = {
    platforms = platforms.unix ++ platforms.windows;
    broken = versionOlder rustPlatform.rust.rustc.version "1.56";
  };
}
