{
  # The `setting` facet for blackbox: the toolchain, and the `bbx` command
  # itself. Measured across the fleet at 16-24% of a repo and almost never
  # what you are editing -- which is why §C says it enters a lens pack as a
  # CONTRACT (one line per guard) rather than as this file.
  #
  # The nixpkgs rev is FOLLOWED, not spelled. A literal rev here is a fourth
  # opinion about the fleet's compiler that nobody refreshes -- and it was
  # already wrong: this file pinned 241313f4 (rustc 1.96.1) while the fleet
  # standard was nixos-25.11. nixpkgs-lock is the one place that rev is
  # decided, for ~80 repos, and it moved to nixos-26.05 (rustc 1.95.0) in
  # pr0d1r2/nixpkgs-lock#19.
  description = "blackbox -- federated SPEC.md for small-context local models";

  # hk is built by `nix-hk` and pushed to this cache. nixos-26.05 ships no hk
  # at all -- the package landed on nixpkgs master after the branch-off -- so
  # without the substituter every entry into this shell BUILDS hk from source.
  # Declared here so the cache travels with the flake; a user outside
  # `trusted-users` still gets a silent source build, and only a warning.
  nixConfig = {
    extra-substituters = [ "https://pr0d1r2.cachix.org" ];
    extra-trusted-public-keys = [
      "pr0d1r2.cachix.org-1:NfWjbhgAj41byXhCKiaE+av3Vnphm1fTezHXEGsiQIM="
    ];
  };

  # THREE declared inputs, ONE nixpkgs. `nix-hk` follows the same lock rather
  # than its own copy: a second nixpkgs edge would fork the rev, the cached hk
  # would be built against a nixpkgs this shell does not have, and every
  # substitution would miss while looking exactly like success.
  inputs = {
    nixpkgs-lock.url = "github:pr0d1r2/nixpkgs-lock";
    nixpkgs.follows = "nixpkgs-lock/nixpkgs";
    nix-hk.url = "github:pr0d1r2/nix-hk";
    nix-hk.inputs.nixpkgs-lock.follows = "nixpkgs-lock";
  };

  outputs =
    {
      self,
      nixpkgs,
      nix-hk,
      ...
    }:
    let
      systems = [
        "aarch64-darwin"
        "x86_64-darwin"
        "aarch64-linux"
        "x86_64-linux"
      ];
      # The overlay is what makes `pkgs.hk` below mean nix-hk's hk. Applied as
      # an OVERLAY rather than referenced as `nix-hk.packages.${s}.hk` so there
      # is exactly one `pkgs` in this file: a second lookup path is a second
      # place to forget, and the package list stays a list of names.
      forAll =
        f: nixpkgs.lib.genAttrs systems (s: f (nixpkgs.legacyPackages.${s}.extend nix-hk.overlays.default));

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
      # `packages.default`, finally, and with `doCheck` ON -- which is the
      # whole point of having it. Two things had to land first: T71 removed
      # the path dep, so a pure build can see every source it needs, and
      # `src/cli:T9` handed the two ambient tests a fixture repository, so
      # the suite no longer asserts facts about wherever the runner sits.
      #
      # Shipping this earlier with `doCheck = false` would have hidden
      # exactly the defect the sandbox is worth having: `src/cli:B1` was
      # invisible for the project's life BECAUSE nothing ever ran the suite
      # outside a checkout. A package that skips its own tests is a build,
      # not a check.
      packages = forAll (pkgs: {
        default = pkgs.rustPlatform.buildRustPackage {
          pname = "bbx-cli";
          version = "0.1.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          # `git` is a test dependency, not a runtime one: `testrepo` builds a
          # real repository with `git init` because the verbs under test read
          # real history. The sandbox has no git unless it is asked for.
          nativeCheckInputs = [ pkgs.git ];
          # BUILD only the published crate, TEST the whole workspace. `dev/`
          # ships to nobody -- that is the reason it is a separate member --
          # so `bbx-dev` must not land in this package's `bin/`, which it did
          # on the first build here. Its tests still run in the sandbox,
          # because a member no sandbox ever exercises is a member whose
          # breakage waits for a human.
          cargoBuildFlags = [
            "-p"
            "bbx-cli"
          ];
          cargoTestFlags = [ "--workspace" ];
          meta = {
            description = "two-axis federation of SPEC.md and source over a dir DAG";
            mainProgram = "bbx";
            licenses = pkgs.lib.licenses.mit;
          };
        };
      });

      devShells = forAll (pkgs: {
        default = pkgs.mkShell {
          packages = [
            (bbxShim pkgs)
            pkgs.rustc
            pkgs.cargo
            pkgs.clippy
            pkgs.rustfmt
            pkgs.git
            # The gate runner: `hk.pkl` holds the ops, hk decides when they run.
            # From nix-hk via the overlay, because nixos-26.05 has no `hk`.
            pkgs.hk
            # Coverage. Here to MEASURE before any floor is chosen: picking a
            # threshold before knowing the number is what `.context-limits`
            # warns against, and adopting a sibling's 98% because it is the
            # sibling's is how a number arrives unreviewed.
            pkgs.cargo-llvm-cov
            # `llvm-cov` and `llvm-profdata` themselves: nixpkgs' rustc does not
            # ship `llvm-tools-preview`, so they come from the LLVM package and
            # are wired through the env vars `cargo-llvm-cov` looks for. Both
            # siblings solve it exactly this way.
            pkgs.llvmPackages.llvm
            # The linters the `hk util` family cannot cover, because each
            # needs a real parser rather than a byte scan. Dev-time only:
            # none is a dependency of the crate, and CI reaches all of them
            # through this shell rather than installing its own copies, so
            # a laptop and a runner cannot disagree about a version.
            #
            # `actionlint` earned its place the day `ci.yml` landed -- a
            # workflow is a file nothing else here parses, and its failure
            # mode is a run that never starts.
            pkgs.actionlint
            # `.envrc` is shell without a `.sh` name, and it runs on every
            # shell entry, which is where a silent mistake costs most.
            pkgs.shellcheck
            # The flake decides what every other step runs with, so drift
            # here is drift everywhere.
            pkgs.nixfmt
            pkgs.taplo
            pkgs.typos
            # Relative links only (`--offline`). A link breaks when its
            # TARGET moves, and the target's referrer is usually not in the
            # changed set -- so this is the one check that must see files
            # nobody staged.
            pkgs.lychee
          ];

          # `cargo-llvm-cov` looks these up by name and gives up if they are
          # absent, which is what "failed to find llvm-tools-preview" means on
          # a nixpkgs toolchain.
          LLVM_COV = "${pkgs.llvmPackages.llvm}/bin/llvm-cov";
          LLVM_PROFDATA = "${pkgs.llvmPackages.llvm}/bin/llvm-profdata";

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
