{
  inputs = {
    devshell = {
      url = "github:numtide/devshell";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    flake-parts.url = "github:hercules-ci/flake-parts";
    nci.url = "github:yusdacra/nix-cargo-integration";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    pre-commit-hooks = {
      url = "github:cachix/pre-commit-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs:
    inputs.flake-parts.lib.mkFlake {inherit inputs;} {
      imports = [
        inputs.nci.flakeModule
        inputs.pre-commit-hooks.flakeModule

        # Derive the output overlay automatically from all packages that we define.
        inputs.flake-parts.flakeModules.easyOverlay
      ];

      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];

      flake = {config, ...}: {
        nixosModules.default = {
          imports = [./nix/nixosModules/idmail.nix];
          nixpkgs.overlays = [config.overlays.default];
        };
      };

      perSystem = {
        config,
        lib,
        pkgs,
        ...
      }: let
        projectName = "idmail";
      in {
        pre-commit.settings.hooks = {
          alejandra.enable = true;
          deadnix.enable = true;
          statix.enable = true;
        };

        nci.projects.${projectName}.path = ./.;
        nci.crates.${projectName} = {
          drvConfig = {
            mkDerivation = let
              tailwindcss =
                pkgs.nodePackages.tailwindcss.overrideAttrs
                (_prevAttrs: {
                  plugins = [
                    pkgs.nodePackages."@tailwindcss/aspect-ratio"
                    pkgs.nodePackages."@tailwindcss/forms"
                    pkgs.nodePackages."@tailwindcss/language-server"
                    pkgs.nodePackages."@tailwindcss/line-clamp"
                    pkgs.nodePackages."@tailwindcss/typography"
                  ];
                });
            in {
              # add trunk and other dependencies
              nativeBuildInputs = [
                pkgs.makeWrapper
                pkgs.wasm-bindgen-cli
                pkgs.binaryen
                pkgs.cargo-leptos
                tailwindcss
              ];

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

        devShells.default = config.nci.outputs.${projectName}.devShell.overrideAttrs (old: {
          shellHook = ''
            export RUSTFLAGS="--cfg=web_sys_unstable_apis"
            ${old.shellHook or ""}
            ${config.pre-commit.installationScript}
          '';
        });

        packages.default = config.nci.outputs.${projectName}.packages.release;
        formatter = pkgs.alejandra; # `nix fmt`

        overlayAttrs = {
          idmail = config.packages.default;
        };
      };
    };
}
