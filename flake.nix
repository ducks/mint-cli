{
  description = "Unofficial privacy-conscious Mint Mobile terminal client";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  outputs =
    { self, nixpkgs }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      eachSystem = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
    in
    {
      packages = eachSystem (pkgs: {
        default = pkgs.rustPlatform.buildRustPackage {
          pname = "mint-cli";
          version = "0.1.0";
          src = pkgs.lib.cleanSource ./.;
          cargoLock.lockFile = ./Cargo.lock;
          nativeBuildInputs = [ pkgs.makeWrapper ];
          postInstall = ''
            wrapProgram $out/bin/mint-cli --prefix PATH : ${pkgs.lib.makeBinPath [ pkgs.libsecret ]}
          '';
          meta = {
            description = "Unofficial Mint Mobile terminal client";
            license = pkgs.lib.licenses.mit;
            mainProgram = "mint-cli";
            platforms = systems;
          };
        };
      });
      apps = eachSystem (pkgs: {
        default = {
          type = "app";
          program = "${self.packages.${pkgs.stdenv.hostPlatform.system}.default}/bin/mint-cli";
        };
      });
      devShells = eachSystem (pkgs: {
        default = pkgs.mkShell {
          packages = with pkgs; [
            cargo
            rustc
            rustfmt
            clippy
            libsecret
            nixfmt-rfc-style
          ];
        };
      });
      formatter = eachSystem (pkgs: pkgs.nixfmt-rfc-style);
      checks = eachSystem (pkgs: {
        package = self.packages.${pkgs.stdenv.hostPlatform.system}.default;
        lint = self.packages.${pkgs.stdenv.hostPlatform.system}.default.overrideAttrs (old: {
          nativeBuildInputs = (old.nativeBuildInputs or [ ]) ++ [
            pkgs.clippy
            pkgs.rustfmt
          ];
          checkPhase = ''
            runHook preCheck
            cargo fmt --all -- --check
            cargo clippy --workspace --all-targets --offline -- -D warnings
            runHook postCheck
          '';
          doCheck = true;
        });
      });
    };
}
