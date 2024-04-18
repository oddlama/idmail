{
  description = "TODO";
  inputs = {
    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };

    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    devshell = {
      url = "github:numtide/devshell";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    pre-commit-hooks = {
      url = "github:cachix/pre-commit-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "flake-utils";
    };

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "flake-utils";
    };
  };

  outputs = {
    self,
    advisory-db,
    devshell,
    crane,
    flake-utils,
    nixpkgs,
    pre-commit-hooks,
    rust-overlay,
  }:
    flake-utils.lib.eachDefaultSystem (localSystem: let
      pkgs = import nixpkgs {
        inherit localSystem;
        overlays = [
          devshell.overlays.default
          rust-overlay.overlays.default
        ];
      };
      inherit (pkgs) lib;

      projectName = "akamail"; # FIXME: too similar to akamai.... :/

      rustToolchain = pkgs.pkgsBuildHost.rust-bin.nightly.latest.default.override {
        extensions = [
          "rust-src"
          "rust-analyzer"
          "clippy"
        ];
        targets = ["wasm32-unknown-unknown"];
      };
      craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

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

      # For each of the classical cargo "functions" like build, doc, test, ...,
      # crane exposes a function that takes some configuration arguments.
      # Common settings that we need for all of these are grouped here.
      commonArgs = {
        src = lib.cleanSourceWith {
          src = ./.;
          filter = path: type:
            (craneLib.filterCargoSources path type)
            || (builtins.baseNameOf path == "README.md")
            || (builtins.baseNameOf path == "tailwind.config.js")
            || (lib.hasInfix "/assets/" path)
            || (lib.hasInfix "/css/" path)
            || (lib.hasSuffix ".html" path);
        };

        nativeBuildInputs = [
          #pkgs.pkg-config
        ];

        # External packages required to compile this project.
        buildInputs =
          [
            #pkgs.openssl
            pkgs.cargo-leptos
            pkgs.binaryen # Provides wasm-opt
            tailwindcss
            #pkgs.protobuf
            #pkgs.binaryen
            #pkgs.cargo-generate
          ]
          ++ lib.optionals pkgs.stdenv.isDarwin [
            # Additional darwin specific inputs can be set here
            pkgs.libiconv
          ];
      };

      # Build *just* the cargo dependencies, so we can reuse
      # all of that work (e.g. via cachix) when running in CI
      cargoArtifacts = craneLib.buildDepsOnly commonArgs;

      # Build the actual package
      package = craneLib.buildPackage (commonArgs
        // {
          inherit cargoArtifacts;

          nativeBuildInputs =
            commonArgs.nativeBuildInputs
            ++ [
              pkgs.makeWrapper
            ];

          buildPhaseCargoCommand = ''
            cargo leptos build --release -vvv
          '';

          installPhaseCommand = ''
            mkdir -p $out/bin
            cp target/server/release/${projectName} $out/bin/
            cp -r target/site $out/bin/
            wrapProgram $out/bin/${projectName} \
              --set LEPTOS_SITE_ROOT $out/bin/site
          '';
        });
    in {
      # Define checks that can be run with `nix flake check`
      checks =
        {
          # Build the crate normally as part of checking, for convenience
          ${projectName} = package;

          # Run clippy (and deny all warnings) on the crate source,
          # again, resuing the dependency artifacts from above.
          #
          # Note that this is done as a separate derivation so that
          # we can block the CI if there are issues here, but not
          # prevent downstream consumers from building our crate by itself.
          "${projectName}-clippy" = craneLib.cargoClippy (commonArgs
            // {
              inherit cargoArtifacts;
              cargoClippyExtraArgs = "--all-targets -- --deny warnings";
            });

          "${projectName}-doc" = craneLib.cargoDoc (commonArgs
            // {
              inherit cargoArtifacts;
            });

          # Check formatting
          "${projectName}-fmt" = craneLib.cargoFmt {
            inherit (commonArgs) src;
          };

          # Audit dependencies
          "${projectName}-audit" = craneLib.cargoAudit {
            inherit (commonArgs) src;
            inherit advisory-db;
          };
        }
        // {
          pre-commit = pre-commit-hooks.lib.${localSystem}.run {
            src = ./.;
            hooks = {
              alejandra.enable = true;
              cargo-check.enable = true;
              rustfmt.enable = true;
              statix.enable = true;
            };
          };
        };

      packages.default = package; # `nix build`
      packages.${projectName} = package; # `nix build .#${projectName}`

      # `nix develop`
      devShells.default = pkgs.devshell.mkShell {
        name = projectName;
        imports = [
          "${devshell}/extra/language/c.nix"
          "${devshell}/extra/language/rust.nix"
        ];

        language.rust.enableDefaultToolchain = false;

        commands = [
          {
            package = pkgs.alejandra;
            help = "Format nix code";
          }
          {
            package = pkgs.statix;
            help = "Lint nix code";
          }
          {
            package = pkgs.deadnix;
            help = "Find unused expressions in nix code";
          }
        ];

        devshell.startup.pre-commit.text = self.checks.${localSystem}.pre-commit.shellHook;
        packages = let
          # Do not expose rust's gcc: https://github.com/oxalica/rust-overlay/issues/70
          # Create a wrapper that only exposes $pkg/bin. This prevents pulling in
          # development deps, like python interpreter + $PYTHONPATH, when adding
          # packages to a nix-shell. This is especially important when packages
          # are combined from different nixpkgs versions.
          mkBinOnlyWrapper = pkg:
            pkgs.runCommand "${pkg.pname}-${pkg.version}-bin" {inherit (pkg) meta;} ''
              mkdir -p "$out/bin"
              for bin in "${lib.getBin pkg}/bin/"*; do
                  ln -s "$bin" "$out/bin/"
              done
            '';
        in
          commonArgs.buildInputs
          ++ [
            (mkBinOnlyWrapper rustToolchain)
            # FIXME: pkgs.rust-analyzer
          ];

        env = [
          {
            name = "RUST_SRC_PATH";
            value = "${rustToolchain}/lib/rustlib/src/rust/library";
          }
        ];
      };

      formatter = pkgs.alejandra; # `nix fmt`
    });
}
