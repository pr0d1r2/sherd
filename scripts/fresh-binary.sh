#!/usr/bin/env bash
# The body of the `fresh-binary` step in hk.pkl: `scripts/fresh-binary.sh [target-dir]`.
#
# Refuses when this repository's binaries were built from `cargo package`'s
# unpacked copy instead of this tree (`.:V125`, `.:B34`). Cargo decides
# freshness from the mtimes of the files a binary's dep-info lists. A build
# from `target/package/sherd-<ver>/src` lists files that never change, so
# `cargo run` calls that binary fresh forever, and every gate step that runs
# it judges code that is not in the tree.
#
# Reads only the dep-info; it builds nothing. A binary not built yet has no
# dep-info, and the next `cargo run` builds it from this tree.
set -u

target=${1:-${CARGO_TARGET_DIR:-target}}
stale=()
examined=0
for bin in sherd sherd-dev; do
  dep="$target/debug/$bin.d"
  [ -f "$dep" ] || continue
  examined=$((examined + 1))
  if grep -Eq '/package/sherd(-dev)?-[0-9][^/]*/' "$dep"; then
    stale+=("$dep")
  fi
done

if [ "${#stale[@]}" -gt 0 ]; then
  for dep in "${stale[@]}"; do
    src=$(grep -Eo '[^ ]*/package/sherd(-dev)?-[0-9][^/]*/' "$dep" | head -1)
    echo "hk: $dep was built from $src, not from this tree." >&2
  done
  echo "hk: cargo calls that binary fresh forever, so every step that runs it would judge OLD code (.:B34). Run 'cargo clean -p sherd -p sherd-dev' and commit again." >&2
  exit 1
fi
echo "fresh-binary: $examined dep-info file(s) examined, none built from target/package"
