{
  inputs = {
    devshell = {
      url = "github:numtide/devshell";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-parts.url = "github:hercules-ci/flake-parts";
    nci = {
      url = "github:yusdacra/nix-cargo-integration";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    pre-commit-hooks = {
      url = "github:cachix/pre-commit-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    inputs:
    inputs.flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [
        inputs.devshell.flakeModule
        inputs.flake-parts.flakeModules.easyOverlay
        inputs.nci.flakeModule
        inputs.pre-commit-hooks.flakeModule
        inputs.treefmt-nix.flakeModule
      ];

      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];

      flake =
        { config, ... }:
        {
          nixosModules.default = {
            imports = [ ./nix/nixosModules/idmail.nix ];
            nixpkgs.overlays = [ config.overlays.default ];
          };
        };

      perSystem =
        {
          config,
          lib,
          pkgs,
          ...
        }:
        let
          projectName = "idmail";

          tailwindcss = pkgs.nodePackages.tailwindcss.overrideAttrs (_prevAttrs: {
            plugins = [
              pkgs.nodePackages."@tailwindcss/aspect-ratio"
              pkgs.nodePackages."@tailwindcss/forms"
              pkgs.nodePackages."@tailwindcss/language-server"
              pkgs.nodePackages."@tailwindcss/line-clamp"
              pkgs.nodePackages."@tailwindcss/typography"
            ];
          });

          extraNativeBuildInputs = [
            pkgs.wasm-bindgen-cli
            pkgs.binaryen
            pkgs.cargo-leptos
            tailwindcss
          ];
        in
        {
          devshells.default = {
            packages =
              [
                config.treefmt.build.wrapper
                pkgs.cargo-release
              ]
              # FIXME: why is this necessary? nci doesn't seem to add them automatically.
              ++ extraNativeBuildInputs;
            devshell.startup.pre-commit.text = config.pre-commit.installationScript;
          };

          pre-commit.settings.hooks.treefmt.enable = true;
          treefmt = {
            projectRootFile = "flake.nix";
            programs = {
              deadnix.enable = true;
              statix.enable = true;
              nixfmt.enable = true;
              rustfmt.enable = true;
            };
          };

          nci.projects.${projectName} = {
            path = ./.;
            numtideDevshell = "default";
          };
          nci.crates.${projectName} = {
            drvConfig = {
              mkDerivation = {
                # add trunk and other dependencies
                nativeBuildInputs = [
                  pkgs.makeWrapper
                ] ++ extraNativeBuildInputs;

                # override build phase to build with trunk instead
                buildPhase = ''
                  export -n CARGO_BUILD_TARGET
                  cargo leptos build --release -vvv
                '';

                installPhase = ''
                  mkdir -p $out/bin $out/share
                  cp target/release/${projectName} $out/bin/
                  cp -r target/site $out/share/
                  wrapProgram $out/bin/${projectName} \
                    --set LEPTOS_SITE_ROOT $out/share/site
                '';

                meta = {
                  description = "idmail, an email alias and account management interface for self-hosted mailservers";
                  homepage = "https://github.com/oddlama/idmail";
                  license = lib.licenses.mit;
                  #maintainers = with lib.maintainers; [oddlama];
                  mainProgram = "idmail";
                };
              };
              env.RUSTFLAGS = "--cfg=web_sys_unstable_apis";
              env.LEPTOS_ENV = "PROD";
            };
          };

          packages.default = config.nci.outputs.${projectName}.packages.release;
          packages.nixosTest = import ./nix/tests/idmail.nix {
            inherit (inputs) self;
            inherit pkgs lib;
          };

          overlayAttrs.idmail = config.packages.default;
        };
    };
}
