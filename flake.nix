{
  description = "Horizon: Minecraft Java Edition 26.3 rewritten in Rust";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      linuxSystems = [ "x86_64-linux" "aarch64-linux" ];
      systems = linuxSystems ++ [ "aarch64-darwin" ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
      forLinuxSystems = f: nixpkgs.lib.genAttrs linuxSystems (system: f nixpkgs.legacyPackages.${system});

      # The game's packages, built from this workspace (nix/, docs/nix.md). Releases are tagged by
      # release.yml, which never uses this flake, so a build from it says which commit it is.
      version = "0-unstable-${builtins.substring 0 8 (self.lastModifiedDate or "19700101")}";
      gamePackages = pkgs:
        let
          call = pkgs.lib.callPackageWith (pkgs // packages // { inherit version; });
          packages = {
            horizon-engine = call ./nix/horizon-engine.nix { };
            horizon-server = call ./nix/horizon-server.nix { };
            horizon-prepare = call ./nix/horizon-prepare.nix { };
            horizon-client = call ./nix/horizon-client.nix { };
          };
        in packages;
    in {
      # YuyuFrame: no horizon-launcher package (Horizon's desktop launcher, launcher/, is removed).
      packages = forAllSystems gamePackages;

      overlays.default = final: prev: gamePackages final;

      # services.horizon-server (docs/nix.md).
      nixosModules = {
        horizon-server = import ./nix/module.nix { inherit self; };
        default = self.nixosModules.horizon-server;
      };

      checks = forAllSystems (pkgs: {
        inherit (self.packages.${pkgs.stdenv.hostPlatform.system}) horizon-server horizon-prepare;
      } // nixpkgs.lib.optionalAttrs pkgs.stdenv.hostPlatform.isLinux {
        # The module in a VM, without any of Mojang's files.
        nixos-module = pkgs.testers.runNixOSTest (import ./nix/tests/server.nix { inherit self; });
      });

      devShells = forLinuxSystems (pkgs: {
        default = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            coreutils
            findutils
            gnused
            gnugrep
            pkg-config
            rustc
            cargo
            clang
            mold
            python3
          ];
          buildInputs = with pkgs; [
            libxkbcommon
            wayland
            vulkan-loader
            fontconfig
            freetype
            alsa-lib
            openssl
            udev
            libx11
            libxcursor
            libxrandr
            libxi
          ];
          LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (with pkgs; [
            libxkbcommon
            wayland
            vulkan-loader
            fontconfig
            freetype
            alsa-lib
            udev
            libx11
            libxcursor
            libxrandr
            libxi
          ]);
        };
      });
    };
}
