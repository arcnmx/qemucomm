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
  cargoSha256 = "sha256-hzFo/g9jFGU+Z8g8hWJYnKIMvoT+G40zCcrNBTIA6+4=";
  inherit buildType;

  doCheck = false;

  meta = {
    platforms = platforms.unix ++ platforms.windows;
    broken = versionOlder rustPlatform.rust.rustc.version "1.56";
  };
}
