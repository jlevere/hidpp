{
  description = "Logitech Options+ RE workbench — HID++ protocol analysis";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          config = {
            allowUnfree = true;
            allowBroken = true;
          };
        };

        python = pkgs.python313.withPackages (ps: [
          ps.r2pipe
          ps.rzpipe
          ps.capstone
          ps.unicorn
          ps.keystone-engine
          ps.protobuf
          ps.construct       # binary struct parsing
        ]);

        rizinWithPlugins = pkgs.rizin.withPlugins (ps: [
          ps.rz-ghidra
          ps.jsdec
          ps.sigdb
        ]);

      in {
        devShells.default = pkgs.mkShell {
          name = "logi-re";

          packages = [
            # Binary analysis
            rizinWithPlugins
            pkgs.radare2

            # Python scripting
            python

            # Protobuf (Logi uses protobuf internally)
            pkgs.protobuf

            # Utilities
            pkgs.file
            pkgs.hexyl
            pkgs.jq
            pkgs.binwalk

            # Rust toolchain
            pkgs.rustc
            pkgs.cargo
            pkgs.rust-analyzer
            pkgs.clippy
            pkgs.rustfmt
            pkgs.wasm-pack
            pkgs.wasm-bindgen-cli

            # Native HID access
            pkgs.hidapi
            pkgs.pkg-config

            # WASM linker
            pkgs.lld

            # Frontend
            pkgs.bun
          ];

          shellHook = ''
            echo "logi-re workbench"
            echo ""
            echo "RE:    rizin <binary>  |  r2 <binary>"
            echo "Rust:  cargo build  |  cargo test  |  wasm-pack build crates/hidpp-web"
            echo ""
          '';
        };
      });
}
