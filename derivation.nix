{ rustPlatform
, nix-gitignore
, buildType ? "release"
, lib
, _arg'qemucomm ? nix-gitignore.gitignoreSourcePure [ ./.gitignore ''
  /.github
  /.git
  *.nix
'' ] ./.
}: with lib; let
  cargoToml = importTOML ./Cargo.toml;
in rustPlatform.buildRustPackage {
  pname = cargoToml.package.name;
  version = cargoToml.package.version;

  src = _arg'qemucomm;
  cargoSha256 = "sha256-mEeYo+PBQGvoFO6qpNme2jOWsXDKhVpDhlPvvnylL6k=";
  inherit buildType;

  doCheck = false;

  meta = {
    platforms = platforms.unix ++ platforms.windows;
    broken = versionOlder rustPlatform.rust.rustc.version "1.56";
  };
}
