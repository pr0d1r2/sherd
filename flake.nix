{
  # The `setting` facet for blackbox: the toolchain, and the `bbx` command
  # itself. Measured across the fleet at 16-24% of a repo and almost never
  # what you are editing -- which is why §C says it enters a lens pack as a
  # CONTRACT (one line per guard) rather than as this file.
  #
  # Same nixpkgs pin as ../itok and ../cavespec. Three sibling crates polished
  # together should not disagree about their compiler.
  description = "blackbox -- federated SPEC.md for small-context local models";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/241313f4e8e508cb9b13278c2b0fa25b9ca27163";

  outputs =
    { self, nixpkgs }:
    let
      systems = [
        "aarch64-darwin"
        "x86_64-darwin"
        "aarch64-linux"
        "x86_64-linux"
      ];
      forAll = f: nixpkgs.lib.genAttrs systems (s: f nixpkgs.legacyPackages.${s});

      # blackbox dogfoods itself (V27) -- `bbx check` gates its own commits --
      # so the dev shell must PROVIDE `bbx`, not merely the toolchain to build
      # it.
      #
      # Deliberately a SHIM and not a package in the shell's closure, for the
      # reason itok's flake gives: a package would have to build the crate in
      # order to ENTER the shell, so a compile error would lock you out of the
      # shell you need to fix it. `cargo run` is a no-op when the build is
      # fresh, rebuilds only on change, and therefore always matches the
      # working tree -- which a pinned package never does.
      #
      # BBX_MANIFEST is exported by the shellHook. BBX_PROFILE=release is the
      # escape hatch when a debug build is too slow over a large tree.
      bbxShim =
        pkgs:
        pkgs.writeShellScriptBin "bbx" ''
          set -eu
          manifest="''${BBX_MANIFEST:-}"
          if [ -z "$manifest" ] || [ ! -f "$manifest" ]; then
            echo "bbx(shim): BBX_MANIFEST unset or missing -- re-enter the dev shell from the crate root" >&2
            exit 2
          fi
          profile=""
          [ "''${BBX_PROFILE:-debug}" = "release" ] && profile="--release"
          exec cargo run --quiet $profile \
            --manifest-path "$manifest" --bin bbx -- "$@"
        '';
    in
    {
      # No `packages.default`. blackbox depends on ../itok and ../cavespec by
      # PATH, and a path dep outside the flake root is not visible to a pure
      # build -- the same reason `git worktree` could not resolve ../itok, and
      # the same fragility a sibling rename exposed on 2026-08-01. It becomes
      # buildable when those two are flake inputs pinned to public revs, which
      # is the fleet packaging work (§R R20), not a local workaround.

      devShells = forAll (pkgs: {
        default = pkgs.mkShell {
          packages = [
            (bbxShim pkgs)
            pkgs.rustc
            pkgs.cargo
            pkgs.clippy
            pkgs.rustfmt
            pkgs.git
          ];

          # Derived, never hardcoded: direnv enters with PWD = the directory
          # holding this flake, which is also the crate root.
          shellHook = ''
            if [ -f "$PWD/Cargo.toml" ]; then
              export BBX_MANIFEST="$PWD/Cargo.toml"
              export BBX_CARGO="$(command -v cargo)"
            else
              echo "bbx(shell): no Cargo.toml in $PWD -- \`bbx\` shim disabled" >&2
            fi
            # The endpoint is a fact about YOUR network, so it is not pinned
            # here. §I defaults to localhost; export BBX_ENDPOINT to point at
            # a LAN box. BBX_MODEL defaults to gpt-oss:20b.
            : "''${BBX_ENDPOINT:=http://localhost:11434}"
            export BBX_ENDPOINT
          '';
        };
      });
    };
}
