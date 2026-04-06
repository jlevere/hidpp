{
  description = "HID++ 2.0 configurator — replace Logi Options+ with Rust";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
  };

  outputs = { self, nixpkgs, flake-utils, crane }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          config.allowUnfree = true;
        };

        craneLib = crane.mkLib pkgs;

        # Source filter: Rust files + data files needed by include_str!().
        # cleanCargoSource strips non-Rust files, so we add back .json data.
        rustSrc = pkgs.lib.cleanSourceWith {
          src = ./.;
          filter = path: type:
            (craneLib.filterCargoSources path type)
            || (builtins.match ".*\\.json$" path != null)
            || (builtins.match ".*\\.toml$" path != null);
        };

        # Common args for all native crate builds.
        # Excludes hidpp-web (WASM-only, can't build natively).
        commonArgs = {
          src = rustSrc;
          cargoExtraArgs = "--workspace --exclude hidpp-web";
          buildInputs = [ pkgs.hidapi ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
            pkgs.apple-sdk_15
          ] ++ pkgs.lib.optionals pkgs.stdenv.isLinux [
            pkgs.udev
          ];
          nativeBuildInputs = [ pkgs.pkg-config ];
        };

        # Build deps once, share across targets.
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        # Build the daemon binary.
        hidpp-daemon = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
          cargoExtraArgs = "-p hidpp-daemon";
        });

        # Build the CLI binary.
        hidpp-cli = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
          cargoExtraArgs = "-p hidpp-cli";
        });

        # macOS .app bundle for the daemon.
        # Wrapping as .app makes Accessibility permissions stick
        # (macOS grants permissions to bundles, not bare binaries).
        hidpp-app = pkgs.stdenv.mkDerivation {
          pname = "hidpp";
          version = "0.1.1";
          src = ./bundle;

          buildInputs = [ hidpp-daemon hidpp-cli ];

          installPhase = ''
            mkdir -p "$out/Applications/HID++.app/Contents/MacOS"
            mkdir -p "$out/Applications/HID++.app/Contents/Resources"

            cp ${./bundle/Info.plist} "$out/Applications/HID++.app/Contents/Info.plist"
            cp ${./bundle/AppIcon.icns} "$out/Applications/HID++.app/Contents/Resources/AppIcon.icns"
            echo 'APPL????' > "$out/Applications/HID++.app/Contents/PkgInfo"

            # Both binaries in the .app bundle.
            cp ${hidpp-daemon}/bin/hidppd "$out/Applications/HID++.app/Contents/MacOS/hidppd"
            cp ${hidpp-cli}/bin/hidpp "$out/Applications/HID++.app/Contents/MacOS/hidpp"

          '';
        };

        # macOS .dmg for distribution.
        # Opens with the .app and an alias to /Applications for drag-to-install.
        hidpp-dmg = pkgs.stdenv.mkDerivation {
          pname = "hidpp-dmg";
          version = "0.1.1";
          src = ./bundle;

          buildInputs = [ hidpp-app ];

          # hdiutil requires native macOS — this only builds on darwin.
          buildPhase = ''
            mkdir -p dmg-staging
            cp -R ${hidpp-app}/Applications/HID++.app dmg-staging/
            ln -s /Applications dmg-staging/Applications
          '';

          # Nix sandbox doesn't have /usr/bin in PATH — use absolute path.
          installPhase = ''
            mkdir -p "$out"
            /usr/bin/hdiutil create -volname "HID++" \
              -srcfolder dmg-staging \
              -ov -format UDZO \
              "$out/HID++.dmg"
          '';
        };

        # RE tools for the dev shell.
        python = pkgs.python313.withPackages (ps: [
          ps.r2pipe
          ps.rzpipe
          ps.capstone
          ps.unicorn
          ps.keystone-engine
          ps.protobuf
          ps.construct
        ]);

        rizinWithPlugins = pkgs.rizin.withPlugins (ps: [
          ps.rz-ghidra
          ps.jsdec
          ps.sigdb
        ]);

      in {
        packages = {
          default = hidpp-app;
          daemon = hidpp-daemon;
          cli = hidpp-cli;
          app = hidpp-app;
          dmg = hidpp-dmg;
        };

        checks = {
          inherit hidpp-daemon hidpp-cli;
          clippy = craneLib.cargoClippy (commonArgs // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "-- -D warnings";
          });
          tests = craneLib.cargoTest (commonArgs // {
            inherit cargoArtifacts;
            # Only run unit tests — integration tests need real HID hardware.
            cargoTestExtraArgs = "--lib";
          });
        };

        devShells.default = craneLib.devShell {
          checks = self.checks.${system};
          packages = [
            # RE tools
            rizinWithPlugins
            pkgs.radare2
            python
            pkgs.file
            pkgs.hexyl
            pkgs.jq
            pkgs.binwalk

            # Protobuf
            pkgs.protobuf

            # WASM (dev only — not built by crane)
            pkgs.wasm-pack
            pkgs.wasm-bindgen-cli
            pkgs.lld

            # Native HID
            pkgs.hidapi
            pkgs.pkg-config

            # Frontend
            pkgs.bun
          ];

          shellHook = ''
            echo "logi-re workbench"
            echo ""
            echo "RE:    rizin <binary>  |  r2 <binary>"
            echo "Rust:  cargo build  |  cargo test  |  wasm-pack build crates/hidpp-web"
            echo "Nix:   nix build .#app  |  nix build .#daemon  |  nix build .#cli"
            echo ""
          '';
        };
      });
}
