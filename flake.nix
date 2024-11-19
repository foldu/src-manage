{
  description = "A thing.";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      crane,
    }@inputs:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        lib = pkgs.lib;
        src-manage =
          let
            cargoToml = builtins.fromTOML (builtins.readFile "${self}/Cargo.toml");
            version = cargoToml.package.version;
            pname = cargoToml.package.name;
            craneLib = crane.mkLib pkgs;
          in
          craneLib.buildPackage {
            inherit pname version;
            src = lib.fileset.toSource {
              root = ./.;
              fileset = lib.fileset.unions [
                (craneLib.fileset.commonCargoSources ./.)
                (lib.fileset.maybeMissing ./completions)
              ];
            };
            buildInputs = with pkgs; [
              makeWrapper
              installShellFiles
            ];
            postInstall = ''
              wrapProgram $out/bin/src-manage \
              --set PATH ${
                lib.makeBinPath (
                  with pkgs;
                  [
                    openssh
                    sshfs
                    rsync
                    fuse
                  ]
                )
              }
              installShellCompletion --cmd src-manage --fish completions/completion.fish
            '';
          };
      in
      {
        packages = {
          inherit src-manage;
          default = src-manage;
        };
        devShells.default = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            sshfs
            rsync
          ];
        };
      }
    );
}
